//! # pdfce-gui — native desktop shell
//!
//! The egui/eframe application shell (docs/ARCHITECTURE.md §2, §3): a
//! single native process, no web server, no browser runtime, no network
//! listener. This is one of two front ends over the shared `pdfce-core`
//! engine — the other being `pdfce-cli` — and the reason the
//! GUI-core-separation invariant exists: everything GUI (eframe, winit,
//! wgpu/glow, rfd) lives here, never in `pdfce-core`/`pdfce-render`.
//!
//! eframe targets native (winit + glow/wgpu) and WASM/canvas from one
//! codebase; keeping all windowing here is what lets the future web fork
//! (docs/ARCHITECTURE.md §1) swap this crate for a WASM shell without
//! touching the engine.
//!
//! Privacy posture (docs/ARCHITECTURE.md §1.1): this binary makes no
//! network calls of any kind. The only file it touches is the one the
//! user explicitly picks in the Open dialog, and it is opened
//! **read-only** — Pass 1 is a viewer; nothing here writes a PDF.
//!
//! ## Pass 1 scope: a read-only page viewer
//!
//! Open a PDF, see its pages. Concretely:
//!
//! - **Open** — [`pdfce_core::document::Document::load`] then
//!   [`pdfce_core::page_tree::pages`], with three distinct outcomes
//!   presented three distinct ways (see "Three ways to fail" below).
//! - **Canvas** — the current page rasterized by `pdfce-render`,
//!   uploaded as a texture, scrollable/pannable, zoomable.
//! - **Thumbnail rail** — lazily rasterized page thumbnails, click to
//!   jump.
//! - **Status bar** — the R20 render-diagnostics disclosure.
//!
//! ## Pass 3.1 addition: the first editing capability, and undo
//!
//! Two edits — document properties (`/Info`, §14.3.3) and page rotation
//! (`/Rotate`, Table 30) — plus **Save a copy…**, Undo and Redo.
//!
//! `ARCHITECTURE.md` §11.4 binds here: *"the first Pass that introduces
//! any editing capability must build the command-log/undo-stack
//! mechanism as part of that Pass"*. So every mutation in this crate
//! goes through one [`EditSession`], and there is deliberately **no**
//! code path from a widget to a `Document`. `pdfce-cli` drives the same
//! type, which is what makes "the two front ends cannot diverge" a
//! structural property rather than a convention.
//!
//! Two UI rules the editing surface obeys, both of them standing rules
//! rather than choices made here:
//!
//! - **Non-destructive by default.** Save opens a *save-as* dialog and
//!   writes an incremental update (§7.5.6) to whatever path the operator
//!   picks. The opened file is never written to unless the operator
//!   names it, and even then the previous bytes are preserved beneath
//!   the appended revision. There is no silent in-place overwrite and no
//!   autosave.
//! - **Fuzzy, never sneaky.** Typing in a properties field is not an
//!   edit; pressing **Apply** is. That is what keeps one Undo step equal
//!   to one intended change, and it means the operator can abandon a
//!   half-typed field by closing the panel.
//!
//! Still absent: annotations, text editing, structural page operations.
//!
//! ⚠️ The renderer reads the **base** document while edits are pending,
//! which is correct only because no Pass 3.1 edit can touch a content
//! stream, a resource or the page tree's shape — the one
//! rendering-relevant value an edit changes is `/Rotate`, and that
//! travels in the [`Page`] values `EditSession::pages` hands back. The
//! first Pass whose edits reach a content stream must give the renderer
//! an overlay-aware view of the document instead.
//!
//! ## Module layout, and why it is split this way
//!
//! | module | responsibility | testable headlessly |
//! |---|---|---|
//! | `main` (this file) | widgets, panels, input, frame orchestration | no |
//! | [`viewer`] | page index, zoom ladder, fit math, raster ceiling | **yes** |
//! | [`raster`] | pixmap → texture, thumbnail cache | no (needs a `Context`) |
//! | [`ui_text`] | every user-facing string (decision 002 R1) | n/a |
//!
//! The split is driven entirely by testability. A windowed UI cannot run
//! on a CI runner, so every piece of *logic* that could be wrong in a
//! way a human would notice — off-by-one page stepping, a fit scale that
//! overflows an axis, a zoom that blows the rasterizer's allocation
//! guard — is pushed into [`viewer`] where it is a pure function with a
//! unit test. What is left in this file is wiring: read input, mutate
//! state, draw widgets. Wiring can be reviewed; arithmetic needs tests.
//!
//! ## Panel order is load-bearing (layout *and* focus order)
//!
//! egui resolves panels in the order they are added, and that order
//! determines both the rectangles they get and the order the Tab key
//! visits their widgets. This frame adds:
//!
//! 1. `toolbar` (top) — full window width.
//! 2. `status` (bottom) — full window width.
//! 3. `thumbnails` (left) — the remaining height between them.
//! 4. `CentralPanel` — whatever is left.
//!
//! The UI review recommended toolbar → rail → canvas → status bar so Tab
//! reaches the canvas before the footer. That order was **not** taken,
//! deliberately: adding the bottom panel after the left panel would make
//! the status bar start at the rail's right edge rather than span the
//! window, and a status bar that does not span the window is not a
//! status bar.
//!
//! **Resolved by Pass 12.0 (the canvas-interaction substrate), not by any
//! single drawing feature.** From Pass 1 this paragraph carried a caveat —
//! *"revisit when the canvas gains focusable content"* — because the canvas
//! was an inert image with no focusable content and the panel-order-vs-Tab
//! trade therefore had zero cost. Pass 12.0 is what makes the canvas
//! focusable (its page image now carries a real click sense and requests
//! focus on click, `docs/ui_specs/pass-12.0-canvas-substrate.md` §1), so
//! the caveat's condition is now false. The trade was re-evaluated and the
//! **existing panel-add order was kept deliberately**: reordering purely
//! for Tab polish would regress the real, permanent layout property (the
//! status bar must be added before any side panel to span the full window)
//! to fix a cosmetic one. What changed is narrower and sufficient — the
//! canvas is now a genuine, reachable Tab stop at the END of the existing
//! chain (Tab from the Tools dock's last widget, or the rail's last
//! thumbnail when the dock is closed, now lands on the canvas, which shows
//! egui's default focus rectangle, rather than skipping to wrap-around).
//! The substrate — not any one tool — had to exist before the caveat could
//! be closed, which is why this is a Pass-12.0 resolution and not a Pass-6.1
//! one.
//!
//! The toolbar is built as `ui.separator()`-divided *groups*
//! (file | navigation | zoom | view) rather than one flat row, so the
//! Passes that add Edit/Comment/Sign clusters insert a group instead of
//! rewriting the row. The window's right side is deliberately left
//! unclaimed for the future contextual/tool panel; nothing in the
//! canvas or rail sizing assumes the left rail is the only side panel
//! that will ever exist.
//!
//! ## Three ways to fail, three ways to say so
//!
//! [`Status`] distinguishes what most viewers conflate:
//!
//! - [`Status::Failed`] — the *file* is wrong: damaged, truncated, not a
//!   PDF. "Something is wrong with your document."
//! - [`Status::Unsupported`] — the file is fine and **pdfce** is not
//!   finished. `pdfce-core` detects such a file and refuses it cleanly
//!   rather than misparsing it; today the live case is an **encrypted**
//!   document (ISO 32000-1 §7.6), which pdfce has no security handler
//!   for and which would otherwise decode to plausible-looking garbage.
//!   Presenting that as "failed to open" would tell the operator a lie
//!   about their own file. The branch is made on structured error data
//!   ([`XrefErrorKind`]), not by matching on a message string — which is
//!   exactly what decision 002 R4's "core errors are stable, structured
//!   diagnostics" is *for*. (Cross-reference streams §7.5.8, object
//!   streams §7.5.7 and hybrid-reference files §7.5.8.4 used to route
//!   here; they are now supported and open normally.)
//! - [`Status::Open`] with a per-page render error — the document
//!   loaded, this one page did not draw. The document stays open.
//!
//! ## Rendering happens on state change, never per frame
//!
//! egui redraws continuously; rasterizing a PDF page at 60 Hz would be
//! absurd. The canvas holds one cached [`raster::PageTexture`] and
//! re-rasterizes only when the cached page index or raster scale no
//! longer matches the view state. Two different staleness policies apply:
//!
//! - **Page change** — commit immediately. There is no stale texture
//!   worth showing (it is a picture of a different page), so any delay
//!   is pure latency.
//! - **Zoom change** — debounce by [`ZOOM_SETTLE`], drawing the existing
//!   texture scaled to the new size in the meantime. A ctrl+scroll
//!   gesture emits dozens of zoom values on the way to the one the
//!   operator wants; rasterizing each would burn CPU producing images
//!   nobody sees. The interim scaled texture is soft, not blank or
//!   blocky — which is exactly what every other document viewer does, so
//!   it reads as normal rather than as a glitch. A discrete command
//!   (a zoom button, Ctrl+0) bypasses the debounce: there is no gesture
//!   in flight, so waiting would just feel unresponsive.
//!
//! ## Input conventions
//!
//! - Plain wheel scrolls/pans the canvas; **Ctrl**+wheel zooms. egui
//!   routes these apart at the input-state level (a wheel event carrying
//!   the zoom modifier becomes `zoom_delta` and contributes *nothing* to
//!   `smooth_scroll_delta`), so the scroll area cannot pan and zoom off
//!   the same gesture. Breaking this convention is the single most
//!   common way a from-scratch viewer feels wrong.
//! - Drag pans, via the scroll area's own drag-to-scroll. Panning
//!   triggers **no** re-raster at all: it moves the viewport over an
//!   existing texture.
//! - PageUp/PageDown step pages; Home/End jump to the first/last.
//! - Ctrl+Plus / Ctrl+Minus / Ctrl+0 are page zoom, matching browsers
//!   and every PDF reader. This requires switching off egui's
//!   `zoom_with_keyboard`, which would otherwise consume those chords to
//!   scale the whole UI — see [`configure_context`].
//!
//! ## Accessibility status — stated honestly, not implied
//!
//! Keyboard navigation is real: every action has a shortcut, every
//! icon-only control has a tooltip naming it, and Tab order follows
//! panel order. Icon-only controls also carry an explicit accessible
//! name (P1-6): egui derives a widget's accessible name from its visible
//! label, which for a glyph button is a bare character, so
//! [`PdfceApp::icon_button`] (and the annotation toggle) publish their
//! tooltip text as the accessible name via `Response::widget_info`, so a
//! screen reader announces "Rotate page clockwise" rather than "↻".
//! Screen-reader support is still a *weaker* claim than that: eframe is
//! built here with the `accesskit` feature, so egui does publish an
//! accessibility tree, but **pdfce has not been tested with a screen
//! reader**, and the page canvas is an image with no text alternative —
//! a rasterized page conveys nothing to an assistive technology. Real
//! accessible document reading needs the tagged-content extraction work
//! that PDF/UA support will build (docs/ROADMAP.md), and until that
//! exists this viewer should not be described as accessible.

// On Windows, prevent a console window from popping up behind the GUI in
// release builds (the process is a GUI app, not a console app). Debug
// builds keep the console so `eprintln!`/panics remain visible while
// developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod canvas;
mod raster;
mod ui_text;
mod viewer;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use pdfce_core::document::{DocError, Document};
use pdfce_core::edit::{EditSession, InfoField};
use pdfce_core::page_tree::Page;
use pdfce_core::signature::{SaveMode, SignatureImpact};
use pdfce_core::writer::SaveOptions;
use pdfce_core::xref::XrefErrorKind;

use canvas::{
    CanvasTargetProvider, CanvasTool, EmptyTargetProvider, EscapeOutcome, GestureInterrupt,
    TargetId,
};
use raster::{PageTexture, ThumbnailCache};
use viewer::{FitMode, ViewState};

/// Initial window size, in logical points.
const INITIAL_WINDOW_SIZE: [f32; 2] = [1100.0, 800.0];

/// How long a continuous zoom gesture must be idle before the view
/// commits to a real re-rasterization. See the module docs.
///
/// Chosen at the short end of the "feels instant" range: long enough to
/// swallow a wheel gesture's intermediate values, short enough that the
/// soft interim image is never on screen long enough to look like the
/// final result.
const ZOOM_SETTLE: Duration = Duration::from_millis(150);

/// How many thumbnails may be rasterized in a single frame.
///
/// A budget rather than "all visible ones" because the rail can show a
/// dozen at once and rasterizing a dozen pages inline would drop frames
/// on the very gesture (scrolling the rail) where smoothness is most
/// noticeable. Whatever is left over is picked up next frame, which the
/// code requests explicitly rather than waiting for some other event to
/// trigger a repaint.
const THUMBNAILS_PER_FRAME: usize = 2;

/// Padding, in points, left around the page inside the canvas so the
/// page does not sit flush against the panel edges under a fit mode.
const CANVAS_MARGIN: f32 = 16.0;

/// Minimum size for an icon-only button, so click targets stay usable
/// regardless of how narrow the glyph inside them happens to be.
const ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(28.0, 24.0);

/// Maximum height, in points, the status bar's body may occupy before it
/// becomes internally scrollable (P0-4).
///
/// Every line the status bar emits is a standing-rule-mandated disclosure
/// that must never be suppressed, but a page that trips several at once
/// (a delete's dangling-reference note, a copy result, the redaction
/// warning, an expanded diagnostics block, the annotation census) can
/// legitimately stack many lines and crowd the canvas. Capping the body
/// and letting it scroll keeps every disclosure visible without letting
/// the footer eat the page. ~8 lines before it scrolls.
const STATUS_BAR_MAX_HEIGHT: f32 = 220.0;

/// Launch the shell, optionally opening a file named on the command line.
///
/// ## The one argument, and why it is not a `clap` surface
///
/// `pdfce <file.pdf>` opens that file. That is the whole command-line
/// contract, and it is deliberately parsed by hand rather than through
/// `clap`: this binary is what a desktop double-click, a "Open with…"
/// menu and a drag-onto-the-icon all invoke, and every one of those
/// hands over exactly one path and nothing else. A flag surface here
/// would be a second, worse `pdfce-cli` — the scriptable interface is
/// that binary's job, and duplicating a slice of it in the GUI is how
/// the two start disagreeing.
///
/// An argument that is not a readable PDF takes the ordinary failure
/// path: the window opens and says why, rather than the process exiting
/// before the operator sees anything. A GUI that dies silently on a bad
/// double-click is indistinguishable from one that did not launch.
fn main() -> eframe::Result {
    let initial = std::env::args_os().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("pdfce")
            .with_inner_size(INITIAL_WINDOW_SIZE)
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "pdfce",
        native_options,
        Box::new(move |cc| {
            configure_context(&cc.egui_ctx);
            let mut app = PdfceApp::default();
            if let Some(path) = initial {
                app.open_path(path);
            }
            Ok(Box::new(app))
        }),
    )
}

/// One-time egui configuration that must happen before the first frame.
///
/// Only one setting so far, and it is not optional: egui's
/// `zoom_with_keyboard` makes Ctrl+Plus/Minus/0 rescale the entire user
/// interface. In a document viewer those chords mean *page* zoom —
/// that is what they do in every browser, in Acrobat, and in every other
/// PDF reader — so egui's handler is switched off and this crate handles
/// them. Without this, the chords would silently do the wrong thing and
/// the toolbar's own tooltips would be advertising a lie.
///
/// Note that ctrl+**scroll** is unaffected: egui converts that to a
/// `zoom_delta` in the input state but does not act on it itself, so the
/// canvas is free to interpret it.
fn configure_context(ctx: &egui::Context) {
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// The whole application state.
struct PdfceApp {
    /// What, if anything, is open.
    status: Status,
    /// Whether the thumbnail rail is showing.
    ///
    /// Session state only — deliberately not persisted to disk. Real UI
    /// preference persistence is a considered feature with its own
    /// storage format and migration story; growing it one ad-hoc field
    /// at a time is how a settings file becomes unmaintainable.
    rail_expanded: bool,
    /// Whether the status bar's diagnostics detail is expanded.
    diagnostics_expanded: bool,
    /// Whether the document-properties panel is showing.
    properties_open: bool,
    /// The outcome of the most recent save, kept until the next one so
    /// the operator gets a persistent answer rather than a toast they
    /// might miss.
    save_result: Option<SaveOutcome>,
    /// Whether the right-hand Tools dock is showing.
    ///
    /// The one new toolbar control Pass 3.2 adds. Session state only, for
    /// the same reason `rail_expanded` is: growing a settings file one
    /// ad-hoc field at a time is how it becomes unmaintainable.
    tools_open: bool,
    /// Which Tools-dock entry is expanded, if any.
    tools_selected: Option<Tool>,
    /// The Combine tool's ordered input list.
    merge_inputs: Vec<PathBuf>,
    /// Whether Combine generates one bookmark per source. On by default,
    /// matching Acrobat's documented Combine-Files default.
    merge_bookmarks: bool,
    /// Operator-supplied font folders (decision 012). Each is walked for
    /// font files that are registered into [`Self::font_env`] and drawn
    /// for the open document's NON-embedded fonts.
    ///
    /// **Session state only**, deliberately not persisted to disk — same
    /// stance as [`Self::rail_expanded`]/[`Self::tools_open`]. Persistence
    /// belongs to the R15 user-state partition, which does not yet exist;
    /// growing a settings file one ad-hoc field at a time is exactly the
    /// trap those fields' comments warn against. The UI states the
    /// session-only scope explicitly rather than letting it pass silently.
    font_folders: Vec<PathBuf>,
    /// The [`pdfce_render::FontEnvironment`] built from [`Self::font_folders`],
    /// cached and rebuilt only when [`Self::font_env_generation`] changes
    /// (not walked every frame). With no folders this is exactly
    /// [`pdfce_render::FontEnvironment::bundled`] — the deterministic
    /// default (R19/R63).
    font_env: pdfce_render::FontEnvironment,
    /// Monotonic generation of [`Self::font_env`], bumped on any
    /// folder-list mutation. Doubles as the [`raster::PageTexture`]
    /// staleness key so adding/removing a folder re-renders the current
    /// page instead of silently doing nothing.
    font_env_generation: u64,
    /// Human-readable notes from the most recent font-folder walk
    /// (registered faces, skipped files) — surfaced in the Font-folders
    /// tool so a mismatch is debuggable from the UI (fuzzy-never-sneaky).
    font_notes: Vec<String>,
    /// The operator's DEFAULT font for the Add-Page-Text tool (Pass 16.2 §5.1,
    /// decision 016 §3.3). A preference set on the Font-folders panel, seeded
    /// into each new `AddTextState.prop_font` on tool entry, then overridable
    /// per-use without touching this. Bundled Helvetica by default (the R79
    /// bundled Standard-14). **Session state only**, the same stance as
    /// [`Self::font_folders`] — persistence belongs to the not-yet-built R15
    /// user-state partition, not an ad-hoc field.
    default_add_text_font: pdfce_core::fontdata::Std14,
    /// A save that is waiting on the operator's answer to the
    /// signature-invalidation question.
    ///
    /// `Some` means the confirmation is on screen and **nothing has been
    /// written**. The native file dialog has not even opened yet: per the
    /// Pass 3.2 UI spec the question is asked *before* it, so an operator
    /// who is going to back out is not made to pick a destination first.
    pending_save: Option<PendingSave>,
    /// The outcome of the most recent Copy-text, kept until the next one.
    ///
    /// Deliberately **not** folded into the render-diagnostics
    /// `CollapsingHeader` in the status bar, on the UI review's
    /// reasoning: render diagnostics are re-read from the current page's
    /// cached texture every frame and are therefore always about what is
    /// on screen, while this is a snapshot of an action the operator
    /// triggered — possibly against a different page, possibly against
    /// the whole document. Merging them would make the merged header
    /// start lying the instant the operator navigated after copying.
    /// This belongs with `save_result`/`edit_note`: the
    /// "did my last requested action work, and what should I know about
    /// it" family, which persists until superseded.
    copy_result: Option<CopyTextOutcome>,
    /// Whether the copy-result detail is expanded.
    copy_detail_expanded: bool,
    /// A copy waiting on the operator's answer to the
    /// mostly-unreadable question.
    ///
    /// `Some` means the confirmation is on screen and **nothing has been
    /// written to the clipboard** — the same before-not-after posture as
    /// `pending_save`, and for a sharper reason: a clipboard write is
    /// destructive to whatever the operator had copied previously, so an
    /// operator who backs out must not have already lost it.
    pending_copy: Option<PendingCopy>,
    /// The narrator line describing the most recent edit — a delete's
    /// dangling-reference disclosure, a reorder's page count.
    ///
    /// Same channel and style as `save_result`, deliberately: the UI spec
    /// asks for the disclosure in the existing status-bar channel rather
    /// than a new surface, and a second notification style is a second
    /// thing for the operator to learn.
    edit_note: Option<String>,
    /// A non-blocking disclosure shown when the open document was loaded via
    /// **cross-reference recovery** (decision 013): its stored xref could
    /// not be parsed and pdfce rebuilt it by scanning. Surfaced in the
    /// existing status-bar narrator channel (same rationale as `edit_note`),
    /// because a recovered document's save is a forced full rewrite and its
    /// cross-reference table was reconstructed, not read as authored —
    /// facts the operator should see (fuzzy-never-sneaky, R20). Exact
    /// surface/wording is a `pdfce-ui-specialist` follow-up; this is the
    /// honest minimal banner. `None` for a cleanly-loaded file.
    recovery_note: Option<String>,
    /// The "current pen" colour for authored markup (Pass 6.1). Session
    /// state, exactly like `rail_expanded` — changing it is not an edit
    /// (`docs/ui_specs/pass-6.1-markup-tools.md` §1.1), only *using* it to
    /// author a shape is. Default red, matching the CLI's per-subtype
    /// default for non-highlight markup.
    markup_color: egui::Color32,
    /// The current pen width in points for stroke-based markup.
    markup_width: f32,
    /// The text-entry popup's target subtype, when open (Pass 6.2). `None`
    /// means the popup is closed. A minimal affordance — the operator types
    /// the text, then it is authored at the page centre through the same
    /// `EditSession::add_text_annotation` path the CLI uses.
    pending_text_kind: Option<GuiTextKind>,
    /// The text-entry popup's buffer.
    text_input: String,
    /// Whether the keyboard-shortcuts reference window is showing (P1-2).
    /// Session state, same progressive-disclosure default as
    /// `properties_open`.
    shortcuts_open: bool,
    /// The window/taskbar title last pushed to the platform layer (P0-3).
    ///
    /// The wanted title is recomputed from `status` every frame, but is
    /// only sent via [`egui::ViewportCommand::Title`] when it differs from
    /// this — pushing an unchanged title to the OS on every one of egui's
    /// continuous repaints would be needless platform-layer churn.
    last_window_title: String,
}

/// A tool in the right-hand dock.
///
/// The dock is pdfce's one "more tools" secondary surface. Future
/// advanced buckets — Bates stamping, OCR, redaction, forms, PDF/A
/// conversion — become entries here rather than each earning a floating
/// window or a toolbar group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    /// Combine several PDFs into a new one.
    Merge,
    /// Split this document into several files.
    Split,
    /// Insert pages from another file.
    Insert,
    /// Manage operator-supplied font folders (decision 012). Unlike the
    /// other dock entries (which act on files the operator has not
    /// opened), this is a standing preference that changes how the
    /// CURRENTLY-open document renders its non-embedded fonts.
    FontFolders,
}

/// A save the operator has been asked to confirm.
///
/// Carries the verdict rather than recomputing it at confirm time: the
/// question the operator answered was about *this* verdict, and
/// recomputing could — after some unrelated frame — answer a different
/// one.
#[derive(Debug, Clone, Copy)]
struct PendingSave {
    /// The verdict the operator is being asked about.
    ///
    /// Stored rather than recomputed at confirm time: the question the
    /// operator answered was about *this* verdict, and a recomputation
    /// after some unrelated frame could answer a different one.
    impact: SignatureImpact,
}

/// Which pages a Copy-text acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyScope {
    /// The page currently on screen.
    Page,
    /// Every page in the document.
    Document,
}

/// A copy the operator has been asked to confirm.
///
/// Carries the extracted text and its counters rather than re-extracting
/// on confirm: the question the operator answered was about *this*
/// result, and a second extraction — after some unrelated frame, against
/// a page the operator may have turned away from — could answer a
/// different one. Exactly the reasoning behind [`PendingSave`] storing
/// its verdict.
struct PendingCopy {
    /// The result the operator is being asked about, ready to deliver.
    outcome: CopyTextOutcome,
    /// The text that will reach the clipboard on confirm.
    text: String,
}

/// What the last Copy-text produced, for the status bar.
///
/// Holds counts rather than the whole `TextDiagnostics` because the
/// status bar needs six numbers and keeping the rest would invite the
/// panel to grow into a second diagnostics surface.
struct CopyTextOutcome {
    /// What the copy covered, for the scope-naming summary line.
    scope: CopyScope,
    /// 1-based page number, for [`CopyScope::Page`].
    page_number: usize,
    /// Pages covered, for [`CopyScope::Document`].
    pages: usize,
    /// Characters placed on the clipboard.
    characters: usize,
    /// Character codes seen — the denominator for `failed`.
    codes: u64,
    /// Codes that reached §9.10.2's failure clause.
    failed: u64,
    /// Word spaces pdfce derived.
    spaces_derived: u64,
    /// Line breaks pdfce derived.
    lines_derived: u64,
    /// Fraction of codes resolved by a sourced ladder rung.
    sourced_fraction: f64,
    /// Pages that held no text at all (document scope only).
    pages_without_text: usize,
}

/// What the last save did, for the status bar.
///
/// Success carries the counters the operator can act on rather than a
/// bare "saved": how many objects the revision holds, and whether
/// anything was moved out of an object stream to make the edit possible
/// (R38). A save that quietly restructured part of the file has changed
/// something worth knowing about.
enum SaveOutcome {
    Saved {
        path: PathBuf,
        objects: usize,
        appended: usize,
        promoted: usize,
    },
    Failed(String),
}

impl Default for PdfceApp {
    /// Startup defaults.
    ///
    /// The rail starts **expanded**, which is not what a derived
    /// `Default` would give. Page navigation is core functionality, not
    /// an advanced tool, so progressive disclosure does not argue for
    /// hiding it — and a rail that is hidden until discovered means an
    /// operator's first impression of a 400-page document is a single
    /// page with no visible way to move through it. Showing one
    /// thumbnail for a one-page file costs nothing; hiding navigation
    /// costs discoverability.
    ///
    /// Diagnostics start **collapsed**: the one-line summary is always
    /// visible (that is the R20 obligation), and the detail is there for
    /// when the summary says there is something to read.
    fn default() -> Self {
        Self {
            status: Status::Idle,
            rail_expanded: true,
            diagnostics_expanded: false,
            // Properties start **closed**: progressive disclosure. Page
            // navigation is core, metadata editing is occasional, and a
            // panel that is always open costs canvas width on every
            // document to serve a task most sessions never perform.
            properties_open: false,
            save_result: None,
            // The dock starts closed: progressive disclosure. Everything
            // in it acts on files the operator has not opened, which is
            // by definition not what they are doing right now.
            tools_open: false,
            tools_selected: None,
            merge_inputs: Vec::new(),
            merge_bookmarks: true,
            // decision 012: no supplied font folders at startup — the
            // bundled deterministic default (R63). Session-only.
            font_folders: Vec::new(),
            font_env: pdfce_render::FontEnvironment::bundled(),
            font_env_generation: 0,
            font_notes: Vec::new(),
            // Pass 16.2 §5.1: bundled Helvetica is the default face for new
            // page text (the R79 non-embedded Standard-14 default).
            default_add_text_font: pdfce_core::fontdata::Std14::Helvetica,
            pending_save: None,
            copy_result: None,
            // Collapsed, like the render diagnostics: the one-line
            // summary is always visible and the detail is there for when
            // the summary says there is something to read.
            copy_detail_expanded: false,
            pending_copy: None,
            edit_note: None,
            recovery_note: None,
            // Pass 6.1: a visible red pen and a 2-point stroke — sensible
            // defaults an operator can change before authoring.
            markup_color: egui::Color32::from_rgb(0xE0, 0x30, 0x30),
            markup_width: 2.0,
            pending_text_kind: None,
            text_input: String::new(),
            // Closed by default: progressive disclosure, like the
            // Properties panel.
            shortcuts_open: false,
            // Matches `main()`'s initial `.with_title("pdfce")`, so the
            // first frame does not push a redundant title command.
            last_window_title: ui_text::window_title_idle().to_owned(),
        }
    }
}

/// Upper bound on a single supplied font file, in bytes (decision 012;
/// ARCHITECTURE.md §10 — never trust a file's size). Mirrors the CLI's
/// `MAX_FONT_FILE_BYTES`; a file past it is skipped-and-noted.
const MAX_FONT_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Font-file extensions the Font-folders walk attempts to parse
/// (decision 012). Twin of `pdfce-cli`'s `FONT_FILE_EXTENSIONS`: the two
/// shells each own their own filesystem walk (R61), so this is
/// deliberately duplicated rather than shared through a crate that would
/// have to sit below both.
const FONT_FILE_EXTENSIONS: [&str; 7] = ["ttf", "otf", "ttc", "cff", "pfb", "pfa", "otc"]; // ui-text-exempt: file extensions are machine tokens, not prose

impl PdfceApp {
    /// Rebuild [`Self::font_env`] from [`Self::font_folders`] and bump
    /// [`Self::font_env_generation`] (decision 012).
    ///
    /// The SHELL owns the filesystem walk; `pdfce-render` only ever
    /// receives bytes (R61). Each readable font-extension file is parsed
    /// ONCE through `pdfce-render`'s single skrifa parser (R21) to read
    /// its advertised name(s), then registered under both those names and
    /// the filename stem. Unreadable / oversized / unparseable files are
    /// skipped and recorded in [`Self::font_notes`], never fatal. Called
    /// only on a folder-list mutation — never per frame.
    fn rebuild_font_env(&mut self) {
        use pdfce_render::FontData;
        use pdfce_render::font::program::FontProgram;

        let mut env = pdfce_render::FontEnvironment::bundled();
        let mut notes: Vec<String> = Vec::new();

        for dir in &self.font_folders {
            let entries = match std::fs::read_dir(dir) {
                Ok(rd) => rd,
                Err(err) => {
                    notes.push(ui_text::font_folder_note_unreadable_dir(
                        &dir.display(),
                        &err,
                    ));
                    continue;
                }
            };
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && has_font_extension(p))
                .collect();
            files.sort();

            for path in files {
                if let Ok(m) = std::fs::metadata(&path)
                    && m.len() > MAX_FONT_FILE_BYTES
                {
                    notes.push(ui_text::font_folder_note_oversized(&path.display()));
                    continue;
                }
                let bytes = match std::fs::read(&path) {
                    Ok(b) => b,
                    Err(err) => {
                        notes.push(ui_text::font_folder_note_skipped(&path.display(), &err));
                        continue;
                    }
                };
                let mut names = match FontProgram::parse(&bytes) {
                    Ok(program) => program.face_names(),
                    Err(_) => {
                        notes.push(ui_text::font_folder_note_unparseable(&path.display()));
                        continue;
                    }
                };
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    && !names.iter().any(|n| n == stem)
                {
                    names.push(stem.to_owned());
                }
                if names.is_empty() {
                    notes.push(ui_text::font_folder_note_unparseable(&path.display()));
                    continue;
                }
                let data = FontData::new(bytes);
                for name in &names {
                    env.insert_named(name, data.clone());
                }
                notes.push(ui_text::font_folder_note_registered(
                    &path.display(),
                    &ui_text::join_names(&names),
                ));
            }
        }

        self.font_env = env;
        self.font_notes = notes;
        self.font_env_generation = self.font_env_generation.wrapping_add(1);
    }
}

/// Whether `path`'s extension is one of [`FONT_FILE_EXTENSIONS`]
/// (case-insensitive).
fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| FONT_FILE_EXTENSIONS.contains(&e.as_str()))
}

/// Which geometric-markup shape the toolbar's Markup menu authors
/// (Pass 6.1 minimal affordance). A representative subset of
/// [`pdfce_core::annot_author::MarkupSpec`]'s ten subtypes — one filled
/// shape, one ellipse, one line, one text-markup — placed at a default
/// rectangle on the current page. The full ten-tool canvas state machine
/// of `docs/ui_specs/pass-6.1-markup-tools.md` §1 is a named follow-up
/// slice (see the Pass 6.1 GUI status in the session log).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiMarkupKind {
    /// `/Square` at a centred rectangle.
    Square,
    /// `/Circle` inscribed in a centred rectangle.
    Circle,
    /// `/Line` across the page centre, open arrowheads.
    Line,
    /// `/Highlight` over a centred band.
    Highlight,
}

impl GuiMarkupKind {
    /// The menu-item label (via `ui_text`, R1).
    fn label(self) -> &'static str {
        match self {
            Self::Square => ui_text::markup_square_item(),
            Self::Circle => ui_text::markup_circle_item(),
            Self::Line => ui_text::markup_line_item(),
            Self::Highlight => ui_text::markup_highlight_item(),
        }
    }
}

/// Which text-bearing annotation the toolbar's "Text" menu authors
/// (Pass 6.2 minimal affordance). Mirrors the three
/// [`pdfce_core::annot_author::TextAnnotSpec`] variants; a full
/// click-to-place canvas text editor is the named follow-up slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuiTextKind {
    /// `/FreeText` — a text box drawn on the page.
    FreeText,
    /// `/Text` — a sticky note whose body opens in a popup.
    Sticky,
    /// `/Stamp` — a Draft stamp with a framed label.
    Stamp,
}

impl GuiTextKind {
    /// The menu-item label (via `ui_text`, R1).
    fn label(self) -> &'static str {
        match self {
            Self::FreeText => ui_text::text_freetext_item(),
            Self::Sticky => ui_text::text_sticky_item(),
            Self::Stamp => ui_text::text_stamp_item(),
        }
    }
}

/// What the shell is currently showing.
///
/// See the module docs, "Three ways to fail" — the split between
/// [`Status::Failed`] and [`Status::Unsupported`] is the whole reason
/// this is not a `Result<Document, String>`.
#[derive(Default)]
enum Status {
    /// No file opened yet this session.
    #[default]
    Idle,
    /// A document is open. Boxed because `Document` retains the file's
    /// entire byte buffer plus every parsed object, which would make
    /// every `Status` value that large — including the two error
    /// variants, and including the `Idle` one.
    Open(Box<OpenDoc>),
    /// The file is damaged, truncated, or not a PDF.
    Failed { path: PathBuf, message: String },
    /// The file is valid and pdfce declines to read it *yet*.
    Unsupported { path: PathBuf, message: String },
}

/// An open document and everything the view needs to draw it.
/// The Pass 14.3 in-place text-editing tool's per-page caret/selection/preview
/// state, rebuilt (not incrementally patched) on tool entry, page navigation
/// and after every accepted edit (`docs/ui_specs/pass-14.3-text-edit-ui.md`
/// §2/§2.1). Session-only, exactly like `active_tool`/`canvas_selection`.
///
/// The Pass 14.0 [`EditableTextModel`](pdfce_core::text_edit::EditableTextModel)
/// borrows the [`PageText`](pdfce_core::text_extract::PageText) it recognizes,
/// so this stores the **owned** `PageText` and rebuilds the (cheap, index-only)
/// model transiently wherever it is needed — sidestepping a self-referential
/// borrow rather than storing the model itself.
struct TextEditState {
    /// The page this state was built against (staleness key).
    page_index: usize,
    /// Pass 14.0's extraction, captured WITH provenance (mandatory — without
    /// it `provenance()` is `None` everywhere and §7's operator-span pinning
    /// cannot work). Owned; the model is rebuilt from it on demand.
    page_text: pdfce_core::text_extract::PageText,
    /// The caret — a `(run, byte-offset)` position; `None` before the first
    /// click lands on a glyph.
    caret: Option<pdfce_core::text_edit::TextPosition>,
    /// The fixed end of a selection; `None` (or equal to `caret`) means a
    /// collapsed caret with no selection.
    anchor: Option<pdfce_core::text_edit::TextPosition>,
    /// An in-progress, UNCOMMITTED edit the operator is composing — `Some` is
    /// exactly this tool's discardable [`GestureInterrupt`](canvas::GestureInterrupt) gesture (§6.2).
    pending: Option<PendingEdit>,
    /// An in-progress, operator-reviewed within-block reflow (Pass 15.2, §2).
    /// **Mutually exclusive** with [`Self::pending`] (§1.4): at most one
    /// uncommitted derived state at a time. `None` unless the reflow sub-mode
    /// is active. Never written anywhere until Accept (§7).
    reflow: Option<ReflowState>,
    /// The most recent ACCEPTED edit's disclosures, rendered verbatim in the
    /// strip until the next accept or tool exit (§8.1). Never auto-dismissed.
    last_disclosures: Vec<String>,
    /// Whether the read-only block-boundary review overlay is shown (§9).
    show_block_overlay: bool,
    /// Property-bar working values (§7), applied through
    /// `EditSession::format_text` on the operator's Apply. Seeded to plain
    /// defaults; the colour MODEL choice is itself the parity-plus surface
    /// (pdfce stores whichever space the operator picks — never force-RGB).
    prop_size: f64,
    /// The fill colour MODEL the property bar will store (`g`/`rg`/`k`).
    prop_model: pdfce_core::text_edit::FillModel,
    /// Colour components (r,g,b,k order for the widest model); only the first
    /// [`FillModel::arity`](pdfce_core::text_edit::FillModel::arity) are used.
    prop_components: [f64; 4],
    /// The font resource key/base-font the family ComboBox selected, if any.
    prop_font: Option<String>,
    /// The most recent property-bar (format) refusal `Display` text, kept
    /// visible in the strip until the next successful format/edit or tool
    /// exit (§8.2). Edit refusals live on [`PendingEdit::last_refusal`]; this
    /// is for a format apply, which has no `PendingEdit`.
    last_refusal: Option<String>,
}

/// An in-progress, operator-composed text edit that the live preview draws
/// and Accept commits — never written anywhere until Accept (§6). A crash
/// mid-composition loses only these uncommitted keystrokes (§10).
struct PendingEdit {
    /// The run being edited (a commit may only span ONE run — §4.4).
    run: usize,
    /// The run's original text, for the reject-revert and for the
    /// `EditRequest::find`.
    original_text: String,
    /// The operator's in-progress replacement (what the preview draws).
    draft_text: String,
    /// The insertion caret as a byte offset into `draft_text` (advanced by
    /// typing, retreated by Backspace).
    cursor: usize,
    /// The last commit attempt's refusal `Display` text, kept visible while
    /// the operator revises (§6.4/§8.2) — never silently cleared.
    last_refusal: Option<String>,
}

/// A small positive wrap-width floor fed to the reflow engine from the
/// on-canvas drag handle (Pass 15.2 §6.1), points. Purely UI-side smoothing so
/// a pointer briefly dragged past the block's left edge does not flash a
/// `BadWidth` error every frame; the engine's own `ReflowError::BadWidth`
/// still fires for any non-positive width that reaches it (via the typed
/// DragValue), so this hides no real refusal.
const MIN_WRAP_WIDTH_PT: f64 = 12.0;

/// An in-progress, operator-reviewed within-block reflow of ONE recognized
/// block (Pass 15.2, decision 015 §3.4/R75) — never written anywhere until
/// Accept (§7). Mutually exclusive with [`TextEditState::pending`] (§1.4). A
/// crash mid-review loses only the uncommitted width/alignment/leading
/// adjustments — the same low-stakes class as a half-composed [`PendingEdit`]
/// (§10 of the 14.3 spec; this Pass changes nothing about that discipline).
struct ReflowState {
    /// The targeted block — an index into the RELAXED-recognition model's
    /// `blocks()` (`reflow_recognition_options`, §1.2), never the
    /// default-recognition model's index space (which may number the same
    /// paragraph's blocks differently).
    block_index: usize,
    /// The engine's own auto-detected alignment for this block, computed ONCE
    /// at entry and held fixed for the review — the stable reference the
    /// "Detected: X" / "you changed this" caption (§6.2) compares against,
    /// independent of whatever the operator later picks.
    detected_alignment: pdfce_core::text_edit::DetectedAlignment,
    /// Current working wrap width, pt — seeded from the first preview's
    /// `wrap_width` (the block's own bbox width) on entry, then edited by the
    /// width DragValue / drag-handle (§6.1). Always concrete; never blank.
    width: f64,
    /// Current working alignment — seeded from `detected_alignment.alignment`
    /// on entry, then edited by the alignment picker (§6.2).
    alignment: pdfce_core::text_edit::BlockAlignment,
    /// Whether the operator has picked a DIFFERENT alignment than the detected
    /// one at least once (§4.3/§6.2). Gates whether the live-preview request
    /// sends an explicit `.with_alignment(..)` override at all — `preview`
    /// reports `AlignmentSource::Overridden` the instant ANY explicit
    /// alignment is supplied (even one equal to the detected value), which
    /// would otherwise make the caption lie the moment the operator re-clicks
    /// the already-selected option.
    alignment_is_override: bool,
    /// Current working leading, pt — seeded from the first preview's leading on
    /// entry, then edited by the leading DragValue (§6.3).
    leading: f64,
    /// The most recently computed live preview, recomputed each frame any of
    /// width/alignment/leading changed (§6). `Err` is shown, not silently
    /// dropped (§6.4) — a bad width mid-drag is still informative.
    preview: Result<pdfce_core::text_edit::ReflowPreview, pdfce_core::text_edit::ReflowError>,
    /// The last Accept ATTEMPT's refusal `Display` text (+ appended hint,
    /// §7.3), kept visible while the operator revises — never a silent dead
    /// end (mirrors [`PendingEdit::last_refusal`]). Cleared the moment any of
    /// width/alignment/leading changes (a revision is in progress; the stale
    /// refusal would otherwise imply a still-current problem).
    last_refusal: Option<String>,
}

/// Whether the "Reflow paragraph…" button is enabled (§1.3/§1.4): a target
/// paragraph resolved from the caret AND no single-run edit is pending —
/// the mutual-exclusion rule that forbids two simultaneous uncommitted derived
/// states. Pure, headless-tested.
#[must_use]
fn reflow_button_enabled(target: Option<usize>, pending_is_some: bool) -> bool {
    target.is_some() && !pending_is_some
}

/// Resolve `alignment_is_override` after the operator clicks alignment `picked`
/// on a block whose detected alignment is `detected` (§4.3/§6.2): picking the
/// detected value un-overrides (so the caption returns to "Detected"); picking
/// anything else overrides. This is the None-until-truly-overridden rule that
/// keeps `AlignmentSource` honest — the live request sends an explicit
/// alignment ONLY while this is `true`. Pure, headless-tested.
///
/// (Deliberate, named deviation from the 15.2 spec §4.3's *illustrative*
/// snippet, whose `else if val == detected` arm reset the flag to `false` on
/// every frame regardless of a click — which would make an override never
/// stick. Deciding override state on the CLICK, from the clicked value, is the
/// spec's stated §2 intent expressed correctly.)
#[must_use]
fn reflow_alignment_is_override(
    detected: pdfce_core::text_edit::BlockAlignment,
    picked: pdfce_core::text_edit::BlockAlignment,
) -> bool {
    picked != detected
}

/// Map a reflow refusal to the fixed "what would lift it" hint (§7.3 table) —
/// the reflow sibling of [`edit_refusal_hint`]/[`format_refusal_hint`]. Every
/// named condition gets a next-step sentence; no refusal ships as a dead end.
fn reflow_refusal_hint(err: &pdfce_core::text_edit::ReflowApplyError) -> &'static str {
    use pdfce_core::text_edit::{ReflowApplyError, ReflowError};
    match err {
        ReflowApplyError::Preview(ReflowError::EmptyBlock(_)) => ui_text::reflow_empty_block_hint(),
        ReflowApplyError::Preview(ReflowError::BadWidth(_)) => ui_text::reflow_bad_width_hint(),
        // `Unsupported` covers both the already-edited-this-session refusal
        // (15.1 judgment call #6, the primary GUI-reachable case) and a
        // rotated/shared/non-contiguous block; the verbatim Display above the
        // hint names which, so one honest next-step sentence serves both.
        ReflowApplyError::Unsupported(_) => ui_text::reflow_already_edited_hint(),
        ReflowApplyError::Refused(_) => ui_text::reflow_font_refused_hint(),
        _ => ui_text::reflow_generic_hint(),
    }
}

/// The smallest box (in PDF-space points, either dimension) a drag must sweep
/// to count as box mode (Pass 16.2 §3.2). A drag below this floor falls back to
/// a POINT at the drag's start — a deliberate divergence from Pass 6.1's
/// "degenerate-drag discards" rule (the logic + reasoning live in
/// [`canvas::resolve_drag_placement`]). Mirrors 15.2's `MIN_WRAP_WIDTH_PT`
/// naming convention.
const MIN_ADD_TEXT_BOX_PT: f64 = 12.0;

/// The Pass 16.2 Add-Page-Text tool's per-page state (§2). `Some` on `OpenDoc`
/// only while [`CanvasTool::AddText`] is the active tool; `None` otherwise and
/// between pages. Mutually exclusive with [`OpenDoc::text_edit`] by
/// construction (a single `active_tool` — §0.1). Session/view state: a draft is
/// never an edit; only an ACCEPTED add is, and that goes through `session` like
/// every other command.
///
/// Unlike [`TextEditState`], this holds NO `EditableTextModel`/`PageText`:
/// Add-Text places a new origin/box and never reads existing page text, so
/// tool entry and page-nav rebuilds are cheap (only `page_index`/prop-bar
/// values carry; the draft clears).
struct AddTextState {
    /// The page this state targets (staleness key). Rebuilt (draft cleared) on
    /// page navigation while the tool stays active (§2.1).
    page_index: usize,
    /// Property-bar working values, seeded on tool entry from the operator's
    /// default preference (§5.1), then editable per-use (§5.2).
    prop_font: pdfce_core::fontdata::Std14,
    /// Font size, points (§5.2).
    prop_size: f64,
    /// Fill colour, read back as [`NewTextColor::Black`] when pure black else
    /// [`NewTextColor::Rgb`] (§5.2 — the honest two-state surface: core's
    /// `NewTextColor` has only Black/Rgb, so no Gray/CMYK is offered).
    prop_color: egui::Color32,
    /// Box-mode alignment (§5.2). Default Left — a fresh box has no glyphs to
    /// auto-detect from (decision 016 §6/16.1).
    prop_alignment: pdfce_core::text_edit::BlockAlignment,
    /// Box-mode leading (line spacing), points; `0.0` means "use the derived
    /// 1.2× default" (§5.2). Ignored in point mode.
    prop_leading: f64,
    /// Manual (keyboard-complete) origin X/Y, points (§3.3) — a fully
    /// keyboard-operable placement path, not just a pointer one.
    manual_origin: [f64; 2],
    /// Manual box width/height, points (§3.3, box half).
    manual_box: [f64; 2],
    /// Whether the manual-entry row is in box mode (Place box) vs point mode
    /// (Place point) — §3.3.
    use_box: bool,
    /// The screen-space anchor of an in-progress rubber-band drag (§3.2), held
    /// across frames so the box is drawn anchor..current every frame. `None`
    /// when not dragging.
    drag_anchor: Option<egui::Pos2>,
    /// The in-progress placement + composition — at most one at a time, this
    /// tool's own discardable [`GestureInterrupt`](canvas::GestureInterrupt)
    /// gesture (§8). `None` until a placement gesture completes.
    draft: Option<AddTextDraft>,
    /// The most recent ACCEPTED add's disclosures, rendered verbatim until the
    /// next Accept or tool exit (§6.1) — never auto-dismissed.
    last_disclosures: Vec<String>,
}

/// A fixed origin/box plus the operator's in-progress new text (§2) — created
/// the moment a placement gesture completes (even before any character is
/// typed, so Reject/Esc is available at once); Accept stays disabled while
/// [`Self::draft_text`] is empty (§6.3).
struct AddTextDraft {
    /// The PDF-space origin/box, fixed at creation (reposition = Reject then
    /// re-place).
    placement: AddPlacement,
    /// The composed text; empty right after placement, grows with typing. No
    /// wrapping touches this in point mode; box mode recomputes
    /// [`Self::wrap_preview`] from it (§4.2).
    draft_text: String,
    /// Box mode ONLY: the live, PURE wrap preview (Ok) or its refusal `Display`
    /// (Err), recomputed every frame the text/box/font/size/alignment/leading
    /// changes via `pdfce_core::text_edit::preview_wrap` — the read-only
    /// analogue of a `ReflowPreview` (§4.2). `None` in point mode.
    wrap_preview: Option<Result<pdfce_core::text_edit::AddTextWrapPreview, String>>,
    /// The last Accept ATTEMPT's refusal `Display` (+ hint), kept visible while
    /// the operator revises — mirrors [`PendingEdit::last_refusal`] (§6.4).
    last_refusal: Option<String>,
}

/// PDF-space placement of an add-text draft, fixed at creation (§2).
enum AddPlacement {
    /// A single-line run growing rightward from `(x, y)` (16.0, shipped).
    Point {
        /// Origin x, PDF user space.
        x: f64,
        /// Origin (baseline) y, PDF user space.
        y: f64,
    },
    /// A wrap box, top-anchored, lower-left `(llx, lly)`, `width`×`height`
    /// (16.1).
    Box {
        /// Lower-left x, PDF user space.
        llx: f64,
        /// Lower-left y, PDF user space.
        lly: f64,
        /// Box width, points.
        width: f64,
        /// Box height, points.
        height: f64,
    },
}

struct OpenDoc {
    /// Where it came from (shown in the toolbar; never reopened).
    path: PathBuf,
    /// The parsed document **and the operator's unsaved edits**.
    ///
    /// Every mutation in this crate goes through this one object — there
    /// is no path from a widget to a `Document` (docs/ARCHITECTURE.md
    /// §11.4). `pdfce-cli` uses the same type for the same reason, which
    /// is what makes "the GUI and the CLI cannot diverge" a structural
    /// claim rather than a promise.
    session: EditSession,
    /// The flattened page list, in document order, inheritance resolved,
    /// **with unsaved `/Rotate` edits applied**.
    ///
    /// Cached rather than recomputed per frame (it walks the whole page
    /// tree) and refreshed by [`OpenDoc::refresh_pages`] after every
    /// edit, undo and redo. Nothing else may write it.
    pages: Vec<Page>,
    /// Draft text for the document-properties panel, one entry per
    /// [`InfoField`], seeded from the file when the panel is opened.
    ///
    /// Kept apart from the session on purpose: typing in a text box is
    /// not an edit. The draft becomes a command — and therefore an undo
    /// entry — only when the operator applies it, which is what keeps
    /// one undo step equal to one intended change rather than one
    /// keystroke.
    properties_draft: Vec<(InfoField, String)>,
    /// Whether any properties field was decoded lossily, so the panel
    /// can say so rather than quietly offering to write back mangled
    /// text (see `pdfce_core::edit::InfoText::exact`).
    properties_lossy: bool,
    /// Which page, at what zoom, fitted how.
    view: ViewState,
    /// The cached raster of the current page, if one has been made.
    page_texture: Option<PageTexture>,
    /// Why the current page could not be rasterized, if it could not.
    /// Cleared on every successful raster.
    render_error: Option<String>,
    /// Lazily built thumbnails for the rail.
    thumbnails: ThumbnailCache,
    /// The zoom value seen at the end of the previous frame, used to
    /// detect that the zoom changed at all.
    observed_zoom: f32,
    /// The earliest instant at which the current zoom may be committed
    /// to a real rasterization — the debounce deadline described in the
    /// module docs.
    zoom_commit_at: Instant,
    /// Set by any *discrete* zoom command during this frame's input
    /// handling, and consumed at the end of the frame. It is what
    /// distinguishes "the operator clicked the zoom-in button" (commit
    /// at once) from "the operator is mid-wheel-gesture" (wait for the
    /// gesture to settle).
    zoom_commanded: bool,
    /// Which pages are selected for a batch operation, by 0-based index.
    ///
    /// Ordered (a `BTreeSet`) so that "the selected pages" is always a
    /// document-order list without a sort at every call site — which
    /// matters because extract writes them in the order it is given.
    selected_pages: BTreeSet<usize>,
    /// The page a Shift+click range-select measures from.
    selection_anchor: Option<usize>,
    /// The thumbnail currently being dragged, if any.
    ///
    /// Hand-rolled rather than pulled in with `egui_dnd`: the state is
    /// two `Option<usize>` fields and one insertion-line paint, which is
    /// less code than the dependency's integration would be, and it
    /// avoids a licence classification (CLAUDE.md rule 13) for something
    /// this small.
    dragged_page: Option<usize>,
    /// The slot the dragged thumbnail would drop into — an index
    /// *between* pages, so `0` means "before the first page" and
    /// `pages.len()` means "after the last".
    drop_target: Option<usize>,
    /// Whether annotation appearances (§12.5) are painted onto the canvas
    /// (Pass 6.0). Default `true` — a reader shows a document's markup,
    /// stamps and form-field widgets. Toggled from the toolbar view group;
    /// it is a **view-state** control (changes pixels, not bytes — no undo
    /// entry), threaded into the canvas raster and used as a texture
    /// staleness key so flipping it re-renders the current page.
    annotations_visible: bool,
    /// A monotonic counter of markup/text annotations authored this
    /// session, used only to jitter each successive default-placed shape
    /// (P1-3c) so two clicks of the same menu item do not stack exactly
    /// on top of each other — an indistinguishable, "did that even work?"
    /// result while the canvas has no drag-to-reposition yet. Purely
    /// cosmetic; taken modulo a small cycle so the offset stays on the
    /// page rather than marching off it.
    author_jitter: u32,
    /// The active canvas tool, if any (Pass 12.0 substrate). **Always
    /// `None` this Pass** — [`CanvasTool`] is uninhabited, so this can only
    /// ever observably be `None` until a future tool-bearing Pass adds a
    /// variant (`docs/ui_specs/pass-12.0-canvas-substrate.md` §3.1). Session
    /// state only, matching `rail_expanded`/`tools_open`.
    active_tool: Option<CanvasTool>,
    /// The substrate's canvas selection set — hit-tested content targets,
    /// in a deterministic order (`BTreeSet`, like `selected_pages`). **Always
    /// empty this Pass**: [`Self::target_provider`] is the no-op
    /// [`EmptyTargetProvider`], which resolves no hits, so nothing is ever
    /// inserted (spec §4.2). Session/view state — a selection is never an
    /// edit, exactly like `selected_pages`.
    canvas_selection: BTreeSet<TargetId>,
    /// Pass 14.3 in-place text-editing tool state (§2). `Some` only while
    /// [`CanvasTool::TextEdit`] is the active tool; `None` otherwise and
    /// between pages. Session/view state — a caret is never an edit; only an
    /// *accepted* edit is, and that goes through `session` like every other.
    text_edit: Option<TextEditState>,
    /// Pass 16.2 Add-Page-Text tool state (§2). `Some` only while
    /// [`CanvasTool::AddText`] is the active tool; `None` otherwise and between
    /// pages. Mutually exclusive with [`Self::text_edit`] (§0.1). Session/view
    /// state — a draft is never an edit; only an accepted add is, and that goes
    /// through `session` like every other command.
    add_text: Option<AddTextState>,
    /// The attached hit-test provider (spec §4.1). Ships as the shippable
    /// no-op [`EmptyTargetProvider`] so the selection scaffold is fully
    /// wired yet selects nothing; Pass 9a swaps in an adapter over
    /// `pdfce-core`'s read-only object model. Boxed `dyn` behind an `Option`
    /// so Pass 9a can detach/replace it without changing this field's type.
    target_provider: Option<Box<dyn CanvasTargetProvider>>,
}

impl OpenDoc {
    /// Build the open-document state for a freshly loaded file.
    fn new(path: PathBuf, session: EditSession, pages: Vec<Page>) -> Self {
        let view = ViewState::default();
        Self {
            path,
            session,
            pages,
            properties_draft: Vec::new(),
            properties_lossy: false,
            observed_zoom: view.zoom,
            view,
            page_texture: None,
            render_error: None,
            thumbnails: ThumbnailCache::default(),
            zoom_commit_at: Instant::now(),
            zoom_commanded: false,
            selected_pages: BTreeSet::new(),
            selection_anchor: None,
            dragged_page: None,
            drop_target: None,
            annotations_visible: true,
            author_jitter: 0,
            // Pass 12.0 substrate: no tool active, an empty selection, and
            // the shippable no-op provider. Every one of these is a
            // structurally-guaranteed no-op this Pass (uninhabited tool,
            // provider that hits nothing) — the "zero behaviour change"
            // acceptance criterion.
            active_tool: None,
            canvas_selection: BTreeSet::new(),
            text_edit: None,
            add_text: None,
            target_provider: Some(Box::new(EmptyTargetProvider)),
        }
    }

    /// (Re)build the Pass 16.2 Add-Page-Text state for the CURRENT page on tool
    /// ENTRY (§2.1), seeding the property bar from the operator's default font
    /// preference (§5.1). Cheap — unlike [`Self::build_text_edit_state`], no
    /// page text is extracted (Add-Text never reads existing content).
    fn build_add_text_state(&mut self, default_font: pdfce_core::fontdata::Std14) {
        self.add_text = Some(AddTextState {
            page_index: self.view.page_index,
            prop_font: default_font,
            prop_size: 12.0,
            prop_color: egui::Color32::BLACK,
            prop_alignment: pdfce_core::text_edit::BlockAlignment::Left,
            prop_leading: 0.0,
            manual_origin: [72.0, 700.0],
            manual_box: [200.0, 40.0],
            use_box: false,
            drag_anchor: None,
            draft: None,
            last_disclosures: Vec::new(),
        });
    }

    /// (Re)build the Pass 14.3 text-edit state for the CURRENT page (§2.1):
    /// extract it WITH provenance and store the owned `PageText`, resetting
    /// caret/selection/pending. Called on tool entry, on page navigation
    /// while the tool stays active, and after every accepted edit — the model
    /// the old state pointed at is stale the moment the content stream
    /// changes, so it is rebuilt rather than patched.
    ///
    /// A failure to extract (a malformed page) leaves `text_edit = None` — the
    /// tool then simply places no caret, rather than panicking.
    fn build_text_edit_state(&mut self) {
        use pdfce_core::text_extract::{self, ExtractOptions};
        let page_index = self.view.page_index;
        let options = ExtractOptions::default().with_provenance(true);
        let base = self.session.document();
        self.text_edit = match text_extract::extract_pages(base, &[page_index], &options) {
            Ok(mut extracted) if !extracted.pages.is_empty() => Some(TextEditState {
                page_index,
                page_text: extracted.pages.remove(0),
                caret: None,
                anchor: None,
                pending: None,
                reflow: None,
                last_disclosures: Vec::new(),
                show_block_overlay: false,
                prop_size: 12.0,
                prop_model: pdfce_core::text_edit::FillModel::Rgb,
                prop_components: [0.0, 0.0, 0.0, 0.0],
                prop_font: None,
                last_refusal: None,
            }),
            _ => None,
        };
    }

    /// The selected pages in document order, or — when nothing is
    /// selected — nothing.
    ///
    /// Deliberately **not** "falls back to the current page". A batch
    /// action with an empty selection doing something to the page that
    /// happens to be on screen is exactly the kind of surprise that
    /// makes an editor untrustworthy; the controls are hidden instead.
    fn selection(&self) -> Vec<usize> {
        self.selected_pages.iter().copied().collect()
    }

    /// Drop any selection entry past the end of the document.
    ///
    /// Run after every structural edit: a delete shortens the page list,
    /// and a stale index would make the next batch action address a page
    /// that no longer exists.
    fn clamp_selection(&mut self) {
        let count = self.pages.len();
        self.selected_pages.retain(|index| *index < count);
        self.selection_anchor = self.selection_anchor.filter(|index| *index < count);
    }

    /// The page currently being viewed, if the document has any pages.
    fn current_page(&self) -> Option<&Page> {
        self.pages.get(self.view.page_index)
    }

    /// Re-read the page list from the session and drop any cached
    /// raster that an edit could have invalidated.
    ///
    /// Called after every edit, undo and redo. Both halves are needed:
    /// the page list carries `/Rotate`, and the cached textures are
    /// pictures of the *old* rotation. Only the current page's texture
    /// and the thumbnails are discarded — the document is not reloaded,
    /// because the base revision (and therefore every byte span the
    /// renderer resolves) has not changed.
    fn refresh_pages(&mut self) {
        if let Ok(pages) = self.session.pages() {
            self.pages = pages;
        }
        self.page_texture = None;
        self.render_error = None;
        self.thumbnails = ThumbnailCache::default();
        self.prune_canvas_selection();
    }

    /// Drop any canvas-selection target the provider can no longer resolve
    /// (spec §4.4) — the geometry analogue of [`Self::clamp_selection`] for
    /// the page rail. Run from [`Self::refresh_pages`], which every edit,
    /// undo and redo already funnels through, so a `TargetId` invalidated by
    /// an edit never lingers as a dangling selection entry. Silent, exactly
    /// like `clamp_selection`. With the no-op provider (this Pass) the
    /// selection is always empty, so this is a structurally-guaranteed
    /// no-op; the call site exists so Pass 9a's real provider gets the
    /// cleanup for free.
    fn prune_canvas_selection(&mut self) {
        if let Some(provider) = self.target_provider.as_deref() {
            let page_index = self.view.page_index;
            self.canvas_selection = canvas::prune_selection(&self.canvas_selection, |target| {
                provider.bounds(page_index, target).is_some()
            });
        }
    }

    /// Seed the properties draft from the document's current metadata.
    ///
    /// Run when the panel is opened rather than continuously, so a
    /// half-typed field is not overwritten under the operator's cursor.
    fn seed_properties_draft(&mut self) {
        self.properties_lossy = false;
        self.properties_draft = InfoField::all()
            .into_iter()
            .map(|field| {
                let value = self.session.info_text(field);
                if value.as_ref().is_some_and(|v| !v.exact) {
                    self.properties_lossy = true;
                }
                (field, value.map(|v| v.text).unwrap_or_default())
            })
            .collect();
    }

    /// The current page's on-screen extent in PDF user-space units,
    /// `/Rotate` applied. Falls back to a nominal US Letter for a
    /// page-less document so the layout math has finite inputs.
    fn current_extent(&self) -> (f32, f32) {
        self.current_page()
            .map_or((612.0, 792.0), viewer::page_extent_pts)
    }

    /// Rasterize the current page and replace the cached texture.
    ///
    /// A failure is recorded in `render_error` rather than propagated:
    /// the document is still open and the operator can still navigate
    /// away from a page that will not draw.
    fn rasterize_current(
        &mut self,
        ctx: &egui::Context,
        raster_scale: f32,
        fonts: &pdfce_render::FontEnvironment,
        font_env_generation: u64,
    ) {
        let Some(page) = self.pages.get(self.view.page_index) else {
            self.page_texture = None;
            return;
        };
        match raster::render_page_texture(
            ctx,
            self.session.document(),
            page,
            self.view.page_index,
            raster_scale,
            self.annotations_visible,
            fonts,
            font_env_generation,
        ) {
            Ok(texture) => {
                self.page_texture = Some(texture);
                self.render_error = None;
            }
            Err(message) => {
                self.page_texture = None;
                self.render_error = Some(message);
            }
        }
        // Rasterization happens *after* the canvas has already been laid
        // out this frame (see `PdfceApp::ui`), so the new texture cannot
        // be drawn until the next one. Without this the display would
        // wait for whatever unrelated input happened to arrive next,
        // which on an idle window is "until the operator wiggles the
        // mouse" — the page would appear to take an arbitrarily long
        // time to show up.
        ctx.request_repaint();
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// A view command, from a button or a key.
///
/// Input is collected into a list of these and applied afterwards rather
/// than mutating state inside the widget/input closures. That is not
/// ceremony: `ctx.input(|i| …)` holds a lock on the input state for the
/// duration of the closure, and egui's `Ui` closures borrow `self`
/// mutably, so mutating application state from inside either is at best
/// awkward and at worst a deadlock. Collect, then apply.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    Open,
    FirstPage,
    PrevPage,
    NextPage,
    LastPage,
    GoToPage(usize),
    ZoomIn,
    ZoomOut,
    ZoomActualSize,
    Fit(FitMode),
    /// Multiply the zoom — the continuous ctrl+scroll path. The only
    /// action that does *not* count as a discrete command, so it is the
    /// only one that gets debounced.
    ZoomBy(f32),
    ToggleRail,
    /// Show or hide annotation appearances on the canvas (Pass 6.0,
    /// §12.5). A **view-state** control: changes pixels, not bytes, so it
    /// carries no undo entry — it only re-rasterizes the current page.
    ToggleAnnotations,
    /// Turn the current page a quarter turn counter-clockwise.
    RotateLeft,
    /// Turn the current page a quarter turn clockwise.
    RotateRight,
    /// Author a geometric-markup annotation on the current page (Pass 6.1
    /// minimal affordance). One `EditSession::add_markup` command; Undo
    /// reverses it, exactly like every other edit.
    AddMarkupShape(GuiMarkupKind),
    /// Open the text-entry popup for a text-bearing annotation (Pass 6.2).
    OpenTextEntry(GuiTextKind),
    /// Author the pending text-bearing annotation from the popup's buffer
    /// (one `EditSession::add_text_annotation` command; Undo reverses it).
    AddPendingText,
    /// Close the text-entry popup without authoring.
    CancelTextEntry,
    /// Enter (`Some`) or exit-to-view-mode (`None`) a canvas tool (Pass 12.0
    /// substrate, spec §3.2). One dispatch action shared by every future
    /// tool family. `Some(_)` is uninhabited this Pass ([`CanvasTool`] has
    /// no variants), so only the `None` (exit) case is ever constructed.
    SelectCanvasTool(Option<CanvasTool>),
    /// Cancel the active tool's in-progress gesture WITHOUT exiting the tool
    /// (the first stage of the two-stage Escape, spec §3.5). No tool exists
    /// to have a gesture this Pass, so applying it is a no-op.
    CancelToolGesture,
    /// Reverse the most recent edit.
    Undo,
    /// Re-apply the most recently reversed edit.
    Redo,
    /// Show or hide the document-properties panel.
    ToggleProperties,
    /// Commit the properties panel's draft text as one undoable edit.
    ApplyProperties,
    /// Write the document (with its unsaved edits) to a path the
    /// operator picks.
    Save,
    /// Show or hide the right-hand Tools dock.
    ToggleTools,
    /// Add or remove one page from the batch selection.
    TogglePageSelection(usize),
    /// Extend the selection from the anchor to this page.
    SelectRangeTo(usize),
    /// Empty the page-rail selection.
    ClearSelection,
    /// Empty the substrate's canvas selection (spec §3.5 step 3). Distinct
    /// from [`Action::ClearSelection`] (the page rail): different selection
    /// set, different Escape-precedence tier. Unreachable this Pass — the
    /// canvas selection is always empty — but wired so Pass 9a gets it free.
    ClearCanvasSelection,
    /// Remove the selected pages from the document.
    DeleteSelection,
    /// Turn the selected pages by a quarter turn (±90).
    RotateSelection(i32),
    /// Move the selected pages one slot earlier (-1) or later (+1).
    MoveSelection(i32),
    /// Drop the page currently being dragged into a slot.
    DropDragged(usize),
    /// Save the selected pages as a new file.
    ExtractSelection,
    /// Proceed with a save the operator has confirmed.
    ConfirmPendingSave,
    /// Abandon a save the operator declined.
    CancelPendingSave,
    /// Write the Combine tool's file list into a new document.
    CommitMerge,
    /// Extract text and put it on the clipboard.
    CopyText(CopyScope),
    /// Proceed with a copy the operator has confirmed.
    ConfirmPendingCopy,
    /// Abandon a copy the operator declined.
    CancelPendingCopy,
}

impl Action {
    /// Whether this action changes the zoom by *command* rather than by
    /// gesture, and so should be rasterized without waiting out the
    /// debounce window.
    ///
    /// Page-navigation actions answer `true` and that is harmless: they
    /// do not change the zoom, so the change-detection in
    /// [`PdfceApp::settle_and_rasterize`] never consults the flag for
    /// them. Encoding it as "is this NOT the one continuous action"
    /// keeps the rule in one place, so a future discrete zoom command
    /// (a zoom-percentage box, a marquee-zoom tool) is correct by
    /// default rather than by remembering to add it to a list.
    const fn is_discrete_zoom_command(self) -> bool {
        !matches!(self, Self::ZoomBy(_))
    }
}

impl PdfceApp {
    /// Show the native file-open dialog and load whatever the user picks.
    ///
    /// `rfd::FileDialog::pick_file` is blocking, which is fine here: it
    /// is invoked synchronously from a button click and returns as soon
    /// as the user dismisses the dialog. (A non-blocking/async variant
    /// will matter for the WASM fork, not for the native shell.)
    fn open_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
            .pick_file()
        else {
            return; // user cancelled — leave the previous status untouched
        };
        self.open_path(path);
    }

    /// Load `path` and set [`Self::status`] to the appropriate outcome.
    ///
    /// The three-way branch is the module docs' "three ways to fail":
    /// a named capability gap is reported as pdfce's limitation, and
    /// everything else as a problem with the file.
    fn open_path(&mut self, path: PathBuf) {
        // P0-1: reset every piece of per-document narration state, not
        // just the save result. `edit_note`, `copy_result` and the copy
        // detail expander all describe an action taken against the
        // *previous* document; left in place, they would sit in the
        // status bar as if they had just happened to the new one — stale
        // narration about the wrong file, which contradicts the
        // "status bar is the narrator" rule and rule 4 (fuzzy, never
        // sneaky). The text-entry popup is not part of the blocking
        // pending-gate, so it can be open across an Open; dismissing it
        // here prevents its buffer (typed against the old document) from
        // authoring onto the new one.
        //
        // Deliberately NOT reset: `pending_save` / `pending_copy` — the
        // `apply()` gate blocks `Action::Open` while either is set, so a
        // new document can never load with one of those outstanding.
        self.save_result = None;
        self.edit_note = None;
        self.recovery_note = None;
        self.copy_result = None;
        self.copy_detail_expanded = false;
        self.pending_text_kind = None;
        self.text_input.clear();
        self.status = match Document::load(&path) {
            Ok(doc) => match pdfce_core::page_tree::pages(&doc) {
                Ok(pages) => {
                    Status::Open(Box::new(OpenDoc::new(path, EditSession::new(doc), pages)))
                }
                Err(err) => Status::Failed {
                    path,
                    message: err.to_string(),
                },
            },
            Err(err) if is_unsupported_structure(&err) => Status::Unsupported {
                path,
                message: err.to_string(),
            },
            Err(err) => Status::Failed {
                path,
                message: err.to_string(),
            },
        };
        // Decision 013 disclosure: if the document opened via
        // cross-reference recovery, narrate it non-blockingly (R20). The
        // full RecoveryReport counts live on the Document for a richer
        // future surface (a pdfce-ui-specialist follow-up); this honest
        // one-liner ships now.
        self.recovery_note = if let Status::Open(doc) = &self.status {
            doc.session.document().recovery().map(|r| {
                format!(
                    "This document had a damaged cross-reference table and was \
                     rebuilt in memory ({} object(s) recovered). Saving will \
                     rewrite (normalize) the file; incremental save is refused.",
                    r.file_level_objects + r.objstm_objects
                )
            })
        } else {
            None
        };
        // P0-2: if the Properties panel happens to be open, reseed its
        // draft from the newly loaded document. `properties_open` outlives
        // any one document, and a fresh `OpenDoc` starts with an empty
        // draft, so without this the still-showing panel renders an empty
        // grid until the operator closes and reopens it.
        if self.properties_open
            && let Status::Open(doc) = &mut self.status
        {
            doc.seed_properties_draft();
        }
    }

    /// Show a save dialog and write the document — edits included — to
    /// whatever path the operator picks.
    ///
    /// **Save-as, always.** The dialog is pre-filled with a name derived
    /// from the original rather than the original itself: the standing
    /// non-destructive-by-default rule says pdfce does not overwrite the
    /// operator's file unless they name it, and a Save button that
    /// silently rewrites the open document is the single easiest way for
    /// an editor to destroy work.
    ///
    /// The mode is **incremental** (§7.5.6): prior bytes are preserved,
    /// which keeps any existing digital signature's byte range intact
    /// (§12.8.1 NOTE 1) and makes the previous revision recoverable. A
    /// full-rewrite option belongs with the optimization feature that
    /// gives it a reason to exist, not on the primary Save control.
    fn save_dialog(&mut self) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        let suggested = ui_text::suggested_save_name(&doc.path);
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
            .set_file_name(suggested)
            .save_file()
        else {
            return; // cancelled — nothing was written, nothing changes
        };

        let outcome = match doc
            .session
            .to_incremental_bytes(&SaveOptions::identity())
            .map_err(|err| err.to_string())
            .and_then(|(bytes, report)| {
                // Atomic write (standing UX rule 5): land the bytes in a
                // temp file in the DESTINATION's directory, then rename
                // over the target. A crash mid-write can therefore never
                // leave a truncated file at `path` — in particular when
                // the operator re-saves over an earlier copy from the
                // same session, the previous good file survives until
                // the new one is complete. Same-directory placement is
                // what keeps the final rename atomic (no cross-volume
                // copy fallback).
                write_atomic(&path, &bytes)
                    .map(|()| report)
                    .map_err(|err| err.to_string())
            }) {
            Ok(report) => SaveOutcome::Saved {
                path,
                objects: report.objects_written,
                appended: report.bytes_appended,
                promoted: report.promoted.len(),
            },
            Err(message) => SaveOutcome::Failed(message),
        };
        self.save_result = Some(outcome);
    }

    /// Commit the properties panel's draft as **one** undoable edit.
    ///
    /// Only fields whose text actually differs from what the document
    /// holds are written, which matters for two reasons: it keeps the
    /// dirty set minimal, and it means a field whose stored bytes could
    /// not be decoded exactly (see `properties_lossy`) is never written
    /// back mangled just because the panel was opened and applied.
    ///
    /// Empty text clears the field — `EditSession::set_info_field(None)`
    /// removes the entry rather than storing an empty string, because
    /// "no title" and "a title that is the empty string" are different
    /// things in the file and the empty box means the former.
    fn apply_properties(&mut self) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let draft = doc.properties_draft.clone();
        for (field, text) in draft {
            let current = doc.session.info_text(field).map(|v| v.text);
            let wanted = if text.is_empty() { None } else { Some(text) };
            if current.as_deref() == wanted.as_deref() {
                continue;
            }
            // A refusal here means a malformed `/Info` (it points at
            // something that is not a dictionary). Reported through the
            // same channel as a save failure rather than silently
            // dropped — the operator pressed Apply and is owed an answer.
            if let Err(err) = doc.session.set_info_field(field, wanted.as_deref()) {
                self.save_result = Some(SaveOutcome::Failed(err.to_string()));
                return;
            }
        }
    }

    // -- Pass 3.2: structural page operations ------------------------

    /// Remove the selected pages, and put the disclosure the operator is
    /// owed into the status bar.
    ///
    /// No confirmation dialog, deliberately. The standing rule reserves
    /// those for what Undo cannot rescue, and a delete is reversible
    /// right up to the save — the Pass 3.1 rotate control set the
    /// precedent (its tooltip names Undo instead of raising a modal) and
    /// inventing a second confirmation style for a reversible action
    /// would teach the operator to dismiss modals.
    ///
    /// The two honesty obligations are met by the tooltip
    /// ("delete is not redaction") and by this method (the
    /// dangling-reference count), not by a gate.
    fn delete_selection(&mut self) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let selection = doc.selection();
        if selection.is_empty() {
            return;
        }
        match doc.session.delete_pages(&selection) {
            Ok(outcome) => {
                doc.refresh_pages();
                doc.selected_pages.clear();
                doc.selection_anchor = None;
                doc.clamp_selection();
                let mut note = if outcome.dangling.outline_items == 0
                    && outcome.dangling.links == 0
                    && outcome.dangling.named_destinations == 0
                {
                    ui_text::delete_succeeded(outcome.pages_removed, outcome.objects_freed)
                } else {
                    ui_text::dangling_references_after_delete(
                        outcome.pages_removed,
                        outcome.dangling.outline_items,
                        outcome.dangling.links,
                        outcome.dangling.named_destinations,
                    )
                };
                if outcome.dangling.page_labels_stale {
                    note.push(' ');
                    note.push_str(ui_text::page_labels_now_stale());
                }
                self.edit_note = Some(note);
            }
            // A refusal here is real and named — a certification
            // signature that forbids the change, or the last page. It
            // goes through the same channel a failed save does, because
            // the operator pressed a button and is owed an answer.
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
    }

    /// Turn the selected pages, or — when nothing is selected — the page
    /// currently on screen.
    ///
    /// The fallback is safe here in a way it would not be for delete:
    /// rotation is visible immediately, trivially reversible, and the
    /// Pass 3.1 toolbar buttons already meant "the current page". Losing
    /// that would be a regression for every operator who never selects
    /// anything.
    fn rotate_selection(&mut self, delta: i32) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let pages = if doc.selected_pages.is_empty() {
            vec![doc.view.page_index]
        } else {
            doc.selection()
        };
        match doc.session.rotate_pages(&pages, delta) {
            Ok(_) => doc.refresh_pages(),
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
    }

    /// Author a geometric-markup annotation on the current page
    /// (Pass 6.1 minimal affordance).
    ///
    /// The shape is placed at a default rectangle derived from the page's
    /// `MediaBox` (a centred box for Square/Circle/Highlight, a centred
    /// segment for Line), using the current pen colour and width. It goes
    /// through the same [`EditSession::add_markup`] command the CLI and
    /// the (future) canvas tools use, so Undo reverses it and an
    /// incremental save keeps every prior byte intact (R47).
    ///
    /// A refusal — an enforced certification signature (X11), a malformed
    /// `/Annots`, an encrypted document (X10) — is surfaced through the
    /// same `save_result` channel `delete_pages`/`rotate_pages` use (the
    /// coordinator's "hard refusal at draw-commit" pattern), so no new
    /// dialog is introduced.
    fn add_markup_shape(&mut self, kind: GuiMarkupKind) {
        use pdfce_core::annot_author::{Color, LineEnding, MarkupSpec, Quad, TextMarkupKind};
        use pdfce_core::page_tree::Rect;

        // Copy the pen settings out before borrowing the session.
        let c = self.markup_color;
        let color = Color::Rgb(
            f64::from(c.r()) / 255.0,
            f64::from(c.g()) / 255.0,
            f64::from(c.b()) / 255.0,
        );
        let width = f64::from(self.markup_width);

        let outcome: Result<(), String> = {
            let Status::Open(doc) = &mut self.status else {
                return;
            };
            let page_index = doc.view.page_index;
            let Some(page) = doc.pages.get(page_index) else {
                return;
            };
            let mb = page.media_box;
            // P1-3c: nudge each successive shape by a small, deterministic
            // step so repeated adds do not land exactly on top of one
            // another. Modulo a short cycle keeps the nudge on the page.
            let jitter = f64::from(doc.author_jitter % 6) * 12.0;
            let cx = f64::midpoint(mb.llx, mb.urx) + jitter;
            let cy = f64::midpoint(mb.lly, mb.ury) - jitter;
            let hw = (mb.urx - mb.llx) * 0.30;
            let hh = (mb.ury - mb.lly) * 0.12;
            let rect = Rect::from_corners(cx - hw, cy - hh, cx + hw, cy + hh);
            let spec = match kind {
                GuiMarkupKind::Square => MarkupSpec::Square {
                    rect,
                    border: Some(color),
                    interior: None,
                    border_width: width,
                },
                GuiMarkupKind::Circle => MarkupSpec::Circle {
                    rect,
                    border: Some(color),
                    interior: None,
                    border_width: width,
                },
                GuiMarkupKind::Line => MarkupSpec::Line {
                    start: (cx - hw, cy),
                    end: (cx + hw, cy),
                    color,
                    width,
                    endings: (LineEnding::OpenArrow, LineEnding::OpenArrow),
                },
                GuiMarkupKind::Highlight => MarkupSpec::TextMarkup {
                    kind: TextMarkupKind::Highlight,
                    quads: vec![Quad::from_rect(rect)],
                    color: Color::Rgb(1.0, 1.0, 0.0), // highlight default: yellow
                },
            };
            match doc.session.add_markup(page_index, &spec) {
                Ok(_) => {
                    doc.author_jitter = doc.author_jitter.wrapping_add(1);
                    doc.refresh_pages();
                    Ok(())
                }
                Err(err) => Err(err.to_string()),
            }
        };
        match outcome {
            Ok(()) => self.edit_note = Some(ui_text::markup_added(kind.label())),
            Err(msg) => self.save_result = Some(SaveOutcome::Failed(msg)),
        }
    }

    /// Author the pending text-bearing annotation (Pass 6.2 minimal
    /// affordance) from the text-entry popup's buffer, at a centred rect on
    /// the current page, through the same `EditSession::add_text_annotation`
    /// path the CLI uses. Closes the popup on success.
    fn add_pending_text(&mut self) {
        use pdfce_core::annot_author::{Color, StampName, StickyIcon, TextAnnotSpec};
        use pdfce_core::page_tree::Rect;
        use pdfce_core::vartext::{Quadding, TextColor};

        let Some(kind) = self.pending_text_kind else {
            return;
        };
        let text = self.text_input.clone();
        let c = self.markup_color;
        let color = Color::Rgb(
            f64::from(c.r()) / 255.0,
            f64::from(c.g()) / 255.0,
            f64::from(c.b()) / 255.0,
        );

        let outcome: Result<(), String> = {
            let Status::Open(doc) = &mut self.status else {
                return;
            };
            let page_index = doc.view.page_index;
            let Some(page) = doc.pages.get(page_index) else {
                return;
            };
            let mb = page.media_box;
            // P1-3c: same per-add nudge as `add_markup_shape`, so two
            // "Text box" clicks are visibly distinct pending the canvas
            // text editor.
            let jitter = f64::from(doc.author_jitter % 6) * 12.0;
            let cx = f64::midpoint(mb.llx, mb.urx) + jitter;
            let cy = f64::midpoint(mb.lly, mb.ury) - jitter;
            let hw = (mb.urx - mb.llx) * 0.30;
            let hh = (mb.ury - mb.lly) * 0.06;
            let rect = Rect::from_corners(cx - hw, cy - hh, cx + hw, cy + hh);
            let spec = match kind {
                GuiTextKind::FreeText => TextAnnotSpec::FreeText {
                    rect,
                    text: text.clone(),
                    font: pdfce_core::fontdata::Std14::Helvetica,
                    font_size: 0.0, // auto-size to the box
                    color: TextColor::from(color),
                    quadding: Quadding::Left,
                    multiline: true,
                    border: None,
                    border_width: 0.0,
                },
                GuiTextKind::Sticky => TextAnnotSpec::Sticky {
                    // A small fixed marker near the top-left of the centred band.
                    rect: Rect::from_corners(cx - hw, cy + hh, cx - hw + 20.0, cy + hh + 20.0),
                    icon: StickyIcon::Note,
                    contents: text.clone(),
                    color: Color::Rgb(1.0, 0.92, 0.30),
                    open: false,
                },
                GuiTextKind::Stamp => TextAnnotSpec::Stamp {
                    rect,
                    name: StampName::Draft,
                    label: if text.trim().is_empty() {
                        None
                    } else {
                        Some(text.clone())
                    },
                    color: Color::Rgb(0.80, 0.10, 0.10),
                },
            };
            match doc.session.add_text_annotation(page_index, &spec) {
                Ok(_) => {
                    doc.author_jitter = doc.author_jitter.wrapping_add(1);
                    doc.refresh_pages();
                    Ok(())
                }
                Err(err) => Err(err.to_string()),
            }
        };
        match outcome {
            Ok(()) => {
                self.edit_note = Some(ui_text::markup_added(kind.label()));
                self.pending_text_kind = None;
                self.text_input.clear();
            }
            Err(msg) => {
                self.save_result = Some(SaveOutcome::Failed(ui_text::text_add_failed(&msg)));
            }
        }
    }

    /// Move the selected pages one slot earlier or later.
    ///
    /// The keyboard equivalent of a drag, and the only reorder path an
    /// assistive technology can drive.
    fn move_selection(&mut self, delta: i32) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let selection = doc.selection();
        if selection.is_empty() || delta == 0 {
            return;
        }
        let count = doc.pages.len();
        // Where the block lands: the first selected page's index, moved
        // by `delta` and clamped so a selection already at an end simply
        // does not move rather than wrapping around.
        let Some(first) = selection.first().copied() else {
            return;
        };
        let target = if delta < 0 {
            first.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            (first + delta.unsigned_abs() as usize).min(count.saturating_sub(selection.len()))
        };
        if target == first {
            return;
        }
        self.apply_reorder(&selection, target);
    }

    /// Reorder so that `selection` (in document order) becomes contiguous
    /// starting at `target`, and everything else keeps its relative
    /// order.
    ///
    /// Shared by the drag path and the keyboard path so the two cannot
    /// disagree about where a multi-page move lands — which they would,
    /// eventually, as two implementations.
    fn apply_reorder(&mut self, selection: &[usize], target: usize) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let count = doc.pages.len();
        let moving: BTreeSet<usize> = selection.iter().copied().collect();
        let rest: Vec<usize> = (0..count).filter(|i| !moving.contains(i)).collect();

        let at = target.min(rest.len());
        let mut order: Vec<usize> = Vec::with_capacity(count);
        order.extend(rest.iter().take(at).copied());
        order.extend(selection.iter().copied());
        order.extend(rest.iter().skip(at).copied());

        match doc.session.reorder_pages(&order) {
            Ok(()) => {
                doc.refresh_pages();
                // The selection follows the pages: after moving three
                // pages, "the selection" should still be those three
                // pages, not whatever now sits at their old indices.
                doc.selected_pages = (at..at + selection.len()).collect();
                doc.selection_anchor = doc.selected_pages.iter().next().copied();
                doc.clamp_selection();
                self.edit_note = Some(ui_text::reorder_succeeded(selection.len()));
            }
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
    }

    /// Finish a drag by dropping the dragged page (or, if it is part of
    /// the selection, the whole selection) into `slot`.
    fn drop_dragged(&mut self, slot: usize) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let Some(dragged) = doc.dragged_page.take() else {
            return;
        };
        doc.drop_target = None;
        // Dragging a page that is part of the selection moves the whole
        // selection; dragging one outside it moves just that page and
        // does not silently redefine what "selected" means.
        let selection = if doc.selected_pages.contains(&dragged) {
            doc.selection()
        } else {
            vec![dragged]
        };
        // `slot` counts positions between pages in the CURRENT order;
        // `apply_reorder` wants an index into the list with the moving
        // pages removed, so subtract the ones that were before it.
        let before = selection.iter().filter(|index| **index < slot).count();
        let target = slot.saturating_sub(before);
        self.apply_reorder(&selection, target);
    }

    /// Save the selected pages as a new file.
    ///
    /// **Not an edit**: it does not touch the open document, creates no
    /// undo entry, and does not clear the unsaved-changes marker. It is
    /// the Pass 3.1 "save a copy" flow restricted to a page subset, and
    /// reuses that flow's dialog, atomic write and status line rather
    /// than inventing a second save style.
    fn extract_selection(&mut self) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        let selection = doc.selection();
        if selection.is_empty() {
            return;
        }
        let suggested = ui_text::suggested_extract_name(&doc.path, &selection);
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
            .set_file_name(suggested)
            .save_file()
        else {
            return; // cancelled — nothing written, nothing changes
        };

        // The graph, not the base document: extracting "page 2" must mean
        // page 2 as the operator currently sees it, unsaved deletes and
        // reorders included.
        let graph = doc.session.graph();
        let base = doc.session.document();
        let view = pdfce_core::pageops::DocumentView::new(&graph, base.bytes(), base.version());
        let signed = doc.session.signature_census().any();

        let outcome = match pdfce_core::pageops::extract(&view, &selection) {
            Ok((bytes, report)) => match write_atomic(&path, &bytes) {
                Ok(()) => {
                    let mut note =
                        ui_text::extract_succeeded(&path, report.pages, report.dangling_references);
                    if signed {
                        note.push(' ');
                        note.push_str(ui_text::extract_note_unsigned_output());
                    }
                    self.edit_note = Some(note);
                    None
                }
                Err(err) => Some(err.to_string()),
            },
            Err(err) => Some(err.to_string()),
        };
        if let Some(message) = outcome {
            self.save_result = Some(SaveOutcome::Failed(message));
        }
    }

    /// Begin a save, asking about signatures first when the answer is not
    /// "there are none".
    ///
    /// The question is asked **before** the native file dialog opens, so
    /// an operator who is going to back out is not made to pick a
    /// destination first.
    ///
    /// Why it cannot live on the editing buttons instead: §11.1 makes the
    /// dirty set a diff computed **at save time**, so whether this save
    /// is structural is not knowable when the edit is committed. Save is
    /// also where the rule draws the real "you cannot take this back"
    /// line.
    fn begin_save(&mut self) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        // Incremental: the GUI's only save mode, and the one that
        // preserves prior bytes.
        match doc.session.signature_impact_of_save(SaveMode::Incremental) {
            SignatureImpact::None => self.save_dialog(),
            SignatureImpact::ByteRangePreserved => {
                self.save_dialog();
                // Informational, after the fact, and never on its own
                // presented as "your signature is still valid" — read
                // the catalog entry's doc comment before rewording it.
                if matches!(self.save_result, Some(SaveOutcome::Saved { .. })) {
                    self.edit_note =
                        Some(ui_text::save_signature_byte_range_preserved().to_owned());
                }
            }
            impact => self.pending_save = Some(PendingSave { impact }),
        }
    }

    // -- copy text (Pass 4) -------------------------------------------

    /// Extract text for `scope` and either put it on the clipboard or
    /// ask first.
    ///
    /// ## Why extraction reads the BASE document, not the edit overlay
    ///
    /// [`EditSession::document`] is the document as loaded, before this
    /// session's unsaved edits. That is correct for every edit this Pass
    /// of pdfce can make: `/Info` metadata is not page content, and
    /// `/Rotate` changes how a page is *displayed* without touching a
    /// single byte of its content stream, so neither can change which
    /// characters a page contains. The moment an editing Pass can alter
    /// page content, this has to move to the overlay-aware
    /// [`pdfce_core::graph::ObjectGraph`] path — recorded here rather
    /// than left as a silent assumption, because the failure mode would
    /// be copying stale text with nothing on screen to suggest it.
    ///
    /// ## Why the question is asked before the clipboard is written
    ///
    /// A clipboard write destroys whatever the operator had copied
    /// before it. An operator who is going to decline a mostly-unreadable
    /// copy must not first lose their previous clipboard to it — the same
    /// before-not-after reasoning as the signature question, with a
    /// sharper edge because there is no undo for a clipboard.
    fn begin_copy_text(&mut self, ctx: &egui::Context, scope: CopyScope) {
        use pdfce_core::text_extract::{self, ExtractOptions};

        let Status::Open(doc) = &self.status else {
            return;
        };
        let options = ExtractOptions::default();
        let base = doc.session.document();
        let page_number = doc.view.page_index + 1;

        let extracted = match scope {
            CopyScope::Page => text_extract::extract_pages(base, &[doc.view.page_index], &options),
            CopyScope::Document => text_extract::extract_document(base, &options),
        };
        let Ok(extracted) = extracted else {
            // The page tree will not walk. Nothing is on the clipboard
            // and the operator is told, rather than left with a button
            // that appeared to do nothing.
            self.copy_result = None;
            self.edit_note = Some(ui_text::copy_text_no_extractable_text().to_owned());
            return;
        };

        let text = extracted.plain_text();
        let d = &extracted.diagnostics;

        // A page with no text at all gets its own sentence rather than a
        // successful copy of nothing: an operator who pastes nothing and
        // is told nothing cannot tell "no text here" from "broken
        // button".
        if d.codes_total == 0 && text.is_empty() {
            self.copy_result = None;
            self.edit_note = Some(ui_text::copy_text_no_extractable_text().to_owned());
            return;
        }

        let outcome = CopyTextOutcome {
            scope,
            page_number,
            pages: extracted.pages.len(),
            characters: text.chars().count(),
            codes: d.codes_total,
            failed: d.ladder_failures,
            spaces_derived: d.spaces_derived,
            lines_derived: d.lines_derived,
            sourced_fraction: d.sourced_fraction().unwrap_or(0.0),
            pages_without_text: extracted
                .pages
                .iter()
                .filter(|p| p.diagnostics.codes_total == 0)
                .count(),
        };

        // The gate. Two conditions, and they are different KINDS of
        // signal rather than one threshold expressed twice:
        //
        //   * `identity_fonts_without_to_unicode > 0` is STRUCTURAL —
        //     pdfce knows, before counting a single failure, that
        //     §9.10.2 leaves no rung available for that font at all.
        //   * `sourced_fraction < 0.5` is a magnitude backstop for every
        //     other cause.
        //
        // Deliberately NOT a low percentage: the measured corpus sits at
        // 99.78% sourced, so a 5%-or-10% trigger would fire on ordinary
        // documents carrying one odd symbol font, and a confirmation
        // that fires on ordinary documents is one operators learn to
        // click through.
        let unreliable =
            d.identity_fonts_without_to_unicode > 0 || d.sourced_fraction().unwrap_or(1.0) < 0.5;
        if unreliable {
            self.pending_copy = Some(PendingCopy { outcome, text });
            return;
        }

        ctx.copy_text(text);
        self.copy_result = Some(outcome);
    }

    /// The mostly-unreadable-copy confirmation.
    ///
    /// Same shape as [`PdfceApp::signature_confirmation`] — a plain
    /// `egui::Window`, non-collapsible, centred, added last — because
    /// this is now the app's *second* blocking confirmation and a second
    /// dialog style would be a second thing for the operator to learn.
    ///
    /// That the two now share a convention is owed a decision-log entry:
    /// it was an implementation detail with one use and is a pattern
    /// with two (`docs/ui_specs/pass-4-text-extraction.md` §3). It is
    /// also what turned a latent gap into a live bug — two independent
    /// pending states can collide — which is why the enforcement lives
    /// in [`PdfceApp::apply`]'s gate rather than in either window.
    fn copy_confirmation(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(pending) = &self.pending_copy else {
            return;
        };
        let (failed, codes) = (pending.outcome.failed, pending.outcome.codes);
        egui::Window::new(ui_text::copy_text_unreliable_title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(520.0);
                ui.label(ui_text::copy_text_unreliable_body(failed, codes));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(ui_text::copy_text_unreliable_cancel_button())
                        .clicked()
                    {
                        actions.push(Action::CancelPendingCopy);
                    }
                    if ui
                        .button(ui_text::copy_text_unreliable_confirm_button())
                        .clicked()
                    {
                        actions.push(Action::ConfirmPendingCopy);
                    }
                });
            });
    }

    /// Whether a document with at least one page is open — the enabling
    /// condition for every control that acts on page content.
    ///
    /// The page count is part of the test, not an afterthought. A
    /// `/Count 0` page tree is spec-legal and degenerate, and offering
    /// Copy-text on one produces a failure whose only available message
    /// is written for "this page has no text" — which is the wrong
    /// answer to "this document has no pages". Not offering the control
    /// is both simpler and more honest than inventing a message for a
    /// case the operator cannot act on anyway.
    fn status_is_open(&self) -> bool {
        matches!(&self.status, Status::Open(doc) if !doc.pages.is_empty())
    }

    /// The copy-text result line and its detail expander.
    ///
    /// Its own header, next to `save_result`/`edit_note` and NOT inside
    /// the render-diagnostics one — see [`PdfceApp::copy_result`] for the
    /// lifecycle reason. The summary names its scope explicitly for the
    /// same reason.
    fn copy_result_bar(&mut self, ui: &mut egui::Ui) {
        let Some(result) = &self.copy_result else {
            return;
        };
        ui.label(match result.scope {
            CopyScope::Page => {
                ui_text::copy_text_succeeded_page(result.page_number, result.characters)
            }
            CopyScope::Document => {
                ui_text::copy_text_succeeded_document(result.pages, result.characters)
            }
        });

        let clean = result.failed == 0;
        let summary = if clean {
            ui_text::copy_text_headline_clean().to_owned()
        } else {
            ui_text::copy_text_headline_unreliable(result.failed, result.codes)
        };

        let response = egui::CollapsingHeader::new(summary)
            .id_salt("copy-diagnostics")
            .open(Some(self.copy_detail_expanded))
            .show(ui, |ui| {
                ui.label(ui_text::copy_text_detail_heading());
                // Tier 1 — routine, uncoloured. Almost every real
                // document needs some derived whitespace; colouring it
                // as a caution would teach operators to distrust the
                // normal case.
                if result.spaces_derived > 0 || result.lines_derived > 0 {
                    ui.label(ui_text::copy_text_derived_whitespace_note(
                        result.spaces_derived,
                        result.lines_derived,
                    ));
                }
                // Tier 2 — genuinely uncertain, warn-coloured AND marked.
                if result.failed > 0 {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        ui_text::copy_text_ladder_failures_note(result.failed, result.codes),
                    );
                    ui.label(ui_text::copy_text_sourced_percent(result.sourced_fraction));
                }
                if result.scope == CopyScope::Document && result.pages_without_text > 0 {
                    ui.label(ui_text::copy_text_pages_without_text_note(
                        result.pages_without_text,
                    ));
                }
            });
        if response.header_response.clicked() {
            self.copy_detail_expanded = !self.copy_detail_expanded;
        }
    }

    /// The signature-invalidation confirmation — one of the app's two
    /// blocking questions (the other is
    /// [`PdfceApp::copy_confirmation`]).
    ///
    /// Rendered as a plain `egui::Window` rather than `egui::Modal`,
    /// added last so it draws over every panel — the same layering trick
    /// the Properties window uses, and the Pass 3.2 UI spec's own named
    /// fallback, taken because it is verifiably available in the pinned
    /// egui rather than probably available.
    ///
    /// **Where "blocking" actually comes from.** An earlier version of
    /// this comment claimed the window was drawn "with the rest of the
    /// UI disabled underneath". It was not, and paint order alone could
    /// never have delivered that: it blocks only the pixels the ~520px
    /// centred window physically covers, and nothing at all on the
    /// keyboard. The property is enforced instead by the pending gate at
    /// the top of [`PdfceApp::apply`] — see its docs for the collision
    /// this closes.
    fn signature_confirmation(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(pending) = self.pending_save else {
            return;
        };
        // Only the invalidating verdict is a blocking question; the other
        // states never reach here, and asserting that in code keeps a
        // future variant from silently acquiring a modal.
        if pending.impact != SignatureImpact::Invalidated {
            self.pending_save = None;
            return;
        }
        egui::Window::new(ui_text::signature_invalidation_title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(520.0);
                ui.label(ui_text::signature_invalidation_body());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(ui_text::signature_invalidation_cancel_button())
                        .clicked()
                    {
                        actions.push(Action::CancelPendingSave);
                    }
                    if ui
                        .button(ui_text::signature_invalidation_confirm_button())
                        .clicked()
                    {
                        actions.push(Action::ConfirmPendingSave);
                    }
                });
            });
    }

    /// The right-hand Tools dock.
    ///
    /// The one secondary surface Pass 3.2 adds, and the pattern every
    /// future advanced bucket follows instead of growing the toolbar.
    fn tools_dock(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.heading(ui_text::tools_dock_title());
        ui.label(ui_text::tools_dock_intro());
        ui.separator();

        for (tool, label) in [
            (Tool::Merge, ui_text::tool_merge_label()),
            (Tool::Split, ui_text::tool_split_label()),
            (Tool::Insert, ui_text::tool_insert_pages_label()),
            (Tool::FontFolders, ui_text::tool_font_folders_label()),
        ] {
            let open = self.tools_selected == Some(tool);
            if ui.selectable_label(open, label).clicked() {
                self.tools_selected = if open { None } else { Some(tool) };
            }
        }
        ui.separator();

        match self.tools_selected {
            None => {}
            Some(Tool::Merge) => self.merge_tool(ui, actions),
            // Split and Insert ship as command-line tools this Pass; the
            // dock names the exact command rather than showing a dead
            // "coming soon", which is information the operator can act on
            // instead of an apology.
            Some(Tool::Split) => {
                ui.label(ui_text::tool_available_in_cli(ui_text::split_cli_command()));
            }
            Some(Tool::Insert) => {
                ui.label(ui_text::tool_available_in_cli(ui_text::insert_cli_command()));
            }
            Some(Tool::FontFolders) => self.font_folders_tool(ui),
        }
    }

    /// The Font-folders tool (decision 012): manage the operator-supplied
    /// font folders whose faces draw the open document's NON-embedded
    /// fonts. Unlike the other dock tools this changes how the CURRENT
    /// document renders, so it states that scope and the two honesty
    /// caveats (session-only; shapes-not-layout) rather than presenting as
    /// an act-on-other-files operation.
    fn font_folders_tool(&mut self, ui: &mut egui::Ui) {
        ui.label(ui_text::font_folders_intro());

        if ui
            .button(ui_text::font_folders_add_button())
            .on_hover_text(ui_text::font_folders_add_tooltip())
            .clicked()
            && let Some(dir) = rfd::FileDialog::new().pick_folder()
            && !self.font_folders.contains(&dir)
        {
            self.font_folders.push(dir);
            self.rebuild_font_env();
        }

        let mut remove: Option<usize> = None;
        for (index, dir) in self.font_folders.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .button(ui_text::selection_clear_button())
                    .on_hover_text(ui_text::font_folders_remove_tooltip())
                    .clicked()
                {
                    remove = Some(index);
                }
                ui.label(ui_text::file_name(dir));
            });
        }
        if let Some(index) = remove {
            self.font_folders.remove(index);
            self.rebuild_font_env();
        }

        if self.font_folders.is_empty() {
            ui.label(ui_text::font_folders_empty_hint());
        } else {
            // Session-level determinism disclosure (R63): supplied faces
            // are machine-dependent, stated once here where the folders
            // live (the per-page "supplied glyph(s)" fact is in the
            // diagnostics expander).
            ui.label(ui_text::font_folders_active_indicator(
                self.font_folders.len(),
            ));
            // The walk notes: which files registered under which names,
            // which were skipped — so a name mismatch is debuggable in the
            // UI (fuzzy-never-sneaky) rather than requiring a log dive.
            if !self.font_notes.is_empty() {
                egui::CollapsingHeader::new(ui_text::font_folders_notes_heading())
                    .id_salt("font-folder-notes")
                    .show(ui, |ui| {
                        for note in &self.font_notes {
                            ui.label(note);
                        }
                    });
            }
        }

        // Pass 16.2 §5.1: the operator's DEFAULT font for the Add-Page-Text
        // tool — a preference seeded into each new draft, overridable per-use.
        // Placed here (thematically part of the Font-folders panel) so the
        // Bundled/Supplied trust label reuses the SAME classify_nonembedded the
        // folder list already surfaces. Disjoint-field closure captures let the
        // ComboBox mutate `default_add_text_font` while reading `font_env`.
        ui.separator();
        ui.label(ui_text::add_text_default_font_label());
        let current = std14_combo_label(self.default_add_text_font, &self.font_env);
        egui::ComboBox::from_id_salt("add-text-default-font")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for face in pdfce_core::fontdata::Std14::ALL {
                    let label = std14_combo_label(face, &self.font_env);
                    ui.selectable_value(&mut self.default_add_text_font, face, label);
                }
            });
    }

    /// The Combine-files tool's widgets.
    fn merge_tool(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        if ui
            .button(ui_text::merge_add_files_button())
            .on_hover_text(ui_text::merge_add_files_tooltip())
            .clicked()
            && let Some(picked) = rfd::FileDialog::new()
                .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
                .pick_files()
        {
            self.merge_inputs.extend(picked);
        }
        // The open document is offered as the first row rather than
        // silently included: an operator who wants to combine two OTHER
        // files should not have to notice that theirs was added.
        if let Status::Open(doc) = &self.status
            && !self.merge_inputs.contains(&doc.path)
            && ui
                .button(ui_text::merge_current_document_label(&doc.path))
                .clicked()
        {
            self.merge_inputs.insert(0, doc.path.clone());
        }

        let mut remove: Option<usize> = None;
        let mut swap: Option<(usize, usize)> = None;
        for (index, path) in self.merge_inputs.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        index > 0,
                        egui::Button::new(ui_text::move_selection_up_button()),
                    )
                    .on_hover_text(ui_text::merge_move_up_tooltip())
                    .clicked()
                {
                    swap = Some((index, index - 1));
                }
                if ui
                    .add_enabled(
                        index + 1 < self.merge_inputs.len(),
                        egui::Button::new(ui_text::move_selection_down_button()),
                    )
                    .on_hover_text(ui_text::merge_move_down_tooltip())
                    .clicked()
                {
                    swap = Some((index, index + 1));
                }
                if ui
                    .button(ui_text::selection_clear_button())
                    .on_hover_text(ui_text::merge_remove_file_tooltip())
                    .clicked()
                {
                    remove = Some(index);
                }
                ui.label(ui_text::file_name(path));
            });
        }
        if let Some((a, b)) = swap {
            self.merge_inputs.swap(a, b);
        }
        if let Some(index) = remove {
            self.merge_inputs.remove(index);
        }

        ui.checkbox(
            &mut self.merge_bookmarks,
            ui_text::merge_bookmarks_checkbox(),
        )
        .on_hover_text(ui_text::merge_bookmarks_tooltip());

        let ready = self.merge_inputs.len() >= 2;
        if ready {
            if ui
                .button(ui_text::merge_commit_button())
                .on_hover_text(ui_text::merge_commit_tooltip(self.merge_inputs.len()))
                .clicked()
            {
                actions.push(Action::CommitMerge);
            }
        } else {
            ui.label(ui_text::merge_needs_two_files_hint());
        }
    }

    /// Combine the listed files into a new document.
    ///
    /// No `EditSession` involvement: merge reads N untouched files and
    /// writes one new one, so there is nothing to undo. Wiring it into
    /// the undo stack "for consistency" would be ceremony without
    /// meaning.
    fn commit_merge(&mut self) {
        if self.merge_inputs.len() < 2 {
            return;
        }
        let Some(first) = self.merge_inputs.first().cloned() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
            .set_file_name(ui_text::suggested_merge_name(&first))
            .save_file()
        else {
            return; // cancelled
        };

        // Every input is loaded up front so that a failure names the file
        // that failed, before anything is written.
        let mut docs = Vec::with_capacity(self.merge_inputs.len());
        for input in &self.merge_inputs {
            match Document::load(input) {
                Ok(doc) => docs.push(doc),
                Err(err) => {
                    self.save_result = Some(SaveOutcome::Failed(ui_text::merge_input_failed(
                        input,
                        &err.to_string(),
                    )));
                    return;
                }
            }
        }
        let views: Vec<pdfce_core::pageops::DocumentView<'_>> = docs
            .iter()
            .map(|doc| pdfce_core::pageops::DocumentView::new(doc, doc.bytes(), doc.version()))
            .collect();
        let titles: Vec<Vec<u8>> = if self.merge_bookmarks {
            self.merge_inputs
                .iter()
                .map(|path| pdfce_core::edit::encode_text_string(&ui_text::file_stem(path)))
                .collect()
        } else {
            Vec::new()
        };
        let signed = docs
            .iter()
            .any(|doc| pdfce_core::signature::census(doc).any());

        match pdfce_core::pageops::merge(&views, &titles) {
            Ok((bytes, report)) => match write_atomic(&path, &bytes) {
                Ok(()) => {
                    let mut note = ui_text::merge_succeeded(
                        &path,
                        self.merge_inputs.len(),
                        report.pages,
                        report.form_fields_renamed,
                    );
                    if signed {
                        note.push(' ');
                        note.push_str(ui_text::merge_note_unsigned_output());
                    }
                    self.edit_note = Some(note);
                }
                Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
            },
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
    }

    /// Apply one collected [`Action`].
    ///
    /// `ctx` is needed only by the clipboard actions, which is why it is
    /// a parameter rather than stored state: the alternative would be a
    /// `Context` clone living on `PdfceApp`, which is a second handle to
    /// the same thing and an invitation for some future action to reach
    /// the UI outside a frame.
    ///
    /// ## The pending-confirmation gate
    ///
    /// A pending confirmation makes this function a **one-question
    /// gate**: while `pending_save` or `pending_copy` is set, the only
    /// actions that survive are that question's own answers. Everything
    /// else — every toolbar and dock click outside the window's ~520px
    /// footprint, and every keyboard chord, since
    /// [`collect_keyboard_actions`] is a free function with no
    /// visibility into either field — is dropped for the frame.
    ///
    /// This is the enforcement point rather than a paint-order or
    /// `ui.disable()` trick, because paint order only blocks the pixels
    /// the window physically covers and disabling every panel would put
    /// the rule in a dozen places instead of one. The check happens here
    /// so that no future action can be added that quietly bypasses it.
    ///
    /// The bug it closes was found by `pdfce-ui-specialist` while
    /// reviewing this Pass and is written up in
    /// `docs/ui_specs/pass-4-text-extraction.md` §3.1: without the gate,
    /// an operator could open the unreliable-copy confirmation and then
    /// press `Ctrl+S`, setting `pending_save` as well. Both windows are
    /// centre-anchored at the same size, so they render exactly on top
    /// of each other and only the later-painted one can receive clicks —
    /// leaving the earlier question invisible and unanswerable. The
    /// missing guard predates Pass 4 (it was always true of
    /// `pending_save` alone), but Pass 4 is what made it *collidable* by
    /// adding a second independent pending state.
    fn apply(&mut self, action: Action, ctx: &egui::Context, pixels_per_point: f32) {
        if self.pending_copy.is_some()
            && !matches!(
                action,
                Action::ConfirmPendingCopy | Action::CancelPendingCopy
            )
        {
            return;
        }
        if self.pending_save.is_some()
            && !matches!(
                action,
                Action::ConfirmPendingSave | Action::CancelPendingSave
            )
        {
            return;
        }
        // Pass 12.0 substrate: the ONE enforcement point for a tool's
        // in-progress gesture (spec §3.3). Any action that is not itself
        // part of continuing/cancelling/committing the gesture consults the
        // active tool's own gesture state and discards or commits it before
        // proceeding — a single place the question is ever asked, so Pass
        // 6.1 (discard) and Pass 7 (commit) each replace only the *result*,
        // never add a second enforcement point. This Pass has no tool and
        // hence no gesture, so it is always a no-op (proven below).
        self.resolve_gesture_interrupt(action);
        match action {
            Action::Open => {
                self.open_dialog();
                return;
            }
            Action::ToggleRail => {
                self.rail_expanded = !self.rail_expanded;
                return;
            }
            Action::ToggleAnnotations => {
                // View-state only: flip the per-document flag; the texture
                // staleness check in `settle_and_rasterize` sees `t.
                // annotations != annotations_visible` and re-rasterizes the
                // current page next frame. No undo entry — no bytes change.
                if let Status::Open(doc) = &mut self.status {
                    doc.annotations_visible = !doc.annotations_visible;
                }
                return;
            }
            Action::ToggleProperties => {
                self.properties_open = !self.properties_open;
                if self.properties_open
                    && let Status::Open(doc) = &mut self.status
                {
                    doc.seed_properties_draft();
                }
                return;
            }
            Action::ApplyProperties => {
                self.apply_properties();
                if let Status::Open(doc) = &mut self.status {
                    doc.refresh_pages();
                }
                return;
            }
            Action::Save => {
                self.begin_save();
                return;
            }
            Action::ToggleTools => {
                self.tools_open = !self.tools_open;
                return;
            }
            Action::DeleteSelection => {
                self.delete_selection();
                return;
            }
            Action::RotateSelection(delta) => {
                self.rotate_selection(delta);
                return;
            }
            Action::AddMarkupShape(kind) => {
                self.add_markup_shape(kind);
                return;
            }
            Action::OpenTextEntry(kind) => {
                self.pending_text_kind = Some(kind);
                return;
            }
            Action::AddPendingText => {
                self.add_pending_text();
                return;
            }
            Action::CancelTextEntry => {
                self.pending_text_kind = None;
                self.text_input.clear();
                return;
            }
            // Pass 12.0 substrate. Entering a tool is uninhabited this Pass
            // (`CanvasTool` has no variants), so `tool` is only ever `None`
            // — exit-to-view-mode, which sets the (already-`None`) field.
            // `CancelToolGesture` has no gesture to cancel yet. Both are
            // no-ops this Pass but are the live dispatch every future tool
            // Pass routes through (spec §3.2).
            Action::SelectCanvasTool(tool) => {
                // Pass 16.2 §5.1: the default add-text font is read BEFORE
                // borrowing `doc` (it lives on `self`), so tool entry can seed
                // the property bar without a second borrow.
                let default_add_text_font = self.default_add_text_font;
                if let Status::Open(doc) = &mut self.status {
                    doc.active_tool = tool;
                    // Pass 14.3/16.2: entering a tool builds ITS per-page state
                    // and tears down the OTHER tool's — the §0.1 mutual
                    // exclusion a single `active_tool` already guarantees. The
                    // dispatch is the pure `tool_builds_*` predicates themselves
                    // (headless-tested to be never both true), so a stale
                    // caret/draft never survives a tool switch.
                    if canvas::tool_builds_text_edit(tool) {
                        doc.build_text_edit_state();
                        doc.add_text = None;
                    } else if canvas::tool_builds_add_text(tool) {
                        doc.build_add_text_state(default_add_text_font);
                        doc.text_edit = None;
                    } else {
                        doc.text_edit = None;
                        doc.add_text = None;
                    }
                }
                return;
            }
            Action::CancelToolGesture => {
                // The gesture-interrupt allow-list (`resolve_gesture_interrupt`)
                // already declined to touch the gesture for this action.
                // Pass 14.3 §3.5-step-1 / §6.2: cancel the in-progress pending
                // text edit WITHOUT exiting the tool (the first stage of the
                // two-stage Escape) — reject the compose buffer, stay editing.
                if let Status::Open(doc) = &mut self.status {
                    if let Some(state) = doc.text_edit.as_mut() {
                        // Pass 15.2 §7.1/§1.4: Esc rejects whichever review is
                        // in progress (single-run edit OR reflow) without
                        // exiting the tool. Mutually exclusive, so clearing both
                        // is correct.
                        state.pending = None;
                        state.reflow = None;
                    }
                    // Pass 16.2 §7.3/§8: Esc rejects an in-progress add-text
                    // draft (nothing was ever written) and cancels any active
                    // rubber-band drag, without exiting the tool.
                    if let Some(state) = doc.add_text.as_mut() {
                        state.draft = None;
                        state.drag_anchor = None;
                    }
                }
                return;
            }
            Action::MoveSelection(delta) => {
                self.move_selection(delta);
                return;
            }
            Action::DropDragged(slot) => {
                self.drop_dragged(slot);
                return;
            }
            Action::ExtractSelection => {
                self.extract_selection();
                return;
            }
            Action::ConfirmPendingSave => {
                self.pending_save = None;
                self.save_dialog();
                if matches!(self.save_result, Some(SaveOutcome::Saved { .. })) {
                    // A durable record, because a modal is dismissed and
                    // forgotten and this question gets asked again later.
                    self.edit_note = Some(ui_text::save_signature_invalidated_note().to_owned());
                }
                return;
            }
            Action::CancelPendingSave => {
                self.pending_save = None;
                return;
            }
            Action::CopyText(scope) => {
                self.begin_copy_text(ctx, scope);
                return;
            }
            Action::ConfirmPendingCopy => {
                if let Some(pending) = self.pending_copy.take() {
                    ctx.copy_text(pending.text);
                    self.copy_result = Some(pending.outcome);
                }
                return;
            }
            Action::CancelPendingCopy => {
                self.pending_copy = None;
                return;
            }
            Action::CommitMerge => {
                self.commit_merge();
                return;
            }
            _ => {}
        }
        let Status::Open(doc) = &mut self.status else {
            return; // every remaining action needs an open document
        };
        let count = doc.pages.len();
        let max_zoom = viewer::max_zoom_for_page(doc.current_extent(), pixels_per_point);

        // Every zoom-affecting action except `ZoomBy` is a *discrete
        // command* — a click or a chord, with no gesture in flight —
        // and so bypasses the debounce and commits on the next frame.
        // `ZoomBy` is the continuous ctrl+scroll path and is the one
        // action that must wait for the gesture to settle.
        doc.zoom_commanded = action.is_discrete_zoom_command();

        match action {
            // Handled by the early returns above, before the
            // open-document guard — these are the actions that are
            // meaningful with no document open, or that need `&mut self`
            // rather than `&mut OpenDoc`.
            Action::Open
            | Action::ToggleRail
            | Action::ToggleAnnotations
            | Action::ToggleProperties
            | Action::ApplyProperties
            | Action::Save
            | Action::ToggleTools
            | Action::DeleteSelection
            | Action::RotateSelection(_)
            | Action::AddMarkupShape(_)
            | Action::OpenTextEntry(_)
            | Action::AddPendingText
            | Action::CancelTextEntry
            | Action::SelectCanvasTool(_)
            | Action::CancelToolGesture
            | Action::MoveSelection(_)
            | Action::DropDragged(_)
            | Action::ExtractSelection
            | Action::ConfirmPendingSave
            | Action::CancelPendingSave
            | Action::CopyText(_)
            | Action::ConfirmPendingCopy
            | Action::CancelPendingCopy
            | Action::CommitMerge => unreachable!(),
            // Editing. `rotate_page_by` composes with the page's
            // *effective* rotation — which may be inherited from an
            // ancestor page-tree node — so two clicks of "turn right"
            // always land 180° from where they started, whatever the
            // file's structure. A refusal is impossible for a ±90 turn
            // on a page the view is already displaying, so the result is
            // deliberately discarded rather than reported through a
            // channel the operator would never see.
            // Rotation now goes through the batch path even for one
            // page: `rotate_pages` is the certification-gated entry
            // point, and having the toolbar bypass a gate the selection
            // bar honours is exactly the kind of divergence that ships.
            Action::RotateLeft | Action::RotateRight => {
                let delta = if action == Action::RotateLeft {
                    -90
                } else {
                    90
                };
                let pages = if doc.selected_pages.is_empty() {
                    vec![doc.view.page_index]
                } else {
                    doc.selection()
                };
                let _ = doc.session.rotate_pages(&pages, delta);
                doc.refresh_pages();
            }
            // Undo/redo can change the page COUNT now, so the selection
            // is re-bounded: a stale index would make the next batch
            // action address a page that no longer exists.
            Action::Undo => {
                doc.session.undo();
                doc.refresh_pages();
                doc.clamp_selection();
                doc.seed_properties_draft();
            }
            Action::Redo => {
                doc.session.redo();
                doc.refresh_pages();
                doc.clamp_selection();
                doc.seed_properties_draft();
            }
            // Selection lives on the open document, so these arms sit
            // below the open-document guard rather than in the early
            // returns above.
            Action::TogglePageSelection(index) => {
                if !doc.selected_pages.insert(index) {
                    doc.selected_pages.remove(&index);
                }
                doc.selection_anchor = Some(index);
            }
            Action::SelectRangeTo(index) => {
                // Shift+click extends from the last page the operator
                // touched. With no anchor it behaves as a plain toggle,
                // which is what every list control does.
                let anchor = doc.selection_anchor.unwrap_or(index);
                let (low, high) = if anchor <= index {
                    (anchor, index)
                } else {
                    (index, anchor)
                };
                doc.selected_pages.extend(low..=high);
            }
            Action::ClearSelection => {
                doc.selected_pages.clear();
                doc.selection_anchor = None;
            }
            // Pass 12.0 substrate: clear the canvas selection (spec §3.5
            // step 3). A view-state change, never an edit. Always a no-op
            // this Pass (the set is always empty); wired for Pass 9a.
            Action::ClearCanvasSelection => doc.canvas_selection.clear(),
            Action::FirstPage => doc.view.go_to_page(0, count),
            Action::PrevPage => doc.view.prev_page(count),
            Action::NextPage => doc.view.next_page(count),
            Action::LastPage => doc.view.go_to_page(count.saturating_sub(1), count),
            Action::GoToPage(i) => doc.view.go_to_page(i, count),
            Action::ZoomIn => doc.view.zoom_in(max_zoom),
            Action::ZoomOut => doc.view.zoom_out(max_zoom),
            Action::ZoomActualSize => doc.view.set_zoom(1.0, max_zoom),
            Action::Fit(mode) => doc.view.set_fit(mode),
            Action::ZoomBy(factor) => doc.view.zoom_by(factor, max_zoom),
        }
    }

    /// The one enforcement point for a tool's in-progress gesture
    /// (spec §3.3), called at the top of [`Self::apply`] for every action.
    ///
    /// Actions that ARE the gesture continuing/committing itself
    /// ([`Action::SelectCanvasTool`], [`Action::CancelToolGesture`], plus a
    /// future `CommitToolGesture` each tool Pass adds to this allow-list)
    /// leave the gesture alone; every other action consults the active
    /// tool's own gesture state ([`Self::current_gesture_interrupt`]) and
    /// discards or commits it before proceeding. This Pass has no tool, so
    /// `current_gesture_interrupt` always returns
    /// [`GestureInterrupt::Nothing`] and this is a no-op — the discard/commit
    /// arms exist for Pass 6.1/7 to fill, never reached today.
    fn resolve_gesture_interrupt(&mut self, incoming: Action) {
        if matches!(
            incoming,
            Action::SelectCanvasTool(_) | Action::CancelToolGesture
        ) {
            return;
        }
        match self.current_gesture_interrupt() {
            GestureInterrupt::Nothing => {}
            GestureInterrupt::Discard => self.discard_active_gesture(),
            GestureInterrupt::Commit => self.commit_active_gesture(),
        }
    }

    /// What the active tool's in-progress gesture would do if interrupted
    /// right now (spec §3.3). **Always [`GestureInterrupt::Nothing`] this
    /// Pass** — no tool exists to hold a gesture. Pass 6.1 replaces this
    /// body with a query against its `DrawState` (returns `Discard`); Pass 7
    /// with a query against its text-field draft flag (returns `Commit`).
    fn current_gesture_interrupt(&self) -> GestureInterrupt {
        // Pass 14.3 §6.2: an uncommitted text edit IS this tool's discardable
        // gesture. Nothing has been written to the `EditSession` yet, so any
        // unrelated action (Undo, Save, page nav, opening a panel) discards it
        // — the same Discard policy as Pass 6.1's half-drawn shape, NOT Pass
        // 7's Commit policy (a text edit's whole design is its separate Accept
        // gesture, so Discard-on-interrupt is what makes "operator-accepted,
        // never silent" mean something).
        match &self.status {
            // Pass 15.2 §1.4: an in-progress reflow review is the SAME class of
            // discardable, never-yet-written gesture as a `PendingEdit` — one
            // more disjunct on this existing query, not a second enforcement
            // point.
            Status::Open(doc)
                if doc
                    .text_edit
                    .as_ref()
                    .is_some_and(|s| s.pending.is_some() || s.reflow.is_some())
                    // Pass 16.2 §8: an in-progress add-text draft is the SAME
                    // class of discardable, never-yet-written gesture — one more
                    // disjunct on this ONE query, not a second enforcement point.
                    || doc.add_text.as_ref().is_some_and(|s| s.draft.is_some()) =>
            {
                GestureInterrupt::Discard
            }
            _ => GestureInterrupt::Nothing,
        }
    }

    /// Discard the active tool's in-progress gesture (spec §3.3). No-op this
    /// Pass — reached only via [`GestureInterrupt::Discard`], which
    /// [`Self::current_gesture_interrupt`] never returns until a real tool
    /// exists. Pass 6.1 clears its half-drawn shape's `DrawState` here.
    fn discard_active_gesture(&mut self) {
        // Pass 14.3 §6.2: discard the tool's uncommitted `PendingEdit`. The
        // compose buffer was never written anywhere, so the only loss is the
        // unaccepted keystrokes (the low-stakes, reversible-action case).
        if let Status::Open(doc) = &mut self.status {
            if let Some(state) = doc.text_edit.as_mut() {
                // Pass 15.2 §1.4: discard whichever uncommitted derived state is
                // active — they are mutually exclusive, so clearing both is safe
                // and neither was ever written to the `EditSession`.
                state.pending = None;
                state.reflow = None;
            }
            // Pass 16.2 §8: discard the add-text draft (never written anywhere,
            // so the only loss is the unaccepted new text) and any live drag.
            if let Some(state) = doc.add_text.as_mut() {
                state.draft = None;
                state.drag_anchor = None;
            }
        }
    }

    /// Commit the active tool's in-progress gesture as one `EditSession`
    /// command (spec §3.3). No-op this Pass — reached only via
    /// [`GestureInterrupt::Commit`], which [`Self::current_gesture_interrupt`]
    /// never returns until a real tool exists. Pass 7 commits its typed
    /// text-field draft here.
    fn commit_active_gesture(&mut self) {}
}

/// Whether a load failure is a *named pdfce capability gap* rather than
/// a problem with the file.
///
/// Matching on [`XrefErrorKind`] variants rather than on the error's
/// message text is the point: the message is presentation and may be
/// reworded, whereas these variants are `pdfce-core`'s deliberate,
/// documented "detected, never misparsed" refusals. `XrefErrorKind` is
/// `#[non_exhaustive]`, so future refusals must be added here
/// consciously — a new variant does not silently start reading as
/// corruption, it just does not match, and the wildcard sends it down
/// the "something is wrong with the file" path until someone decides
/// otherwise.
///
/// **History (deliberate, R1):** this list used to hold
/// `XrefStreamUnsupported` and `HybridUnsupported`. Cross-reference
/// streams, object streams and hybrid-reference files are now fully
/// supported, so those refusals no longer exist and those files take
/// the normal [`Status::Open`] path. Encryption (§7.6) took their place
/// as the live capability gap — the branch, its `ui_text` strings and
/// this classifier stay because "named gap ≠ damaged file" is a
/// standing honesty commitment, not scaffolding for one feature.
fn is_unsupported_structure(err: &DocError) -> bool {
    matches!(
        err,
        DocError::Xref(x) if matches!(x.kind, XrefErrorKind::EncryptionUnsupported)
    )
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

impl eframe::App for PdfceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let pixels_per_point = ctx.pixels_per_point();
        let mut actions: Vec<Action> = Vec::new();

        // Pass 12.0 substrate: the three flags Escape precedence resolves
        // through (spec §3.5), computed once from the open document (or
        // `false` when nothing is open). All three are always `false` this
        // Pass — uninhabited tool, no gesture, no-op provider — so Escape
        // still falls through to the existing rail-clear unchanged.
        let (canvas_tool_active, add_text_active, canvas_selection_nonempty) = match &self.status {
            Status::Open(doc) => (
                doc.active_tool.is_some(),
                doc.active_tool == Some(CanvasTool::AddText),
                !doc.canvas_selection.is_empty(),
            ),
            _ => (false, false, false),
        };
        let canvas_gesture_discardable =
            matches!(self.current_gesture_interrupt(), GestureInterrupt::Discard);
        collect_keyboard_actions(
            &ctx,
            &mut actions,
            canvas_tool_active,
            add_text_active,
            canvas_gesture_discardable,
            canvas_selection_nonempty,
        );

        // P0-5: drop-to-open. Read the frame's dropped files (set by the
        // egui-winit backend) and open the first `.pdf` among them —
        // deliberately restricted to states where nothing is at risk.
        // Accepting a drop while a document with unsaved edits is open
        // would silently discard those edits with no confirmation, a
        // hazard this Pass has no infrastructure for; drop-to-*replace* an
        // open document (reusing the `pending_save`-style confirmation) is
        // a named follow-up, not part of this fix.
        let dropped_pdf = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .find_map(|f| f.path.clone())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
        });
        if let Some(path) = dropped_pdf
            && matches!(
                self.status,
                Status::Idle | Status::Failed { .. } | Status::Unsupported { .. }
            )
        {
            self.open_path(path);
        }

        // Panels, in the order documented at the top of this file:
        // toolbar, status bar, rail, canvas.
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui, &mut actions));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if self.rail_expanded {
            egui::Panel::left("thumbnails")
                .default_size(raster::THUMBNAIL_WIDTH_PTS + 40.0)
                .show(ui, |ui| {
                    self.thumbnail_rail(ui, &mut actions, pixels_per_point)
                });
        }
        // The Tools dock claims RIGHT space, so like the rail it must be
        // added before the CentralPanel and after the full-width status
        // bar. Order is load-bearing here, not stylistic.
        if self.tools_open {
            egui::Panel::right("tools")
                .default_size(320.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("tools-dock")
                        .show(ui, |ui| self.tools_dock(ui, &mut actions));
                });
        }
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui, &mut actions));
        // Last, so the floating window draws over the panels rather
        // than being clipped by whichever one it happens to overlap.
        self.properties_window(&ctx, &mut actions);
        // Later still: the signature confirmation is the one blocking
        // question in this Pass, so it draws over everything including
        // Properties.
        self.signature_confirmation(&ctx, &mut actions);
        // And the copy confirmation alongside it: the same blocking
        // treatment, because a clipboard write is destructive to
        // whatever the operator had copied before.
        self.copy_confirmation(&ctx, &mut actions);
        // The Pass 6.2 text-entry popup: a small non-blocking window that
        // collects the text before authoring.
        self.text_entry_popup(&ctx, &mut actions);
        // The keyboard-shortcuts reference (P1-2): modeless, like
        // Properties — reading a reference while looking at the document
        // is exactly the use case, so it never blocks the canvas.
        self.shortcuts_window(&ctx);

        for action in actions {
            self.apply(action, &ctx, pixels_per_point);
        }

        // Raster bookkeeping runs after the actions, so a page or zoom
        // change made this frame is honoured this frame rather than
        // showing one frame of the previous page.
        self.settle_and_rasterize(&ctx, pixels_per_point);

        // P0-3: keep the window/taskbar title in step with the open file.
        // Computed after the actions are applied so `is_modified()`
        // reflects this frame's edits, and pushed to the platform layer
        // only when it changes (see `last_window_title`). A failed or
        // unsupported open does not rename the window — only a document
        // that actually opened earns a place in the window chrome.
        let wanted_title = match &self.status {
            Status::Open(doc) => ui_text::window_title_open(&doc.path, doc.session.is_modified()),
            _ => ui_text::window_title_idle().to_owned(),
        };
        if wanted_title != self.last_window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(wanted_title.clone()));
            self.last_window_title = wanted_title;
        }
    }
}

/// Read the keyboard and translate it into [`Action`]s.
///
/// Deliberately not registered as egui "shortcuts" on the buttons: these
/// must work when the canvas or the rail holds focus, which is where the
/// operator's attention actually is. `consume_key` is used so a chord
/// pdfce handles cannot also be interpreted by a widget underneath.
///
/// ## Escape precedence — the four-way chain, fixed once (spec §3.5)
///
/// Pressing Escape resolves through [`canvas::resolve_escape`] in strict
/// priority order, so every future tool-bearing Pass *reads* this precedence
/// rather than each re-deriving a partial version of it:
///
/// 1. A tool has an in-progress, **discardable** gesture → cancel the
///    gesture, stay in the tool ([`Action::CancelToolGesture`]).
/// 2. A tool is active with **no** gesture → exit to view mode
///    ([`Action::SelectCanvasTool`]`(None)`).
/// 3. No tool, the substrate's **canvas selection** is non-empty → clear it
///    ([`Action::ClearCanvasSelection`]).
/// 4. Otherwise → the EXISTING page-rail [`Action::ClearSelection`],
///    unchanged.
///
/// Branches 1–3 are **provably unreachable this Pass** — `tool_active` and
/// `gesture_discardable` are always `false` (uninhabited [`CanvasTool`], no
/// gesture state) and `canvas_selection_nonempty` is always `false` (the
/// no-op provider selects nothing) — so branch 4 is the only live path,
/// byte-for-byte as before. The three flags are computed once by the caller
/// from the open document (or `false` when nothing is open) and passed in,
/// because this free function has no view into `PdfceApp` state.
///
/// ## Global vs. focused dispatch — flagged, Pass 7's problem
///
/// This function reads `ctx.input()` **unconditionally every frame** — a
/// global keyboard model. Pass 7's real `TextEdit` overlay will need
/// focused-widget key handling (typing must not also fire a global chord);
/// reconciling the two is explicitly Pass 7's decision, named here so it is
/// not rediscovered as a surprise.
fn collect_keyboard_actions(
    ctx: &egui::Context,
    actions: &mut Vec<Action>,
    tool_active: bool,
    add_text_active: bool,
    gesture_discardable: bool,
    canvas_selection_nonempty: bool,
) {
    use egui::{Key, Modifiers};

    let mut pressed = |modifiers: Modifiers, key: Key, action: Action| {
        if ctx.input_mut(|i| i.consume_key(modifiers, key)) {
            actions.push(action);
        }
    };

    pressed(Modifiers::NONE, Key::PageDown, Action::NextPage);
    pressed(Modifiers::NONE, Key::PageUp, Action::PrevPage);
    // Home/End jump to the first/last page — but when a canvas tool is active
    // they are yielded to it (Pass 14.4 §4.5: Home/End become line-start/end
    // caret motion in the text-edit tool). Not consuming them here leaves them
    // for a focused canvas OR a focused property-bar DragValue, both of which
    // want Home/End for their own text navigation.
    if !tool_active {
        pressed(Modifiers::NONE, Key::Home, Action::FirstPage);
        pressed(Modifiers::NONE, Key::End, Action::LastPage);
    }

    pressed(Modifiers::COMMAND, Key::O, Action::Open);
    // Both spellings of "the + key": on most layouts the unshifted key
    // reports as `Equals` and only reports as `Plus` when shifted, so
    // binding only one of them makes Ctrl+Plus work on some keyboards
    // and not others.
    pressed(Modifiers::COMMAND, Key::Plus, Action::ZoomIn);
    pressed(Modifiers::COMMAND, Key::Equals, Action::ZoomIn);
    pressed(Modifiers::COMMAND, Key::Minus, Action::ZoomOut);
    pressed(Modifiers::COMMAND, Key::Num0, Action::ZoomActualSize);

    pressed(Modifiers::COMMAND, Key::S, Action::Save);
    // Undo/redo, with BOTH conventional redo chords bound. Ctrl+Y is
    // the Windows convention and Ctrl+Shift+Z is the cross-platform
    // one; operators arrive with one or the other in muscle memory and
    // an editor that honours only one feels broken to half of them.
    // Ctrl+Shift+Z is consumed BEFORE plain Ctrl+Z, because egui's
    // `consume_key` matches modifiers exactly and testing the more
    // specific chord first keeps the two from racing.
    pressed(
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::Z,
        Action::Redo,
    );
    pressed(Modifiers::COMMAND, Key::Y, Action::Redo);
    pressed(Modifiers::COMMAND, Key::Z, Action::Undo);

    // Pass 3.2. Rotation had NO keyboard shortcut at all through Pass
    // 3.1 — carried from that Pass's UI review and more pressing now
    // that rotation is a batch operation. `[` and `]` are the image-
    // viewer convention and are unclaimed here; deliberately not
    // Acrobat's bindings.
    pressed(Modifiers::NONE, Key::OpenBracket, Action::RotateLeft);
    pressed(Modifiers::NONE, Key::CloseBracket, Action::RotateRight);

    // Selection and reorder. Alt+arrow rather than a drag, because
    // drag-and-drop is not keyboard-operable and egui's assistive-
    // technology support is a known, tracked gap — this is the
    // compensating path, not a convenience.
    pressed(Modifiers::ALT, Key::ArrowUp, Action::MoveSelection(-1));
    pressed(Modifiers::ALT, Key::ArrowDown, Action::MoveSelection(1));
    // Delete/Backspace remove selected pages — but when a canvas tool is active
    // they are the text-edit tool's forward/backward character delete (Pass
    // 14.4 §6.1), so the global page-delete binding yields to the canvas. (This
    // also un-swallows the text-edit Backspace, which was previously consumed
    // here before the tool's own handler ran.)
    if !tool_active {
        pressed(Modifiers::NONE, Key::Delete, Action::DeleteSelection);
        pressed(Modifiers::NONE, Key::Backspace, Action::DeleteSelection);
    }

    // Pass 4. Ctrl+Shift+C copies the current page's text; the
    // whole-document copy stays menu-only, being both rarer and the one
    // with a visible delay on a long file.
    //
    // Plain Ctrl+C is deliberately left UNBOUND. It belongs to the
    // canvas text-selection slice that is deferred out of this Pass, and
    // binding it to page-copy now would mean taking the chord back from
    // operators later — a worse outcome than not having it yet.
    pressed(
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::C,
        Action::CopyText(CopyScope::Page),
    );

    // Pass 14.3 §1.1: Ctrl+E toggles the in-place text-edit tool. Verified
    // free before binding — no other `pressed` chord above claims `E`. A
    // repetitive power-user action, so a chord is appropriate (unlike Pass
    // 8's deliberate no-chord-on-Apply); entering/exiting the TOOL is not the
    // destructive act (an accepted edit is, and that goes through the ordinary
    // undo). Placed here, after the `pressed` closure's last use, so pushing
    // to `actions` directly is sound. The enter/exit choice reads the
    // frame-start `tool_active` flag this function is already handed.
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::E)) {
        actions.push(Action::SelectCanvasTool(if tool_active {
            None
        } else {
            Some(CanvasTool::TextEdit)
        }));
    }

    // Pass 16.2 §1.2: Ctrl+Shift+E toggles the Add-Page-Text tool. COMMAND+SHIFT
    // +E is unclaimed (only Ctrl+Shift+Z and Ctrl+Shift+C are bound above, and
    // `consume_key` matches modifiers EXACTLY, so this never races the plain
    // Ctrl+E Edit-Text chord). Shift signals the "create" variant. It toggles
    // AddText SPECIFICALLY (from `add_text_active`, not "any tool active"), so
    // pressing it while Edit Text is active switches straight to Add Text.
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::E)) {
        actions.push(Action::SelectCanvasTool(if add_text_active {
            None
        } else {
            Some(CanvasTool::AddText)
        }));
    }

    // Escape resolves through the four-way precedence chain (see this
    // function's doc comment, spec §3.5). Placed LAST — after the `pressed`
    // closure's final use, so its mutable borrow of `actions` has ended and
    // the computed outcome can be pushed directly. Consumed centrally so a
    // widget underneath cannot also act on it.
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        let outcome =
            canvas::resolve_escape(tool_active, gesture_discardable, canvas_selection_nonempty);
        actions.push(match outcome {
            EscapeOutcome::CancelGesture => Action::CancelToolGesture,
            EscapeOutcome::ExitTool => Action::SelectCanvasTool(None),
            EscapeOutcome::ClearCanvasSelection => Action::ClearCanvasSelection,
            EscapeOutcome::FallThroughToRailClear => Action::ClearSelection,
        });
    }
}

impl PdfceApp {
    // -- toolbar helpers --------------------------------------------

    /// Add an icon-only button with a consistent click target
    /// ([`ICON_BUTTON_SIZE`]) **and** an accessible name for screen
    /// readers (P1-6).
    ///
    /// egui derives a widget's accessible name from its visible label,
    /// which for these controls is a bare glyph (`↻`, `▶`, `▲`, …), so a
    /// screen reader would otherwise announce the raw character rather
    /// than what it does. [`egui::WidgetInfo::labeled`] overrides that
    /// with the tooltip text (the same `ui_text` string already shown on
    /// hover — no duplicated copy), so assistive technology announces
    /// "Rotate page clockwise" instead of "↻". The name reuses the
    /// tooltip verbatim, and the widget's enabled state is captured from
    /// the current `ui` so a disabled control announces as disabled.
    fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: impl Into<String>) -> egui::Response {
        let name = tooltip.into();
        let enabled = ui.is_enabled();
        let response = ui
            .add_sized(ICON_BUTTON_SIZE, egui::Button::new(glyph))
            .on_hover_text(name.clone());
        response.widget_info(move || {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone())
        });
        response
    }

    /// Style a toolbar toggle's label so its active state carries a
    /// non-colour cue (rule 6, P1-1): a selectable label's only built-in
    /// selected signal is a background-colour fill, so the active label is
    /// additionally shown **bold**. Applied consistently to every
    /// selectable toggle so colour is never the sole signal.
    fn toggle_label(selected: bool, text: &str) -> egui::RichText {
        let rt = egui::RichText::new(text);
        if selected { rt.strong() } else { rt }
    }

    // -- toolbar ----------------------------------------------------

    /// The top toolbar: separated groups, per the module docs.
    fn toolbar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            // Group: file.
            if ui
                .button(ui_text::open_button())
                .on_hover_text(ui_text::open_tooltip())
                .clicked()
            {
                actions.push(Action::Open);
            }
            // Save is in the file group, next to Open, and is hidden
            // rather than disabled when nothing is open — there is
            // nothing to discover about saving with no document.
            if matches!(self.status, Status::Open(_))
                && ui
                    .button(ui_text::save_button())
                    .on_hover_text(ui_text::save_tooltip())
                    .clicked()
            {
                actions.push(Action::Save);
            }
            ui.separator();

            // Group: view (rail toggle, annotation-visibility). These
            // govern what is on screen rather than the document, per the
            // placement taxonomy (view-state → toolbar view group).
            if Self::icon_button(
                ui,
                ui_text::rail_toggle_button(),
                ui_text::rail_toggle_tooltip(),
            )
            .clicked()
            {
                actions.push(Action::ToggleRail);
            }
            // Annotation-visibility toggle (Pass 6.0). A `SelectableLabel`
            // rather than a plain button: on a lightly-annotated page,
            // flipping it can produce no visible canvas change, so the
            // control must itself carry and announce its on/off state
            // (the ui-specialist's Rule-6 note) — the highlight, the bold
            // active label (P1-1) and a state-stating tooltip are the
            // non-colour cues. Wrapped in `add_sized` so it honours the
            // same click-target minimum as every other icon-only control
            // (P0-6), and given an explicit accessible name (P1-6) since
            // its visible label is a single glyph. Shown only with a
            // document open: unlike the rail, it acts on the current
            // page's canvas.
            if let Status::Open(doc) = &self.status {
                let visible = doc.annotations_visible;
                let tooltip = if visible {
                    ui_text::annotations_toggle_tooltip_shown()
                } else {
                    ui_text::annotations_toggle_tooltip_hidden()
                };
                let response = ui
                    .add_sized(
                        ICON_BUTTON_SIZE,
                        egui::Button::selectable(
                            visible,
                            Self::toggle_label(visible, ui_text::annotations_toggle_button()),
                        ),
                    )
                    .on_hover_text(tooltip);
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        visible,
                        tooltip,
                    )
                });
                if response.clicked() {
                    actions.push(Action::ToggleAnnotations);
                }
            }
            ui.separator();

            // Groups: navigation and zoom. Both are meaningless without
            // a document, so they are hidden rather than shown disabled
            // — there is nothing to discover about a page control when
            // no pages exist.
            if let Status::Open(doc) = &self.status {
                let count = doc.pages.len();
                let current = doc.view.page_index + 1;

                ui.add_enabled_ui(doc.view.page_index > 0, |ui| {
                    if Self::icon_button(
                        ui,
                        ui_text::prev_page_button(),
                        ui_text::prev_page_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::PrevPage);
                    }
                });
                ui.label(ui_text::page_nav_label(current, count));
                ui.add_enabled_ui(current < count, |ui| {
                    if Self::icon_button(
                        ui,
                        ui_text::next_page_button(),
                        ui_text::next_page_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::NextPage);
                    }
                });
                ui.separator();

                if Self::icon_button(ui, ui_text::zoom_out_button(), ui_text::zoom_out_tooltip())
                    .clicked()
                {
                    actions.push(Action::ZoomOut);
                }
                ui.label(ui_text::zoom_percent_label(doc.view.zoom_percent()));
                if Self::icon_button(ui, ui_text::zoom_in_button(), ui_text::zoom_in_tooltip())
                    .clicked()
                {
                    actions.push(Action::ZoomIn);
                }
                // Fit modes are shown as selectable, because they are
                // modes: the operator can see at a glance whether the
                // view is currently *being kept* fitted or is pinned.
                let fit_page = doc.view.fit == FitMode::Page;
                if ui
                    .selectable_label(
                        fit_page,
                        Self::toggle_label(fit_page, ui_text::fit_page_button()),
                    )
                    .on_hover_text(ui_text::fit_page_tooltip())
                    .clicked()
                {
                    actions.push(Action::Fit(FitMode::Page));
                }
                let fit_width = doc.view.fit == FitMode::Width;
                if ui
                    .selectable_label(
                        fit_width,
                        Self::toggle_label(fit_width, ui_text::fit_width_button()),
                    )
                    .on_hover_text(ui_text::fit_width_tooltip())
                    .clicked()
                {
                    actions.push(Action::Fit(FitMode::Width));
                }
                if ui
                    .button(ui_text::zoom_100_button())
                    .on_hover_text(ui_text::zoom_100_tooltip())
                    .clicked()
                {
                    actions.push(Action::ZoomActualSize);
                }
                ui.separator();

                // Group: edit. A new group rather than additions to an
                // existing one, exactly as the module docs anticipated:
                // rotation acts on the document, whereas everything to
                // its left acts on the view, and mixing the two would
                // make "does this button change my file?" unanswerable
                // at a glance.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    if Self::icon_button(
                        ui,
                        ui_text::rotate_left_button(),
                        ui_text::rotate_left_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::RotateLeft);
                    }
                    if Self::icon_button(
                        ui,
                        ui_text::rotate_right_button(),
                        ui_text::rotate_right_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::RotateRight);
                    }
                });
                if ui
                    .selectable_label(
                        self.properties_open,
                        Self::toggle_label(self.properties_open, ui_text::properties_button()),
                    )
                    .on_hover_text(ui_text::properties_tooltip())
                    .clicked()
                {
                    actions.push(Action::ToggleProperties);
                }
                // Pass 6.1 markup authoring — an edit tool, so it lives in
                // the toolbar edit group per the settled placement taxonomy
                // (edit → toolbar; ARCHITECTURE.md §12 continuation-23). A
                // menu rather than one button per subtype, because four
                // (eventually ten) shape tools would swamp the group. The
                // canvas drawing tools of the ui-spec are a follow-up slice;
                // this minimal affordance authors a default-placed shape on
                // the current page through the same command path.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    ui.menu_button(ui_text::markup_menu_button(), |ui| {
                        ui.label(ui_text::markup_menu_hint());
                        // The current pen colour: changing it is not an
                        // edit (ui-spec §1.1), only authoring is.
                        ui.horizontal(|ui| {
                            ui.label(ui_text::markup_color_label());
                            ui.color_edit_button_srgba(&mut self.markup_color);
                        });
                        ui.separator();
                        for kind in [
                            GuiMarkupKind::Square,
                            GuiMarkupKind::Circle,
                            GuiMarkupKind::Line,
                            GuiMarkupKind::Highlight,
                        ] {
                            if ui.button(kind.label()).clicked() {
                                actions.push(Action::AddMarkupShape(kind));
                            }
                        }
                    })
                    .response
                    .on_hover_text(ui_text::markup_menu_tooltip());
                });

                // Pass 6.2 text-bearing authoring — the same edit-group,
                // same minimal-affordance approach as Markup. A menu opens
                // the text-entry popup; the actual authoring happens on
                // confirm (see the popup below). A full canvas text editor
                // is the named follow-up slice.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    ui.menu_button(ui_text::text_menu_button(), |ui| {
                        ui.label(ui_text::text_menu_hint());
                        // P1-3b: surface the same pen-colour control the
                        // Markup menu has, plus an honest note that it
                        // applies to the text box only — a FreeText box
                        // authored without ever opening Markup otherwise
                        // takes an unexplained default colour, and sticky
                        // notes/stamps deliberately ignore it.
                        ui.horizontal(|ui| {
                            ui.label(ui_text::markup_color_label());
                            ui.color_edit_button_srgba(&mut self.markup_color);
                        });
                        ui.label(ui_text::text_menu_color_note());
                        ui.separator();
                        for kind in [
                            GuiTextKind::FreeText,
                            GuiTextKind::Sticky,
                            GuiTextKind::Stamp,
                        ] {
                            if ui.button(kind.label()).clicked() {
                                actions.push(Action::OpenTextEntry(kind));
                            }
                        }
                    })
                    .response
                    .on_hover_text(ui_text::text_menu_tooltip());
                });

                // Pass 14.3 in-place page-text editing — a DISTINCT control
                // from Markup/Text (decision 014 §1 draws a hard line between
                // editing words already on the page and authoring new
                // annotations; conflating them re-introduces the confusion the
                // decision resolved). The first real `CanvasTool` occupant
                // (spec §1.1): a selectable toggle, same widget as the
                // annotation-visibility toggle above, greyed when there are no
                // pages to edit (§1.2). The tooltip's second sentence is the
                // required disambiguator from Markup/Text.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    let active = doc.active_tool == Some(CanvasTool::TextEdit);
                    let response = ui
                        .add_sized(
                            ICON_BUTTON_SIZE,
                            egui::Button::selectable(active, ui_text::edit_text_tool_button()),
                        )
                        .on_hover_text(ui_text::edit_text_tool_tooltip());
                    if response.clicked() {
                        actions.push(Action::SelectCanvasTool(if active {
                            None
                        } else {
                            Some(CanvasTool::TextEdit)
                        }));
                    }
                });

                // Pass 16.2 Add-Page-Text — the THIRD occupant of the page-text
                // family, immediately after Edit Text and inside the SAME visual
                // group (the adjacency signals "the page-content-editing pair,"
                // distinct from the Text ▾/Markup ▾ annotation cluster — spec
                // §1.2). A bare toggle, IDENTICAL widget/sizing to Edit Text; a
                // distinct "+ Aa" glyph (add, not the "✎" modify); greyed (not
                // hidden) with no pages. The tooltip is the R78 disambiguator
                // naming the competing Text ▾ and Edit Text controls (§1.1/§10).
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    let active = doc.active_tool == Some(CanvasTool::AddText);
                    let response = ui
                        .add_sized(
                            ICON_BUTTON_SIZE,
                            egui::Button::selectable(active, ui_text::add_text_tool_button()),
                        )
                        .on_hover_text(ui_text::add_text_tool_tooltip());
                    if response.clicked() {
                        actions.push(Action::SelectCanvasTool(if active {
                            None
                        } else {
                            Some(CanvasTool::AddText)
                        }));
                    }
                });
                ui.separator();

                // Group: history. Disabled rather than hidden, because
                // the *absence* of an Undo control and a greyed-out one
                // say different things — the second confirms there is
                // nothing to undo, which is information.
                // The tooltips name the specific operation rather than
                // saying "undo" twice: `EditSession` hands out a
                // structured `CommandKind` precisely so a front end can
                // say "Undo delete 3 pages" instead.
                ui.add_enabled_ui(doc.session.can_undo(), |ui| {
                    if Self::icon_button(
                        ui,
                        ui_text::undo_button(),
                        ui_text::undo_tooltip_for(doc.session.undo_kind()),
                    )
                    .clicked()
                    {
                        actions.push(Action::Undo);
                    }
                });
                ui.add_enabled_ui(doc.session.can_redo(), |ui| {
                    if Self::icon_button(
                        ui,
                        ui_text::redo_button(),
                        ui_text::redo_tooltip_for(doc.session.redo_kind()),
                    )
                    .clicked()
                    {
                        actions.push(Action::Redo);
                    }
                });
                ui.separator();
            }

            // P1-4: a fixed space before the ungrouped-utility cluster,
            // emitted unconditionally so the cluster starts from the same
            // offset whether or not a document is open (Copy-text only
            // shows with a document open, so without this the gap before
            // the cluster shifted with document state). A plain space, not
            // a `ui.separator()`: a separator would visually promote the
            // utility controls to a seventh "group", which the placement
            // taxonomy explicitly says not to do.
            ui.add_space(6.0);

            // Pass 4's only toolbar growth, and it takes the SAME
            // ungrouped-utility slot the Tools toggle established rather
            // than opening a seventh group. Copy-text belongs to neither
            // the view group (it changes nothing on screen) nor the edit
            // group (it structurally cannot touch the file), and forcing
            // it into either would make that group's own organizing
            // question unanswerable at a glance. It opens a menu because
            // the operator must choose a scope: a Copy button that
            // silently picked one would be exactly the guess this
            // feature exists not to make.
            if self.status_is_open() {
                ui.menu_button(ui_text::copy_text_button(), |ui| {
                    if ui
                        .button(ui_text::copy_page_text_menu_item())
                        .on_hover_text(ui_text::copy_page_text_tooltip())
                        .clicked()
                    {
                        actions.push(Action::CopyText(CopyScope::Page));
                        ui.close();
                    }
                    if ui
                        .button(ui_text::copy_document_text_menu_item())
                        .on_hover_text(ui_text::copy_document_text_tooltip())
                        .clicked()
                    {
                        actions.push(Action::CopyText(CopyScope::Document));
                        ui.close();
                    }
                })
                .response
                .on_hover_text(ui_text::copy_text_tooltip());
            }

            // The whole of Pass 3.2's toolbar growth: ONE toggle. Every
            // other new capability lives on the thumbnails (page-scoped)
            // or in the dock this opens (file-scoped). The toolbar is
            // capped at its existing six groups plus this.
            if ui
                .add_sized(ICON_BUTTON_SIZE, egui::Button::new(ui_text::tools_button()))
                .on_hover_text(ui_text::tools_tooltip())
                .clicked()
            {
                actions.push(Action::ToggleTools);
            }
            // Keyboard-shortcuts reference (P1-2), the other ungrouped
            // utility control: a disclosure surface, not an edit or a
            // document-scoped tool, so it sits beside Tools rather than in
            // any group. Shown always (its content is document-independent
            // — the chords work the same with or without a file open).
            if Self::icon_button(
                ui,
                ui_text::shortcuts_button(),
                ui_text::shortcuts_tooltip(),
            )
            .clicked()
            {
                self.shortcuts_open = !self.shortcuts_open;
            }

            // Right-aligned so the summary stays pinned to the row's far
            // edge instead of drifting rightward (and eventually
            // crowding/wrapping) as future Passes append tool groups to
            // the left-to-right sequence above.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(self.status_summary());
            });
        });
    }

    // -- document properties -----------------------------------------

    /// The document-properties panel: the `/Info` fields (§14.3.3) as
    /// editable text, with an explicit Apply.
    ///
    /// A floating window rather than a modal dialog. A modal would block
    /// the document the metadata describes, and the operator frequently
    /// wants to read the title page while typing the title.
    ///
    /// Apply is one undo step for the whole panel, however many fields
    /// changed. That matches the operator's mental model — they made one
    /// change to the document's properties — and it is why the draft
    /// text lives in [`OpenDoc::properties_draft`] rather than going
    /// straight into the session on each keystroke.
    /// The Pass 6.2 text-entry popup: collects the text for a FreeText /
    /// sticky / stamp, then authors it on confirm through the same
    /// `EditSession::add_text_annotation` path the CLI uses. A minimal
    /// affordance — not the in-canvas text editor (that is the named
    /// follow-up slice) — so it deliberately reuses the modeless
    /// `egui::Window` pattern rather than adding a blocking modal.
    fn text_entry_popup(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(kind) = self.pending_text_kind else {
            return;
        };
        let mut open = true;
        egui::Window::new(kind.label())
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(ui_text::text_input_label());
                ui.text_edit_multiline(&mut self.text_input);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(ui_text::text_add_button()).clicked() {
                        actions.push(Action::AddPendingText);
                    }
                    if ui.button(ui_text::text_cancel_button()).clicked() {
                        actions.push(Action::CancelTextEntry);
                    }
                });
            });
        // The window's own close (X) button cancels, same as Cancel.
        if !open {
            actions.push(Action::CancelTextEntry);
        }
    }

    fn properties_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.properties_open {
            return;
        }
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let mut open = true;
        egui::Window::new(ui_text::properties_window_title())
            .open(&mut open)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                if doc.properties_lossy {
                    // Honesty over tidiness: some stored bytes could not
                    // be decoded with certainty, so the operator is told
                    // before they overwrite them (the code that decides
                    // this is `pdfce_core::edit::decode_text_string`).
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        ui_text::properties_lossy_warning(),
                    );
                }
                egui::Grid::new("properties-grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        for (field, text) in &mut doc.properties_draft {
                            // Per-field lossy marking, carried from the
                            // Pass 3.1 review: the panel-level warning
                            // says SOMETHING here is uncertain, which
                            // leaves the operator guessing which box.
                            let lossy = doc
                                .session
                                .info_text(*field)
                                .is_some_and(|value| !value.exact);
                            let label = ui.label(ui_text::info_field_label(*field));
                            if lossy {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    ui_text::info_field_lossy_marker(),
                                )
                                .on_hover_text(ui_text::info_field_lossy_tooltip());
                                label.on_hover_text(ui_text::info_field_lossy_tooltip());
                            }
                            ui.add(
                                egui::TextEdit::singleline(text)
                                    .desired_width(f32::INFINITY)
                                    .hint_text(ui_text::info_field_hint()),
                            );
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.label(ui_text::properties_help());
                // Apply and Revert are greyed out when the draft already
                // matches the document: a live control that provably does
                // nothing trains the operator to distrust the panel.
                // Carried from the Pass 3.1 review.
                let dirty = doc.properties_draft.iter().any(|(field, text)| {
                    let stored = doc.session.info_text(*field).map(|value| value.text);
                    let wanted = if text.is_empty() {
                        None
                    } else {
                        Some(text.clone())
                    };
                    stored != wanted
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(dirty, egui::Button::new(ui_text::properties_apply_button()))
                        .on_hover_text(if dirty {
                            ui_text::properties_apply_tooltip()
                        } else {
                            ui_text::properties_apply_unchanged_tooltip()
                        })
                        .clicked()
                    {
                        actions.push(Action::ApplyProperties);
                    }
                    if ui
                        .add_enabled(
                            dirty,
                            egui::Button::new(ui_text::properties_revert_button()),
                        )
                        // P1-5: the disabled Revert now explains itself the
                        // way the disabled Apply beside it already does.
                        .on_hover_text(if dirty {
                            ui_text::properties_revert_tooltip()
                        } else {
                            ui_text::properties_revert_unchanged_tooltip()
                        })
                        .clicked()
                    {
                        doc.seed_properties_draft();
                    }
                });
            });
        if !open {
            self.properties_open = false;
        }
    }

    /// The keyboard-shortcuts reference window (P1-2).
    ///
    /// Modeless, like [`Self::properties_window`]: reading a shortcut
    /// reference while looking at the document is exactly the use case, so
    /// it never blocks the canvas. The whole chord list is one catalog
    /// entry ([`ui_text::shortcuts_reference`]) shown inside a scroll area,
    /// so a small window still reaches every line.
    fn shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.shortcuts_open {
            return;
        }
        let mut open = true;
        egui::Window::new(ui_text::shortcuts_window_title())
            .open(&mut open)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("shortcuts-scroll")
                    .show(ui, |ui| {
                        ui.label(ui_text::shortcuts_reference());
                    });
            });
        if !open {
            self.shortcuts_open = false;
        }
    }

    /// One-line document status for the toolbar. All text comes from
    /// [`ui_text`] (decision 002 R1 — this fn only selects which catalog
    /// entry applies).
    fn status_summary(&self) -> String {
        match &self.status {
            Status::Idle => ui_text::status_idle().to_owned(),
            Status::Open(doc) => ui_text::status_open(
                &doc.path,
                doc.session.document().version(),
                doc.pages.len(),
                doc.session.is_modified(),
            ),
            Status::Failed { path, .. } => ui_text::status_failed(path),
            Status::Unsupported { path, .. } => ui_text::status_unsupported(path),
        }
    }

    // -- status bar (R20 diagnostics) --------------------------------

    /// The bottom status bar: the render-honesty disclosure required by
    /// decision 004 rule R20.
    ///
    /// The affordance is present in *every* state, including the clean
    /// one. A control that appears only when something is wrong forces
    /// the operator to interpret its absence — "is this page clean, or
    /// did the indicator break?" — which is precisely the ambiguity R20
    /// exists to remove. Clean gets a positive statement and an expander
    /// that says what was checked, so "faithful" is verifiable rather
    /// than merely asserted.
    ///
    /// Diagnostics describe the page **currently on screen**, not the
    /// document, and are re-read from the cached texture each frame —
    /// there is no separate state to keep in sync.
    ///
    /// P0-4: the body is wrapped in a height-capped vertical scroll area.
    /// Nothing is hidden — every disclosure line the body emits is still
    /// present and still mandatory — but once the stack grows past
    /// [`STATUS_BAR_MAX_HEIGHT`] it becomes internally scrollable instead
    /// of consuming an ever-larger slice of the window and crowding the
    /// canvas. The body itself is factored into [`Self::status_bar_body`]
    /// so the cap is a pure wrapper with no re-indentation of the
    /// disclosure logic.
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("status-bar-scroll")
            .max_height(STATUS_BAR_MAX_HEIGHT)
            .show(ui, |ui| self.status_bar_body(ui));
    }

    /// The status bar's disclosure body (see [`Self::status_bar`] for why
    /// it is a separate method). Emits, in order: the edit narrator, the
    /// save result, the copy result, and — for an open document — the
    /// redaction-pending warning, the render diagnostics header, and the
    /// annotation census. None of these lines may be suppressed.
    fn status_bar_body(&mut self, ui: &mut egui::Ui) {
        // The save result sits above the render diagnostics and
        // persists until the next save. A transient toast would be the
        // conventional choice and is the wrong one here: "did that
        // write?" is a question the operator may ask a minute later,
        // and a message that has already faded cannot answer it.
        // The edit narrator sits with the save result, in the same
        // channel and style — the UI spec asks for the delete disclosure
        // here rather than on a new surface, and a second notification
        // style is a second thing for the operator to learn.
        // Decision 013: the cross-reference-recovery disclosure sits in the
        // same narrator channel, in a warning colour because it changes how
        // a save behaves (forced full rewrite; incremental refused).
        if let Some(note) = &self.recovery_note {
            ui.colored_label(ui.visuals().warn_fg_color, note);
        }
        if let Some(note) = &self.edit_note {
            ui.label(note);
        }
        match &self.save_result {
            Some(SaveOutcome::Saved {
                path,
                objects,
                appended,
                promoted,
            }) => {
                ui.label(ui_text::save_succeeded(path, *objects, *appended));
                if *promoted > 0 {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        ui_text::save_promoted_objects(*promoted),
                    );
                }
            }
            Some(SaveOutcome::Failed(message)) => {
                ui.colored_label(ui.visuals().error_fg_color, ui_text::save_failed(message));
            }
            None => {}
        }

        // The copy result sits with the save result and the edit
        // narrator — the "did my last requested action work" family,
        // which persists until superseded. Deliberately NOT merged into
        // the render-diagnostics header below: those are re-derived from
        // the current page's texture every frame and are therefore
        // always about what is on screen, while this is a snapshot of a
        // copy that may have named a different page entirely.
        self.copy_result_bar(ui);

        let Status::Open(doc) = &self.status else {
            ui.label(ui_text::diagnostics_no_document());
            return;
        };
        let Some(texture) = &doc.page_texture else {
            ui.label(ui_text::diagnostics_no_document());
            return;
        };
        let d = &texture.diagnostics;
        // Document-scoped /NeedAppearances disclosure (R51) — computed
        // from the current document each frame (a cheap catalog lookup),
        // since it is not part of the per-page render diagnostics.
        let need_appearances = pdfce_core::annot::need_appearances(doc.session.document());

        // Pass 8 (R52 / ui-spec §GUI, the ONE non-negotiable redaction GUI
        // item): a PERSISTENT disclosure of UNAPPLIED /Redact marks,
        // computed from the document's own annotations every frame — never a
        // session counter — so a marked-but-not-applied document can never be
        // mistaken for a redacted one (the #1 real-world redaction failure:
        // saving a marked file believing the content is gone).
        let pending_redactions = pdfce_core::redact::count_redaction_marks(doc.session.document());
        if pending_redactions > 0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                ui_text::redaction_marks_pending(pending_redactions),
            );
        }

        // "Unsupported items" folds the three counters that mean
        // "pdfce did not draw something" into one headline number; the
        // expander separates them again. A summary line with five
        // numbers in it is a summary nobody reads.
        let unsupported = d.fonts_unsupported
            + d.deferred_ops
            + d.unknown_ops
            + d.images_unsupported
            + d.xobject_depth_overflows;
        // decision 012 / R62: a supplied glyph is still a SUBSTITUTE, not
        // the document's own program, so a page rendered from supplied
        // faces is NOT "clean" — the banner must not claim the letterforms
        // are the document's own. `glyphs_supplied` therefore joins the
        // clean check.
        let clean = unsupported == 0
            && d.glyphs_substituted == 0
            && d.glyphs_supplied == 0
            && d.glyphs_notdef == 0
            && d.tolerated == 0
            && d.image_notes.is_empty();

        let summary = if clean {
            ui_text::diagnostics_clean().to_owned()
        } else {
            // The one-liner names the two substitute trust levels
            // DISTINCTLY (R62): bundled ("pdfce guessed") vs supplied
            // ("the operator's own face") are never folded into one count.
            ui_text::diagnostics_summary(d.glyphs_substituted, d.glyphs_supplied, unsupported)
        };

        // A non-modal collapsing header: discoverable, expandable in
        // place, and it never blocks the document behind a dialog.
        let header = egui::CollapsingHeader::new(summary)
            .id_salt("diagnostics")
            .open(Some(self.diagnostics_expanded));
        let response = header.show(ui, |ui| {
            if clean {
                ui.label(ui_text::diagnostics_clean_detail());
                return;
            }
            ui.label(ui_text::diagnostics_detail_heading());
            if d.glyphs_substituted > 0 {
                ui.label(ui_text::diagnostics_substituted_fonts(
                    d.glyphs_substituted,
                    &ui_text::join_names(&d.substituted_fonts),
                ));
            }
            // decision 012 / R62: supplied faces on their own line, worded
            // to make the shapes-not-layout point (positions come from the
            // PDF's own widths) independently — an operator reading this
            // line must not infer that supplying a font made the layout
            // authoritative, and must not have to remember that caveat
            // from the bundled line above.
            if d.glyphs_supplied > 0 {
                ui.label(ui_text::diagnostics_supplied_fonts(
                    d.glyphs_supplied,
                    &ui_text::join_names(&d.supplied_fonts),
                ));
            }
            if d.fonts_unsupported > 0 {
                ui.label(ui_text::diagnostics_fonts_unsupported(
                    d.fonts_unsupported,
                    &d.fonts_unsupported_by_reason,
                ));
            }
            if d.glyphs_notdef > 0 {
                ui.label(ui_text::diagnostics_glyphs_notdef(d.glyphs_notdef));
            }
            if d.deferred_ops > 0 {
                ui.label(ui_text::diagnostics_deferred_ops(
                    d.deferred_ops,
                    &ui_text::join_names(&d.sample_ops),
                ));
            }
            if d.unknown_ops > 0 {
                ui.label(ui_text::diagnostics_unknown_ops(d.unknown_ops));
            }
            if d.images_unsupported > 0 {
                ui.label(ui_text::diagnostics_images_unsupported(
                    d.images_unsupported,
                    &ui_text::join_names(&d.image_notes),
                ));
                // The two codec-specific breakdowns are shown BELOW the
                // headline rather than instead of it: the headline
                // answers "is this page complete?", these answer "what
                // would fix it?" — and those have different answers for
                // a missing codec and a missing codec variant
                // (decision 005 R27).
                if d.images_codec_unsupported > 0 {
                    ui.label(ui_text::diagnostics_codec_unsupported(
                        d.images_codec_unsupported,
                        &ui_text::join_names(&d.image_notes),
                    ));
                }
                if !d.codec_feature_unsupported.is_empty() {
                    let features: Vec<String> = d
                        .codec_feature_unsupported
                        .keys()
                        .map(|f| (*f).to_owned())
                        .collect();
                    ui.label(ui_text::diagnostics_codec_feature_unsupported(
                        d.codec_feature_unsupported.values().sum(),
                        &ui_text::join_names(&features),
                    ));
                }
            } else if !d.image_notes.is_empty() {
                // Images DID draw, but with a divergence worth naming
                // (a deferred /SMask being much the commonest).
                ui.label(ui_text::diagnostics_image_divergences(
                    &ui_text::join_names(&d.image_notes),
                ));
            }
            // These three concern images that DID draw, so they sit
            // outside the drawn/not-drawn branch above.
            if d.codec_geometry_mismatch > 0 {
                ui.label(ui_text::diagnostics_codec_geometry_mismatch(
                    d.codec_geometry_mismatch,
                ));
            }
            if d.dct_cmyk_images > 0 {
                ui.label(ui_text::diagnostics_dct_cmyk(d.dct_cmyk_images));
            }
            if d.dct_cmyk_polarity_unverifiable > 0 {
                ui.label(ui_text::diagnostics_dct_cmyk_unverifiable(
                    d.dct_cmyk_polarity_unverifiable,
                ));
            }
            if d.jpx_smask_in_data_preblended > 0 {
                ui.label(ui_text::diagnostics_jpx_preblended(
                    d.jpx_smask_in_data_preblended,
                ));
            }
            if d.lzw_framing_anomalies > 0 {
                ui.label(ui_text::diagnostics_lzw_framing(d.lzw_framing_anomalies));
            }
            if d.xobject_depth_overflows > 0 {
                ui.label(ui_text::diagnostics_xobject_overflows(
                    d.xobject_depth_overflows,
                ));
            }
            if d.tolerated > 0 {
                ui.label(ui_text::diagnostics_tolerated(d.tolerated));
            }
        });
        if response.header_response.clicked() {
            self.diagnostics_expanded = !self.diagnostics_expanded;
        }

        // Pass 6.0 annotation disclosure (ISO 32000-1 §12.5). A separate
        // always-evaluated line rather than a branch inside the content
        // header above: annotation faithfulness is a distinct concern from
        // content-render faithfulness, and this keeps the disclosure alive
        // through the content header's clean-return path. The toggle-off,
        // Hidden/NoView, and /NeedAppearances cases are all file facts the
        // operator is entitled to see whether or not the content rendered
        // cleanly (R50/R27/R51).
        annotation_status(ui, d, doc.annotations_visible, need_appearances);
    }

    // -- thumbnail rail ----------------------------------------------
    //
    // (annotation_status is a free fn below the impl.)

    /// The left thumbnail rail: navigation, batch selection, and reorder.
    ///
    /// Thumbnails are rasterized **lazily and viewport-bounded**: only
    /// pages whose row is actually visible get drawn, at most
    /// [`THUMBNAILS_PER_FRAME`] per frame. Rasterizing all pages at open
    /// time would stall the Open action for seconds on a long document
    /// to produce pictures nobody has scrolled to.
    ///
    /// A page not yet drawn shows a placeholder sized to that page's own
    /// aspect ratio, with its number. The number and the shape are both
    /// free (they come from the page tree, not from rendering), so the
    /// rail never has a row that says nothing. No spinner: a dozen
    /// spinning icons is motion, not information.
    ///
    /// ## Why page-scoped tools live here and not on the toolbar
    ///
    /// Delete, Extract and batch Rotate are meaningless with nothing
    /// selected. As toolbar buttons they would be permanently
    /// disabled-and-mysterious for most of a session; on the rail they
    /// appear exactly when they mean something, right beside the pages
    /// they act on.
    ///
    /// ## The selection hit-test, and a deliberate deviation
    ///
    /// The UI spec asks for the checkbox to be *"allocated and interacted
    /// separately"* from the thumbnail, and flags the hit-test question
    /// as something to verify rather than assume. pdfce instead gives the
    /// thumbnail **one** interaction and decides from the click's
    /// position which half was hit. Same behaviour — clicking the box
    /// selects without navigating, clicking the body navigates — with no
    /// dependence on which of two overlapping `interact` calls egui
    /// happens to prefer. Recorded as a deviation with its reason, per
    /// the spec's own preamble.
    fn thumbnail_rail(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut Vec<Action>,
        pixels_per_point: f32,
    ) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let ctx = ui.ctx().clone();
        let current = doc.view.page_index;
        let selected_count = doc.selected_pages.len();
        let width = (ui.available_width() - 12.0).clamp(48.0, raster::THUMBNAIL_WIDTH_PTS);
        let mut budget = THUMBNAILS_PER_FRAME;
        let mut more_pending = false;

        // The selection action bar: pinned ABOVE the scroll area so it
        // stays put while the rail scrolls, and HIDDEN rather than
        // disabled when nothing is selected — the same precedent the
        // Save button set, because there is nothing to discover about a
        // batch action with zero pages picked.
        if selected_count > 0 {
            ui.label(ui_text::selection_bar_summary(selected_count));
            ui.horizontal_wrapped(|ui| {
                if Self::icon_button(
                    ui,
                    ui_text::rotate_left_button(),
                    ui_text::batch_rotate_left_tooltip(selected_count),
                )
                .clicked()
                {
                    actions.push(Action::RotateSelection(-90));
                }
                if Self::icon_button(
                    ui,
                    ui_text::rotate_right_button(),
                    ui_text::batch_rotate_right_tooltip(selected_count),
                )
                .clicked()
                {
                    actions.push(Action::RotateSelection(90));
                }
                // The keyboard-operable reorder path. Drag-and-drop is
                // not reachable from a keyboard and egui's assistive-
                // technology support is a known gap, so these are the
                // compensating control, not a convenience.
                if Self::icon_button(
                    ui,
                    ui_text::move_selection_up_button(),
                    ui_text::move_selection_up_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::MoveSelection(-1));
                }
                if Self::icon_button(
                    ui,
                    ui_text::move_selection_down_button(),
                    ui_text::move_selection_down_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::MoveSelection(1));
                }
                if ui
                    .button(ui_text::selection_delete_button())
                    .on_hover_text(ui_text::selection_delete_tooltip(selected_count))
                    .clicked()
                {
                    actions.push(Action::DeleteSelection);
                }
                if ui
                    .button(ui_text::selection_extract_button())
                    .on_hover_text(ui_text::selection_extract_tooltip(selected_count))
                    .clicked()
                {
                    actions.push(Action::ExtractSelection);
                }
                if ui
                    .button(ui_text::selection_clear_button())
                    .on_hover_text(ui_text::selection_clear_tooltip())
                    .clicked()
                {
                    actions.push(Action::ClearSelection);
                }
            });
            ui.separator();
        }

        // Drag state, read once: whether a drag is in flight decides both
        // how thumbnails paint and whether a drop fires this frame.
        let dragging = doc.dragged_page;
        let mut hovered_slot: Option<usize> = None;
        let pointer_released = ctx.input(|i| i.pointer.any_released());
        let shift = ctx.input(|i| i.modifiers.shift);

        egui::ScrollArea::vertical()
            .id_salt("thumbnail-rail")
            .show(ui, |ui| {
                for index in 0..doc.pages.len() {
                    let Some(page) = doc.pages.get(index) else {
                        continue;
                    };
                    let (pw, ph) = viewer::page_extent_pts(page);
                    let height = if pw > 0.0 { width * ph / pw } else { width };
                    let size = egui::vec2(width, height);
                    let is_selected = doc.selected_pages.contains(&index);

                    let response = ui
                        .vertical_centered(|ui| {
                            // Allocating the exact rect first is what
                            // makes laziness possible AND keeps the
                            // scroll bar honest: every row occupies its
                            // final height whether or not its picture
                            // exists yet, so nothing jumps as
                            // thumbnails arrive.
                            let (rect, response) =
                                ui.allocate_exact_size(size, egui::Sense::click_and_drag());

                            if ui.is_rect_visible(rect) {
                                if let Some(texture) = doc.thumbnails.get(index) {
                                    egui::Image::from_texture(texture)
                                        .fit_to_exact_size(size)
                                        .paint_at(ui, rect);
                                } else {
                                    // Placeholder: page-shaped, bordered,
                                    // numbered.
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        ui.visuals().extreme_bg_color,
                                    );
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        ui.visuals().widgets.noninteractive.bg_stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                    if budget > 0 {
                                        if doc.thumbnails.is_pending(index) {
                                            budget -= 1;
                                            doc.thumbnails.build(
                                                &ctx,
                                                doc.session.document(),
                                                page,
                                                index,
                                                pixels_per_point,
                                            );
                                        }
                                    } else if doc.thumbnails.is_pending(index) {
                                        more_pending = true;
                                    }
                                }
                                // The current page gets a highlight
                                // ring, not just a caption change:
                                // "which page am I on" must be
                                // answerable at a glance.
                                if index == current {
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                // The selection checkbox. A glyph AND a
                                // fill, never colour alone: a colour-only
                                // state is invisible to a substantial
                                // fraction of operators.
                                let box_rect = selection_box(rect);
                                let visuals = ui.visuals();
                                ui.painter().rect_filled(
                                    box_rect,
                                    2.0,
                                    if is_selected {
                                        visuals.selection.bg_fill
                                    } else {
                                        visuals.extreme_bg_color
                                    },
                                );
                                ui.painter().rect_stroke(
                                    box_rect,
                                    2.0,
                                    visuals.widgets.active.fg_stroke,
                                    egui::StrokeKind::Inside,
                                );
                                if is_selected {
                                    ui.painter().text(
                                        box_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        ui_text::selection_check_glyph(),
                                        egui::FontId::proportional(13.0),
                                        visuals.selection.stroke.color,
                                    );
                                }
                                // A page being dragged renders ghosted,
                                // so the operator can see what is moving.
                                if dragging == Some(index) {
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        visuals.selection.bg_fill.gamma_multiply(0.35),
                                    );
                                }
                            }
                            ui.label(ui_text::thumbnail_page_number(index + 1));
                            (rect, response)
                        })
                        .inner;
                    let (rect, response) = response;

                    // The insertion line: painted between rows, tracking
                    // the pointer, so the drop position is visible before
                    // the drop rather than discovered after it.
                    if dragging.is_some() && response.hovered() {
                        let above = ctx
                            .pointer_interact_pos()
                            .is_none_or(|p| p.y < rect.center().y);
                        hovered_slot = Some(if above { index } else { index + 1 });
                        let y = if above { rect.top() } else { rect.bottom() };
                        ui.painter().hline(
                            rect.x_range(),
                            y,
                            egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                        );
                    }

                    if response.drag_started() {
                        doc.dragged_page = Some(index);
                    }

                    let response =
                        response.on_hover_text(ui_text::thumbnail_drag_tooltip(index + 1));
                    if response.clicked() {
                        // One interaction, two meanings, decided by
                        // WHERE the click landed — see the method docs
                        // for why this beats two overlapping interacts.
                        let on_checkbox = ctx
                            .pointer_interact_pos()
                            .is_some_and(|p| selection_box(rect).contains(p));
                        if on_checkbox {
                            actions.push(Action::TogglePageSelection(index));
                        } else if shift {
                            actions.push(Action::SelectRangeTo(index));
                        } else {
                            actions.push(Action::GoToPage(index));
                        }
                    }
                }
            });

        // A drop fires on release, wherever the pointer ended up. A
        // release outside every row cancels rather than dropping at a
        // guessed position — guessing is how a 400-page document gets
        // silently reordered.
        if pointer_released && doc.dragged_page.is_some() {
            match hovered_slot.or(doc.drop_target) {
                Some(slot) => actions.push(Action::DropDragged(slot)),
                None => doc.dragged_page = None,
            }
        } else {
            doc.drop_target = hovered_slot;
        }

        // Whatever the budget could not cover is picked up next frame —
        // requested explicitly, because otherwise the remaining
        // thumbnails would not appear until some unrelated event caused
        // a repaint.
        if more_pending {
            ctx.request_repaint();
        }
    }

    // -- canvas -------------------------------------------------------

    /// The central page canvas.
    ///
    /// The page is drawn at the size the **current** zoom implies, which
    /// is not necessarily the texture's native pixel size — that gap is
    /// exactly the debounce window, and letting egui scale the texture
    /// across it is what makes zooming feel continuous. See the module
    /// docs.
    fn canvas(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        match &self.status {
            Status::Idle => {
                // P0-5: a real empty state — the app name, an inline Open
                // affordance (a second, more discoverable entry point to
                // the same `Action::Open` the toolbar button uses), and a
                // hint that drop-to-open (handled in `ui`) works.
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(ui_text::empty_state_heading());
                        ui.add_space(8.0);
                        ui.label(ui_text::canvas_idle_hint());
                        ui.add_space(12.0);
                        if ui
                            .button(ui_text::open_button())
                            .on_hover_text(ui_text::open_tooltip())
                            .clicked()
                        {
                            actions.push(Action::Open);
                        }
                        ui.add_space(6.0);
                        ui.label(ui_text::canvas_idle_drop_hint());
                    });
                });
                return;
            }
            Status::Failed { path, message } => {
                let text = ui_text::canvas_failed(path, message);
                ui.centered_and_justified(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, text);
                });
                return;
            }
            Status::Unsupported { path, message } => {
                let text = ui_text::canvas_unsupported(path, message);
                // Deliberately NOT `error_fg_color`: this is not an
                // error, it is an unfinished feature, and painting it
                // red would contradict the words next to it.
                ui.centered_and_justified(|ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, text);
                });
                return;
            }
            Status::Open(_) => {}
        }

        // The viewport the fit modes are measured against, minus the
        // margin the page sits inside.
        let viewport = (
            (ui.available_width() - CANVAS_MARGIN).max(1.0),
            (ui.available_height() - CANVAS_MARGIN).max(1.0),
        );

        // decision 012 / Pass 14.3: capture the app-level font environment as
        // a borrow of a field disjoint from `self.status`, so the text-edit
        // tool's property bar can classify a run's font trust level through
        // the SAME shared classifier the raster path uses (the split-borrow
        // pattern `settle_and_rasterize` established).
        let font_env = &self.font_env;
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        if doc.pages.is_empty() {
            ui.centered_and_justified(|ui| ui.label(ui_text::canvas_no_pages()));
            return;
        }

        // Resolve a fit mode against this frame's viewport. Under
        // FitMode::None this is a no-op, so it is safe to call always.
        let extent = doc.current_extent();
        let max_zoom = viewer::max_zoom_for_page(extent, ui.ctx().pixels_per_point());
        doc.view.apply_fit(extent, viewport, max_zoom);

        if let Some(message) = &doc.render_error {
            let text = ui_text::canvas_render_failed(doc.view.page_index + 1, message);
            ui.centered_and_justified(|ui| {
                ui.colored_label(ui.visuals().error_fg_color, text);
            });
            return;
        }

        let display_size = egui::vec2(extent.0 * doc.view.zoom, extent.1 * doc.view.zoom);
        let texture = doc.page_texture.as_ref().map(|t| t.texture.clone());
        let zoom = doc.view.zoom;

        // Pass 12.0 substrate (spec §1). The full-page image gains an
        // explicit CLICK sense so the canvas is focusable (§1.4/§6.2) and
        // click-to-select routes through it (§4.2). It gains a DRAG sense
        // ONLY when a tool is active: egui 0.35's `ScrollArea` interacts its
        // drag-to-pan BEFORE its content ("or we will steal input from the
        // widgets we contain", `scroll_area.rs`), so an unconditionally-
        // draggable full-page image would suppress plain-drag-pans-the-canvas
        // even with `drag_to_scroll(true)`. Gating the drag sense on
        // `suppress_pan` is the §1.2 "consult the inner widget's response"
        // resolution: this Pass has no tool, so the image senses click only
        // and panning is byte-for-byte unchanged. `canvas_suppresses_pan`
        // (always `false` here) is the same pure function Pass 6.1/7 will
        // feed differently — neither invents its own `drag_to_scroll` call.
        let tool_active = doc.active_tool.is_some();
        let suppress_pan = canvas::canvas_suppresses_pan(tool_active, None);
        let canvas_sense = if suppress_pan {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::click()
        };

        // `ScrollSource::ALL` = scroll bars + plain mouse wheel +
        // drag-to-pan with a mouse (egui's default only enables
        // drag-scrolling on touch). Ctrl+wheel never reaches this,
        // because egui routes a modified wheel event to `zoom_delta`
        // instead of to the scroll delta — see the module docs.
        //
        // Pan suppression (spec §1.2/§1.3) is expressed by flipping this
        // source's `drag` off (egui 0.35 has no `drag_to_scroll` builder;
        // `ScrollSource.drag` is the knob). Always `DragScroll::Always` this
        // Pass, since `suppress_pan` is always `false`.
        let mut scroll_source = egui::scroll_area::ScrollSource::ALL;
        if suppress_pan {
            scroll_source.drag = egui::scroll_area::DragScroll::Never;
        }
        let image_response = egui::ScrollArea::both()
            .id_salt("page-canvas")
            .scroll_source(scroll_source)
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    if let Some(texture) = texture {
                        ui.add(
                            egui::Image::from_texture(&texture)
                                .fit_to_exact_size(display_size)
                                .sense(canvas_sense),
                        )
                    } else {
                        // First frame after an open: the texture is made
                        // at the end of this frame. Reserve the page's
                        // space (same sense) so nothing jumps when it
                        // arrives — and so the substrate's canonical canvas
                        // response exists even before the first raster.
                        ui.allocate_exact_size(display_size, canvas_sense).1
                    }
                })
                .inner
            })
            .inner;

        // This `image_response` — not the outer ScrollArea's — is the
        // substrate's canonical canvas response; its `.rect` is the
        // `image_rect` every §2 geometry call takes this frame.
        let image_rect = image_response.rect;

        // §1.4: clicking the canvas (or, once a tool drags, drag-starting it)
        // requests focus for the canvas's own id, making it a genuine Tab
        // stop (§6.2) rather than an inert image.
        if image_response.clicked() || image_response.drag_started() {
            image_response.request_focus();
        }

        // §4.2 selection dispatch — a plain/Shift click resolved against the
        // attached provider. With the no-op provider (this Pass) `hit_test`
        // returns `None`, so a plain click clears the already-empty selection
        // and Shift+click is a no-op — nothing ever selects. Marquee is
        // deliberately NOT wired: a drag-starting-on-empty-canvas conflicts
        // with pan, and that disambiguation is Pass 9a's call (§4.2), so
        // `hit_test_rect`/`selection_after_marquee` are proven in isolation
        // (canvas.rs tests) rather than wired to a live drag here.
        // Pass 14.3: when the in-place text-edit tool is active, the canvas is
        // its own caret/selection surface — clicks place a caret, not an
        // object selection — so it takes over the click dispatch and the
        // overlay entirely. Otherwise the Pass 12.0 object-selection path runs
        // unchanged (a no-op with the shippable empty provider).
        if doc.active_tool == Some(CanvasTool::TextEdit) {
            run_text_edit_tool(doc, ui, &image_response, image_rect, extent, zoom, font_env);
        } else if doc.active_tool == Some(CanvasTool::AddText) {
            // Pass 16.2: the Add-Page-Text tool owns the canvas — click places a
            // point caret, drag rubber-bands a wrap box, typing composes a live
            // preview, Accept commits ONE `CommandKind::AddText` (§3-§7). Its
            // own click/drag handler always means "place new text," so it needs
            // no hit-vs-miss branch (§0.1).
            run_add_text_tool(doc, ui, &image_response, image_rect, extent, zoom, font_env);
        } else {
            if image_response.clicked()
                && let Some(screen_pos) = image_response.interact_pointer_pos()
            {
                let canvas_pos = viewer::screen_to_page(screen_pos, image_rect, extent, zoom);
                let shift = ui.input(|i| i.modifiers.shift);
                let hit = doc
                    .target_provider
                    .as_deref()
                    .and_then(|p| p.hit_test(doc.view.page_index, canvas_pos));
                doc.canvas_selection =
                    canvas::selection_after_click(&doc.canvas_selection, hit, shift);
            }

            // §5.1 live-preview overlay: painted above the raster via the
            // painter, NEVER a re-raster. This Pass has an empty selection and
            // the no-op provider, so `selection_outline_bounds` returns empty and
            // nothing is stroked — the "paints nothing this Pass" criterion,
            // exercised (not omitted) so Pass 9a's selection outlines and future
            // tools' in-progress previews plug into a live call site. A selection
            // outline is a 2px SHAPE, not a tint (rule 6): a real boundary.
            let outlines = canvas::selection_outline_bounds(
                &doc.canvas_selection,
                doc.target_provider.as_deref(),
                doc.view.page_index,
            );
            if !outlines.is_empty() {
                let painter = ui.painter_at(image_rect);
                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
                for canvas_bounds in outlines {
                    let min = viewer::page_to_screen(canvas_bounds.min, image_rect, extent, zoom);
                    let max = viewer::page_to_screen(canvas_bounds.max, image_rect, extent, zoom);
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(min, max),
                        0.0,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        } // end: non-text-edit-tool object-selection path (Pass 14.3 gate)

        // Ctrl+wheel over the canvas: multiply the zoom. Gated on hover
        // so a ctrl+wheel aimed at the thumbnail rail does not zoom the
        // page out from under the operator.
        if image_response.hovered() {
            let factor = ui.ctx().input(|i| i.zoom_delta());
            if (factor - 1.0).abs() > f32::EPSILON {
                actions.push(Action::ZoomBy(factor));
            }
        }
    }

    // -- raster bookkeeping ------------------------------------------

    /// Decide whether the cached page texture is still valid and, if
    /// not, whether to re-rasterize now or wait for a zoom gesture to
    /// settle. See the module docs, "Rendering happens on state change".
    fn settle_and_rasterize(&mut self, ctx: &egui::Context, pixels_per_point: f32) {
        // decision 012: capture the app-level font environment (a disjoint
        // field from `self.status`, so the split borrow below is fine).
        let font_env = &self.font_env;
        let font_env_generation = self.font_env_generation;
        let Status::Open(doc) = &mut self.status else {
            return;
        };

        // Did the zoom change since last frame, and by what route?
        let now = Instant::now();
        if (doc.observed_zoom - doc.view.zoom).abs() > f32::EPSILON {
            doc.observed_zoom = doc.view.zoom;
            doc.zoom_commit_at = if doc.zoom_commanded {
                now // discrete command: no gesture in flight, do not wait
            } else {
                now + ZOOM_SETTLE
            };
        }
        doc.zoom_commanded = false;

        let wanted_scale = viewer::raster_scale(doc.view.zoom, pixels_per_point);
        let stale_page = doc
            .page_texture
            .as_ref()
            .is_none_or(|t| t.page_index != doc.view.page_index);
        let stale_scale = doc
            .page_texture
            .as_ref()
            .is_some_and(|t| (t.raster_scale - wanted_scale).abs() > f32::EPSILON);
        // Flipping the annotation-visibility toggle changes neither the
        // page nor the scale, so without this third staleness key the
        // cached texture would not invalidate and the toggle would
        // silently do nothing (Pass 6.0). Treated like `stale_page` —
        // committed immediately, not debounced like a zoom gesture,
        // because it is a discrete click, not a gesture in flight.
        let stale_annotations = doc
            .page_texture
            .as_ref()
            .is_some_and(|t| t.annotations != doc.annotations_visible);
        // decision 012: adding/removing a font folder changes neither the
        // page, the scale, nor the annotation flag. Without this fourth
        // key the cached texture would not invalidate and supplying a font
        // would silently do nothing. Committed immediately, like the
        // annotation toggle — a discrete action, not a gesture in flight.
        let stale_fonts = doc
            .page_texture
            .as_ref()
            .is_some_and(|t| t.font_env_generation != font_env_generation);

        // A page whose previous render failed must not be retried every
        // frame: the failure is deterministic (same bytes, same code),
        // so retrying would peg a core producing the same error.
        if doc.render_error.is_some() && !stale_page {
            return;
        }

        if stale_page || stale_annotations || stale_fonts {
            doc.rasterize_current(ctx, wanted_scale, font_env, font_env_generation);
        } else if stale_scale {
            if now >= doc.zoom_commit_at {
                doc.rasterize_current(ctx, wanted_scale, font_env, font_env_generation);
            } else {
                // Nothing else will wake egui up when the debounce
                // expires, so schedule it.
                ctx.request_repaint_after(doc.zoom_commit_at - now);
            }
        }
    }
}

// ===================================================================
// Pass 14.3 — the in-place text-editing tool's per-frame handler
// (`docs/ui_specs/pass-14.3-text-edit-ui.md` §3–§9)
// ===================================================================

/// A run's PDF-user-space box (baseline-relative, the same one-em-tall,
/// quarter-em-descender approximation Pass 14.0's model uses for a line).
fn glyph_pdf_box(g: &pdfce_core::text_extract::ExtractedGlyph) -> egui::Rect {
    let x0 = g.x.min(g.x + g.advance);
    let x1 = g.x.max(g.x + g.advance);
    egui::Rect::from_min_max(
        egui::pos2(x0, g.y - g.size * 0.25),
        egui::pos2(x1, g.y + g.size * 0.75),
    )
}

/// The caret's PDF-user-space vertical segment `(bottom, top)` for a
/// `(run, byte-offset)` position: the leading edge of the glyph that starts
/// at that offset, or the trailing edge of the glyph that ends there. `None`
/// when no glyph boundary matches (a stale position).
fn caret_pdf_segment(
    page_text: &pdfce_core::text_extract::PageText,
    pos: pdfce_core::text_edit::TextPosition,
) -> Option<(egui::Pos2, egui::Pos2)> {
    let run = page_text.runs.get(pos.run)?;
    for g in &run.glyphs {
        let lo = g.text_start as usize;
        let hi = lo + g.text_len as usize;
        let x = if pos.byte_offset == lo {
            Some(g.x)
        } else if pos.byte_offset == hi {
            Some(g.x + g.advance)
        } else {
            None
        };
        if let Some(x) = x {
            return Some((
                egui::pos2(x, g.y - g.size * 0.25),
                egui::pos2(x, g.y + g.size * 0.75),
            ));
        }
    }
    None
}

/// The page's `/Resources /Font` entries as `(resource-key, label)` pairs for
/// the property bar's family ComboBox (§7). The label states the trust level
/// with the SAME Embedded/Bundled/Supplied vocabulary as the disclosure strip,
/// computed through the ONE shared classifier
/// [`FontEnvironment::classify_nonembedded`](pdfce_render::FontEnvironment::classify_nonembedded).
fn page_font_entries(
    base: &pdfce_core::document::Document,
    page: &Page,
    font_env: &pdfce_render::FontEnvironment,
) -> Vec<(String, String)> {
    use pdfce_core::object::Object;
    let mut out = Vec::new();
    let Some(fonts) = page
        .resources
        .get(b"Font")
        .map(|o| base.resolve(o))
        .and_then(Object::as_dict)
    else {
        return out;
    };
    for (key, val) in fonts.iter() {
        let key_str = String::from_utf8_lossy(key.as_bytes()).into_owned();
        let Some(dict) = base.resolve(val).as_dict() else {
            continue;
        };
        let base_font = dict
            .get(b"BaseFont")
            .map(|o| base.resolve(o))
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n.as_bytes()).into_owned())
            .unwrap_or_else(|| key_str.clone());
        let embedded = dict
            .get(b"FontDescriptor")
            .map(|o| base.resolve(o))
            .and_then(Object::as_dict)
            .is_some_and(|d| {
                d.contains_key(b"FontFile")
                    || d.contains_key(b"FontFile2")
                    || d.contains_key(b"FontFile3")
            });
        let trust = if embedded {
            ui_text::font_trust_embedded()
        } else {
            match font_env.classify_nonembedded(&base_font) {
                pdfce_render::GlyphSource::Supplied => ui_text::font_trust_supplied(),
                _ => ui_text::font_trust_bundled(),
            }
        };
        out.push((key_str, ui_text::font_entry_label(&base_font, trust)));
    }
    out
}

/// Map an edit refusal to the fixed "what would lift it" hint (§8.2 table).
fn edit_refusal_hint(err: &pdfce_core::text_edit::EditError) -> &'static str {
    use pdfce_core::text_edit::{EditError, RInvTrigger};
    match err {
        EditError::Refused(r) => match r.trigger {
            RInvTrigger::TargetAbsent => ui_text::r_inv_1_hint(),
            RInvTrigger::SymbolicNoEncoding
            | RInvTrigger::ToUnicodeOnly
            | RInvTrigger::Composite => ui_text::r_inv_encoding_hint(),
            RInvTrigger::LigatureOnly => ui_text::r_inv_ligature_hint(),
            RInvTrigger::CodeOccupied => ui_text::r_inv_code_occupied_hint(),
            RInvTrigger::BeyondRepertoire => ui_text::r_inv_repertoire_hint(),
            _ => ui_text::r_inv_encoding_hint(),
        },
        _ => ui_text::edit_generic_hint(),
    }
}

/// Map a format refusal to the fixed "what would lift it" hint (§8.2 table).
fn format_refusal_hint(err: &pdfce_core::text_edit::FormatError) -> &'static str {
    use pdfce_core::text_edit::FormatError;
    match err {
        FormatError::CoverageFailure(_) => ui_text::format_coverage_hint(),
        FormatError::TargetFontMissing(_) => ui_text::format_target_missing_hint(),
        FormatError::Refused(_) => ui_text::r_inv_encoding_hint(),
        _ => ui_text::edit_generic_hint(),
    }
}

/// Drive the Pass 14.3 text-edit tool for one frame: caret/selection hit +
/// render, the live `PendingEdit` preview, the property bar, the
/// disclosure/refusal strip and the read-only block overlay — committing an
/// accepted edit through `EditSession` as one undo-able command.
///
/// A free function (not a `PdfceApp` method) so it can take `&mut OpenDoc`
/// plus `&self.font_env` as two disjoint borrows — the same split
/// `settle_and_rasterize` uses. The mutation is staged across three phases so
/// the `EditableTextModel` (which borrows the owned `PageText`) is never alive
/// while `text_edit`/`session` is mutated.
#[allow(
    clippy::too_many_lines,
    reason = "one tool = one handler; splitting the tightly-coupled phases would need shared owned scratch structs that obscure more than they clarify" // ui-text-exempt: clippy lint justification, never displayed
)]
fn run_text_edit_tool(
    doc: &mut OpenDoc,
    ui: &mut egui::Ui,
    image_response: &egui::Response,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
    font_env: &pdfce_render::FontEnvironment,
) {
    use pdfce_core::text_edit::{
        AlignmentSource, BlockAlignment, BlockRecognitionOptions, EditOptions, EditRequest,
        EditableTextModel, FillModel, FontSelector, FormatOptions, FormatRequest, GlyphRef,
        NewFill, ReflowEngine, ReflowRequest, TextPosition, reflow_recognition_options,
    };

    let page_index = doc.view.page_index;
    // (Re)build state if missing or pointing at a different page (§2.1).
    if doc
        .text_edit
        .as_ref()
        .is_none_or(|s| s.page_index != page_index)
    {
        doc.build_text_edit_state();
    }
    if doc.text_edit.is_none() || doc.pages.get(page_index).is_none() {
        return;
    }

    // Owned outputs of phase A (computed while the model is borrowed).
    let mut click_result: Option<(Option<TextPosition>, Option<TextPosition>)> = None;
    let mut cross_run = false;
    let mut pinned_span: Option<pdfce_core::span::ByteSpan> = None;
    let mut caret_run_text: Option<(usize, String)> = None;
    // Pass 15.2: the block the caret is in, resolved against the RELAXED
    // recognition (§1.2), and a width captured from the on-canvas drag handle
    // (§6.1) — both owned out of Phase A the same way `click_result` is.
    let mut reflow_target: Option<usize> = None;
    let mut reflow_width_drag: Option<f64> = None;

    // ---- Phase A: model geometry, click hit-test, and all painting ----
    if let (Some(page), Some(state)) = (doc.pages.get(page_index), doc.text_edit.as_ref()) {
        let model =
            EditableTextModel::recognize(&state.page_text, &BlockRecognitionOptions::default());
        // Pass 15.2 §1.2: a SECOND, parallel recognition with first-line-indent
        // splitting relaxed (`reflow_recognition_options`), so a right/centre/
        // justified paragraph stays ONE block for reflow targeting. Both are
        // "cheap, index-only" per `EditableTextModel`'s docs; building it twice
        // per frame is a DELIBERATE cost of this Pass (do not "simplify" it back
        // to one model — that reintroduces §0.3's wrong-block bug). `model`
        // stays the default-recognition overlay 14.3 shipped, unchanged.
        let reflow_model =
            EditableTextModel::recognize(&state.page_text, &reflow_recognition_options());
        let painter = ui.painter_at(image_rect);
        let visuals = ui.visuals();
        let to_screen = |pdf: egui::Pos2| -> Option<egui::Pos2> {
            let canvas = viewer::pdf_space_to_canvas(pdf, page)?;
            Some(viewer::page_to_screen(canvas, image_rect, extent, zoom))
        };

        // Caret/selection gestures (spec §3/§4). A closure maps a screen point
        // through the SAME canvas→PDF→hit-test bridge for every gesture, so
        // drag/triple/double/single all resolve identically to §3's forward
        // path. Precedence: an active drag wins over a click, then triple over
        // double over single (egui fires exactly one of clicked/double/triple
        // per release, but ordering them defensively keeps the intent explicit).
        let hit_at = |sp: egui::Pos2| -> Option<TextPosition> {
            let canvas = viewer::screen_to_page(sp, image_rect, extent, zoom);
            viewer::canvas_to_pdf_space(canvas, page)
                .and_then(|pdf| model.hit_test(f64::from(pdf.x), f64::from(pdf.y)))
        };
        if image_response.drag_started() {
            // §4.1: press sets BOTH ends to the start caret; the drag then moves
            // the focus end (the `dragged()` arm below) while the anchor holds.
            if let Some(sp) = image_response.interact_pointer_pos() {
                let hit = hit_at(sp);
                click_result = Some((hit, hit));
            }
        } else if image_response.dragged() {
            // §4.1: each drag frame moves the focus caret to the pointer; the
            // anchor (set at drag_started, carried on `state`) is unchanged.
            if let Some(sp) = image_response.interact_pointer_pos()
                && let Some(hit) = hit_at(sp)
            {
                let anchor = state.anchor.or(state.caret).or(Some(hit));
                click_result = Some((Some(hit), anchor));
            }
        } else if image_response.triple_clicked() {
            // §4.3: triple-click selects the caret's LINE (line_range_at); caret
            // → line end, anchor → line start (mirrors the double-click order).
            if let Some(sp) = image_response.interact_pointer_pos() {
                click_result = Some(match hit_at(sp).and_then(|h| model.line_range_at(h)) {
                    Some((start, end)) => (Some(end), Some(start)),
                    None => (None, None),
                });
            }
        } else if image_response.double_clicked() {
            // §4.3: double-click selects the caret's WORD (word_range_at).
            if let Some(sp) = image_response.interact_pointer_pos() {
                click_result = Some(match hit_at(sp) {
                    Some(h) => {
                        let (a, b) = model.word_range_at(h);
                        (Some(b), Some(a))
                    }
                    None => (None, None),
                });
            }
        } else if image_response.clicked() {
            // §3/§4.2: plain click places a caret; Shift+click extends the span.
            if let Some(sp) = image_response.interact_pointer_pos() {
                let hit = hit_at(sp);
                let shift = ui.input(|i| i.modifiers.shift);
                click_result = Some(canvas::text_caret_after_click(
                    state.caret,
                    state.anchor,
                    hit,
                    shift,
                ));
            }
        }

        // The effective caret/selection for THIS frame's render.
        let (caret, anchor) = match &click_result {
            Some((c, a)) => (*c, *a),
            None => (state.caret, state.anchor),
        };

        // Pass 15.2 §1.1/§1.2: which relaxed-recognition block the caret is in
        // — the target of the "Reflow paragraph…" button. `block_at` is pure
        // index arithmetic (no new hit-testing); resolved against `reflow_model`
        // (NOT `model`), or the button would target the wrong fragmented block.
        reflow_target = caret.and_then(|c| reflow_model.block_at(c));

        // Selection highlight + cross-run detection.
        if let (Some(c), Some(a)) = (caret, anchor)
            && c != a
        {
            let covered = model.resolve_range(a, c);
            cross_run = canvas::selection_spans_multiple_runs(&covered);
            let fill = egui::Color32::from_rgba_unmultiplied(90, 140, 220, 70);
            for g in &covered {
                if let Some(glyph) = model.glyph(*g) {
                    let b = glyph_pdf_box(glyph);
                    if let (Some(min), Some(max)) = (
                        to_screen(egui::pos2(b.min.x, b.max.y)),
                        to_screen(egui::pos2(b.max.x, b.min.y)),
                    ) {
                        painter.rect_filled(egui::Rect::from_two_pos(min, max), 0.0, fill);
                    }
                }
            }
        }

        // Caret line (a real SHAPE, rule 6). A simple 1s blink.
        if let Some(c) = caret {
            let on = (ui.input(|i| i.time) * 1.6) as i64 % 2 == 0;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(400));
            if on
                && let Some((bot, top)) = caret_pdf_segment(&state.page_text, c)
                && let (Some(b), Some(t)) = (to_screen(bot), to_screen(top))
            {
                painter.line_segment([b, t], egui::Stroke::new(1.5, visuals.text_color()));
            }
            pinned_span = model
                .provenance(GlyphRef::new(c.run, 0))
                .map(|p| p.operator_span);
            caret_run_text = state
                .page_text
                .runs
                .get(c.run)
                .map(|r| (c.run, r.text.clone()));
        }

        // Read-only block-boundary overlay (§9): dashed Block.bbox outlines —
        // a DIFFERENT visual vocabulary (dashed) from the solid caret/
        // selection. Split/merge/resize/reorder are a DEFERRED non-goal for
        // this Pass (they need a core persistence API + FF-A reflow to
        // consume a correction); this is read-only visualization only.
        if state.show_block_overlay {
            let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 120, 40));
            for block in model.blocks() {
                let b = block.bbox;
                if let (Some(min), Some(max)) = (
                    to_screen(egui::pos2(b.llx as f32, b.ury as f32)),
                    to_screen(egui::pos2(b.urx as f32, b.lly as f32)),
                ) {
                    let rect = egui::Rect::from_two_pos(min, max);
                    // A dashed rectangle: four dashed edges.
                    for (a, z) in [
                        (rect.left_top(), rect.right_top()),
                        (rect.right_top(), rect.right_bottom()),
                        (rect.right_bottom(), rect.left_bottom()),
                        (rect.left_bottom(), rect.left_top()),
                    ] {
                        painter.add(egui::Shape::dashed_line(&[a, z], stroke, 4.0, 3.0));
                    }
                }
            }
        }

        // Live preview of the pending edit (§6.3): a translucent mask over the
        // original run's box + the draft text drawn in an egui built-in font +
        // a dashed border and a "PREVIEW — not yet applied" tag. NEVER a
        // re-raster; the real glyphs appear only after a real commit.
        if let Some(pending) = &state.pending
            && let Some(run) = state.page_text.runs.get(pending.run)
        {
            let mut bbox: Option<egui::Rect> = None;
            for g in &run.glyphs {
                let gb = glyph_pdf_box(g);
                bbox = Some(bbox.map_or(gb, |acc| acc.union(gb)));
            }
            if let Some(pdf_box) = bbox
                && let (Some(min), Some(max)) = (
                    to_screen(egui::pos2(pdf_box.min.x, pdf_box.max.y)),
                    to_screen(egui::pos2(pdf_box.max.x, pdf_box.min.y)),
                )
            {
                let screen_box = egui::Rect::from_two_pos(min, max).expand(2.0);
                painter.rect_filled(
                    screen_box,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(250, 250, 250, 220),
                );
                let size = (run.glyphs.first().map_or(12.0, |g| g.size) * zoom).clamp(6.0, 96.0);
                painter.text(
                    screen_box.left_bottom() + egui::vec2(1.0, -2.0),
                    egui::Align2::LEFT_BOTTOM,
                    &pending.draft_text,
                    egui::FontId::proportional(size),
                    egui::Color32::from_rgb(20, 20, 20),
                );
                let dash = egui::Stroke::new(1.5, egui::Color32::from_rgb(210, 90, 40));
                for (a, z) in [
                    (screen_box.left_top(), screen_box.right_top()),
                    (screen_box.right_top(), screen_box.right_bottom()),
                    (screen_box.right_bottom(), screen_box.left_bottom()),
                    (screen_box.left_bottom(), screen_box.left_top()),
                ] {
                    painter.add(egui::Shape::dashed_line(&[a, z], dash, 4.0, 3.0));
                }
                painter.text(
                    screen_box.left_top() + egui::vec2(1.0, -13.0),
                    egui::Align2::LEFT_BOTTOM,
                    ui_text::preview_tag(),
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(210, 90, 40),
                );
            }
        }

        // Pass 15.2 §5: the reflow ghost — reusing 14.3's marked/applied visual
        // language (translucent mask + draft text + dashed PREVIEW tag),
        // generalized from one run to one block. Painted only while reviewing;
        // nothing is applied until Accept (rule 4). Same painter, same
        // `to_screen`, no re-raster.
        if let Some(r) = state.reflow.as_ref() {
            let amber = egui::Color32::from_rgb(200, 120, 40);
            let preview_orange = egui::Color32::from_rgb(210, 90, 40);

            // §5.1: the targeted block's SOLID (not dashed) highlight, 2.5px —
            // shape + weight is the signal, not a new colour (rule 6). May span
            // several of the general overlay's dashed single-line boxes for a
            // right/centre/justified paragraph (expected, §3).
            if state.show_block_overlay
                && let Some(block) = reflow_model.blocks().get(r.block_index)
            {
                let b = block.bbox;
                if let (Some(min), Some(max)) = (
                    to_screen(egui::pos2(b.llx as f32, b.ury as f32)),
                    to_screen(egui::pos2(b.urx as f32, b.lly as f32)),
                ) {
                    let rect = egui::Rect::from_two_pos(min, max);
                    let stroke = egui::Stroke::new(2.5, amber);
                    for (a, z) in [
                        (rect.left_top(), rect.right_top()),
                        (rect.right_top(), rect.right_bottom()),
                        (rect.right_bottom(), rect.left_bottom()),
                        (rect.left_bottom(), rect.left_top()),
                    ] {
                        painter.line_segment([a, z], stroke);
                    }
                }
            }

            // §5.2 item 1/4/6: mask the OLD block (available even when the
            // preview errored — taken from the block's own bbox) and draw its
            // muted, short-dashed outline + a "current" corner tag.
            if let Some(ob) = reflow_model.blocks().get(r.block_index).map(|b| b.bbox)
                && let (Some(min), Some(max)) = (
                    to_screen(egui::pos2(ob.llx as f32, ob.ury as f32)),
                    to_screen(egui::pos2(ob.urx as f32, ob.lly as f32)),
                )
            {
                let old_box = egui::Rect::from_two_pos(min, max);
                painter.rect_filled(
                    old_box,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(250, 250, 250, 220),
                );
                let muted = egui::Stroke::new(1.0, visuals.weak_text_color());
                for (a, z) in [
                    (old_box.left_top(), old_box.right_top()),
                    (old_box.right_top(), old_box.right_bottom()),
                    (old_box.right_bottom(), old_box.left_bottom()),
                    (old_box.left_bottom(), old_box.left_top()),
                ] {
                    painter.add(egui::Shape::dashed_line(&[a, z], muted, 2.0, 3.0));
                }
                painter.text(
                    old_box.left_top() + egui::vec2(1.0, -1.0),
                    egui::Align2::LEFT_BOTTOM,
                    ui_text::reflow_current_tag(),
                    egui::FontId::proportional(10.0),
                    visuals.weak_text_color(),
                );
            }

            // The block's representative size (largest line size) — the ghost
            // text approximation, exactly as the engine measures it.
            let rep_size = reflow_model
                .blocks()
                .get(r.block_index)
                .map(|b| {
                    b.line_indices
                        .iter()
                        .filter_map(|&li| reflow_model.lines().get(li))
                        .map(|l| f64::from(l.size))
                        .fold(0.0_f64, f64::max)
                        .max(1.0)
                })
                .unwrap_or(12.0);

            // §5.2 items 2/3/5: the new ghost lines, the dashed PREVIEW border +
            // tag on the new box, and the wrap-width guide — Ok(preview) only.
            // Err skips this (nothing sensible to draw) and is surfaced in the
            // status strip (§6.4), never a silent blank (§5.2 item 6).
            if let Ok(pv) = &r.preview {
                let size = (rep_size * f64::from(zoom)).clamp(6.0, 96.0) as f32;
                for line in &pv.lines {
                    if let Some(sp) =
                        to_screen(egui::pos2(line.origin_x as f32, line.baseline_y as f32))
                    {
                        painter.text(
                            sp,
                            egui::Align2::LEFT_BOTTOM,
                            &line.text,
                            egui::FontId::proportional(size),
                            egui::Color32::from_rgb(20, 20, 20),
                        );
                    }
                }
                if let (Some(min), Some(max)) = (
                    to_screen(egui::pos2(pv.new_bbox.llx as f32, pv.new_bbox.ury as f32)),
                    to_screen(egui::pos2(pv.new_bbox.urx as f32, pv.new_bbox.lly as f32)),
                ) {
                    let new_box = egui::Rect::from_two_pos(min, max);
                    let dash = egui::Stroke::new(1.5, preview_orange);
                    for (a, z) in [
                        (new_box.left_top(), new_box.right_top()),
                        (new_box.right_top(), new_box.right_bottom()),
                        (new_box.right_bottom(), new_box.left_bottom()),
                        (new_box.left_bottom(), new_box.left_top()),
                    ] {
                        painter.add(egui::Shape::dashed_line(&[a, z], dash, 4.0, 3.0));
                    }
                    painter.text(
                        new_box.right_top() + egui::vec2(-1.0, -13.0),
                        egui::Align2::RIGHT_BOTTOM,
                        ui_text::preview_tag(),
                        egui::FontId::proportional(11.0),
                        preview_orange,
                    );
                    // §5.2 item 5: the re-flushed right edge, for right/centre/
                    // justified (all three place text relative to it).
                    if pv.alignment.alignment != BlockAlignment::Left {
                        let guide = egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 90, 40));
                        painter.add(egui::Shape::dashed_line(
                            &[new_box.right_top(), new_box.right_bottom()],
                            guide,
                            3.0,
                            3.0,
                        ));
                        painter.text(
                            new_box.right_bottom() + egui::vec2(-1.0, 1.0),
                            egui::Align2::RIGHT_TOP,
                            ui_text::reflow_wrap_width_label(),
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_rgb(160, 90, 40),
                        );
                    }
                }

                // §6.1: the on-canvas width drag-handle — a mouse CONVENIENCE
                // over the keyboard-complete DragValue. Recomputed from the
                // ABSOLUTE pointer position (no delta drift): new width =
                // pointer.x − old_bbox.llx (the block is always left-anchored).
                let mid_y = (pv.new_bbox.lly + pv.new_bbox.ury) / 2.0;
                if let Some(handle_screen) =
                    to_screen(egui::pos2(pv.new_bbox.urx as f32, mid_y as f32))
                {
                    let handle_rect =
                        egui::Rect::from_center_size(handle_screen, egui::vec2(10.0, 24.0));
                    let resp = ui.interact(
                        handle_rect,
                        egui::Id::new("pdfce-reflow-width-handle"),
                        egui::Sense::drag(),
                    );
                    painter.rect_filled(
                        handle_rect,
                        2.0,
                        egui::Color32::from_rgba_unmultiplied(210, 90, 40, 70),
                    );
                    if resp.dragged()
                        && let Some(sp) = resp.interact_pointer_pos()
                    {
                        let canvas = viewer::screen_to_page(sp, image_rect, extent, zoom);
                        if let Some(pdf) = viewer::canvas_to_pdf_space(canvas, page) {
                            let new_width =
                                (f64::from(pdf.x) - pv.old_bbox.llx).max(MIN_WRAP_WIDTH_PT);
                            reflow_width_drag = Some(new_width);
                        }
                    }
                    resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                        .on_hover_text(ui_text::reflow_width_handle_tooltip());
                }
            }
        }
    }

    // ---- Phase B: apply the click, handle typing, draw the widgets ----
    // Owned intents captured for phase C (the session mutation).
    let mut do_accept = false;
    let mut do_reject = false;
    let mut apply_size: Option<f64> = None;
    let mut apply_fill: Option<NewFill> = None;
    let mut apply_font: Option<String> = None;
    // Pass 15.2 reflow intents, resolved after the property-bar/status closures.
    let mut enter_reflow = false;
    let mut do_accept_reflow = false;
    let mut do_reject_reflow = false;
    let mut reflow_changed = false;
    // The page cropbox for building overflow-aware reflow requests (§0.2/§6),
    // captured here (disjoint from the `&mut text_edit` borrow below).
    let reflow_page_crop = doc.pages.get(page_index).map(|page| page.crop_box);

    // Font list for the property bar (needs the base document + page; disjoint
    // from the `&mut text_edit` borrow below since they are separate fields).
    let font_entries = doc
        .pages
        .get(page_index)
        .map(|page| page_font_entries(doc.session.document(), page, font_env))
        .unwrap_or_default();

    if let Some(state) = doc.text_edit.as_mut() {
        // The click resolves AFTER render: an in-progress pending edit is this
        // tool's discardable GestureInterrupt (§6.2) — a click discards it.
        if let Some((c, a)) = click_result {
            if state.pending.is_some() {
                state.pending = None;
            }
            state.caret = c;
            state.anchor = a;
        }

        // Typing → build/extend the PendingEdit (§6.1). No core call per
        // keystroke. Suppressed for a cross-run selection (§4.4) — nothing to
        // commit as one edit, so typing does nothing.
        // Typing is suppressed while a reflow review is active (§1.4) — the
        // exact same "cross-run selection suppresses typing" precedent, one more
        // conjunct. The reflow sub-mode has its own Accept/Reject.
        if image_response.has_focus() && !cross_run && state.reflow.is_none() {
            let events = ui.input(|i| i.events.clone());
            for ev in events {
                match ev {
                    egui::Event::Text(t) if !t.is_empty() => {
                        text_edit_insert(state, &t);
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => text_edit_backspace(state),
                    egui::Event::Key {
                        key: egui::Key::Delete,
                        pressed: true,
                        ..
                    } => text_edit_delete(state),
                    _ => {}
                }
            }
        }

        // Arrow / Home / End caret navigation (spec §4.5). Gated to the plain
        // caret model — a `PendingEdit` owns its own draft cursor, so model-space
        // arrow nav is deliberately suppressed while composing (a named first-cut
        // line: the draft is committed/rejected as a unit) — and to a focused
        // canvas. While the tool is active these keys are gated OUT of the global
        // page-nav bindings (`collect_keyboard_actions`), so only a focused
        // canvas moves the caret while a focused property-bar DragValue still gets
        // Home/End for its own text editing. Shift extends the selection via the
        // SAME rule as a Shift+click (`text_caret_after_click`); a plain arrow
        // collapses it. Up/Down preserve the caret's page-space column
        // (`caret_x`), landing on the adjacent line's nearest slot.
        if image_response.has_focus() && state.reflow.is_none() && state.pending.is_none() {
            let nav: Vec<(egui::Key, bool)> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } if matches!(
                            key,
                            egui::Key::ArrowLeft
                                | egui::Key::ArrowRight
                                | egui::Key::ArrowUp
                                | egui::Key::ArrowDown
                                | egui::Key::Home
                                | egui::Key::End
                        ) =>
                        {
                            Some((*key, modifiers.shift))
                        }
                        _ => None,
                    })
                    .collect()
            });
            if !nav.is_empty() {
                // Scope the model's borrow of `state.page_text` so it is dropped
                // before the caret/anchor are written back.
                let (new_caret, new_anchor) = {
                    let model = EditableTextModel::recognize(
                        &state.page_text,
                        &BlockRecognitionOptions::default(),
                    );
                    let mut caret = state.caret;
                    let mut anchor = state.anchor;
                    for (key, shift) in nav {
                        let Some(cur) = caret else { break };
                        let target = match key {
                            egui::Key::ArrowLeft => model.caret_left(cur),
                            egui::Key::ArrowRight => model.caret_right(cur),
                            egui::Key::ArrowUp => {
                                model.caret_up(cur, model.caret_x(cur).unwrap_or(0.0))
                            }
                            egui::Key::ArrowDown => {
                                model.caret_down(cur, model.caret_x(cur).unwrap_or(0.0))
                            }
                            egui::Key::Home => model.line_range_at(cur).map_or(cur, |(s, _)| s),
                            egui::Key::End => model.line_range_at(cur).map_or(cur, |(_, e)| e),
                            _ => cur,
                        };
                        let (c, a) =
                            canvas::text_caret_after_click(caret, anchor, Some(target), shift);
                        caret = c;
                        anchor = a;
                    }
                    (caret, anchor)
                };
                state.caret = new_caret;
                state.anchor = new_anchor;
            }
        }

        // Property bar (§7): a floating top panel, appearing with the tool.
        egui::Area::new(egui::Id::new("pdfce-text-edit-propbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(image_rect.min + egui::vec2(8.0, 8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(360.0);
                    // Pass 15.2 §4: two mutually-exclusive bodies. `r`'s borrow
                    // ends with its arm, so the shared block-overlay checkbox
                    // below can take `&mut state.show_block_overlay` freely.
                    if let Some(r) = state.reflow.as_mut() {
                        // §4.2 reflow body (replaces the size/colour/font rows).
                        ui.label(ui_text::reflow_body_title());
                        // §6.2 alignment caption — from the FIXED detected value
                        // vs the live pick, never paraphrasing core's disclosure.
                        let caption = if r.alignment_is_override {
                            ui_text::reflow_overridden_caption(
                                r.detected_alignment.alignment.as_str(),
                                r.alignment.as_str(),
                            )
                        } else {
                            match r.detected_alignment.source {
                                AlignmentSource::Detected => {
                                    ui_text::reflow_detected_caption(r.alignment.as_str())
                                }
                                AlignmentSource::SingleLineDefault
                                | AlignmentSource::AmbiguousDefault => {
                                    ui_text::reflow_ambiguous_caption().to_owned()
                                }
                                _ => ui_text::reflow_detected_caption(r.alignment.as_str()),
                            }
                        };
                        ui.label(caption);
                        // §4.4/§6.1 width — DragValue is primary + keyboard-
                        // complete; the canvas handle (Phase A) is the mouse
                        // convenience layered on top.
                        ui.horizontal(|ui| {
                            ui.label(ui_text::reflow_width_label());
                            if ui
                                .add(
                                    egui::DragValue::new(&mut r.width)
                                        .range(MIN_WRAP_WIDTH_PT..=100_000.0)
                                        .speed(0.5),
                                )
                                .changed()
                            {
                                reflow_changed = true;
                            }
                        });
                        // §4.3 alignment picker — real selectable_values,
                        // pre-filled with the detected value. Override is decided
                        // on the CLICK from the clicked value (§6.2), so
                        // AlignmentSource stays honest.
                        ui.horizontal(|ui| {
                            ui.label(ui_text::reflow_alignment_label());
                            for (val, label) in [
                                (BlockAlignment::Left, "Left"),
                                (BlockAlignment::Center, "Center"),
                                (BlockAlignment::Right, "Right"),
                                (BlockAlignment::Justified, "Justify"),
                            ] {
                                if ui.selectable_value(&mut r.alignment, val, label).clicked() {
                                    r.alignment_is_override = reflow_alignment_is_override(
                                        r.detected_alignment.alignment,
                                        val,
                                    );
                                    reflow_changed = true;
                                }
                            }
                        });
                        // §4.5/§6.3 leading — a plain DragValue, no canvas handle.
                        ui.horizontal(|ui| {
                            ui.label(ui_text::reflow_leading_label());
                            if ui
                                .add(
                                    egui::DragValue::new(&mut r.leading)
                                        .range(0.1..=10_000.0)
                                        .speed(0.2),
                                )
                                .changed()
                            {
                                reflow_changed = true;
                            }
                        });
                    } else {
                        // §4.1 normal body — 14.3's shipped layout, with the
                        // reflow entry button + divergence caption appended.
                        ui.label(ui_text::text_edit_propbar_title());
                        ui.horizontal(|ui| {
                            ui.label(ui_text::format_size_label());
                            ui.add(egui::DragValue::new(&mut state.prop_size).range(1.0..=1000.0));
                            if ui.button(ui_text::format_apply_size()).clicked() {
                                apply_size = Some(state.prop_size);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(ui_text::format_color_model_label());
                            ui.selectable_value(&mut state.prop_model, FillModel::Rgb, "RGB");
                            ui.selectable_value(&mut state.prop_model, FillModel::Cmyk, "CMYK");
                            ui.selectable_value(&mut state.prop_model, FillModel::Gray, "Gray");
                        });
                        ui.horizontal(|ui| {
                            for i in 0..state.prop_model.arity() {
                                if let Some(c) = state.prop_components.get_mut(i) {
                                    ui.add(egui::DragValue::new(c).range(0.0..=1.0).speed(0.01));
                                }
                            }
                            if ui.button(ui_text::format_apply_color()).clicked() {
                                let comps: Vec<f64> = state
                                    .prop_components
                                    .iter()
                                    .take(state.prop_model.arity())
                                    .copied()
                                    .collect();
                                apply_fill = NewFill::new(state.prop_model, comps).ok();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(ui_text::format_font_label());
                            let selected = state.prop_font.clone().unwrap_or_default();
                            egui::ComboBox::from_id_salt("pdfce-te-font")
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    for (key, label) in &font_entries {
                                        let is_sel =
                                            state.prop_font.as_deref() == Some(key.as_str());
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            state.prop_font = Some(key.clone());
                                        }
                                    }
                                });
                            if ui.button(ui_text::format_apply_font()).clicked() {
                                apply_font = state.prop_font.clone();
                            }
                        });
                        // §1.3 reflow entry button (grey-not-hidden) — targets the
                        // caret's block; §3 divergence caption when a target
                        // resolves (fuzzy-never-sneaky about the two recognitions).
                        ui.separator();
                        let enabled = reflow_button_enabled(reflow_target, state.pending.is_some());
                        let resp = ui.add_enabled(
                            enabled,
                            egui::Button::new(ui_text::reflow_button_label()),
                        );
                        if resp.clicked() {
                            enter_reflow = true;
                        }
                        resp.on_hover_text(if enabled {
                            ui_text::reflow_button_tooltip()
                        } else if state.pending.is_some() {
                            ui_text::reflow_disabled_pending_tooltip()
                        } else {
                            ui_text::reflow_disabled_no_block_tooltip()
                        });
                        if reflow_target.is_some() {
                            ui.label(ui_text::reflow_recognition_note());
                        }
                    }
                    // Common to both bodies (§4.2 keeps it unchanged).
                    ui.checkbox(
                        &mut state.show_block_overlay,
                        ui_text::block_overlay_toggle(),
                    )
                    .on_hover_text(ui_text::block_overlay_toggle_tooltip());
                });
            });

        // Accept/Reject (§6.4) + the disclosure/refusal strip (§8): a floating
        // bottom panel. Accept/Reject are REAL `ui.button`s (accesskit win,
        // §10), not painter-drawn.
        egui::Area::new(egui::Id::new("pdfce-text-edit-status"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(image_rect.min.x + 8.0, image_rect.max.y - 8.0))
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(460.0);
                    if cross_run {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            ui_text::cross_run_selection_notice(),
                        );
                    }
                    if state.pending.is_some() {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    ICON_BUTTON_SIZE,
                                    egui::Button::new(ui_text::accept_edit()),
                                )
                                .clicked()
                            {
                                do_accept = true;
                            }
                            if ui
                                .add_sized(
                                    ICON_BUTTON_SIZE,
                                    egui::Button::new(ui_text::reject_edit()),
                                )
                                .clicked()
                            {
                                do_reject = true;
                            }
                        });
                    }
                    // Pass 15.2 §7: reflow Accept/Reject + live diagnostics —
                    // mutually exclusive with `pending`, so it shares this strip.
                    if let Some(r) = state.reflow.as_ref() {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    ICON_BUTTON_SIZE,
                                    egui::Button::new(ui_text::reflow_accept()),
                                )
                                .clicked()
                            {
                                do_accept_reflow = true;
                            }
                            if ui
                                .add_sized(
                                    ICON_BUTTON_SIZE,
                                    egui::Button::new(ui_text::reflow_reject()),
                                )
                                .clicked()
                            {
                                do_reject_reflow = true;
                            }
                        });
                        // §8.1 live diagnostics — the engine's own strings
                        // verbatim, shown WHILE deciding so overflow (R76)
                        // informs the decision, not follows it. Calm ⓘ bullets,
                        // never a Pass-8 acknowledgement gate.
                        match &r.preview {
                            Ok(pv) => {
                                for d in &pv.diagnostics.disclosures {
                                    ui.label(ui_text::disclosure_bullet(d));
                                }
                            }
                            Err(e) => {
                                ui.colored_label(
                                    ui.visuals().error_fg_color,
                                    ui_text::refusal_line(&e.to_string()),
                                );
                            }
                        }
                        // §7.3 the last Accept attempt's refusal, verbatim + hint.
                        if let Some(text) = &r.last_refusal {
                            ui.separator();
                            ui.label(ui_text::refusal_strip_title());
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                ui_text::refusal_line(text),
                            );
                        }
                    }
                    // Refusal (verbatim core Display + the fixed hint, §8.2).
                    let refusal = state
                        .pending
                        .as_ref()
                        .and_then(|p| p.last_refusal.clone())
                        .or_else(|| state.last_refusal.clone());
                    if let Some(text) = refusal {
                        ui.separator();
                        ui.label(ui_text::refusal_strip_title());
                        ui.colored_label(ui.visuals().error_fg_color, ui_text::refusal_line(&text));
                    }
                    // Last accepted edit's disclosures (verbatim, §8.1).
                    if !state.last_disclosures.is_empty() {
                        ui.separator();
                        ui.label(ui_text::disclosure_strip_title());
                        for d in &state.last_disclosures {
                            ui.label(ui_text::disclosure_bullet(d));
                        }
                    }
                });
            });

        // ---- Pass 15.2: resolve the reflow view-state intents ----
        // Still inside the `&mut text_edit` borrow; `doc.session`/`doc.pages`
        // are touched only in Phase C. Nothing here mutates the `EditSession`.
        if let (Some(w), Some(r)) = (reflow_width_drag, state.reflow.as_mut())
            && (r.width - w).abs() > f64::EPSILON
        {
            r.width = w;
            reflow_changed = true;
        }
        if reflow_changed && let Some(r) = state.reflow.as_mut() {
            // Any input change invalidates a stale Accept refusal (§2).
            r.last_refusal = None;
        }
        // §2.1 enter reflow from the property-bar button: seed from an initial
        // read-only preview. The relaxed model borrows page_text, so it is built
        // and dropped inside a scope before `state.reflow` is written.
        if enter_reflow
            && state.reflow.is_none()
            && let Some(idx) = reflow_target
            && let Some(crop) = reflow_page_crop
        {
            let seeded = {
                let model =
                    EditableTextModel::recognize(&state.page_text, &reflow_recognition_options());
                let engine = ReflowEngine::new(&model);
                engine.detect_alignment(idx).ok().map(|detected| {
                    let seed = ReflowRequest::new().with_page_cropbox(crop);
                    (detected, engine.preview(idx, &seed))
                })
            };
            if let Some((detected, preview)) = seeded {
                let (width, leading) = match &preview {
                    Ok(pv) => (pv.wrap_width, pv.leading),
                    Err(_) => (0.0, 0.0), // shown as an error immediately (§6.4)
                };
                state.reflow = Some(ReflowState {
                    block_index: idx,
                    detected_alignment: detected,
                    width,
                    alignment: detected.alignment,
                    alignment_is_override: false,
                    leading,
                    preview,
                    last_refusal: None,
                });
                state.show_block_overlay = true; // §5.1 convenience
            }
        }
        // §6 live re-preview each frame while reviewing — a pure, cheap,
        // read-only derivation; mutate nothing. Scoped so the model's borrow of
        // page_text is released before `preview` is stored.
        if let Some((idx, req)) = state.reflow.as_ref().map(|r| {
            let req = ReflowRequest::new()
                .with_wrap_width(r.width)
                .with_leading(r.leading)
                .with_alignment_opt(r.alignment_is_override.then_some(r.alignment));
            let req = match reflow_page_crop {
                Some(crop) => req.with_page_cropbox(crop),
                None => req,
            };
            (r.block_index, req)
        }) {
            let new_preview = {
                let model =
                    EditableTextModel::recognize(&state.page_text, &reflow_recognition_options());
                ReflowEngine::new(&model).preview(idx, &req)
            };
            if let Some(r) = state.reflow.as_mut() {
                r.preview = new_preview;
            }
        }
    }

    // ---- Phase C: the session mutation (one undo-able command) ----
    if do_reject && let Some(state) = doc.text_edit.as_mut() {
        state.pending = None;
        return;
    }

    if do_accept {
        // Build the request from the pending draft; commit through EditSession.
        let req = doc.text_edit.as_ref().and_then(|s| {
            s.pending.as_ref().map(|p| {
                let mut r = EditRequest::find_replace(page_index, &p.original_text, &p.draft_text);
                r.pinned_span = pinned_span;
                r
            })
        });
        if let Some(req) = req {
            match doc.session.edit_text(&req, &EditOptions::default()) {
                Ok(report) => {
                    doc.refresh_pages();
                    doc.build_text_edit_state();
                    if let Some(state) = doc.text_edit.as_mut() {
                        state.last_disclosures = report.disclosures;
                    }
                }
                Err(err) => {
                    let msg = ui_text::refusal_with_hint(&err.to_string(), edit_refusal_hint(&err));
                    if let Some(pending) = doc.text_edit.as_mut().and_then(|s| s.pending.as_mut()) {
                        pending.last_refusal = Some(msg);
                    }
                }
            }
        }
        return;
    }

    // Pass 15.2 §7.1 Reject: nothing was ever written (rule 7).
    if do_reject_reflow {
        if let Some(state) = doc.text_edit.as_mut() {
            state.reflow = None;
        }
        return;
    }
    // Pass 15.2 §7.2 Accept: ONE undo-able `EditSession::reflow_block` command,
    // planned + committed from the session's own current content (§0.2). The
    // request is freshly rebuilt from the state the operator last adjusted, so
    // the bytes committed match the ghost reviewed (WYSIWYG by construction).
    if do_accept_reflow {
        let plan = doc
            .text_edit
            .as_ref()
            .and_then(|s| s.reflow.as_ref())
            .map(|r| {
                let req = ReflowRequest::new()
                    .with_wrap_width(r.width)
                    .with_leading(r.leading)
                    .with_alignment_opt(r.alignment_is_override.then_some(r.alignment));
                (r.block_index, req)
            });
        if let Some((block_index, req)) = plan {
            let req = match doc.pages.get(page_index).map(|page| page.crop_box) {
                Some(crop) => req.with_page_cropbox(crop),
                None => req,
            };
            match doc.session.reflow_block(page_index, block_index, &req) {
                Ok(report) => {
                    doc.refresh_pages();
                    // Content changed — rebuild for the SAME page, exactly the
                    // EditText/FormatText post-accept rule (14.3 §2.1).
                    doc.build_text_edit_state();
                    if let Some(state) = doc.text_edit.as_mut() {
                        state.reflow = None;
                        state.last_disclosures = report.disclosures; // verbatim (§8.2)
                    }
                }
                Err(err) => {
                    // Refusal (incl. the already-edited-this-session condition,
                    // judgment call #6) surfaced verbatim + a next-step hint;
                    // the reflow stays for revision, never a crash (rule 4).
                    let msg =
                        ui_text::refusal_with_hint(&err.to_string(), reflow_refusal_hint(&err));
                    if let Some(r) = doc.text_edit.as_mut().and_then(|s| s.reflow.as_mut()) {
                        r.last_refusal = Some(msg);
                    }
                }
            }
        }
        return;
    }

    // Property-bar format applies.
    let format_req = |op: FormatOp| -> Option<FormatRequest> {
        let (run, text) = caret_run_text.clone()?;
        let _ = run;
        let mut r = FormatRequest::new(page_index, &text);
        r.pinned_span = pinned_span;
        Some(match op {
            FormatOp::Size(pt) => r.size(pt),
            FormatOp::Fill(f) => r.fill(f),
            FormatOp::Font(sel) => r.font(FontSelector::new(&sel)),
        })
    };
    let chosen = if let Some(pt) = apply_size {
        Some(FormatOp::Size(pt))
    } else if let Some(f) = apply_fill {
        Some(FormatOp::Fill(f))
    } else {
        apply_font.map(FormatOp::Font)
    };
    if let Some(op) = chosen
        && let Some(req) = format_req(op)
    {
        match doc.session.format_text(&req, &FormatOptions::default()) {
            Ok(report) => {
                doc.refresh_pages();
                doc.build_text_edit_state();
                if let Some(state) = doc.text_edit.as_mut() {
                    state.last_disclosures = report.disclosures;
                }
            }
            Err(err) => {
                let msg = ui_text::refusal_with_hint(&err.to_string(), format_refusal_hint(&err));
                if let Some(state) = doc.text_edit.as_mut() {
                    state.last_refusal = Some(msg);
                }
            }
        }
    }
}

// ===================================================================
// Pass 16.2 — the Add-Page-Text tool's per-frame handler
// (docs/ui_specs/pass-16.2-add-text-ui.md §3–§8)
// ===================================================================

/// A Std-14 face's font-combo label: its exact `/BaseFont` name plus a
/// Bundled/Supplied trust tag (Pass 16.2 §5.1/§5.2), via the ONE shared
/// `classify_nonembedded` — the SAME trust computation `cmd_add_text` and the
/// 14.3 property bar use, never re-derived (R79 provenance disclosure).
fn std14_combo_label(
    face: pdfce_core::fontdata::Std14,
    font_env: &pdfce_render::FontEnvironment,
) -> String {
    let name = pdfce_core::fontdata::std14_base_font_name(face);
    let trust = match font_env.classify_nonembedded(name) {
        pdfce_render::GlyphSource::Supplied => ui_text::font_trust_supplied(),
        _ => ui_text::font_trust_bundled(),
    };
    ui_text::font_entry_label(name, trust)
}

/// Read the operator's chosen fill colour back as core's `NewTextColor`
/// (Pass 16.2 §5.2). Core's Add-Text colour is only Black/Rgb (no Gray/CMYK),
/// so the honest surface is: pure black → `Black`, anything else → `Rgb` — no
/// widget for a colour model core would silently coerce.
fn add_text_color(c: egui::Color32) -> pdfce_core::text_edit::NewTextColor {
    use pdfce_core::text_edit::NewTextColor;
    if c == egui::Color32::BLACK {
        NewTextColor::Black
    } else {
        NewTextColor::Rgb(
            f64::from(c.r()) / 255.0,
            f64::from(c.g()) / 255.0,
            f64::from(c.b()) / 255.0,
        )
    }
}

/// Convert a resolved canvas placement into an [`AddPlacement`] and install a
/// FRESH (empty) draft (Pass 16.2 §3). A new placement always discards the
/// prior draft — a click/drag while composing is this tool's discardable
/// gesture (§3.1/§8), and the new gesture starts a clean composition.
fn install_add_placement(state: &mut AddTextState, placement: canvas::AddTextPlacement) {
    let placement = match placement {
        canvas::AddTextPlacement::Point { x, y } => AddPlacement::Point { x, y },
        canvas::AddTextPlacement::Box {
            llx,
            lly,
            width,
            height,
        } => AddPlacement::Box {
            llx,
            lly,
            width,
            height,
        },
    };
    state.draft = Some(AddTextDraft {
        placement,
        draft_text: String::new(),
        wrap_preview: None,
        last_refusal: None,
    });
}

/// Map an [`AddTextError`](pdfce_core::text_edit::AddTextError) to the fixed
/// "what would lift it" hint (§6.2 table) — the add-text sibling of
/// [`edit_refusal_hint`]/[`reflow_refusal_hint`]. Font refusals reuse 14.3's
/// existing `r_inv_*_hint` functions, keyed by trigger, rather than a copy.
fn add_text_refusal_hint(err: &pdfce_core::text_edit::AddTextError) -> &'static str {
    use pdfce_core::text_edit::{AddTextError, RInvTrigger};
    match err {
        AddTextError::Refused(r) => match r.trigger {
            RInvTrigger::TargetAbsent => ui_text::r_inv_1_hint(),
            RInvTrigger::BeyondRepertoire => ui_text::r_inv_repertoire_hint(),
            RInvTrigger::LigatureOnly => ui_text::r_inv_ligature_hint(),
            RInvTrigger::CodeOccupied => ui_text::r_inv_code_occupied_hint(),
            _ => ui_text::r_inv_encoding_hint(),
        },
        AddTextError::InvalidSize(_) => ui_text::add_text_invalid_size_hint(),
        AddTextError::InvalidBox(_, _) => ui_text::add_text_invalid_box_hint(),
        AddTextError::NoWordsToWrap => ui_text::add_text_no_words_hint(),
        AddTextError::Encrypted => ui_text::add_text_encrypted_hint(),
        AddTextError::HiddenObjects { .. } => ui_text::add_text_hidden_objects_hint(),
        // Should be unreachable from the GUI (the page is the open one; Accept
        // is gated on a non-empty draft) — framed as a bug, not an action.
        AddTextError::PageIndex(_) | AddTextError::EmptyText => {
            ui_text::add_text_internal_bug_hint()
        }
        // Structural/save failures render their own Display verbatim above.
        _ => ui_text::add_text_generic_hint(),
    }
}

/// Paint the reviewable "PREVIEW — not yet applied" frame (§4.1): a dashed
/// amber border (the project's reserved "reviewable, not-yet-applied" hue, rule
/// 6) + the corner tag, reused verbatim from the 14.3/15.2 visual language.
fn paint_add_preview_frame(painter: &egui::Painter, screen_box: egui::Rect, colour: egui::Color32) {
    let dash = egui::Stroke::new(1.5, colour);
    for (a, z) in [
        (screen_box.left_top(), screen_box.right_top()),
        (screen_box.right_top(), screen_box.right_bottom()),
        (screen_box.right_bottom(), screen_box.left_bottom()),
        (screen_box.left_bottom(), screen_box.left_top()),
    ] {
        painter.add(egui::Shape::dashed_line(&[a, z], dash, 4.0, 3.0));
    }
    painter.text(
        screen_box.left_top() + egui::vec2(1.0, -13.0),
        egui::Align2::LEFT_BOTTOM,
        ui_text::preview_tag(),
        egui::FontId::proportional(11.0),
        colour,
    );
}

/// Drive the Pass 16.2 Add-Page-Text tool for one frame: placement (click→point
/// / drag→box / keyboard-entry), composing + live preview, the property bar,
/// the disclosure/refusal strip, and committing an accepted add through
/// `EditSession::add_text` as ONE undo-able `CommandKind::AddText` (§3–§7).
///
/// A free function (not a `PdfceApp` method) so it can take `&mut OpenDoc` plus
/// `&self.font_env` as two disjoint borrows — the same split
/// `run_text_edit_tool`/`settle_and_rasterize` use. `doc.pages` (read for the
/// coordinate bridge) and `doc.add_text` (the mutated tool state) are disjoint
/// fields, so both are borrowed across the frame; `doc.session`/`refresh_pages`
/// are touched only in the Phase-C commit, after the state borrow is dropped.
#[allow(
    clippy::too_many_lines,
    reason = "one tool = one handler; splitting the tightly-coupled placement/compose/preview/commit phases would need shared owned scratch structs that obscure more than they clarify" // ui-text-exempt: clippy lint justification, never displayed
)]
fn run_add_text_tool(
    doc: &mut OpenDoc,
    ui: &mut egui::Ui,
    image_response: &egui::Response,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
    font_env: &pdfce_render::FontEnvironment,
) {
    use pdfce_core::text_edit::{AddTextRequest, BlockAlignment, FontProvenance, preview_wrap};

    let page_index = doc.view.page_index;
    // (Re)point state on page navigation while the tool stays active (§2.1):
    // keep prop-bar values, clear the draft (an unaccepted add on page 3 has no
    // meaning on page 4). No page text is read (Add-Text never reads content).
    match doc.add_text.as_mut() {
        Some(st) if st.page_index != page_index => {
            st.page_index = page_index;
            st.draft = None;
            st.drag_anchor = None;
        }
        Some(_) => {}
        None => return,
    }
    if doc.pages.get(page_index).is_none() {
        return;
    }

    // Intents captured in the UI closures, applied in Phase C.
    let mut do_accept = false;
    let mut do_reject = false;
    let mut switch_to_edit = false;

    // ---- Phase A/B: gestures, typing, live preview, property bar, strip ----
    {
        let page = &doc.pages[page_index];
        let crop = page.crop_box;
        let painter = ui.painter_at(image_rect);
        let text_color = ui.visuals().text_color();
        let preview_orange = egui::Color32::from_rgb(210, 90, 40);
        let mask_fill = egui::Color32::from_rgba_unmultiplied(250, 250, 250, 220);
        let ink = egui::Color32::from_rgb(20, 20, 20);
        // Coordinate bridges — capture `page` (immutable) + Copy values, never
        // `state`, so they coexist with the mutable state borrow below. The
        // SAME canvas→PDF bridge 14.3 §3 established (never `screen_to_page`
        // alone — that is device/rotated canvas space, not PDF user space).
        let to_screen = |pdf: egui::Pos2| -> Option<egui::Pos2> {
            viewer::pdf_space_to_canvas(pdf, page)
                .map(|c| viewer::page_to_screen(c, image_rect, extent, zoom))
        };
        let to_pdf = |sp: egui::Pos2| -> Option<egui::Pos2> {
            viewer::canvas_to_pdf_space(viewer::screen_to_page(sp, image_rect, extent, zoom), page)
        };

        let Some(state) = doc.add_text.as_mut() else {
            return;
        };

        // -- Placement gestures (§3.1/§3.2): click→point, drag→box (rubber-band,
        //    rotation/zoom-correct via the bridge), degenerate drag→point at the
        //    drag START (canvas::resolve_drag_placement, §3.2's deliberate
        //    divergence from Pass 6.1's discard rule). --
        if image_response.drag_started() {
            state.draft = None;
            state.drag_anchor = image_response.interact_pointer_pos();
        } else if image_response.drag_stopped() {
            if let (Some(anchor), Some(end)) = (
                state.drag_anchor.take(),
                image_response.interact_pointer_pos(),
            ) && let (Some(p0), Some(p1)) = (to_pdf(anchor), to_pdf(end))
            {
                let placement = canvas::resolve_drag_placement(
                    (f64::from(p0.x), f64::from(p0.y)),
                    (f64::from(p1.x), f64::from(p1.y)),
                    MIN_ADD_TEXT_BOX_PT,
                );
                install_add_placement(state, placement);
            }
            state.drag_anchor = None;
        } else if image_response.clicked()
            && let Some(sp) = image_response.interact_pointer_pos()
            && let Some(pdf) = to_pdf(sp)
        {
            // A click always means "place new text" here — no hit-vs-miss
            // branch (§0.1). A click while a draft exists discards it and
            // places anew (§3.1).
            install_add_placement(
                state,
                canvas::AddTextPlacement::Point {
                    x: f64::from(pdf.x),
                    y: f64::from(pdf.y),
                },
            );
        }

        // -- Composing (§4.1): append/remove from draft_text directly. NO core
        //    call per keystroke. In box mode a plain Enter is a paragraph break;
        //    Ctrl+Enter accepts. In point mode Enter accepts (single line). --
        if image_response.has_focus() {
            let events = ui.input(|i| i.events.clone());
            if let Some(draft) = state.draft.as_mut() {
                let is_box = matches!(draft.placement, AddPlacement::Box { .. });
                for ev in events {
                    match ev {
                        egui::Event::Text(t) if !t.is_empty() => draft.draft_text.push_str(&t),
                        egui::Event::Key {
                            key: egui::Key::Backspace,
                            pressed: true,
                            ..
                        } => {
                            draft.draft_text.pop();
                        }
                        egui::Event::Key {
                            key: egui::Key::Enter,
                            pressed: true,
                            modifiers,
                            ..
                        } => {
                            if is_box && !modifiers.command {
                                draft.draft_text.push('\n');
                            } else if !draft.draft_text.is_empty() {
                                do_accept = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // -- Box-mode live wrap preview (§4.2): a PURE, per-frame read-only
        //    `preview_wrap` — NOT the "no core call per keystroke" rule (that
        //    targets MUTATING calls that force an incremental-save/reparse), the
        //    exact 15.2 `ReflowEngine::preview` precedent. Recomputed from the
        //    draft + prop-bar values; the wrap the operator reviews is the wrap
        //    the commit re-derives (one shared layout path, decision 016 §0.3).
        let (pf, ps, pa, pl) = (
            state.prop_font,
            state.prop_size,
            state.prop_alignment,
            state.prop_leading,
        );
        if let Some(draft) = state.draft.as_mut() {
            match draft.placement {
                AddPlacement::Box {
                    llx,
                    lly,
                    width,
                    height,
                } if !draft.draft_text.is_empty() => {
                    let bx = pdfce_core::page_tree::Rect::from_corners(
                        llx,
                        lly,
                        llx + width,
                        lly + height,
                    );
                    let leading = (pl > 0.0).then_some(pl);
                    draft.wrap_preview = Some(
                        preview_wrap(&draft.draft_text, bx, crop, pf, ps, pa, leading)
                            .map_err(|e| e.to_string()),
                    );
                }
                _ => draft.wrap_preview = None,
            }
        }

        // -- Rubber-band while dragging a box (§3.2): a plain dashed amber rect,
        //    the "not yet real" hue at 1.5px; no PREVIEW tag until text exists. --
        if image_response.dragged()
            && let (Some(anchor), Some(cur)) =
                (state.drag_anchor, image_response.interact_pointer_pos())
        {
            paint_add_preview_frame(
                &painter,
                egui::Rect::from_two_pos(anchor, cur),
                preview_orange,
            );
        }

        // -- Live preview render (§4.1/§4.2/§4.3): reuse 14.3's mask/ink/dashed/
        //    tag visual language, generalized to a placement origin/box. --
        if let Some(draft) = state.draft.as_ref() {
            let size = (ps * f64::from(zoom)).clamp(6.0, 96.0) as f32;
            let font_id = egui::FontId::proportional(size);
            match &draft.placement {
                AddPlacement::Point { x, y } => {
                    if let Some(origin) = to_screen(egui::pos2(*x as f32, *y as f32)) {
                        if draft.draft_text.is_empty() {
                            // §4.3 blinking insertion caret before typing, sized
                            // from the property bar's current size (no glyph to
                            // read a size from). A real line, never a tint.
                            let top = egui::pos2(origin.x, origin.y - size * 0.75);
                            let bot = egui::pos2(origin.x, origin.y + size * 0.25);
                            if (ui.input(|i| i.time) * 1.5).fract() < 0.5 {
                                painter
                                    .line_segment([top, bot], egui::Stroke::new(1.5, text_color));
                            }
                            ui.ctx().request_repaint(); // keep the caret blinking
                        } else {
                            // Measure (layout only) so the mask exactly contains
                            // the drawn draft text; then mask, ink, dashed + tag.
                            let sz = painter
                                .layout_no_wrap(draft.draft_text.clone(), font_id.clone(), ink)
                                .size();
                            let top_left = egui::pos2(origin.x, origin.y - sz.y + size * 0.25);
                            let screen_box = egui::Rect::from_min_size(top_left, sz).expand(2.0);
                            painter.rect_filled(screen_box, 0.0, mask_fill);
                            painter.text(
                                top_left,
                                egui::Align2::LEFT_TOP,
                                &draft.draft_text,
                                font_id,
                                ink,
                            );
                            paint_add_preview_frame(&painter, screen_box, preview_orange);
                        }
                    }
                }
                AddPlacement::Box {
                    llx,
                    lly,
                    width,
                    height,
                } => {
                    if let (Some(tl), Some(br)) = (
                        to_screen(egui::pos2(*llx as f32, (*lly + *height) as f32)),
                        to_screen(egui::pos2((*llx + *width) as f32, *lly as f32)),
                    ) {
                        let box_rect = egui::Rect::from_two_pos(tl, br);
                        painter.rect_filled(box_rect, 0.0, mask_fill);
                        // Ghost lines from the pure wrap preview (§4.2), at the
                        // exact per-line origins the commit will emit.
                        if let Some(Ok(pv)) = &draft.wrap_preview {
                            for line in &pv.lines {
                                if line.text.is_empty() {
                                    continue;
                                }
                                if let Some(sp) = to_screen(egui::pos2(
                                    line.origin_x as f32,
                                    line.baseline_y as f32,
                                )) {
                                    painter.text(
                                        sp,
                                        egui::Align2::LEFT_BOTTOM,
                                        &line.text,
                                        font_id.clone(),
                                        ink,
                                    );
                                }
                            }
                        }
                        if draft.draft_text.is_empty() {
                            // §4.3 the placed box outline before typing: solid,
                            // thin, neutral — a real boundary, no PREVIEW tag yet.
                            let stroke = egui::Stroke::new(1.0, text_color);
                            for (a, z) in [
                                (box_rect.left_top(), box_rect.right_top()),
                                (box_rect.right_top(), box_rect.right_bottom()),
                                (box_rect.right_bottom(), box_rect.left_bottom()),
                                (box_rect.left_bottom(), box_rect.left_top()),
                            ] {
                                painter.line_segment([a, z], stroke);
                            }
                        } else {
                            paint_add_preview_frame(&painter, box_rect, preview_orange);
                        }
                    }
                }
            }
        }

        // -- Property bar (§3.3/§5.2): a floating top panel. Every control is a
        //    REAL egui widget (accesskit), never painter-drawn. --
        egui::Area::new(egui::Id::new("pdfce-add-text-propbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(image_rect.min + egui::vec2(8.0, 8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(380.0);
                    ui.label(ui_text::add_text_propbar_title());
                    ui.label(ui_text::add_text_hint());

                    // §3.3 keyboard-complete placement (point AND box halves) —
                    // a genuine keyboard path to the SAME draft a click/drag
                    // reaches, not a lesser fallback.
                    ui.horizontal(|ui| {
                        ui.label(ui_text::add_text_origin_label());
                        ui.add(egui::DragValue::new(&mut state.manual_origin[0]).speed(1.0));
                        ui.add(egui::DragValue::new(&mut state.manual_origin[1]).speed(1.0));
                    });
                    ui.checkbox(&mut state.use_box, ui_text::add_text_use_box_checkbox());
                    if state.use_box {
                        ui.horizontal(|ui| {
                            ui.label(ui_text::add_text_box_size_label());
                            ui.add(
                                egui::DragValue::new(&mut state.manual_box[0])
                                    .range(1.0..=100_000.0)
                                    .speed(1.0),
                            );
                            ui.add(
                                egui::DragValue::new(&mut state.manual_box[1])
                                    .range(1.0..=100_000.0)
                                    .speed(1.0),
                            );
                        });
                        if ui
                            .add_sized(
                                ICON_BUTTON_SIZE,
                                egui::Button::new(ui_text::add_text_place_box_button()),
                            )
                            .clicked()
                        {
                            let [x, y] = state.manual_origin;
                            let [w, h] = state.manual_box;
                            install_add_placement(
                                state,
                                canvas::AddTextPlacement::Box {
                                    llx: x,
                                    lly: y,
                                    width: w,
                                    height: h,
                                },
                            );
                        }
                    } else if ui
                        .add_sized(
                            ICON_BUTTON_SIZE,
                            egui::Button::new(ui_text::add_text_place_point_button()),
                        )
                        .clicked()
                    {
                        let [x, y] = state.manual_origin;
                        install_add_placement(state, canvas::AddTextPlacement::Point { x, y });
                    }

                    ui.separator();
                    // §5.2 font — the 14 Std-14 faces + Bundled/Supplied trust
                    // (the SAME classify_nonembedded the diagnostics use), from
                    // the spec-frozen Std14::ALL.
                    ui.horizontal(|ui| {
                        ui.label(ui_text::format_font_label());
                        let current = std14_combo_label(state.prop_font, font_env);
                        egui::ComboBox::from_id_salt("pdfce-add-text-font")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for face in pdfce_core::fontdata::Std14::ALL {
                                    let label = std14_combo_label(face, font_env);
                                    ui.selectable_value(&mut state.prop_font, face, label);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label(ui_text::format_size_label());
                        ui.add(egui::DragValue::new(&mut state.prop_size).range(1.0..=1000.0));
                    });
                    // §5.2 colour — the honest two-state surface (black default /
                    // custom RGB); core has no Gray/CMYK for Add-Text.
                    ui.horizontal(|ui| {
                        ui.label(ui_text::markup_color_label());
                        ui.color_edit_button_srgba(&mut state.prop_color);
                    });
                    // §5.2 alignment + leading — box mode only (a fresh box has
                    // no glyphs to auto-detect from; default Left).
                    let is_box = state.use_box
                        || matches!(
                            state.draft.as_ref().map(|d| &d.placement),
                            Some(AddPlacement::Box { .. })
                        );
                    if is_box {
                        ui.horizontal(|ui| {
                            ui.label(ui_text::add_text_align_label());
                            for (val, label) in [
                                (BlockAlignment::Left, "Left"),
                                (BlockAlignment::Center, "Center"),
                                (BlockAlignment::Right, "Right"),
                                (BlockAlignment::Justified, "Justify"),
                            ] {
                                ui.selectable_value(&mut state.prop_alignment, val, label);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(ui_text::add_text_leading_label());
                            ui.add(
                                egui::DragValue::new(&mut state.prop_leading)
                                    .range(0.0..=10_000.0)
                                    .speed(0.2),
                            );
                        });
                    }
                });
            });

        // -- Accept/Reject + disclosure/refusal strip (§6/§7): a floating bottom
        //    panel. Accept/Reject are REAL buttons (accesskit). --
        egui::Area::new(egui::Id::new("pdfce-add-text-status"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(image_rect.min.x + 8.0, image_rect.max.y - 8.0))
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(460.0);
                    if let Some(draft) = state.draft.as_ref() {
                        let empty = draft.draft_text.is_empty();
                        ui.horizontal(|ui| {
                            let accept = ui
                                .add_enabled(!empty, egui::Button::new(ui_text::add_text_accept()));
                            let accept = if empty {
                                accept.on_hover_text(ui_text::add_text_empty_tooltip())
                            } else {
                                accept
                            };
                            if accept.clicked() {
                                do_accept = true;
                            }
                            if ui
                                .add(egui::Button::new(ui_text::add_text_reject()))
                                .clicked()
                            {
                                do_reject = true;
                            }
                        });
                        // §4.2/§6: box-mode live derived/overflow disclosures OR
                        // the wrap refusal — verbatim, while deciding (R76).
                        if let Some(preview) = &draft.wrap_preview {
                            match preview {
                                Ok(pv) => {
                                    for d in &pv.disclosures {
                                        ui.label(ui_text::disclosure_bullet(d));
                                    }
                                }
                                Err(e) => {
                                    ui.colored_label(
                                        ui.visuals().error_fg_color,
                                        ui_text::refusal_line(e),
                                    );
                                }
                            }
                        }
                        // §6.4: the last Accept attempt's refusal (verbatim +
                        // hint), kept visible for revision — never a dead end.
                        if let Some(text) = &draft.last_refusal {
                            ui.separator();
                            ui.label(ui_text::refusal_strip_title());
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                ui_text::refusal_line(text),
                            );
                        }
                    }
                    // §6.1: the last ACCEPTED add's disclosures, VERBATIM (all
                    // three possible: font provenance R79, /Resources
                    // inheritance §7.7.3.4, tagged-untagged R73). Persist until
                    // the next Accept or tool exit. §7.2 continuity link below.
                    if !state.last_disclosures.is_empty() {
                        ui.separator();
                        ui.label(ui_text::disclosure_strip_title());
                        for d in &state.last_disclosures {
                            ui.label(ui_text::disclosure_bullet(d));
                        }
                        if ui.button(ui_text::edit_this_text_now_button()).clicked() {
                            switch_to_edit = true;
                        }
                    }
                });
            });
    }

    // ---- Phase C: the session mutation (one undo-able command) ----
    if do_reject {
        // §7.3: nothing was ever written (rule 7) — drop the draft, stay in tool.
        if let Some(state) = doc.add_text.as_mut() {
            state.draft = None;
            state.drag_anchor = None;
        }
        return;
    }
    if switch_to_edit {
        // §7.2 P1 continuity: switch to Edit Text so the just-added run (now
        // ordinary page content) is re-editable — its extraction happens on the
        // TextEdit tool's own entry (14.3 §2.1), unchanged by this Pass.
        doc.active_tool = Some(CanvasTool::TextEdit);
        doc.build_text_edit_state();
        doc.add_text = None;
        return;
    }
    if do_accept {
        // §7.1: build ONE `AddTextRequest` from the draft (point OR box), commit
        // through the ALREADY-SHIPPED `EditSession::add_text` — one undo-able
        // `CommandKind::AddText`, normal Ctrl+Z.
        let req = doc.add_text.as_ref().and_then(|s| {
            s.draft.as_ref().map(|d| {
                let base = match d.placement {
                    AddPlacement::Point { x, y } => {
                        AddTextRequest::new(page_index, (x, y), d.draft_text.clone())
                    }
                    AddPlacement::Box {
                        llx,
                        lly,
                        width,
                        height,
                    } => AddTextRequest::new(page_index, (0.0, 0.0), d.draft_text.clone())
                        .with_box(llx, lly, width, height)
                        .with_alignment(s.prop_alignment)
                        .with_leading((s.prop_leading > 0.0).then_some(s.prop_leading)),
                };
                // §0.4 provenance: the ONE classify_nonembedded call, wrapped —
                // Bundled/Supplied is a preview-fidelity fact, the WRITTEN dict
                // is identical either way (R79).
                let provenance = match font_env
                    .classify_nonembedded(pdfce_core::fontdata::std14_base_font_name(s.prop_font))
                {
                    pdfce_render::GlyphSource::Supplied => FontProvenance::Supplied,
                    _ => FontProvenance::Bundled,
                };
                base.with_font(s.prop_font)
                    .with_provenance(provenance)
                    .with_size(s.prop_size)
                    .with_color(add_text_color(s.prop_color))
            })
        });
        if let Some(req) = req {
            match doc.session.add_text(&req) {
                Ok(report) => {
                    doc.refresh_pages(); // the page's content changed
                    // Tool STAYS active (§7.2): drop the draft, keep prop-bar
                    // values, surface the disclosures verbatim (§6.1). No
                    // rebuild-from-default — that would reset the operator's
                    // chosen font/size mid-session.
                    if let Some(state) = doc.add_text.as_mut() {
                        state.draft = None;
                        state.drag_anchor = None;
                        state.last_disclosures = report.disclosures;
                    }
                }
                Err(err) => {
                    // §6.2: verbatim Display + the fixed hint; the draft stays
                    // for revision, never a crash (rule 4).
                    let msg =
                        ui_text::refusal_with_hint(&err.to_string(), add_text_refusal_hint(&err));
                    if let Some(d) = doc.add_text.as_mut().and_then(|s| s.draft.as_mut()) {
                        d.last_refusal = Some(msg);
                    }
                }
            }
        }
    }
}

/// A single property-bar format operation, chosen in phase B and applied in
/// phase C (only one per frame — the operator clicked one Apply).
enum FormatOp {
    Size(f64),
    Fill(pdfce_core::text_edit::NewFill),
    Font(String),
}

/// Insert typed text `t` into the pending edit, creating the `PendingEdit`
/// from the caret's run on the first keystroke (§6.1). The insertion point is
/// tracked as a byte offset (`PendingEdit::cursor`) into the draft.
///
/// Pass 14.4: a printable char typed over a NON-empty single-run selection
/// REPLACES it — [`consume_selection_into_pending`] removes the span and
/// collapses the caret to its start first, then the char inserts there. (A
/// cross-run selection never reaches here: the typing loop is suppressed for
/// `cross_run`, §4.4.) The font-on-edit gate is unaffected — a replacement char
/// the run's font cannot provide is still refused at Accept time (§8.2).
fn text_edit_insert(state: &mut TextEditState, t: &str) {
    consume_selection_into_pending(state);
    ensure_pending(state);
    if let Some(p) = state.pending.as_mut() {
        let at = p.cursor.min(p.draft_text.len());
        if p.draft_text.is_char_boundary(at) {
            p.draft_text.insert_str(at, t);
            p.cursor = at + t.len();
            p.last_refusal = None;
        }
    }
}

/// Backspace from the pending draft (§6.1). Pass 14.4: with an active selection
/// this deletes the SELECTION (and nothing more) — the removal is exactly what
/// [`consume_selection_into_pending`] does, so on a consumed selection the
/// function returns before deleting an additional character. Otherwise it
/// removes the one character before the cursor.
fn text_edit_backspace(state: &mut TextEditState) {
    if consume_selection_into_pending(state) {
        return;
    }
    ensure_pending(state);
    if let Some(p) = state.pending.as_mut() {
        let at = p.cursor.min(p.draft_text.len());
        // Find the previous char boundary.
        let prev = p.draft_text[..at]
            .char_indices()
            .next_back()
            .map(|(i, _)| i);
        if let Some(prev) = prev {
            p.draft_text.replace_range(prev..at, "");
            p.cursor = prev;
            p.last_refusal = None;
        }
    }
}

/// Forward-delete (the Delete key, §6.1). Pass 14.4: with an active selection
/// this deletes the SELECTION (via [`consume_selection_into_pending`]); with a
/// bare caret it removes the one character AT the cursor, leaving the cursor in
/// place. A no-op at the draft's end.
fn text_edit_delete(state: &mut TextEditState) {
    if consume_selection_into_pending(state) {
        return;
    }
    ensure_pending(state);
    if let Some(p) = state.pending.as_mut() {
        let at = p.cursor.min(p.draft_text.len());
        if !p.draft_text.is_char_boundary(at) {
            return;
        }
        if let Some(c) = p.draft_text[at..].chars().next() {
            p.draft_text.replace_range(at..at + c.len_utf8(), "");
            p.last_refusal = None;
        }
    }
}

/// Start the pending edit with the active single-run selection removed and the
/// caret collapsed to its start — the "typing or deleting REPLACES the
/// selection" spine (Pass 14.4, spec §6.1). Returns `true` when it consumed a
/// selection, so a bare Backspace/Delete knows the removal WAS the delete and
/// must not then remove an additional character.
///
/// A no-op (returns `false`) when a pending edit already exists, when there is
/// no selection, or when the selection spans more than one run
/// ([`canvas::single_run_selection_range`] returns `None` for a cross-run span,
/// which the typing loop also refuses via `cross_run`, §4.4). The removal is
/// staged in `draft_text` only — never written anywhere — so it stays a
/// reviewable `PendingEdit` the operator Accepts or Rejects (rule 4), and the
/// font-on-edit gate still fires at Accept if a later inserted char is
/// unavailable (§8.2).
fn consume_selection_into_pending(state: &mut TextEditState) -> bool {
    if state.pending.is_some() {
        return false;
    }
    let Some((run, lo, hi)) = canvas::single_run_selection_range(state.caret, state.anchor) else {
        return false;
    };
    let Some(src) = state.page_text.runs.get(run) else {
        return false;
    };
    let original = src.text.clone();
    let (draft, cursor) = canvas::selection_after_type(&original, lo, hi, "");
    state.pending = Some(PendingEdit {
        run,
        original_text: original,
        draft_text: draft,
        cursor,
        last_refusal: None,
    });
    // Collapse to the removed span's start: same run, so `caret.run` still pins
    // the operator_span at Accept; anchor cleared so no re-trigger.
    state.caret = Some(pdfce_core::text_edit::TextPosition::new(run, lo));
    state.anchor = None;
    true
}

/// Create the `PendingEdit` from the caret's run if none exists (§6.1): the
/// draft starts as the run's original text with the caret positioned at the
/// caret's byte offset within that run. Insertion/deletion then acts at the
/// cursor. Selection-replace is handled BEFORE this, by
/// [`consume_selection_into_pending`], which may already have created the
/// pending edit (in which case this is a no-op).
fn ensure_pending(state: &mut TextEditState) {
    if state.pending.is_some() {
        return;
    }
    let Some(caret) = state.caret else {
        return;
    };
    let Some(run) = state.page_text.runs.get(caret.run) else {
        return;
    };
    let original = run.text.clone();
    let cursor = caret.byte_offset.min(original.len());
    let cursor = if original.is_char_boundary(cursor) {
        cursor
    } else {
        0
    };
    state.pending = Some(PendingEdit {
        run: caret.run,
        original_text: original.clone(),
        draft_text: original,
        cursor,
        last_refusal: None,
    });
}

/// The selection checkbox's rect inside a thumbnail's rect.
///
/// Top-left corner, at least the click-target size the icon buttons use,
/// so it is hittable without precision aiming. One function rather than
/// two literals, because the paint and the hit-test must agree exactly —
/// if they drift, the box shows in one place and responds in another.
fn selection_box(thumbnail: egui::Rect) -> egui::Rect {
    const INSET: f32 = 4.0;
    const SIDE: f32 = 18.0;
    egui::Rect::from_min_size(
        thumbnail.min + egui::vec2(INSET, INSET),
        egui::vec2(SIDE, SIDE),
    )
}

/// Write `bytes` to `path` atomically: land them in a temp file in the
/// **destination's own directory**, flush to disk, then rename over the
/// target.
///
/// WHY (standing UX rule 5 — crash-safe writes): a plain
/// `std::fs::write` truncates the destination first, so a crash or kill
/// mid-write leaves a corrupt file where a good one may have stood —
/// exactly the re-save-over-my-previous-copy workflow the save dialog
/// invites. With temp-then-rename the previous good file survives until
/// the complete new bytes exist; the rename is the commit point.
///
/// Mechanics that matter:
/// - The temp file lives in the SAME directory as `path`, because
///   `std::fs::rename` is atomic only within a filesystem; a temp dir on
///   another volume would silently degrade to copy+delete.
/// - The temp name embeds the process id so two pdfce instances saving
///   into the same directory cannot collide.
/// - `sync_all` before the rename makes the data durable before the
///   commit point, closing the window where the rename lands but the
///   bytes are still only in the OS cache.
/// - On failure after the temp file was created, the temp file is
///   removed on a best-effort basis; the destination is untouched.
/// - `std::fs::rename` replaces an existing destination on both Windows
///   (`MOVEFILE_REPLACE_EXISTING`) and Unix, which is what "save over
///   the earlier copy" needs.
fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp_name = format!(".{}.pdfce-tmp-{}", file_name, std::process::id());
    let tmp_path = match dir {
        Some(dir) => dir.join(&tmp_name),
        None => std::path::PathBuf::from(&tmp_name),
    };

    let write_result = (|| {
        let mut tmp = std::fs::File::create(&tmp_path)?;
        tmp.write_all(bytes)?;
        tmp.sync_all()?;
        drop(tmp);
        std::fs::rename(&tmp_path, path)
    })();

    if write_result.is_err() {
        // Best-effort cleanup; the original error is what matters.
        let _ = std::fs::remove_file(&tmp_path);
    }
    write_result
}

/// Paint the Pass 6.0 annotation-disclosure lines below the content
/// render diagnostics (ISO 32000-1 §12.5; R50/R27/R51).
///
/// A free function rather than a method so it borrows only what it needs
/// (the `Diagnostics`, the toggle state, and the `/NeedAppearances`
/// flag), keeping it out of the `PdfceApp` borrow that the surrounding
/// `status_bar` already holds against `self.status`.
///
/// Renders **nothing** when there is nothing to disclose — a page with no
/// annotations, annotations visible, none hidden, no `/NeedAppearances`
/// stays silent, so the presence of a line is a real signal (the same
/// discipline the content diagnostics follow). When the toggle is OFF and
/// the page has annotations, that fact is stated regardless, so a
/// suppressed view is never silently empty of markup the operator forgot
/// they hid.
fn annotation_status(
    ui: &mut eframe::egui::Ui,
    d: &pdfce_render::Diagnostics,
    annotations_visible: bool,
    need_appearances: bool,
) {
    let no_appearance: usize = d.annotations_without_ap.values().sum();

    // Toggle OFF: state that markup exists but is hidden by choice.
    if !annotations_visible {
        if d.annotations_total > 0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                ui_text::annotations_display_off(d.annotations_total),
            );
        }
    } else if d.annotations_total > 0 {
        // Toggle ON: an informational summary, then the R43/R27 gaps.
        ui.label(ui_text::annotations_painted_summary(
            d.annotations_total,
            d.annotations_painted,
            d.annotations_widget,
        ));
        if no_appearance > 0 {
            let by_subtype: Vec<(String, usize)> = d
                .annotations_without_ap
                .iter()
                .map(|(subtype, count)| (subtype.clone(), *count))
                .collect();
            ui.colored_label(
                ui.visuals().warn_fg_color,
                ui_text::annotations_no_appearance(no_appearance, &by_subtype),
            );
        }
        if d.annotations_appearance_state_missing > 0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                ui_text::annotations_state_missing(d.annotations_appearance_state_missing),
            );
        }
        if d.annotations_placement_degenerate > 0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                ui_text::annotations_degenerate(d.annotations_placement_degenerate),
            );
        }
    }

    // Hidden/NoView is a file fact independent of the toggle (R50) — a
    // document-forensics-relevant disclosure, shown either way.
    if d.annotations_hidden > 0 {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            ui_text::annotations_hidden(d.annotations_hidden),
        );
    }
    // /NeedAppearances disclosure (R51), likewise document-level.
    if need_appearances {
        ui.label(ui_text::annotations_need_appearances());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three-way load-outcome branch is the module docs' central
    /// honesty claim, so the classifier behind it is pinned here: a
    /// deliberate "not yet supported" refusal must never be reported as
    /// a damaged file.
    #[test]
    fn named_capability_gaps_are_not_reported_as_damage() {
        use pdfce_core::xref::XrefError;
        // Encryption (§7.6) is currently the only such gap; when a
        // second one lands, extend this into a loop over the list.
        let err = DocError::Xref(XrefError {
            offset: 0,
            kind: XrefErrorKind::EncryptionUnsupported,
        });
        assert!(is_unsupported_structure(&err));
    }

    #[test]
    fn real_structural_damage_is_not_a_capability_gap() {
        use pdfce_core::xref::XrefError;
        // A broken xref is a broken FILE, and must read as one.
        for kind in [
            XrefErrorKind::StartxrefNotFound,
            XrefErrorKind::BadEntry,
            XrefErrorKind::NotAnXrefSection,
            XrefErrorKind::PrevChainCycle,
        ] {
            let err = DocError::Xref(XrefError { offset: 0, kind });
            assert!(!is_unsupported_structure(&err));
        }
        let not_a_pdf = DocError::Header(pdfce_core::PdfError::MissingHeader { searched: 8 });
        assert!(!is_unsupported_structure(&not_a_pdf));
        let io = DocError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert!(!is_unsupported_structure(&io));
    }

    /// The save dialog must never open pre-aimed at the file the
    /// operator has open. Non-destructive by default is a standing UI
    /// rule, and a default file name equal to the input turns one
    /// reflexive Enter into an overwrite.
    #[test]
    fn the_suggested_save_name_is_never_the_original_name() {
        let path = PathBuf::from("C:/docs/contract.pdf");
        let suggested = ui_text::suggested_save_name(&path);
        assert_ne!(suggested, "contract.pdf");
        assert!(suggested.starts_with("contract"), "{suggested}"); // ui-text-exempt: test assertion; the R1 grep's heuristic matches the gap BETWEEN two literals, not prose
        assert!(suggested.ends_with(".pdf"), "{suggested}"); // ui-text-exempt: test assertion; heuristic matches the gap between two literals
        // A path with no stem still yields something usable rather than
        // an empty file name the dialog would reject.
        let suggested = ui_text::suggested_save_name(&PathBuf::from("/"));
        assert!(suggested.ends_with(".pdf"), "{suggested}"); // ui-text-exempt: test assertion; heuristic matches the gap between two literals
        assert!(suggested.len() > 4, "{suggested}");
    }

    /// Every editable metadata field needs a real label. The catalog's
    /// wildcard arm exists for forward compatibility with a
    /// `#[non_exhaustive]` enum, and this pins that it is never reached
    /// for a field the panel actually shows.
    #[test]
    fn every_editable_metadata_field_has_a_distinct_label() {
        let mut labels: Vec<&str> = InfoField::all()
            .into_iter()
            .map(ui_text::info_field_label)
            .collect();
        assert!(
            !labels.iter().any(|l| l.starts_with('(')),
            "a shown field fell through to the placeholder arm: {labels:?}" // ui-text-exempt: test assertion, never shown to an operator
        );
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), before, "two fields share a label"); // ui-text-exempt: test assertion message
    }

    /// The unsaved-changes marker is driven by the same save-time diff
    /// a save computes, so it cannot disagree with what saving writes.
    /// Here we only pin that the two states are distinguishable at all —
    /// a marker that renders identically either way is not a marker.
    #[test]
    fn the_toolbar_distinguishes_saved_from_unsaved() {
        let path = PathBuf::from("doc.pdf");
        let version = pdfce_core::PdfVersion { major: 1, minor: 7 };
        let clean = ui_text::status_open(&path, version, 3, false);
        let dirty = ui_text::status_open(&path, version, 3, true);
        assert_ne!(clean, dirty);
        assert!(dirty.len() > clean.len());
    }

    /// The debounce only exists to swallow a *gesture*'s intermediate
    /// values. If a button or chord ever got classified as continuous,
    /// every zoom click would feel 150 ms late for no benefit — so the
    /// classification is pinned rather than left to review.
    #[test]
    fn only_the_scroll_gesture_is_debounced() {
        assert!(!Action::ZoomBy(1.1).is_discrete_zoom_command());
        for action in [
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomActualSize,
            Action::Fit(FitMode::Page),
            Action::Fit(FitMode::Width),
        ] {
            assert!(action.is_discrete_zoom_command());
        }
    }

    // ---- Pass 15.2 reflow sub-mode: pure state-machine helpers ----

    #[test]
    fn reflow_button_enabled_requires_a_target_and_no_pending_edit() {
        use super::reflow_button_enabled;
        // Enabled only when a paragraph resolved AND no single-run edit is in
        // flight (the mutual-exclusion rule, §1.4).
        assert!(reflow_button_enabled(Some(0), false));
        assert!(!reflow_button_enabled(None, false), "no target -> disabled");
        assert!(
            !reflow_button_enabled(Some(3), true),
            "a pending edit blocks entering reflow"
        );
        assert!(!reflow_button_enabled(None, true));
    }

    #[test]
    fn reflow_alignment_override_is_none_until_a_real_deviation() {
        use super::reflow_alignment_is_override;
        use pdfce_core::text_edit::BlockAlignment;
        // Picking the detected value is NOT an override (keeps AlignmentSource
        // honest: the request omits an explicit alignment); picking anything
        // else IS; picking back to detected un-overrides (§6.2).
        assert!(!reflow_alignment_is_override(
            BlockAlignment::Left,
            BlockAlignment::Left
        ));
        assert!(reflow_alignment_is_override(
            BlockAlignment::Left,
            BlockAlignment::Center
        ));
        assert!(reflow_alignment_is_override(
            BlockAlignment::Justified,
            BlockAlignment::Right
        ));
        assert!(!reflow_alignment_is_override(
            BlockAlignment::Center,
            BlockAlignment::Center
        ));
    }

    #[test]
    fn reflow_refusal_hint_names_a_next_step_for_each_condition() {
        use super::reflow_refusal_hint;
        use pdfce_core::text_edit::{ReflowApplyError, ReflowError};
        // The already-edited-this-session refusal (judgment call #6) maps to the
        // save-and-reopen hint, not a generic dead end.
        let already = ReflowApplyError::Unsupported(
            "the page's content was already edited this session".to_owned(),
        );
        assert_eq!(
            reflow_refusal_hint(&already),
            ui_text::reflow_already_edited_hint()
        );
        assert_eq!(
            reflow_refusal_hint(&ReflowApplyError::Preview(ReflowError::BadWidth(-1.0))),
            ui_text::reflow_bad_width_hint()
        );
        assert_eq!(
            reflow_refusal_hint(&ReflowApplyError::Preview(ReflowError::EmptyBlock(0))),
            ui_text::reflow_empty_block_hint()
        );
        // Every hint is a non-empty next-step sentence.
        assert!(!reflow_refusal_hint(&already).is_empty());
    }
}
