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
//! - **Drag does NOT pan** (corrected 2026-08-04; this previously read
//!   "Drag pans, via the scroll area's own drag-to-scroll"). Pass 9a made
//!   a drag on the canvas a rubber-band MARQUEE and moved panning to the
//!   wheel and scrollbars — deliberately, but its own comment records that
//!   a UX review was owed on that default and never happened. The operator
//!   asking for middle-drag panning IS that review arriving. Panning, by
//!   whatever gesture, triggers **no** re-raster: it moves the viewport
//!   over an existing texture.
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
mod diag;
mod dock;
mod icons;
mod measure_tool;
mod object_provider;
mod object_summary;
mod raster;
mod redact_apply;
mod ui_text;
mod vector_edit_tool;
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

use canvas::{CanvasTargetProvider, CanvasTool, EscapeOutcome, GestureInterrupt, TargetId};
use dock::{DockPanel, DockTree};
use object_provider::ObjectModelProvider;
use object_summary::{ObjectSummary, census, describe_object};
use raster::{PageTexture, ThumbnailCache};
use viewer::{FitMode, ViewState};

/// Initial window size, in logical points.
const INITIAL_WINDOW_SIZE: [f32; 2] = [1100.0, 800.0];

/// Fixed outer height of the bottom status panel, in points.
///
/// Sized for the common case — the one-line summary plus a selection or edit
/// note, roughly two to three lines — with the status bar's own `ScrollArea`
/// absorbing anything longer inside this budget rather than growing the panel.
///
/// It is a CONSTANT on purpose; see the call site for the defect that a
/// content-driven height caused (the page re-fitting because a click added a
/// line of text somewhere else on screen).
const STATUS_PANEL_HEIGHT_PTS: f32 = 92.0;

/// Outline colour for a selected SUBPATH inside an entered object.
///
/// Deliberately not the theme's selection accent, which means "this object is
/// selected". A subpath selection means something else — "inside this object,
/// this part" — and giving the two states the same cue would make a descent
/// look like an ordinary selection of a suspiciously small object.
///
/// Amber, matching the "in progress / not yet an edit" hue the measure and
/// add-text previews already use, because an entered object IS a transient
/// working state rather than a committed one.
const SUBPATH_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(210, 140, 40);

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

/// Minimum size for a button, so click targets stay usable regardless of how
/// narrow the content inside them happens to be.
///
/// # It is a FLOOR, not a size — use `.min_size()`, not `add_sized()`
///
/// `ui.add_sized(ICON_BUTTON_SIZE, button)` allocates *exactly* this
/// rectangle and lays the button out inside it. For an icon that is what you
/// want. For a button carrying a WORD it is a cap, and egui responds to a
/// 28 pt cap by wrapping the label one character at a time.
///
/// This was not a hypothetical. Six word-buttons were built with `add_sized`,
/// and observing the running Add-Text tool showed "Place point" rendered as
/// four stacked fragments — `Pla` / `ce` / `poi` / `nt` — in a column barely
/// wider than a scrollbar. The same defect had "Accept reflow" and "Reject
/// reflow" in it, on controls that terminate an edit gesture.
///
/// Worth recording *how* it surfaced: the glyph-coverage gate had just
/// replaced those buttons' tofu check/cross marks with drawable ones, and the
/// screenshot taken to confirm that fix is what exposed the layout. A test
/// can prove a character has a glyph; only looking proves the operator can
/// read the button (standing rule R86).
///
/// So: `ui.add(egui::Button::new(text).min_size(ICON_BUTTON_SIZE))` for
/// anything with a label — the accessibility floor without the cap — and
/// `add_sized` only for genuinely icon-only controls, which is what
/// `icon_button`/`glyph_button` already do.
const ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(28.0, 24.0);

/// Default width, in points, of the right-hand panel dock.
///
/// Raised from the historic 320 pt when the dock became an `egui_tiles`
/// panel host (decision 017 Amendment A.3). Two reasons, both structural
/// rather than aesthetic:
///
/// 1. **The dock now holds real content**, not a four-row tool list — an
///    object tree whose rows carry a kind, a paint disposition, a colour and
///    a node count, and a metadata form with a label column beside a text
///    column. 320 pt truncated both.
/// 2. **`egui_tiles` draws HORIZONTAL tab bars only**, and 0.16.0's answer
///    to a bar that does not fit is scroll arrows — i.e. it hides tabs.
///    A.3 is explicit that scroll arrows appearing in the DEFAULT layout
///    mean the default layout is wrong, so the width has to be chosen to
///    keep the widest default tab bar ("Properties" + "Batch Tools") whole.
///
/// Still a *default*: the panel is resizable, and this is only where it
/// starts.
const DOCK_DEFAULT_WIDTH_PTS: f32 = 380.0;

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

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("pdfce")
        .with_inner_size(INITIAL_WINDOW_SIZE)
        .with_min_inner_size([640.0, 480.0]);

    // Test harness: place the window explicitly and do NOT let it take focus.
    //
    // A GUI defect can only be settled by driving the real application (R86),
    // but doing that on the operator's own desktop takes their focus and covers
    // their work — on 2026-08-04 they were mid-task and had asked that the
    // screen be left alone, which would have made the one available oracle
    // unusable. Given a position off the visible desktop plus `with_active`
    // off, the process runs a genuine event loop that synthesized window
    // messages can drive and [`diag`] can report on, while nothing appears in
    // front of anyone.
    //
    // Deliberately NOT `with_visible(false)`: a hidden window is not merely an
    // invisible one — it stops being laid out, so the very interactions under
    // test would be skipped and the trace would show a fault that is only an
    // artefact of the harness.
    if let Some(spec) = std::env::var_os("PDFCE_DIAG_VIEWPORT") {
        // ui-text-exempt: environment variable name, never displayed
        let nums: Vec<f32> = spec
            .to_string_lossy()
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if let [x, y, w, h] = nums[..] {
            viewport = viewport
                .with_position([x, y])
                .with_inner_size([w, h])
                .with_active(false);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // Unconditional first line when tracing: without it an empty trace has two
    // very different meanings — "the process never saw PDFCE_DIAG" and "the
    // process saw nothing worth reporting" — and a harness cannot tell them
    // apart. Cost the investigation a round trip on 2026-08-04.
    diag::trace(|| {
        format!(
            "start argv1={initial:?} viewport={:?}",
            native_options.viewport
        )
    });

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
    /// An opt-in scripted input run ([`diag::Script`]), `None` in every normal
    /// launch. Present only when `PDFCE_DIAG_SCRIPT` was set, which is how a
    /// GUI defect gets investigated on a machine whose screen belongs to
    /// someone else.
    diag_script: Option<diag::Script>,
    /// Whether the thumbnail rail is showing.
    ///
    /// Session state only — deliberately not persisted to disk. Real UI
    /// preference persistence is a considered feature with its own
    /// storage format and migration story; growing it one ad-hoc field
    /// at a time is how a settings file becomes unmaintainable.
    rail_expanded: bool,
    /// Whether the status bar's diagnostics detail is expanded.
    diagnostics_expanded: bool,
    /// Whether the status bar's **selection** explanation is expanded.
    ///
    /// Its own flag rather than sharing [`Self::diagnostics_expanded`]: the
    /// two answer unrelated questions ("did this page render faithfully?" vs
    /// "what did I just click?"), and an operator who expanded one would be
    /// surprised to find the other opened too — a shared flag is the same
    /// class of second-source-of-truth defect the retired `properties_open`
    /// boolean was.
    ///
    /// Defaults CLOSED, and that is a deliberate layout decision rather than
    /// a taste one: the status bar is a bottom panel, so every line it grows
    /// takes height from the canvas — and under "Fit page" a shorter canvas
    /// re-fits the page at a smaller zoom, so the page visibly shrinks and
    /// jumps the instant something is selected. A one-line headline plus an
    /// expander keeps that to the single line the readout genuinely needs,
    /// which is also exactly what ui-spec §C.5 asked for ("the status bar
    /// gets ONE short summary line, not the full detail").
    selection_notes_expanded: bool,
    /// The right-hand dock's layout — which panels exist, how they are
    /// split, and which tab of each group is in front (decision 017 +
    /// Amendment A; standing rule R80).
    ///
    /// This replaced a `properties_open: bool`. The old flag was a *second*
    /// source of truth about whether the operator was looking at Properties,
    /// and it could disagree with the screen; the tree is the only source
    /// now, queried through [`dock::panel_is_active`].
    ///
    /// **Session state only, and disclosed as such in the dock header**
    /// (§7 / A.6 / R82) — the same stance as [`Self::rail_expanded`] and
    /// [`Self::font_folders`], for a sharper reason here: persisting it
    /// would mean either eframe's platform app-data Storage (which breaks
    /// decision 003's single-folder-portable posture) or R15's user-state
    /// partition, which does not exist yet.
    dock: DockTree,
    /// The outcome of the most recent save, kept until the next one so
    /// the operator gets a persistent answer rather than a toast they
    /// might miss.
    save_result: Option<SaveOutcome>,
    /// Whether the right-hand panel dock is showing.
    ///
    /// Visibility only — [`Self::dock`] owns the *shape*. Keeping the two
    /// apart is what lets the operator close the dock and reopen it to the
    /// arrangement they left, and it is why "is Properties on screen?" is
    /// `tools_open && dock::panel_is_active(&dock, DockPanel::Properties)`
    /// rather than a flag that has to be kept in step by hand.
    ///
    /// Session state only, for the same reason `rail_expanded` is: growing
    /// a settings file one ad-hoc field at a time is how it becomes
    /// unmaintainable.
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
    /// A redaction apply waiting on the operator's answer (Pass 8.1,
    /// ui-spec §4).
    ///
    /// `Some` means the Apply report is on screen and **nothing has been
    /// written to disk** — the same before-not-after posture as
    /// `pending_save`/`pending_copy`, with one sharper edge and one softer
    /// one:
    ///
    /// * sharper — unlike those two, the operation this confirms has no
    ///   cheap reversal once its bytes land. There is no undo for a file
    ///   whose content is gone.
    /// * softer — the removal has ALREADY happened, in memory, before this
    ///   is ever `Some` (see [`redact_apply::prepare_redaction_apply`]). That
    ///   is what lets the report state measurements instead of predictions,
    ///   and it is why cancelling costs nothing: the bytes are simply
    ///   dropped, and the open document was never touched.
    pending_redaction_apply: Option<PendingRedactionApply>,
    /// The literal-text query in the redaction panel's Find-&-mark box.
    ///
    /// Application state rather than per-document state, matching
    /// `markup_color`'s precedent: it is a control's contents, not a
    /// property of the file. Cleared on Open with the rest of the narrator
    /// state, because a query typed against the previous document is stale
    /// narration about the wrong file.
    redact_search_query: String,
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

/// A redaction apply the operator has been asked to confirm (Pass 8.1,
/// ui-spec §4).
///
/// Carries the FINISHED bytes rather than a plan to compute them on
/// confirm. Same reasoning as [`PendingSave`] and [`PendingCopy`] — the
/// question the operator answered was about *this* result — but load-bearing
/// here in a way it is not for those two: the report the operator reads
/// (§4.3) is generated from an apply that has already run, so every figure
/// in it is a measurement of the exact bytes that will be written, not a
/// forecast of bytes that will be produced later from a document that may
/// have changed in between.
///
/// The two acknowledgement flags are separate on purpose (§4.4/§4.5). One is
/// the ordinary "I understand what applying means"; the other appears ONLY
/// when the report names something pdfce could not remove, and exists so a
/// partial redaction can never be accepted by the same single click that
/// accepts a complete one.
struct PendingRedactionApply {
    /// The completed, verified, unwritten redaction.
    prepared: redact_apply::PreparedRedaction,
    /// Whether the operator has ticked the mandatory acknowledgement.
    acknowledged: bool,
    /// Whether the operator has ticked the EXTRA acknowledgement that the
    /// report's named residuals will not be removed. Meaningless — and
    /// never shown — when the report has no residual section.
    acknowledged_residuals: bool,
}

impl PendingRedactionApply {
    /// Every residual line the report must show, in the order it shows
    /// them: carriers core could not scrub, byte-level survivors the
    /// absence proof found outside page content, and objects promoted out
    /// of a compressed container by the materialisation.
    ///
    /// Built here rather than in the drawing code so that "does this apply
    /// have residuals?" — the question that gates the extra checkbox and
    /// picks the post-apply wording — is answered by ONE expression rather
    /// than by three conditions that could drift apart. A residual that the
    /// gate counts but the report does not print (or the reverse) would be
    /// precisely the "partial redaction mistaken for a complete one" failure
    /// §4.4 exists to prevent.
    fn residual_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for carrier in &self.prepared.report.carriers {
            if carrier.action == pdfce_core::redact::CarrierAction::DisclosedNotScrubbed {
                lines.push(ui_text::redact_apply_residual_carrier_line(carrier.carrier));
            }
        }
        for text in &self.prepared.verification.raw_byte_residuals {
            lines.push(ui_text::redact_apply_raw_residual_line(text));
        }
        if !self.prepared.promoted_by_materialisation.is_empty() {
            lines.push(ui_text::redact_apply_promotion_line(
                self.prepared.promoted_by_materialisation.len(),
            ));
        }
        lines
    }

    /// Whether the confirm button may be enabled: the ordinary
    /// acknowledgement always, PLUS the residual acknowledgement whenever
    /// there is a residual section to acknowledge.
    fn ready_to_confirm(&self) -> bool {
        self.acknowledged && (self.residual_lines().is_empty() || self.acknowledged_residuals)
    }
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
            diag_script: diag::Script::from_env(),
            diagnostics_expanded: false,
            selection_notes_expanded: false,
            // The dock's default arrangement (decision 017 A.3): the object
            // tree on top, Properties and Batch Tools sharing the group
            // below it, so the tree and the properties form are visible AT
            // THE SAME TIME. Built even while the dock is closed, because
            // `tools_open` decides visibility and this decides shape — two
            // separate questions, and conflating them is what made the old
            // `properties_open` flag able to lie.
            dock: dock::default_tree(),
            save_result: None,
            // The dock starts CLOSED: progressive disclosure, and the
            // status quo before decision 017 — an operator who wants it
            // opens it from the toolbar, or from the Properties control,
            // which now routes here.
            //
            // Note that the ORIGINAL justification for closed-by-default
            // ("everything in it acts on files the operator has not
            // opened") is no longer true — the dock now also holds a
            // page-scoped object tree and the open document's own metadata.
            // The default was nevertheless left alone: decision 017 gives
            // no guidance on it, an always-open 380 pt panel costs canvas
            // width on every document, and flipping a startup default is a
            // product call to make deliberately rather than as a side
            // effect of a layout-engine change.
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
            pending_redaction_apply: None,
            redact_search_query: String::new(),
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

    /// The menu-row glyph (ui-spec §3.3).
    ///
    /// Kept beside [`Self::label`] rather than in the toolbar so the row's
    /// two halves cannot drift apart — adding a fifth markup kind is a
    /// non-exhaustive-match error in BOTH, which is exactly the reminder
    /// a new kind needs.
    fn icon(self) -> icons::Icon {
        match self {
            Self::Square => icons::Icon::ShapeRect,
            Self::Circle => icons::Icon::ShapeEllipse,
            Self::Line => icons::Icon::ShapeArrow,
            Self::Highlight => icons::Icon::ShapeHighlight,
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

    /// The menu-row glyph (ui-spec §3.3; Stamp shares §3.4's rubber-stamp
    /// silhouette with the future Bates-numbering feature).
    fn icon(self) -> icons::Icon {
        match self {
            Self::FreeText => icons::Icon::TextFreeText,
            Self::Sticky => icons::Icon::TextSticky,
            Self::Stamp => icons::Icon::Stamp,
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
    // -- Pass 19.3: the spacing/style rows (decision 019 §6 slice 19.3) --
    /// `Tc`, the operator's typed number, in whichever unit
    /// [`Self::prop_tc_unit`] names.
    prop_char_spacing: f64,
    /// Which [`MetricSpec`](pdfce_core::text_edit::MetricSpec) variant
    /// [`Self::prop_char_spacing`] is expressed in. Defaults to
    /// [`MetricUnit::Relative`] per decision 019 §3.2's GUI-default call
    /// (tracking is a typographic ‰-of-em quantity).
    prop_tc_unit: MetricUnit,
    /// `Tw`, the operator's typed number, in whichever unit
    /// [`Self::prop_tw_unit`] names (Pass 19.4).
    ///
    /// Meaningful only on a **simple**-font run: the row renders as a
    /// read-only disclosure on a composite one, where §9.3.3 makes `Tw`
    /// void. That gate is R83 (no affordance without the capability) and it
    /// reads the published `GlyphProvenance::composite` flag rather than
    /// re-deriving anything — see [`AmbientSnapshot::composite`].
    prop_word_spacing: f64,
    /// Which unit [`Self::prop_word_spacing`] is expressed in. Defaults to
    /// [`MetricUnit::Relative`], matching the character-spacing row: word
    /// spacing is the same kind of typographic quantity (unscaled
    /// text-space units, R89), and a ‰-of-em space keeps its proportion
    /// through a later resize.
    prop_tw_unit: MetricUnit,
    /// `Tz` as a percentage (100 = normal). No unit choice — `Tz` is a
    /// dimensionless percentage (§9.3.4), so there is nothing to be relative
    /// to.
    prop_h_scale: f64,
    /// The baseline control's live selection. This is the whole of the
    /// mutual exclusion between the script toggle and the free-form rise:
    /// they are ONE control with one live member, not two controls that
    /// could both be armed.
    prop_baseline: BaselineChoice,
    /// The free-form rise, meaningful only when [`Self::prop_baseline`] is
    /// [`BaselineChoice::Custom`].
    prop_rise: f64,
    /// Which unit [`Self::prop_rise`] is expressed in. Defaults to
    /// [`MetricUnit::Absolute`] per decision 019 §3.2 — "what the operator
    /// typed is what they get" for a baseline nudge.
    prop_rise_unit: MetricUnit,
    /// Synthetic-bold checkbox. Independent of [`Self::prop_baseline`]:
    /// weight/slant and script position are unrelated axes.
    prop_bold: bool,
    /// Synthetic-italic checkbox.
    prop_italic: bool,
    /// The run [`Self::prop_char_spacing`]/`prop_word_spacing`/
    /// `prop_h_scale`/`prop_baseline`/`prop_rise` were last seeded from, so
    /// a caret move onto a DIFFERENT run re-seeds them and a caret move
    /// within the same run does not stomp what the operator is mid-way
    /// through typing.
    props_seeded_for: Option<usize>,
    /// The caret run's ambient §9.3 text state, refreshed every frame from
    /// provenance — the source of every "Now:" caption.
    ///
    /// This exists because the pre-19.3 property bar seeded from a FIXED
    /// default and never re-seeded. That is survivable for Size/Colour/Font,
    /// where the operator can simply look at the glyphs, and it is not
    /// survivable for `Tc`/`Tz`/`Ts`: a tracking of 0.24 is invisible at
    /// reading zoom, so a panel that displayed `0` while the run carried
    /// `0.24` would be stating something false about the document.
    prop_ambient: Option<AmbientSnapshot>,
    /// The last `preview_style_resolution` answer, and the state it was
    /// computed for — so the core query runs on a real state change rather
    /// than on every frame (it walks the page's `/Font` resources).
    ///
    /// A `Result`, not an `Option`: a query that cannot answer must SAY so.
    /// Swallowing the error would make "pdfce could not work out what this
    /// would do" look identical to "there is nothing to say", and the
    /// operator would then click Apply with no warning at all — which is the
    /// exact silence rule 4 exists to forbid.
    style_preview: Option<Result<pdfce_core::text_edit::StyleResolution, String>>,
    /// Cache key for [`Self::style_preview`]: `(run, bold, italic)`.
    style_preview_key: Option<(usize, bool, bool)>,
    /// The most recent property-bar (format) refusal `Display` text, kept
    /// visible in the strip until the next successful format/edit or tool
    /// exit (§8.2). Edit refusals live on [`PendingEdit::last_refusal`]; this
    /// is for a format apply, which has no `PendingEdit`.
    last_refusal: Option<String>,
}

/// Which [`MetricSpec`](pdfce_core::text_edit::MetricSpec) variant a numeric
/// spacing/rise field is currently expressed in — the GUI-local mirror of
/// core's discriminated unit model (R89).
///
/// A plain enum rather than a live `MetricSpec` because the field carries a
/// number the operator is still editing; reconstructing a `MetricSpec` every
/// frame just to read its discriminant back out would be noise.
///
/// The two variants are **genuinely different behaviours**, not two spellings
/// of one number: under a later size change, a `Relative` quantity moves with
/// the text and an `Absolute` one stays put. The UI therefore labels them by
/// that behaviour ("scales with size" / "fixed"), not by their PDF names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricUnit {
    /// Unscaled text-space units, written exactly as typed.
    Absolute,
    /// Thousandths of an em, re-resolved against the run's size at emit time.
    Relative,
}

/// The baseline control's four mutually exclusive positions.
///
/// [`Self::Custom`] is a GUI-only state, not a
/// [`ScriptPosition`](pdfce_core::text_edit::ScriptPosition) variant: it
/// means "show the free numeric rise field instead of a fixed script
/// position". Because exactly one of the four is selected at any time, and
/// because the single Apply button builds `.script(…)` OR `.rise(…)` from
/// that selection, `FormatError::ConflictingRise` is unreachable from this
/// panel by construction rather than by a runtime check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaselineChoice {
    /// `Ts` 0, no size reduction — how an inherited rise is flattened.
    Normal,
    /// pdfce's documented superscript metrics.
    Superscript,
    /// pdfce's documented subscript metrics.
    Subscript,
    /// A free-form numeric rise (Pass 19.2's deliberate exceed over Acrobat).
    Custom,
}

/// The caret run's ambient §9.3 text state, flattened to the four numbers the
/// property bar shows plus the facts it gates on.
///
/// Read from [`GlyphProvenance`](pdfce_core::text_extract::GlyphProvenance)
/// — SOURCED from the file, never guessed.
///
/// [`Self::composite`] is the one field that changes the panel's *shape*
/// rather than its numbers: since Pass 19.4 the word-spacing row is a live
/// control on a simple-font run and a read-only disclosure on a composite
/// one, because §9.3.3 makes `Tw` void for multi-byte codes. The flag is
/// consumed, never derived — it arrives already computed on provenance, from
/// the same `ExtractFont::is_simple` answer `pdfce-core`'s own R91 refusal
/// uses, so the affordance the shell draws and the request the core accepts
/// cannot drift apart (R74).
#[derive(Debug, Clone, Copy, PartialEq)]
struct AmbientSnapshot {
    /// `Tc` operand, unscaled text-space units.
    char_spacing: f64,
    /// `Tz` operand, a percentage (100 = normal).
    h_scale: f64,
    /// `Ts` operand, unscaled text-space units.
    rise: f64,
    /// `Tw` operand, unscaled text-space units.
    word_spacing: f64,
    /// The governing `Tf` size operand — the base every `Relative` quantity
    /// resolves against (R89, as amended by decision 019 Amendment B.3).
    font_size: f64,
    /// Whether the run's font segments show strings into multi-byte codes.
    /// `Tw` is spec-void for those (§9.3.3), so this is what decides
    /// whether the word-spacing row draws a control at all (R83).
    composite: bool,
    /// Whether `Tc` is provably still at its Table 105 default on this run.
    tc_at_default: bool,
    /// Whether `Tw` is provably still at its Table 105 default.
    tw_at_default: bool,
    /// Whether `Tz` is provably still at its Table 105 default.
    tz_at_default: bool,
    /// Whether `Ts` is provably still at its Table 105 default.
    rise_at_default: bool,
}

impl AmbientSnapshot {
    /// Flatten one glyph's provenance into the snapshot.
    ///
    /// `tf_size` is the `Tf` operand rather than the glyph's effective size:
    /// R89 resolves ratios against the size the surgery re-emits, and the
    /// effective size folds in the matrices' scale, which the emitted operand
    /// does not.
    fn from_provenance(p: &pdfce_core::text_extract::GlyphProvenance) -> Self {
        use pdfce_core::text_state::AmbientOrigin;
        let ts = &p.text_state;
        let is_default = |o: &AmbientOrigin| matches!(o, AmbientOrigin::Initial);
        Self {
            char_spacing: ts.char_spacing.value,
            h_scale: ts.h_scale.value,
            rise: ts.rise.value,
            word_spacing: ts.word_spacing.value,
            font_size: f64::from(p.tf_size),
            composite: p.composite,
            tc_at_default: is_default(&ts.char_spacing.origin),
            tw_at_default: is_default(&ts.word_spacing.origin),
            tz_at_default: is_default(&ts.h_scale.origin),
            rise_at_default: is_default(&ts.rise.origin),
        }
    }

    /// An unscaled text-space quantity expressed in thousandths of the em.
    ///
    /// Guards a zero/absent font size rather than producing an infinity: a
    /// malformed run with no `Tf` must not make the caption unreadable.
    fn per_mille(self, absolute: f64) -> f64 {
        if self.font_size.abs() < f64::EPSILON {
            0.0
        } else {
            absolute * 1000.0 / self.font_size
        }
    }

    /// The inverse of [`Self::per_mille`] — a ‰ quantity as its operand.
    ///
    /// Deliberately not named `from_per_mille`: a `from_*` method that takes
    /// `self` reads as a constructor and is not one.
    fn per_mille_to_operand(self, per_mille: f64) -> f64 {
        per_mille * self.font_size / 1000.0
    }
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

/// The inputs to a zoom-to-cursor solve, captured on the frame the Ctrl+wheel
/// was seen and consumed on the next one.
///
/// All four are measured in the SAME frame, which is what makes them a
/// consistent snapshot: mixing this frame's pointer with last frame's offset
/// would anchor to a page position that was never on screen.
#[derive(Clone, Copy, Debug)]
struct ZoomAnchor {
    /// The pointer as a fraction of the page's drawn size, `(0,0)` at the
    /// page's top-left corner and `(1,1)` at its bottom-right. Outside that
    /// range when the pointer is in the centring margin beside the page —
    /// which is legitimate and is not clamped, so zooming while pointing just
    /// off the page edge still tracks that spot.
    frac: (f32, f32),
    /// The scroll area's offset that frame.
    offset_before: (f32, f32),
    /// The page's drawn size in points-on-screen that frame.
    display_before: (f32, f32),
    /// The scroll area's inner viewport that frame. Assumed unchanged next
    /// frame; if the window is resized in the very same frame as a wheel step
    /// the anchor is off by the resize, which self-corrects on the next step.
    viewport: (f32, f32),
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
    /// Recorded on the frame a Ctrl+wheel arrives, consumed on the next one:
    /// where the pointer was over the page, so the scroll offset can be moved
    /// to keep that point still. See [`canvas::zoom_anchor_offset`].
    ///
    /// It has to span two frames because the new zoom is not known when the
    /// wheel is seen — the zoom is an [`Action`] applied after the UI is built
    /// and it clamps, so the only honest source of "how big is the page now"
    /// is the next frame's own `display_size`. Recording the *inputs* and
    /// solving later avoids predicting a clamp we do not control.
    zoom_anchor: Option<ZoomAnchor>,
    /// The scroll offset the canvas settled on at the end of the last frame.
    ///
    /// Kept because middle-drag panning has to compute "where the view should
    /// be now" BEFORE the scroll area is built, and the area's own state is
    /// only readable after. Storing last frame's settled value lets the pan be
    /// applied in the same frame as the movement rather than a frame late,
    /// which is the difference between panning that tracks the hand and
    /// panning that lags it.
    last_scroll_offset: egui::Vec2,
    /// Which object the operator has entered, and which subpath inside it is
    /// selected — the second selection level (`canvas::EnteredObject`).
    ///
    /// Lives on the document rather than in the tool because it is a property
    /// of what is selected, not of which tool is armed: descending into a view,
    /// switching tools, and still being inside it is correct behaviour.
    entered: Option<canvas::EnteredObject>,
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
    /// in a deterministic order (`BTreeSet`, like `selected_pages`). Populated
    /// by [`Self::target_provider`]'s hits (Pass 9a); empty when no object
    /// model is built (a degenerate page) or nothing is under the pointer
    /// (spec §4.2). Session/view state — a selection is never an edit, exactly
    /// like `selected_pages`.
    canvas_selection: BTreeSet<TargetId>,
    /// Where click-through cycling stands (ui-spec §C.3,
    /// [`canvas::ClickCycle`]).
    ///
    /// Set by every canvas selection click — including a plain one, which
    /// records ordinal 0 so the status readout can DISCLOSE that other
    /// objects sit under the pointer. Read by
    /// [`selection_readout`] and by the next click.
    ///
    /// Session/view state, and *derived-live*: it is never trusted on its
    /// own, only when [`canvas::ClickCycle::describes`]/`continues` say it
    /// still matches the page and the selection. That is what makes a stale
    /// cycle harmless rather than a trap — and it is why the only place this
    /// is explicitly cleared is `prune_canvas_selection`, where an EDIT may
    /// have kept the same `TargetId` while changing which object it names.
    click_cycle: Option<canvas::ClickCycle>,
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
    /// The current page's concrete object-model provider (Pass 9a's
    /// [`ObjectModelProvider`]), or `None` when the page could not be
    /// decomposed (a degenerate/undecodable page — selection then finds
    /// nothing, exactly as the old no-op `EmptyTargetProvider` did). Held as
    /// the CONCRETE type, not a `Box<dyn CanvasTargetProvider>`, so Pass 12.M1's
    /// snap engine (and the future Taubin fit) can read the SAME decomposition
    /// through [`ObjectModelProvider::page_objects`] without a second
    /// `decompose_page` per frame (ui-spec §3.3 / §10 ask #4). Selection
    /// reaches it as a `&dyn CanvasTargetProvider` via [`Self::target_provider`].
    object_model: Option<ObjectModelProvider>,
    /// The object-tree row the panel has already scrolled into view — the
    /// first target of the selection as it stood the last time the tree drew
    /// (ui-spec §B.5, "canvas selection → row").
    ///
    /// Exists to make the reveal fire **once per selection change** instead
    /// of every frame. Without it, a selection made on the canvas would drag
    /// the tree's scroll position back to the selected row on all 60 frames
    /// a second, so an operator scrolling the tree to look at something else
    /// could not move — the panel would fight them. Comparing against the
    /// current first-selected target makes the reveal an edge trigger.
    ///
    /// `None` means "nothing revealed yet", which is also the correct state
    /// after the selection is cleared: the next selection, even of the same
    /// object, is a fresh change worth revealing.
    objects_revealed: Option<TargetId>,
    /// The page index [`Self::object_model`] was last built for (Pass 9a).
    /// The provider decomposes only the CURRENT page (module docs of
    /// `object_provider`), so it is rebuilt lazily whenever this stops
    /// matching `view.page_index` or an edit invalidates it (set to `None`
    /// by [`Self::refresh_pages`]). `None` means "no object model built yet —
    /// rebuild on the next `canvas` frame."
    provider_page: Option<usize>,
    /// The in-progress rubber-band marquee's start point, in **canvas
    /// space** (Pass 9a). `Some` only between a canvas drag's start and its
    /// release while in object-selection mode; `None` otherwise. Session
    /// state — a marquee is never an edit.
    marquee_start: Option<egui::Pos2>,
    /// Pass 12.M2b measure-tool state (`docs/ui_specs/pass-12.M2-dimension-
    /// tools.md`). `Some` only while one of the three `CanvasTool::Measure*`
    /// tools is active; `None` otherwise and between pages. Mutually exclusive
    /// with [`Self::text_edit`]/[`Self::add_text`] (a single `active_tool`).
    /// Session/view state — a pick, fit, or scale entry is never an edit; only
    /// an accepted Accept is, and that goes through `session` (one undoable
    /// `EditSession::add_dimension`/`set_group_scale`), like every other tool.
    measure: Option<measure_tool::MeasureState>,
    /// Pass 9c-min vector-edit drag state (decision 011 §2.5). `Some` only
    /// between a `CanvasTool::VectorEdit` drag's start and its release; `None`
    /// otherwise. Session/view state — the drag is never itself an edit; only
    /// the release commits one undoable `EditSession::{move_object, move_node}`
    /// command.
    vector_drag: Option<vector_edit_tool::VectorDrag>,
    /// Whether the modeless "Dimension Groups" panel is open (ui-spec §5).
    /// Opened from the Measure ▾ menu; independent of the active tool so a
    /// scale can be set/units changed/layer toggled without drawing a line
    /// (ui-spec §7.2 accessibility). Session/view state.
    dimension_groups_open: bool,
    /// The "+ New Group" draft name in the group panel (ui-spec §5.2).
    group_new_name: String,
    /// The "+ New Group" draft unit in the group panel.
    group_new_unit: pdfce_core::dimension::Unit,
    /// The group currently expanded for scale editing in the panel, with its
    /// live scale-entry fields (ui-spec §5.2 — the SAME scale-entry widget the
    /// MeasureScale dialog uses). `None` when no row is being edited.
    group_scale_edit: Option<(
        pdfce_core::dimension::GroupId,
        measure_tool::ScaleEntryFields,
    )>,
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
            zoom_anchor: None,
            last_scroll_offset: egui::Vec2::ZERO,
            entered: None,
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
            click_cycle: None,
            text_edit: None,
            add_text: None,
            // Pass 9a/12.M1: the concrete object-model provider is built lazily
            // for the current page on the first `canvas` frame (and rebuilt on
            // page change / edit) — `None` here (and after every edit) forces
            // that first build; until then selection finds nothing.
            object_model: None,
            objects_revealed: None,
            provider_page: None,
            marquee_start: None,
            // Pass 12.M2b: no measure tool active, the group panel closed, a
            // fresh (metre) new-group draft. Built on measure-tool entry.
            measure: None,
            // Pass 9c-min: no vector-edit drag in flight.
            vector_drag: None,
            dimension_groups_open: false,
            group_new_name: String::new(),
            group_new_unit: pdfce_core::dimension::Unit::Meter,
            group_scale_edit: None,
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
        // SESSION READ (Pass 17.1 audit, decision 018 §8's "triage
        // individually" row). This is the most consequential of the audit's
        // reads, because the model it builds does not merely *display* — it
        // is the input to the NEXT mutation.
        //
        // `EditSession::edit_text` splices the SESSION-current content
        // stream (`current_page_content`), so edits accumulate. Rebuilding
        // this model from `session.document()` therefore re-extracted the
        // text as it was before ANY of this session's edits: after one
        // accepted change, the caret, the selection and every run offset
        // described a page that no longer existed, and the next
        // `EditRequest` built from them targeted text that had already been
        // replaced. Stale display would have been bad enough; a stale model
        // feeding a mutation is worse.
        //
        // `session.view()` — the full view rather than `graph()` — because
        // extraction walks CONTENT STREAM bytes, including any this session
        // authored into the R45 staging buffer, and only the view's
        // `StreamSource::Split` can resolve those spans.
        let view = self.session.view();
        self.text_edit = match text_extract::extract_pages_view(&view, &[page_index], &options) {
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
                // Pass 19.3: these are placeholders only. Every one of them
                // is re-seeded from the caret run's own ambient state before
                // the panel is drawn (`seed_spacing_props`), because a fixed
                // default shown beside a run that carries something else is
                // the panel stating a falsehood about the document.
                prop_char_spacing: 0.0,
                prop_tc_unit: MetricUnit::Relative,
                prop_word_spacing: 0.0,
                prop_tw_unit: MetricUnit::Relative,
                prop_h_scale: 100.0,
                prop_baseline: BaselineChoice::Normal,
                prop_rise: 0.0,
                prop_rise_unit: MetricUnit::Absolute,
                prop_bold: false,
                prop_italic: false,
                props_seeded_for: None,
                prop_ambient: None,
                style_preview: None,
                style_preview_key: None,
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
    /// pictures of the *old* content. Only the current page's texture and
    /// the thumbnails are discarded — the document is not reloaded, because
    /// `EditSession` **is** the open document: the next rasterization reads
    /// `self.session.view()`, which composes the base revision with the
    /// session's overlay and its R45 staging buffer live.
    ///
    /// ## Correction (decision 018 §1.1)
    ///
    /// This comment used to end *"the document is not reloaded, because the
    /// base revision (and therefore every byte span the renderer resolves)
    /// has not changed."* That was true through Pass 3.1 and **false from
    /// Pass 6.1 onward**, when annotation authoring began staging appearance
    /// streams whose spans point past the base file. The claim was the
    /// fossil of the defect: cache invalidation here was always correct —
    /// the GUI faithfully re-rasterized on every edit and faithfully
    /// reproduced the base, because the *read path* passed
    /// `session.document()`. No generation key was ever needed; the
    /// parameter type was the bug.
    ///
    /// ## `Page` is a snapshot — this is the only place it is refreshed
    ///
    /// [`Page`] is captured here (`contents`, `resources`, boxes,
    /// `/Rotate`) and then held until the next call. Every commit path must
    /// therefore funnel through this method, or the canvas ends up pairing
    /// a correct *view* with a stale *page* — an object whose content id
    /// changed under an edit would render from the old stream. Decision 018
    /// §10 hazard 2 names that risk explicitly; `apply_edit` and the canvas
    /// tools' commit helpers are the funnels that discharge it.
    fn refresh_pages(&mut self) {
        if let Ok(pages) = self.session.pages() {
            self.pages = pages;
        }
        self.page_texture = None;
        self.render_error = None;
        self.thumbnails = ThumbnailCache::default();
        // Pass 9a: an edit (rotate/delete/reorder, and later move/delete of
        // vector objects) can change the current page's object set, so the
        // provider is stale — force a rebuild on the next `canvas` frame and
        // drop any in-progress marquee. `prune_canvas_selection` then drops
        // selection targets the freshly-built provider can no longer resolve.
        self.provider_page = None;
        self.marquee_start = None;
        self.prune_canvas_selection();
    }

    /// Ensure [`Self::target_provider`] is the object-model provider for the
    /// CURRENT page (Pass 9a), rebuilding it when the page changed or an
    /// edit invalidated it. Cheap on the steady state (a single index
    /// compare); on a rebuild it decomposes exactly one page's content.
    ///
    /// If the page's content cannot be decoded, [`Self::object_model`] is left
    /// `None` so selection simply finds nothing rather than
    /// the app breaking — the same honesty posture the renderer takes on an
    /// undecodable page.
    fn ensure_object_provider(&mut self) {
        let page_index = self.view.page_index;
        if self.provider_page == Some(page_index) {
            return;
        }
        // Build the CONCRETE provider once and keep it (not a boxed dyn): the
        // snap engine reads its `page_objects()` and selection reaches it via
        // `target_provider()`, so there is exactly ONE decomposition per page
        // (ui-spec §3.3). `None` (an undecodable page) makes selection find
        // nothing, exactly as the old no-op `EmptyTargetProvider` did.
        // `session.view()`, NOT `session.document()` (decision 018). The
        // provider must decompose the SAME revision the canvas rasterizes,
        // or the operator gets a page showing an object they cannot click —
        // and, worse, can click an object that is no longer there. Building
        // both from one view makes that consistency structural rather than a
        // thing two call sites have to remember.
        let view = self.session.view();
        self.object_model = self
            .pages
            .get(page_index)
            .and_then(|page| ObjectModelProvider::build(&view, page, page_index));
        self.provider_page = Some(page_index);
    }

    /// The current page's hit-test provider as a `&dyn CanvasTargetProvider`
    /// (Pass 9a selection), or `None` when the page has no object model (a
    /// degenerate page — selection then finds nothing). Derived from the
    /// concrete [`Self::object_model`] so there is no separate boxed provider to
    /// keep in sync — the Pass 12.M1 §10 ask #4 wiring that lets the snap engine
    /// and selection share one decomposition.
    /// Apply one canvas click to the selection **depth**, and report whether
    /// the click was consumed at the subpath level.
    ///
    /// # Why both click paths call this rather than each doing it
    ///
    /// The plain (no-tool) selection path and the object-edit tool already
    /// duplicate the object-level click logic. Duplicated predicates drift
    /// (R92), and depth is exactly the kind of state where drift is invisible
    /// until an operator finds that double-click works with one tool armed and
    /// not another. One method, two callers.
    ///
    /// # The return value
    ///
    /// `true` when the click landed inside an entered object — meaning the
    /// caller should NOT also change the object-level selection, or descending
    /// into a view and clicking one of its lines would simultaneously re-select
    /// the whole view and undo the descent visually.
    fn apply_click_depth(&mut self, canvas_pos: egui::Pos2, tol: f64, double: bool) -> bool {
        let page_index = self.view.page_index;
        let object_hit = self
            .object_model
            .as_ref()
            .and_then(|p| p.hit_test(page_index, canvas_pos, tol))
            .map(|t| t.0 as usize);

        // Probe for a subpath in the object the RULES will consult: the one
        // under the pointer for a double-click (which may be a different
        // object), the already-entered one otherwise. Probing the wrong one
        // would make "click away to leave" impossible, because a hit on some
        // other object's subpath would read as a hit inside this one.
        let probe = if double {
            object_hit
        } else {
            self.entered.map(|e| e.object)
        };
        let subpath_hit = probe.and_then(|o| {
            self.object_model
                .as_ref()
                .and_then(|p| p.subpath_hits(o, canvas_pos, tol).first().copied())
        });

        let before = self.entered;
        self.entered = canvas::depth_after_click(before, double, object_hit, subpath_hit);
        self.entered.is_some()
    }

    fn target_provider(&self) -> Option<&dyn CanvasTargetProvider> {
        self.object_model
            .as_ref()
            .map(|p| p as &dyn CanvasTargetProvider)
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
        // The entered object cannot survive an edit or a page change, for the
        // same reason the click cycle cannot: after a content rewrite the SAME
        // paint-order index can name a different object, and the same subpath
        // ordinal a different line. Keeping it would leave an outline drawn
        // around whatever now happens to occupy that slot — a selection that
        // silently changed what it refers to, which is worse than no selection.
        // This runs from `refresh_pages`, which every edit, undo and redo
        // already funnels through.
        self.entered = None;
        // A click cycle cannot survive an edit or a page change. Its
        // liveness checks compare `TargetId`s, and after a content rewrite
        // the SAME index can name a different object — so "2 of 3 at this
        // point" would describe a stack that no longer exists, which is the
        // one thing a disclosure must never do. Dropped unconditionally
        // here: this runs from `refresh_pages`, which every edit, undo and
        // redo already funnels through.
        self.click_cycle = None;
        // Borrow the CONCRETE `object_model` field (disjoint from
        // `canvas_selection`) so the closure can hold the provider while the
        // selection is reassigned — a `self.target_provider()` call would
        // borrow all of `self` and conflict with the mutation.
        if let Some(provider) = self.object_model.as_ref() {
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
        // `session.view()`, NOT `session.document()` (decision 018 §1). This
        // one argument is why every editing feature from Pass 3.1 to Pass
        // 16.2 was invisible: `document()` is the BASE revision, so the
        // canvas faithfully re-rendered the file as it was opened after
        // every single edit. The view composes the overlay and the R45
        // staging buffer, so authored dimensions, markup appearances,
        // spliced content streams and vector edits all resolve.
        match raster::render_page_texture(
            ctx,
            &self.session.view(),
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
    /// Show or hide the modeless "Dimension Groups" panel (Pass 12.M2b
    /// ui-spec §5). Opened from the Measure ▾ menu; independent of the active
    /// tool. Pure view-state — opening a panel is never an edit.
    ToggleDimensionGroups,
    /// Reverse the most recent edit.
    Undo,
    /// Re-apply the most recently reversed edit.
    Redo,
    /// Bring the document-properties panel to the front of the dock, or —
    /// if it is already the panel on screen — close the dock.
    ///
    /// **The name is kept, and so is its toolbar control and shortcut; only
    /// the effect moved** (decision 017 §8.3 / A.4 #2). Before the dock this
    /// toggled a floating window. It now opens the dock and activates
    /// [`DockPanel::Properties`], which is the same operator intent
    /// ("show me the document's metadata") pointed at the surface that
    /// actually hosts it.
    ToggleProperties,
    /// Rebuild the dock's layout from scratch (decision 017 §8.12 / A.4 #6).
    ///
    /// Its own action rather than an inline mutation because it is a
    /// **command**, not a widget state flip: it must run in `apply()` with
    /// every other command, after the frame's drawing is finished and the
    /// real tree has been restored from the borrow-dance swap.
    ResetPanelLayout,
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
    /// Bring the redaction review panel to the front of the dock, or — if
    /// it is already the panel on screen — close the dock. The same
    /// toggle semantics as [`Action::ToggleProperties`], for the same
    /// reason: a control that opens a surface should also put it away.
    ToggleRedactPanel,
    /// Mark the whole of the current page for redaction (Pass 8.1,
    /// ui-spec §2.4). One `EditSession::add_redaction`; Undo reverses it.
    /// **Marks nothing about content** — it authors a reviewable
    /// annotation and removes nothing.
    MarkWholePageForRedaction,
    /// Run the redaction panel's literal-text search and author one mark
    /// per match (ui-spec §2.5). A reviewable batch, never a removal —
    /// rule 4 applied to a bulk-authored mark.
    SearchAndMarkForRedaction,
    /// Take one `/Redact` mark off the document before it is ever applied
    /// (ui-spec §3.2's ✕). Reversible like any edit; changes no content.
    RemoveRedactionMark(pdfce_core::object::ObjId),
    /// Run the whole apply in memory and open the report modal. **Writes
    /// nothing** — see [`PendingRedactionApply`].
    BeginRedactionApply,
    /// Write the prepared redaction to a path the operator picks. The one
    /// irreversible action in this application.
    ConfirmRedactionApply,
    /// Throw away a prepared redaction the operator declined. Costs
    /// nothing: the bytes are dropped and the open document was never
    /// touched.
    CancelRedactionApply,
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
        // Deliberately NOT reset: `pending_save` / `pending_copy` /
        // `pending_redaction_apply` — the `apply()` gate blocks
        // `Action::Open` while any of them is set, so a new document can
        // never load with one of those outstanding.
        self.save_result = None;
        // Pass 8.1: a query typed against the previous document is stale
        // narration about the wrong file, exactly like `copy_result` below.
        self.redact_search_query.clear();
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
        //
        // BASE READ ON PURPOSE (Pass 17.1 audit, decision 018 §8 —
        // "legitimately base reads — leave alone"). Do not "fix" this to
        // `session.view()`. Recovery is a property of the FILE AS PARSED:
        // "the bytes on disk had a damaged cross-reference table and were
        // rebuilt in memory." No edit can make that true, and no edit can
        // make it false — the operator cannot un-damage the file they
        // opened by typing in it. A session-aware answer here would be
        // either identical (`SessionGraph` has no recovery of its own) or,
        // worse, a claim that the damage went away. The sentence it
        // produces is also about saving ("incremental save is refused"),
        // which is governed by the base's provenance (R67), not by the
        // overlay.
        self.recovery_note = if let Status::Open(doc) = &self.status {
            doc.session.document().recovery().map(|r| {
                // The stream-length clause is appended only when it
                // applies. It is a DIFFERENT claim from the rebuild — the
                // rebuild recovers where objects are, this recovers how
                // long they are — and it carries a residual doubt the
                // rebuild does not: a re-derived extent is pdfce's reading
                // of the bytes rather than the file's own statement, so
                // the operator is told rather than left to assume the
                // content is verbatim (R20, fuzzy-never-sneaky).
                let lengths = if r.stream_lengths_recovered > 0 {
                    format!(
                        " {} stream(s) also disagreed with their own recorded length, so their \
                         extent was re-read from the file's endstream markers.",
                        r.stream_lengths_recovered
                    )
                } else {
                    String::new()
                };
                format!(
                    "This document had a damaged cross-reference table and was \
                     rebuilt in memory ({} object(s) recovered).{lengths} Saving will \
                     rewrite (normalize) the file; incremental save is refused.",
                    r.file_level_objects + r.objstm_objects
                )
            })
        } else {
            None
        };
        // P0-2, WIDENED to unconditional by decision 017 §8.4 / A.4 #3 —
        // the prerequisite bugfix that had to ship with or before the
        // Properties migration.
        //
        // A fresh `OpenDoc` starts with an EMPTY `properties_draft`, and the
        // draft used to be seeded only when the properties surface was open
        // at open time. That was survivable while Properties was a floating
        // window an operator explicitly opened (opening it ran
        // `seed_properties_draft` on the way in), and the guard here patched
        // the one case where the window outlived the document.
        //
        // A DOCK PANEL makes the blast radius worse in a way no guard can
        // cover: the panel is persistently mounted, so it can be drawn
        // against a document it was never "opened" for. The failure mode is
        // silent and looks like data loss — an empty metadata form for a
        // document that has a title and an author — and the operator's
        // natural next move is to type into the blank boxes and press Apply,
        // which would then WRITE that emptiness over real metadata.
        //
        // Seeding unconditionally costs one pass over four `/Info` fields
        // per open. There is no "half-typed field to protect" here: the
        // document just changed, so any draft still in memory describes a
        // file that is no longer on screen.
        if let Status::Open(doc) = &mut self.status {
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
    ///
    /// ## The pending-redaction-marks disclosure (Pass 8.1, ui-spec §3.4)
    ///
    /// A successful save of a document that still carries `/Redact` marks
    /// emits an EXTRA narrator line, in addition to — never instead of —
    /// the ordinary save confirmation. The save really did succeed; what
    /// the operator also needs to know is what it did *not* do.
    ///
    /// This is the direct structural answer to the most-cited real-world
    /// redaction failure: a marked-but-never-applied file saved and shared
    /// as if finished. The status bar already discloses pending marks
    /// continuously; this fires at the exact moment the misunderstanding
    /// becomes consequential, which is when the operator produces a file to
    /// hand someone.
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
        // Read the census from the SESSION graph, so a mark made this
        // session — the one most likely to be forgotten — is counted.
        let pending_marks = pdfce_core::redact::count_redaction_marks(&doc.session.graph());
        let saved = matches!(outcome, SaveOutcome::Saved { .. });
        self.save_result = Some(outcome);
        if saved && pending_marks > 0 {
            self.edit_note = Some(ui_text::redact_save_kept_pending_marks(pending_marks));
        }
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

    /// Delete the currently-selected canvas OBJECT (Pass 9c-min, decision 011
    /// §2.5) — content-stream surgery through
    /// [`EditSession::delete_object`](pdfce_core::edit::EditSession::delete_object),
    /// one undoable command. Rebuilds the object provider and clears the
    /// canvas selection so a stale target never lingers. A refusal (an
    /// enforced certification, an already-edited page needing reopen) is
    /// surfaced through the same channel a failed save uses.
    fn delete_selected_object(&mut self) {
        let (page_index, object_index) = {
            let Status::Open(doc) = &self.status else {
                return;
            };
            let Some(idx) = doc.canvas_selection.iter().next().map(|t| t.0 as usize) else {
                return;
            };
            (doc.view.page_index, idx)
        };
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        match doc.session.delete_object(page_index, object_index) {
            Ok(()) => {
                doc.canvas_selection.clear();
                doc.vector_drag = None;
                // `refresh_pages` FIRST (decision 018 §10 hazard 2 audit,
                // Pass 17.0). `delete_object` rewrites the page's content
                // stream, so this is a commit like any other and owes the
                // same invalidation: the cached raster is a picture of the
                // deleted object, and `ensure_object_provider` alone cannot
                // help because it early-returns while `provider_page` still
                // equals the current page. Before Pass 17.0 the missing
                // texture drop was invisible — the canvas re-rendered the
                // base either way — so this call site looked correct.
                doc.refresh_pages();
                doc.ensure_object_provider();
                self.edit_note = Some(ui_text::vector_object_deleted().to_owned());
            }
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
    }

    /// Delete the selected **subpath** — one part of an entered object (Pass
    /// 25.2).
    ///
    /// # Why the entered state is dropped on success
    ///
    /// `refresh_pages` → `prune_canvas_selection` already clears it, and that
    /// is deliberate rather than incidental: after the content stream is
    /// rewritten, part #668 of the object is a DIFFERENT line from the one
    /// that had that ordinal a moment ago. Staying "inside" with an ordinal
    /// that has silently re-pointed is worse than being put back at object
    /// level, because the outline would look authoritative while naming
    /// something the operator never chose. Decision 025 scopes the better
    /// answer (Pass 26.2 — survive the edit and say so); until that lands, the
    /// honest behaviour is to step back out.
    ///
    /// # Failures are shown, not swallowed
    ///
    /// The refusals this can hit are real and specific — a clipping path, a
    /// structure that cannot be safely indexed — and they are the reason the
    /// operation is safe. Reporting them through the same channel as a failed
    /// save means an operator who is refused finds out WHY.
    fn delete_selected_subpath(&mut self) {
        let (page_index, object_index, subpath_index) = {
            let Status::Open(doc) = &self.status else {
                return;
            };
            let Some(entered) = doc.entered else {
                return;
            };
            let Some(subpath) = entered.subpath else {
                return;
            };
            (doc.view.page_index, entered.object, subpath)
        };
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let outcome = doc
            .session
            .delete_subpath(page_index, object_index, subpath_index);
        let reported = outcome
            .as_ref()
            .map_err(std::string::ToString::to_string)
            .err();
        match outcome {
            Ok(()) => {
                doc.canvas_selection.clear();
                doc.vector_drag = None;
                // `refresh_pages` FIRST, for the same reason whole-object
                // delete needs it (decision 018 §10 hazard 2): the cached
                // raster is a picture of the part that just went, and
                // `ensure_object_provider` alone early-returns while
                // `provider_page` still equals the current page.
                doc.refresh_pages();
                doc.ensure_object_provider();
                self.edit_note = Some(ui_text::subpath_deleted(subpath_index));
            }
            Err(err) => self.save_result = Some(SaveOutcome::Failed(err.to_string())),
        }
        // Traced AFTER the outcome is applied, and carrying what the operator
        // will actually be told. A trace of the return value alone would have
        // said `Ok(())` for a delete whose disclosure never reached the status
        // bar — which is exactly the gap that has to be visible here (R93: the
        // trace must report the real outcome, not the intent).
        diag::trace(|| {
            format!(
                "commit-delete-subpath object={object_index} subpath={subpath_index} err={reported:?} note={:?}",
                self.edit_note
            )
        });
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

        // SESSION READ, and a HALF-FIX COMPLETED (Pass 17.1 audit).
        //
        // Extracting "page 2" must mean page 2 as the operator currently
        // sees it — unsaved deletes and reorders included — so this already
        // used `session.graph()` rather than the base document. But it then
        // paired that session graph with `base.bytes()` as the byte source,
        // and that pairing is exactly the hazard `DocumentView::new`'s own
        // doc comment warns about: a stream this session AUTHORED (a
        // dimension or markup appearance, a spliced content stream) carries
        // an R45 span starting at `base.len()`, which cannot be sliced out
        // of the base buffer at all. Such a stream would have been copied as
        // EMPTY into the extracted file — a silent content loss, visible
        // only by opening the output.
        //
        // `session.view()` is the same graph plus the correct
        // `StreamSource::Split` byte source, which resolves base spans and
        // staged spans alike. It is the constructor
        // `EditSession::view`/`DocumentView::new` both point callers at.
        //
        // NOTE this is a `pageops` copier, NOT the incremental writer, so it
        // does not run against decision 018 §10 hazard 1 (`DocumentView`
        // must never be the WRITER's input). `pageops::assemble` reads
        // through `view.slice(...)` and builds a brand-new document; the
        // minimal-diff writer's source of truth remains `&Document` +
        // `DirtySet::combined_source`, untouched here.
        let view = doc.session.view();
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
    /// ## Why extraction reads the SESSION, not the base document
    ///
    /// It used to read the base, and said so, with an explicit expiry
    /// date attached:
    ///
    /// > *"[`EditSession::document`] is the document as loaded, before this
    /// > session's unsaved edits. That is correct for every edit this Pass
    /// > of pdfce can make … The moment an editing Pass can alter page
    /// > content, this has to move to the overlay-aware `ObjectGraph`
    /// > path — recorded here rather than left as a silent assumption,
    /// > because the failure mode would be copying stale text with nothing
    /// > on screen to suggest it."*
    ///
    /// That moment arrived several Passes ago and the note was not acted
    /// on: Pass 14.1's `edit_text`, Pass 14.2's `format_text`, Pass 15.2's
    /// `reflow_block`, Pass 16.0's `add_text` and Pass 8's `redact-apply`
    /// all rewrite page content. Pass 17.1 (decision 018 §8) discharges it.
    /// Copy Text now extracts from `session.view()`, so Ctrl+C copies the
    /// words the operator can see — including a correction they just typed,
    /// and *excluding* one they just deleted.
    ///
    /// The predicted failure mode was exactly right, and is worth keeping
    /// on the record: the bug was invisible, because a stale copy looks
    /// like a successful copy. Nothing on screen would have suggested it.
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
        // SESSION READ (Pass 17.1 audit) — see this method's doc comment.
        // `view()` rather than `graph()`: extraction reads content-stream
        // bytes, and a stream this session authored lives in the R45
        // staging buffer that only the view's `StreamSource` can resolve.
        let view = doc.session.view();
        let page_number = doc.view.page_index + 1;

        let extracted = match scope {
            CopyScope::Page => {
                text_extract::extract_pages_view(&view, &[doc.view.page_index], &options)
            }
            CopyScope::Document => text_extract::extract_document_view(&view, &options),
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

    // -- Pass 8.1: redaction review & apply (ui-spec §3/§4) --------------

    /// Bring `panel` to the front of the dock, or close the dock if it is
    /// already the panel on screen. Returns `true` when the panel is now on
    /// screen (so a caller can seed whatever state that panel displays).
    ///
    /// "Already showing" means BOTH that the dock is open and that `panel`
    /// is the front tab of its group — the two halves of "is it on screen?".
    /// Asking the tree rather than a flag of our own is what stops a toolbar
    /// toggle from ever disagreeing with what the operator can see, which is
    /// exactly the failure the retired `properties_open` boolean was capable
    /// of (decision 017 §8.3).
    ///
    /// Extracted at Pass 8.1 because Redact is the second control with this
    /// behaviour and two copies of a "toggle means show-or-hide, and hide
    /// means only if it is the one you are looking at" rule is how the two
    /// come to differ.
    fn toggle_dock_panel(&mut self, panel: DockPanel) -> bool {
        if self.tools_open && dock::panel_is_active(&self.dock, panel) {
            self.tools_open = false;
            return false;
        }
        self.tools_open = true;
        // A `false` here means the panel is not mounted at all — possible
        // once panes can be closed, and cheap to survive: fall back to the
        // default layout rather than opening a dock that does not contain
        // what was asked for. Fail-soft, the same posture decision 017 §7
        // binds the future layout-restore path to.
        if !dock::activate(&mut self.dock, panel) {
            self.dock = dock::default_tree();
            dock::activate(&mut self.dock, panel);
        }
        true
    }

    /// The redaction review panel (ui-spec §3) — the dock pane that answers
    /// "what is marked in this document, and how do I finish or undo it?".
    ///
    /// ## Everything here is rebuilt from the document, every frame
    ///
    /// The mark list is [`pdfce_core::redact::redaction_marks`] over
    /// `session.graph()` — the same walk the status-bar census uses — and
    /// nothing about it is cached between frames. That is deliberate and it
    /// is the panel's most important property: a cached list could disagree
    /// with the document after an undo, a page delete, or a mark authored by
    /// some other path, and a review surface that lists a mark which is not
    /// there (or omits one that is) is worse than no review surface at all.
    /// It costs one dictionary walk per frame, which is the same order as
    /// the disclosure already in the status bar.
    ///
    /// ## Why `session.graph()` and not `session.document()`
    ///
    /// The base revision cannot contain a mark the operator made this
    /// session, which is precisely the set of marks a review panel exists to
    /// show. This is the Pass 17.1 / decision 018 §8 lesson applied at
    /// authoring time rather than re-learned.
    fn redact_panel(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.heading(ui_text::dock_panel_redact_label());
        ui.label(ui_text::redact_panel_intro());
        ui.separator();

        let Status::Open(doc) = &self.status else {
            ui.label(ui_text::redact_panel_no_document_hint());
            return;
        };
        let has_pages = !doc.pages.is_empty();
        let marks = pdfce_core::redact::redaction_marks(&doc.session.graph());

        // -- authoring entry points --
        //
        // R83 (no affordance without the capability): both are disabled,
        // not hidden, when the document has no pages — a control that
        // vanishes teaches nothing, while a disabled one with a tooltip
        // teaches what would enable it.
        ui.add_enabled_ui(has_pages, |ui| {
            if ui
                .button(ui_text::redact_mark_whole_page_button())
                .on_hover_text(ui_text::redact_mark_whole_page_tooltip())
                .clicked()
            {
                actions.push(Action::MarkWholePageForRedaction);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.redact_search_query);
                // The button is dead while the box is empty rather than
                // silently no-oping on click — the same R83 reading.
                ui.add_enabled_ui(!self.redact_search_query.trim().is_empty(), |ui| {
                    if ui.button(ui_text::redact_search_button()).clicked() {
                        actions.push(Action::SearchAndMarkForRedaction);
                    }
                });
            });
            ui.label(
                egui::RichText::new(ui_text::redact_search_hint())
                    .small()
                    .weak(),
            );
        });

        ui.separator();

        // -- state, then action, then detail --
        //
        // The count line is warn-coloured whenever it is non-zero, paired
        // with the ⚠ glyph the status bar already uses (R84: the state is
        // never carried by colour alone). "There are marks" is a warning,
        // not a statistic: it means content the operator may believe is
        // gone is still in the file.
        let count_label = ui_text::redact_marks_count_label(marks.len());
        if marks.is_empty() {
            ui.label(count_label);
        } else {
            ui.colored_label(ui.visuals().warn_fg_color, count_label);
        }

        // **The Apply button sits ABOVE the mark list, not below it — a
        // deliberate reversal of the ui-spec's §3.2 order, forced by
        // observing the real window (R86).**
        //
        // The spec put the list first, in its own `max_height(240.0)`
        // `ScrollArea`, and the Apply button after it. In the dock as it
        // actually ships, this pane's height comes from the vertical split
        // and is roughly 250 pt — so on a document with three marks, that
        // layout pushed "Review & Apply Redactions…" off the bottom of the
        // pane, leaving only a scrollbar to suggest anything followed.
        //
        // That is not a cosmetic loss. The failure this whole Pass exists to
        // prevent is an operator concluding that MARKING is redacting; a
        // panel that shows marks and hides the way to finish them is an
        // active push toward exactly that conclusion. State → action →
        // detail keeps the way out visible at every pane height.
        //
        // Nothing about safety is traded for it: the button opens the report
        // modal, which IS the review, and which cannot be confirmed without
        // reading past two gates. Clicking it before scrolling the list
        // costs an operator nothing but a cancel.
        ui.separator();
        let can_apply = !marks.is_empty();
        ui.add_enabled_ui(can_apply, |ui| {
            if ui
                .button(ui_text::redact_review_apply_button())
                .on_hover_text(ui_text::redact_review_apply_tooltip(can_apply))
                .clicked()
            {
                actions.push(Action::BeginRedactionApply);
            }
        });
        ui.separator();

        // The rows flow into the pane's OWN scroll area rather than a nested
        // one of their own — the second half of the same observation. Two
        // nested scroll contexts in a 250 pt pane means the operator's wheel
        // sometimes moves the list and sometimes moves the panel, depending
        // on where the pointer is, for no benefit at this size.
        for mark in &marks {
            ui.horizontal(|ui| {
                let size = mark.rect.map(|[llx, lly, urx, ury]| (urx - llx, ury - lly));
                // A plain button, not a `selectable_label`: a row is
                // a navigation command, and rendering it as a
                // selection control would imply a selected state the
                // panel does not have.
                if ui
                    .button(ui_text::redact_mark_row_label(mark.page_index + 1, size))
                    .on_hover_text(ui_text::redact_mark_row_tooltip())
                    .clicked()
                {
                    actions.push(Action::GoToPage(mark.page_index));
                }
                // A worded button, not the ui-spec's `✕` glyph.
                // `docs/ui_specs/menu-affordance-and-glyph-coverage.md`
                // records what a decorative Unicode glyph costs in this
                // app: U+25BE was in none of egui's default font chain
                // and shipped as a tofu box on every menu button. A bare
                // `✕` risks the same, and an icon-only control that
                // renders as a box in a list of destructive-looking rows
                // is worse here than anywhere else. The word also
                // announces itself.
                if ui
                    .button(ui_text::redact_mark_remove_button())
                    .on_hover_text(ui_text::redact_mark_remove_tooltip())
                    .clicked()
                {
                    actions.push(Action::RemoveRedactionMark(mark.annot_id));
                }
            });
        }
    }

    /// Mark the whole of the current page (ui-spec §2.4).
    ///
    /// No confirmation, deliberately — §2.1's governing rule. Marking is
    /// non-destructive and fully reversible right up to Apply, and a
    /// confirmation on a reversible action is how operators learn to dismiss
    /// confirmations, which would then also be how they dismiss the one in
    /// §4 that matters.
    ///
    /// It DOES get a narrator line, because marking an entire page takes one
    /// click and is easy to do without noticing. Disclosure without a gate
    /// is the correct weight for a reversible surprise.
    fn mark_whole_page_for_redaction(&mut self) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let Some(page) = doc.pages.get(doc.view.page_index) else {
            return;
        };
        let page_index = doc.view.page_index;
        // The page's own visible box — the region an operator means by "this
        // whole page", not the MediaBox, which on a cropped page covers area
        // the operator cannot see and did not ask about.
        let rect = page.crop_box;
        let spec = pdfce_core::annot_author::RedactSpec {
            quads: vec![pdfce_core::annot_author::Quad::from_rect(rect)],
            fill: None,
            overlay_text: None,
            quadding: pdfce_core::vartext::Quadding::Left,
        };
        match doc.session.add_redaction(page_index, &spec) {
            Ok(_) => {
                doc.refresh_pages();
                self.edit_note = Some(ui_text::redact_whole_page_marked(page_index + 1));
            }
            Err(err) => {
                self.save_result = Some(SaveOutcome::Failed(ui_text::redact_mark_failed(
                    &err.to_string(),
                )));
            }
        }
    }

    /// Author one mark per literal-text match (ui-spec §2.5).
    ///
    /// Rule 4 at full force: this produces a **reviewable batch of proposed
    /// marks**, never a removal. Every match lands in the same list as a
    /// hand-authored mark and is individually removable before Apply — which
    /// is what [`pdfce_core::edit::EditSession::delete_redaction_mark`] was
    /// added for.
    ///
    /// A zero-match result gets its own narrator line rather than silence.
    /// The distinction that line draws — "no matches in the text pdfce can
    /// extract" versus "nothing sensitive is there" — is the named
    /// scanned-document failure mode: an operator who reads a silent no-op
    /// as the second of those ships an un-redacted scan.
    fn search_and_mark_for_redaction(&mut self) {
        let query = self.redact_search_query.trim().to_owned();
        if query.is_empty() {
            return;
        }
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        // Case-insensitive: an operator searching for a name wants every
        // casing of it marked, and over-marking is the safe direction of
        // error for a feature whose failure mode is leaving content behind.
        match doc.session.mark_redactions_by_search(&query, true) {
            Ok(created) if created.is_empty() => {
                self.edit_note = Some(ui_text::redact_search_no_matches(&query));
            }
            Ok(created) => {
                doc.refresh_pages();
                self.edit_note = Some(ui_text::redact_search_marked(created.len(), &query));
            }
            Err(err) => {
                self.save_result = Some(SaveOutcome::Failed(ui_text::redact_mark_failed(
                    &err.to_string(),
                )));
            }
        }
    }

    /// Take one mark off before it is ever applied (ui-spec §3.2's ✕).
    ///
    /// No confirmation, for §2.1's reason and one more: removing a mark
    /// removes nothing from the document, so there is nothing to protect.
    /// The narrator line says so explicitly, because "remove" in a redaction
    /// panel could otherwise be read as "un-redact".
    fn remove_redaction_mark(&mut self, annot_id: pdfce_core::object::ObjId) {
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        // Resolve the page BEFORE the removal, so the narrator line can name
        // it — afterwards the mark is gone and there is nothing to ask.
        let page_number = pdfce_core::redact::redaction_marks(&doc.session.graph())
            .iter()
            .find(|m| m.annot_id == annot_id)
            .map_or(0, |m| m.page_index + 1);
        match doc.session.delete_redaction_mark(annot_id) {
            Ok(()) => {
                doc.refresh_pages();
                self.edit_note = Some(ui_text::redact_mark_removed(page_number));
            }
            Err(err) => {
                self.save_result = Some(SaveOutcome::Failed(ui_text::redact_mark_remove_failed(
                    &err.to_string(),
                )));
            }
        }
    }

    /// Run the apply in memory and open the report (ui-spec §4.2).
    ///
    /// **Nothing is written here.** The whole removal — both forced full
    /// rewrites and the absence proof — happens inside
    /// [`redact_apply::prepare_redaction_apply`], and its result is parked in
    /// `pending_redaction_apply` for the operator to read. That ordering is
    /// what makes the report honest: it describes bytes that exist, so its
    /// numbers are measurements rather than a forecast, and there is no
    /// window in which the document could change between the report and the
    /// write.
    ///
    /// A refusal never opens the modal. It goes to the same `save_result`
    /// channel every other refusal in this codebase uses, because a refusal
    /// is an answer to what the operator asked for, not a new question — and
    /// putting it in a dialog would make the operator dismiss something
    /// before they could read it twice.
    fn begin_redaction_apply(&mut self) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        match redact_apply::prepare_redaction_apply(&doc.session) {
            Ok(prepared) => {
                self.pending_redaction_apply = Some(PendingRedactionApply {
                    prepared,
                    acknowledged: false,
                    acknowledged_residuals: false,
                });
            }
            Err(refusal) => {
                self.save_result = Some(SaveOutcome::Failed(
                    ui_text::redact_apply_refusal_message(&refusal),
                ));
            }
        }
    }

    /// Write the prepared redaction to a path the operator picks (ui-spec
    /// §4.6) — **the one irreversible action in this application**.
    ///
    /// Four properties, each load-bearing:
    ///
    /// 1. **Save-as, never save-over.** The dialog is pre-filled with
    ///    `suggested_redaction_name` — `"{stem} (redacted).pdf"` — and never
    ///    with the open file's own name. Overwriting the source would
    ///    destroy the only remaining copy of the content the operator is
    ///    removing, on the operation least able to survive a mistake.
    /// 2. **It does not go through `save_dialog`.** Conflating the two save
    ///    paths is how one silently inherits the other's defaults, and
    ///    `save_dialog`'s default is *incremental* — the exact mode a
    ///    redaction must never use. Keeping them separate means there is no
    ///    parameter anywhere that could make an apply write incrementally.
    /// 3. **The open document is not modified.** The session still holds its
    ///    marks; the redacted document is a new file. So there is nothing to
    ///    undo, which is why the wording corrects the operator's learned
    ///    Undo expectation instead of relying on it.
    /// 4. **Atomic write** (standing UX rule 5), same `write_atomic` as
    ///    every other save: a crash mid-write cannot leave a truncated file
    ///    at the destination.
    ///
    /// The post-apply line is durable and picks its wording from whether the
    /// report had residuals, per §5.1 — an operator who acknowledged a
    /// residual and then closed the modal is still owed a standing record of
    /// what remains.
    fn confirm_redaction_apply(&mut self) {
        let Some(pending) = self.pending_redaction_apply.take() else {
            return;
        };
        let Status::Open(doc) = &self.status else {
            return;
        };
        let suggested = ui_text::suggested_redaction_name(&doc.path);
        let Some(path) = rfd::FileDialog::new()
            .add_filter(ui_text::open_dialog_filter_label(), &["pdf"])
            .set_file_name(suggested)
            .save_file()
        else {
            // Cancelled at the file dialog: nothing was written, and the
            // prepared bytes are dropped. The marks are all still on the
            // document, so the operator can start over losing nothing.
            return;
        };

        let residuals = pending.residual_lines().len();
        let regions = pending.prepared.report.marks_applied;
        let pages = pending.prepared.report.pages_redacted;
        match write_atomic(&path, &pending.prepared.bytes) {
            Ok(()) => {
                self.save_result = None;
                self.edit_note = Some(if residuals == 0 {
                    ui_text::redact_apply_succeeded_clean(&path, regions, pages)
                } else {
                    ui_text::redact_apply_succeeded_residual(&path, regions, residuals)
                });
            }
            Err(err) => {
                self.save_result = Some(SaveOutcome::Failed(err.to_string()));
            }
        }
    }

    /// The Apply report — pdfce's **third** confirmation-dialog convention,
    /// adopted deliberately rather than by drift (ui-spec §4.1).
    ///
    /// The existing convention is a fixed 520 pt, non-resizable, centred
    /// window with a short body (`signature_confirmation`,
    /// `copy_confirmation`). It does not fit here, and the mismatch is
    /// structural rather than aesthetic: this dialog's body is a **report**
    /// whose length varies with the document and the mark count, and
    /// squeezing a variable-length report into a fixed short box would bury
    /// the one thing the operator is supposed to read. So this window is
    /// **resizable**, larger by default, and scrolls its body.
    ///
    /// **The inconsistency is the point.** A destructive, irreversible action
    /// should not wear the same clothes as the two reversible questions the
    /// operator has already learned to click through; looking different is
    /// part of how it stops being reflexive.
    ///
    /// ## Two things this window deliberately does NOT have
    ///
    /// * **No default-button binding, so Enter cannot confirm.** egui does
    ///   not bind Enter to a focused `Button` (activation is Space or
    ///   Enter on the *focused* widget only, and nothing here takes focus on
    ///   open), and nothing in this method adds one. An operator reading a
    ///   long report and pressing Enter out of habit must not commit the
    ///   most destructive action in the application.
    /// * **No keyboard shortcut anywhere, to open OR to confirm.** A
    ///   deliberate asymmetry with every other destructive action in pdfce
    ///   (Delete has the Delete key, rotate has `[`/`]`) — those are
    ///   reversible before save; this is not, ever. The heaviest action in
    ///   the app gets zero frictionless paths, and says so on screen
    ///   (`redact_apply_no_shortcut_note`) rather than leaving its absence
    ///   to be noticed.
    fn redaction_apply_confirmation(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(pending) = &mut self.pending_redaction_apply else {
            return;
        };
        let report = &pending.prepared.report;
        let verification = &pending.prepared.verification;
        // Computed BEFORE the closure so the residual gate and the residual
        // section read the same list (see `residual_lines`' docs).
        let residuals = pending.residual_lines();
        // Read BEFORE the checkboxes are drawn, so the confirm button
        // reflects last frame's acknowledgement state. That one-frame lag is
        // a feature, not an oversight to "fix": it makes it impossible for
        // the tick that enables the button and the click that presses it to
        // land in the same frame, so a fast double-click on the checkbox
        // cannot spill onto a control that was disabled when the gesture
        // started. egui repaints on input, so the operator sees the button
        // enable immediately.
        let ready = pending.ready_to_confirm();

        egui::Window::new(ui_text::redact_apply_title())
            .collapsible(false)
            // The one deliberate deviation from the other two dialogs'
            // `.resizable(false)`: the body is a variable-length report.
            .resizable(true)
            .default_size([760.0, 560.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading(ui_text::redact_apply_report_heading());
                // Warn-coloured AND first in the body: the permanence
                // statement is not fine print, and it is what the operator
                // must have read before anything below it makes sense.
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    ui_text::redact_apply_permanence_statement(),
                );
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("redact-apply-report")
                    .max_height(320.0)
                    .show(ui, |ui| {
                        ui.strong(ui_text::redact_apply_will_remove_heading());
                        ui.label(ui_text::redact_apply_removal_summary(
                            report.marks_applied,
                            report.pages_redacted,
                            report.glyphs_removed,
                            report.content_streams_rewritten,
                        ));
                        // Each of these is shown only when it happened.
                        // A report padded with "0 annotations removed" lines
                        // is a report whose real lines get skimmed past.
                        if report.annotations_removed > 0 {
                            ui.label(ui_text::redact_apply_annotations_removed(
                                report.annotations_removed,
                            ));
                        }
                        if report.info_strings_scrubbed > 0 {
                            ui.label(ui_text::redact_apply_info_scrubbed(
                                report.info_strings_scrubbed,
                            ));
                        }
                        if report.containers_decomposed > 0 {
                            ui.label(ui_text::redact_apply_containers_decomposed(
                                report.containers_decomposed,
                                report.objects_promoted,
                            ));
                        }
                        ui.label(ui_text::redact_apply_single_revision_note());

                        // The verification result. The affirmative form is
                        // the ONLY place this UI is allowed to say
                        // "verified", and it is licensed by an absence proof
                        // that actually ran over these bytes.
                        if verification.is_clean() && verification.strings_checked > 0 {
                            ui.label(ui_text::redact_apply_verified_line(
                                verification.strings_checked,
                            ));
                        }
                        if verification.strings_too_short_for_raw_check > 0 {
                            ui.label(ui_text::redact_apply_verification_limit_line(
                                verification.strings_too_short_for_raw_check,
                            ));
                        }

                        // -- the refusal section (§4.4) --
                        if !residuals.is_empty() {
                            ui.separator();
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                ui_text::redact_apply_refused_heading(),
                            );
                            for line in &residuals {
                                ui.colored_label(ui.visuals().warn_fg_color, line);
                            }
                        }

                        ui.separator();
                        ui.label(ui_text::redact_apply_scope_reminder());
                    });

                ui.separator();
                // The extra acknowledgement exists ONLY when there is
                // something to acknowledge. Showing it always would make it
                // a box operators tick without reading, which is the same as
                // not having it.
                if !residuals.is_empty() {
                    ui.checkbox(
                        &mut pending.acknowledged_residuals,
                        ui_text::redact_apply_refusal_acknowledgement_checkbox(),
                    );
                }
                ui.checkbox(
                    &mut pending.acknowledged,
                    ui_text::redact_apply_confirm_checkbox(),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(ui_text::redact_apply_cancel_button()).clicked() {
                        actions.push(Action::CancelRedactionApply);
                    }
                    // R83: the confirm button is disabled until every
                    // required acknowledgement is ticked — the affordance
                    // appears exactly when the capability does.
                    ui.add_enabled_ui(ready, |ui| {
                        if ui.button(ui_text::redact_apply_confirm_button()).clicked() {
                            actions.push(Action::ConfirmRedactionApply);
                        }
                    });
                });
                ui.label(
                    egui::RichText::new(ui_text::redact_apply_no_shortcut_note())
                        .small()
                        .weak(),
                );
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

        // Icon + text rows (ui-spec §2 #24–27). Icons here are pure
        // recognition aids: the dock is read top-to-bottom, so the text
        // stays the label and a repeated glyph (Open and Font folders
        // share the folder art, §3.5) degrades gracefully — unlike two
        // icon-only toolbar buttons, which would have no such fallback.
        for (tool, label, icon) in [
            (
                Tool::Merge,
                ui_text::tool_merge_label(),
                icons::Icon::Combine,
            ),
            (Tool::Split, ui_text::tool_split_label(), icons::Icon::Split),
            (
                Tool::Insert,
                ui_text::tool_insert_pages_label(),
                icons::Icon::InsertPages,
            ),
            (
                Tool::FontFolders,
                ui_text::tool_font_folders_label(),
                icons::Icon::FontFolders,
            ),
        ] {
            let open = self.tools_selected == Some(tool);
            let row = (
                icons::toggle_image(ui, icon, open),
                Self::toggle_label(open, label),
            );
            if ui.add(egui::Button::selectable(open, row)).clicked() {
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
                // Routed through `icon_button` — the SAME accessible-name
                // wrapper the page rail's identical up/down pair uses. These
                // two call sites were the ones that never migrated: a bare
                // `Button::new` on a triangle glyph announces the glyph (or
                // nothing) rather than "Move file up", which is precisely the
                // gap the removed `glyph_button`'s doc comment cited this pair as
                // its reason for existing.
                if ui
                    .add_enabled_ui(index > 0, |ui| {
                        Self::icon_button(
                            ui,
                            icons::Icon::ChevronUp,
                            ui_text::merge_move_up_tooltip(),
                        )
                    })
                    .inner
                    .clicked()
                {
                    swap = Some((index, index - 1));
                }
                if ui
                    .add_enabled_ui(index + 1 < self.merge_inputs.len(), |ui| {
                        Self::icon_button(
                            ui,
                            icons::Icon::ChevronDown,
                            ui_text::merge_move_down_tooltip(),
                        )
                    })
                    .inner
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
        // Pass 8.1 — the THIRD independent pending state, added to the same
        // gate rather than getting one of its own. The collision this
        // closes is the one already documented above (two centre-anchored
        // windows rendering on top of each other, only the later one
        // clickable), and it is strictly worse here: the window that would
        // be hidden is the confirmation for the only irreversible operation
        // in the application. An operator must never be able to answer a
        // destructive question they cannot see.
        if self.pending_redaction_apply.is_some()
            && !matches!(
                action,
                Action::ConfirmRedactionApply | Action::CancelRedactionApply
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
                // Decision 017 §8.3: same entry point, new destination.
                //
                // "Already showing" means BOTH that the dock is open and
                // that Properties is the front tab of its group — the two
                // halves of "is it on screen?". Asking the tree rather than
                // a flag of our own is what stops the toolbar toggle from
                // ever disagreeing with what the operator can see, which is
                // exactly the failure the retired `properties_open` boolean
                // was capable of.
                if self.toggle_dock_panel(DockPanel::Properties)
                    && let Status::Open(doc) = &mut self.status
                {
                    doc.seed_properties_draft();
                }
                return;
            }
            Action::ResetPanelLayout => {
                // Wholesale replacement, not a repair: the operator reached
                // for this because the arrangement is wrong in some way they
                // could not undo by dragging, and a partial fix that left
                // some of the damage would send them straight back to the
                // button. `default_tree` is also the single definition of
                // "where the panels start", so reset and startup cannot
                // drift apart.
                self.dock = dock::default_tree();
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
                // Pass 9c-min: when the vector-edit tool is active and a canvas
                // OBJECT is selected, Delete removes that object (surgery), not
                // the page-rail page selection. The two selections are distinct
                // (a page-rail selection vs. a canvas object selection), so the
                // tool disambiguates which Delete means.
                // Pass 25.2: a selected SUBPATH outranks both. It is the most
                // specific thing selected, and reaching it took two deliberate
                // acts (a double-click to enter, a click to pick), so Delete
                // can only mean it.
                //
                // Deliberately NOT gated on the object-edit tool, unlike
                // whole-object delete. That gate exists because Delete
                // otherwise means "delete the selected pages", and the tool is
                // what disambiguates. With a subpath entered there is no
                // ambiguity to resolve, so requiring a tool as well would be a
                // rule with no reason behind it — the kind an operator
                // experiences as the application being arbitrary.
                let delete_subpath = matches!(
                    &self.status,
                    Status::Open(doc) if doc.entered.is_some_and(|e| e.subpath.is_some())
                );
                let delete_object = matches!(
                    &self.status,
                    Status::Open(doc)
                        if doc.active_tool == Some(CanvasTool::VectorEdit)
                            && !doc.canvas_selection.is_empty()
                );
                if delete_subpath {
                    self.delete_selected_subpath();
                } else if delete_object {
                    self.delete_selected_object();
                } else {
                    self.delete_selection();
                }
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
                    // Pass 9c-min: a tool switch always abandons any in-flight
                    // vector-edit drag (it is transient view state, never an
                    // edit) — the VectorEdit tool rebuilds it on the next drag.
                    doc.vector_drag = None;
                    // Pass 14.3/16.2: entering a tool builds ITS per-page state
                    // and tears down the OTHER tool's — the §0.1 mutual
                    // exclusion a single `active_tool` already guarantees. The
                    // dispatch is the pure `tool_builds_*` predicates themselves
                    // (headless-tested to be never both true), so a stale
                    // caret/draft never survives a tool switch.
                    if canvas::tool_builds_text_edit(tool) {
                        doc.build_text_edit_state();
                        doc.add_text = None;
                        doc.measure = None;
                    } else if canvas::tool_builds_add_text(tool) {
                        doc.build_add_text_state(default_add_text_font);
                        doc.text_edit = None;
                        doc.measure = None;
                    } else if canvas::tool_builds_measure(tool) {
                        // Pass 12.M2b §1.3: entering any of the three measure
                        // tools builds the shared per-page pick state; the other
                        // tools' state is torn down (the single-`active_tool`
                        // mutual exclusion). A tool SWITCH among the three
                        // measure tools keeps the state (same page) so the
                        // active group / snap toggle persist.
                        if doc
                            .measure
                            .as_ref()
                            .is_none_or(|m| m.page_index != doc.view.page_index)
                        {
                            doc.measure =
                                Some(measure_tool::MeasureState::new(doc.view.page_index));
                        }
                        doc.text_edit = None;
                        doc.add_text = None;
                    } else {
                        doc.text_edit = None;
                        doc.add_text = None;
                        doc.measure = None;
                    }
                }
                return;
            }
            Action::ToggleDimensionGroups => {
                // Pass 12.M2b ui-spec §5: flip the modeless group-panel window.
                if let Status::Open(doc) = &mut self.status {
                    doc.dimension_groups_open = !doc.dimension_groups_open;
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
                    // Pass 12.M2b §1.3: Esc stage 1 discards the in-progress
                    // measure gesture (the first pick, the circular pick-set,
                    // the scale line/dialog, or a completed-but-unauthored
                    // linear pending) WITHOUT exiting the tool. Nothing was ever
                    // written (rule 7).
                    if let Some(state) = doc.measure.as_mut() {
                        state.clear_gesture();
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
            // -- Pass 8.1 redaction (ui-spec §3/§4) ---------------------
            //
            // All seven are handled here, above the open-document guard,
            // because every one of them needs `&mut self` rather than
            // `&mut OpenDoc`: they write the app-level narrator channels
            // (`edit_note`, `save_result`) and the app-level pending state.
            Action::ToggleRedactPanel => {
                self.toggle_dock_panel(DockPanel::Redact);
                return;
            }
            Action::MarkWholePageForRedaction => {
                self.mark_whole_page_for_redaction();
                return;
            }
            Action::SearchAndMarkForRedaction => {
                self.search_and_mark_for_redaction();
                return;
            }
            Action::RemoveRedactionMark(id) => {
                self.remove_redaction_mark(id);
                return;
            }
            Action::BeginRedactionApply => {
                self.begin_redaction_apply();
                return;
            }
            Action::ConfirmRedactionApply => {
                self.confirm_redaction_apply();
                return;
            }
            Action::CancelRedactionApply => {
                // Costs nothing: the prepared bytes are dropped and the open
                // document was never touched, so the marks are all still
                // there to review again.
                self.pending_redaction_apply = None;
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
            | Action::ResetPanelLayout
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
            | Action::ToggleDimensionGroups
            | Action::MoveSelection(_)
            | Action::DropDragged(_)
            | Action::ExtractSelection
            | Action::ConfirmPendingSave
            | Action::CancelPendingSave
            | Action::CopyText(_)
            | Action::ConfirmPendingCopy
            | Action::CancelPendingCopy
            | Action::ToggleRedactPanel
            | Action::MarkWholePageForRedaction
            | Action::SearchAndMarkForRedaction
            | Action::RemoveRedactionMark(_)
            | Action::BeginRedactionApply
            | Action::ConfirmRedactionApply
            | Action::CancelRedactionApply
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
            Action::ClearCanvasSelection => {
                doc.canvas_selection.clear();
                // Escape also LEAVES an entered object. It is the universal
                // "back out one level" key, and an operator who has descended
                // into a drawing view and pressed Escape means to be out of it
                // — not to be left inside with nothing selected, which looks
                // identical to being outside and behaves differently.
                doc.entered = None;
            }
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
        if Self::action_preserves_gesture(incoming) {
            return;
        }
        match self.current_gesture_interrupt() {
            GestureInterrupt::Nothing => {}
            GestureInterrupt::Discard => self.discard_active_gesture(),
            GestureInterrupt::Commit => self.commit_active_gesture(),
        }
    }

    /// Whether `action` leaves an in-progress tool gesture (a half-picked
    /// dimension, a half-drawn shape) UNTOUCHED.
    ///
    /// # Why this list exists
    ///
    /// The original rule was "only tool selection and explicit cancel are
    /// safe; every other action discards the gesture." That is right in
    /// spirit — an unrelated action should not leave a stale half-gesture
    /// armed — but it swept in the **view controls**, and that made the
    /// measure tools painful to use in exactly the situation they matter
    /// most.
    ///
    /// Concretely: the operator picks point A of a linear dimension, then
    /// ctrl+scrolls to zoom in so they can place point B precisely on a
    /// drawing feature. Zooming pushes [`Action::ZoomBy`], which is not
    /// tool selection or cancel, so point A was silently discarded — with
    /// no message. Zooming in to place an accurate pick is the single most
    /// natural thing to do while measuring, and it was punished.
    ///
    /// # The rule
    ///
    /// **Changing how the page is VIEWED is not an edit and must not
    /// disturb what is being AUTHORED.** Zoom and scroll change the camera,
    /// not the document, and every gesture pdfce holds is stored in
    /// page/PDF space — so a zoom cannot invalidate one. Actions that change
    /// the *subject* (page navigation, opening another document) or that
    /// touch the document (undo, save, an edit command) still discard, since
    /// a gesture anchored to page N is meaningless on page M.
    ///
    /// Page navigation is deliberately NOT on this list: `MeasureState` is
    /// built per page, and a pick from another page would author geometry
    /// against the wrong content.
    fn action_preserves_gesture(action: Action) -> bool {
        matches!(
            action,
            // The gesture's own controls.
            Action::SelectCanvasTool(_)
                | Action::CancelToolGesture
                // Pure camera changes — same page, same document.
                | Action::ZoomIn
                | Action::ZoomOut
                | Action::ZoomBy(_)
                | Action::ZoomActualSize
                | Action::Fit(_)
        )
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
                    || doc.add_text.as_ref().is_some_and(|s| s.draft.is_some())
                    // Pass 12.M2b §8: an in-progress measure gesture (a pick, a
                    // circular fit-set, a scale line/dialog, a linear pending)
                    // is the SAME class of discardable, never-yet-written
                    // gesture — one more disjunct on this ONE query, not a
                    // second enforcement point.
                    || doc.measure.as_ref().is_some_and(measure_tool::MeasureState::gesture_in_progress) =>
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
            // Pass 12.M2b §8: discard the measure gesture (a pick, fit-set,
            // scale line/dialog, or linear pending — none ever written).
            if let Some(state) = doc.measure.as_mut() {
                state.clear_gesture();
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
    /// Feed one scripted input step into the frame about to be built.
    ///
    /// This is the correct seam for it: `raw_input_hook` runs *before* egui
    /// digests the frame's input, so an injected event is indistinguishable
    /// from one the window delivered — the pointer state, the click detection,
    /// and every `Response` are all computed from it normally. Pushing events
    /// from inside [`Self::ui`] would not work at all: by then the pointer
    /// state for the frame has already been resolved, so widgets would see
    /// nothing and the harness would "prove" a defect that does not exist.
    ///
    /// One step per frame, deliberately. egui distinguishes a click from a
    /// drag by what happens across frames, so collapsing press and release
    /// into one frame would test a gesture the application can never receive.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let Some(script) = self.diag_script.as_mut() else {
            return;
        };
        // An off-screen window gets no natural repaint traffic, so the script
        // would stall between steps. Ask for the next frame unconditionally
        // while it runs.
        ctx.request_repaint();
        let Some(step) = script.advance() else {
            // Finished: ask the window to close, so a run lasts exactly as
            // long as its script rather than a guessed timeout.
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        let modifiers = egui::Modifiers::default();
        match step {
            diag::Step::Wait => {}
            diag::Step::Move(x, y) => {
                raw_input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
            }
            diag::Step::Down(x, y) | diag::Step::Up(x, y) => {
                let pressed = matches!(step, diag::Step::Down(..));
                // The move rides along with the button so the pointer is at
                // the right place even if the preceding step was a `Wait`.
                raw_input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
                raw_input.events.push(egui::Event::PointerButton {
                    pos: egui::pos2(x, y),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                });
            }
            diag::Step::Zoom(factor) => raw_input.events.push(egui::Event::Zoom(factor)),
            diag::Step::Middle(pressed, x, y) => {
                raw_input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
                raw_input.events.push(egui::Event::PointerButton {
                    pos: egui::pos2(x, y),
                    button: egui::PointerButton::Middle,
                    pressed,
                    modifiers,
                });
            }
            diag::Step::Tool(which) => {
                // Routed through the SAME action the toolbar pushes, so a
                // scripted tool entry builds exactly the per-tool state a real
                // entry does. Setting `active_tool` directly would skip
                // `build_measure_state`, and the measure tool would then be
                // "armed" with no state — a harness artefact indistinguishable
                // from a bug.
                let tool = match which {
                    diag::ScriptTool::None => None,
                    diag::ScriptTool::Obj => Some(CanvasTool::VectorEdit),
                    diag::ScriptTool::Measure => Some(CanvasTool::MeasureLinear),
                };
                self.apply(Action::SelectCanvasTool(tool), ctx, ctx.pixels_per_point());
            }
            diag::Step::Delete => {
                for pressed in [true, false] {
                    raw_input.events.push(egui::Event::Key {
                        key: egui::Key::Delete,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers,
                    });
                }
            }
        }
        diag::trace(|| format!("step {step:?}"));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let pixels_per_point = ctx.pixels_per_point();
        let mut actions: Vec<Action> = Vec::new();

        // The outermost trace point: whether a frame ran at all, and whether
        // any pointer input reached egui. Everything else in the canvas trace
        // is downstream of both, so an empty canvas trace is only meaningful
        // once this line has shown a frame with a pressed pointer.
        if diag::enabled() {
            let (pressed, released, pos, events) = ctx.input(|i| {
                (
                    i.pointer.any_pressed(),
                    i.pointer.any_released(),
                    i.pointer.latest_pos(),
                    i.events.clone(),
                )
            });
            if !events.is_empty() {
                diag::trace(|| {
                    format!(
                        "frame pressed={pressed} released={released} pos={pos:?} doc={} events={events:?}",
                        matches!(self.status, Status::Open(_))
                    )
                });
            }
        }

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
        // Whether the CANVAS has something Delete should remove while a tool is
        // armed — see the binding in `collect_keyboard_actions`. Only the
        // object-edit tool qualifies: the text tools own Delete as a character
        // operation, and with no tool armed the key is collected anyway.
        let canvas_delete_target = match &self.status {
            Status::Open(doc) if doc.active_tool == Some(CanvasTool::VectorEdit) => {
                doc.entered.is_some_and(|e| e.subpath.is_some()) || !doc.canvas_selection.is_empty()
            }
            _ => false,
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
            canvas_delete_target,
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
        // The status panel's height is FIXED, not content-driven.
        //
        // ui-spec `gesture-commit-and-shell-conventions-audit.md` §3.3. With an
        // automatic height, anything the status bar has to say changes how much
        // vertical space is left for the canvas — and `apply_fit` re-derives the
        // zoom from that every frame. So merely SELECTING an object, which adds
        // one line to the selection readout, shrank the canvas and re-fitted the
        // page: measured on 2026-08-04 as the canvas going from
        // [[313.5 71.0]-[1466.5 962.0]] at zoom 0.7279 to
        // [[325.1 71.0]-[1454.9 944.0]] at zoom 0.7132 purely from a click.
        // The page visibly jumped and shrank when the operator clicked it.
        //
        // A constant makes the panel's contribution to the central area
        // invariant, so `apply_fit` only ever sees a viewport change the
        // operator caused on purpose — a window resize, a Fit-mode click,
        // toggling the rail or dock — all of which SHOULD re-fit.
        //
        // No disclosure is suppressed (rule R20, and rule 4's non-suppression
        // clause): `status_bar` already scrolls internally, so every line stays
        // reachable. Only the OUTER height stops reacting to how many lines
        // happen to exist this frame. The cost is a little permanently-reserved
        // space when there is nothing to say, which is how a status/terminal
        // panel behaves in most desktop editors, and strictly better than a
        // page that jumps on click.
        egui::Panel::bottom("status")
            .exact_size(STATUS_PANEL_HEIGHT_PTS)
            .show(ui, |ui| self.status_bar(ui));
        if self.rail_expanded {
            egui::Panel::left("thumbnails")
                .default_size(raster::THUMBNAIL_WIDTH_PTS + 40.0)
                .show(ui, |ui| {
                    self.thumbnail_rail(ui, &mut actions, pixels_per_point)
                });
        }
        // The panel dock claims RIGHT space, so like the rail it must be
        // added before the CentralPanel and after the full-width status
        // bar. Order is load-bearing here, not stylistic.
        //
        // Tab-chain (decision 017 §8.7): the panel-add order is UNCHANGED —
        // toolbar → status → rail → dock → canvas. Inside the dock, the tab
        // bars are drawn before their panes' widgets, so Tab still visits
        // pick-then-fill, exactly as the hand-rolled row list would have.
        if self.tools_open {
            egui::Panel::right("tools")
                .default_size(DOCK_DEFAULT_WIDTH_PTS)
                .show(ui, |ui| self.dock_body(ui, &mut actions));
        }
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui, &mut actions));
        // (Decision 017 §8.3 / A.4 #2: the document-properties floating
        // window used to be drawn here, after the panels, so it would not be
        // clipped. It is now a dock panel — see `DockPanel::Properties` —
        // and there is deliberately NO float-OR-dock dual mode.)
        // The signature confirmation is the one blocking question in this
        // Pass, so it draws over everything including the dock.
        self.signature_confirmation(&ctx, &mut actions);
        // And the copy confirmation alongside it: the same blocking
        // treatment, because a clipboard write is destructive to
        // whatever the operator had copied before.
        self.copy_confirmation(&ctx, &mut actions);
        // Pass 8.1: the redaction Apply report — the third and heaviest
        // blocking question, drawn with the other two so it too paints over
        // every panel. Its "blocking" comes from the `apply()` gate, not
        // from paint order (see that function's docs); the ordering here
        // only decides what is on top if two ever coexist, which the gate
        // makes impossible.
        self.redaction_apply_confirmation(&ctx, &mut actions);
        // The Pass 6.2 text-entry popup: a small non-blocking window that
        // collects the text before authoring.
        self.text_entry_popup(&ctx, &mut actions);
        // The keyboard-shortcuts reference (P1-2): modeless — reading a
        // reference while looking at the document is exactly the use case,
        // so it never blocks the canvas. R81 permits this as a TRANSIENT,
        // modeless reference; it is not something an operator keeps open
        // while working, which is what would make it owe a dock panel.
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
    canvas_delete_target: bool,
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
    //
    // `canvas_delete_target` re-opens exactly one hole in that yield: the
    // object-edit tool, which has its own Delete meaning and no text caret to
    // protect. Without it the binding was strictly unreachable for that tool —
    // `Action::DeleteSelection`'s own `delete_object` branch requires
    // `active_tool == VectorEdit`, and this gate guaranteed the key never
    // arrived in that state. A guard behind an unpassable filter is dead code
    // wearing a feature's clothes (R96), and it had been dead since Pass
    // 9c-min. Found on 2026-08-04 while wiring subpath delete, which would
    // have inherited the same silence the moment an operator armed the tool
    // first — the natural thing to do.
    //
    // The text tools are deliberately NOT given this hole: Delete there is
    // forward-character-delete, and hijacking it would break typing.
    if !tool_active || canvas_delete_target {
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
    fn icon_button(
        ui: &mut egui::Ui,
        icon: icons::Icon,
        tooltip: impl Into<String>,
    ) -> egui::Response {
        let image = icons::image(ui, icon);
        Self::labeled_icon_button(ui, egui::Button::new(image), tooltip)
    }

    // REMOVED 2026-08-03: `glyph_button`.
    //
    // It existed for "the handful of icon-only controls that have no assigned
    // SVG yet and so still draw a bare Unicode glyph" — in practice just the
    // page-rail and Combine-files reorder arrows. Those now draw real chevron
    // icons, because observation showed their U+25B2/U+25BC glyphs rendered as
    // empty boxes in egui's default font chain.
    //
    // With them converted, pdfce has NO text-glyph buttons left, so clippy
    // correctly flagged this as dead. That is a meaningful milestone rather
    // than a cleanup detail: every icon-only control in the app is now a drawn
    // icon whose appearance does not depend on the host font stack having a
    // codepoint nobody verified. The accessible-name guarantee is unaffected —
    // `icon_button` was always the sibling entry point into the same
    // `labeled_icon_button` body, which is where `WidgetInfo::labeled` lives.

    /// Size an icon-only button to [`ICON_BUTTON_SIZE`], attach its
    /// tooltip, and override its accessible name with that same tooltip
    /// text (P1-6).
    ///
    /// egui derives a widget's accessible name from its visible label.
    /// For a bare-glyph button that meant a screen reader announced "↻";
    /// for an IMAGE button it would announce nothing at all, which is
    /// strictly worse — so this override is more load-bearing after the
    /// icon swap than before it, and is why every icon-only control must
    /// come through here rather than calling `ui.add_sized` directly.
    fn labeled_icon_button(
        ui: &mut egui::Ui,
        button: egui::Button<'_>,
        tooltip: impl Into<String>,
    ) -> egui::Response {
        let name = tooltip.into();
        let enabled = ui.is_enabled();
        let response = ui
            .add_sized(ICON_BUTTON_SIZE, button)
            .on_hover_text(name.clone());
        response.widget_info(move || {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone())
        });
        response
    }

    /// A toolbar **menu** button: leading icon, label, and a trailing
    /// disclosure chevron — with an accessible name that says it opens a menu.
    ///
    /// # Why this wrapper exists
    ///
    /// The visible cue and the announced cue have to be supplied *separately*,
    /// and neither is automatic:
    ///
    /// - **Visible.** The affordance used to be a `▾` appended to the label.
    ///   U+25BE is in none of the fonts in egui's default Proportional chain
    ///   (Ubuntu-Light → NotoEmoji → emoji-icon-font), so it rendered as a tofu
    ///   box on every menu button pdfce shipped. It is now
    ///   [`icons::menu_chevron`], a real drawn glyph.
    /// - **Announced.** egui's [`egui::WidgetType`] has no menu/has-popup role
    ///   and `Ui::menu_button` sets no `WidgetInfo`, so "opens a menu" can only
    ///   reach assistive technology as literal text. An image announces
    ///   nothing, so replacing the glyph with a picture *alone* would have made
    ///   these controls LESS accessible than the bug did — a tofu box at least
    ///   carries a Unicode name some readers speak.
    ///
    /// Routing every menu button through one function is what stops those two
    /// halves drifting apart: you cannot add the chevron here and forget the
    /// name. Same reasoning as [`Self::labeled_icon_button`], which is the
    /// sibling wrapper for icon-only controls.
    ///
    /// See `docs/ui_specs/menu-affordance-and-glyph-coverage.md`.
    fn menu_button_labeled<R>(
        ui: &mut egui::Ui,
        icon: icons::Icon,
        label: String,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let image = icons::image(ui, icon);
        let rich = egui::RichText::new(label.clone());
        Self::menu_button_atoms(ui, image, rich, &label, add_contents)
    }

    /// [`Self::menu_button_labeled`] for a menu button whose icon and label
    /// carry their own **state styling** — currently the Measure menu, whose
    /// glyph goes bold and whose label changes while a sub-tool is active, so
    /// the active state is announced by weight as well as by colour (the
    /// standing "never colour alone" rule).
    ///
    /// Split out rather than folded in because the caller must be able to pass
    /// an already-styled image and `RichText`, while the ACCESSIBLE name must
    /// still be built from the plain text — a screen reader should hear
    /// "Measure: Linear, opens a menu", not markup.
    fn menu_button_atoms<R>(
        ui: &mut egui::Ui,
        icon: egui::Image<'static>,
        label: egui::RichText,
        plain_label: &str,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let atoms = (icon, label, icons::menu_chevron(ui));
        let enabled = ui.is_enabled();
        let name = ui_text::menu_button_accessible_name(plain_label);
        let inner = ui.menu_button(atoms, add_contents);
        inner.response.widget_info(move || {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone())
        });
        inner
    }

    /// An icon-only **toggle** (a `Button::selectable`), carrying three
    /// simultaneous selected-state cues.
    ///
    /// Standing rule: a selected state is never colour alone (project
    /// rule 6 / P1-1; ui-spec §5.3 names this as the one place an icon
    /// swap can silently regress an existing guarantee). A text toggle
    /// satisfies it by going **bold** — [`Self::toggle_label`] — but an
    /// icon-only toggle has no text to embolden. So when selected it
    /// gets:
    ///
    /// 1. egui's own selected background fill (colour — the cue that is
    ///    *not* sufficient on its own),
    /// 2. a heavier glyph ([`icons::IconWeight::Bold`], a **weight** cue),
    /// 3. an explicit outline ring ([`Self::selected_icon_ring`], a
    ///    **shape** cue).
    ///
    /// Two of the three survive being viewed in greyscale or by an
    /// operator with a colour-vision deficiency, which is the point.
    fn icon_toggle(
        ui: &mut egui::Ui,
        icon: icons::Icon,
        selected: bool,
        tooltip: &str,
    ) -> egui::Response {
        let image = icons::toggle_image(ui, icon, selected);
        let response = ui
            .add_sized(ICON_BUTTON_SIZE, egui::Button::selectable(selected, image))
            .on_hover_text(tooltip.to_owned());
        if selected {
            Self::selected_icon_ring(ui, response.rect);
        }
        let name = tooltip.to_owned();
        response.widget_info(move || {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                selected,
                name.clone(),
            )
        });
        response
    }

    /// An icon **plus text** toggle — Edit Text ("Aa"), Add Text ("Aa"),
    /// Edit Objects ("Obj").
    ///
    /// Lower risk than [`Self::icon_toggle`] because the text half of the
    /// label survives the icon swap untouched, so
    /// [`Self::toggle_label`]'s bolding still supplies a non-colour cue
    /// exactly as it did before (ui-spec §5.3: "no change needed"). The
    /// glyph is emboldened too, so the icon and its label agree rather
    /// than the icon looking inert beside bold text.
    fn icon_text_toggle(
        ui: &mut egui::Ui,
        icon: icons::Icon,
        selected: bool,
        text: &str,
        tooltip: &str,
    ) -> egui::Response {
        let image = icons::toggle_image(ui, icon, selected);
        ui.add(egui::Button::selectable(
            selected,
            (image, Self::toggle_label(selected, text)),
        ))
        .on_hover_text(tooltip.to_owned())
    }

    /// A plain (non-toggle) icon **plus text** button — Open, Save,
    /// Tools, and the three ▾ menu buttons.
    ///
    /// No accessible-name override is needed or wanted here: the visible
    /// text IS the name, and egui derives it correctly. Overriding it
    /// would be the same mistake in the opposite direction.
    fn icon_text(ui: &egui::Ui, icon: icons::Icon, text: &str) -> egui::Button<'static> {
        egui::Button::new((icons::image(ui, icon), egui::RichText::new(text.to_owned())))
    }

    /// Paint the selected-state outline ring described in
    /// [`Self::icon_toggle`].
    ///
    /// Drawn `Inside` the widget rect and inset by 1 pt so it reads as a
    /// frame around the control rather than as the control's own border
    /// growing — and so it cannot bleed into the neighbouring button and
    /// look like a rendering bug at tight toolbar spacing.
    fn selected_icon_ring(ui: &egui::Ui, rect: egui::Rect) {
        let visuals = ui.visuals();
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            visuals.widgets.active.corner_radius,
            egui::Stroke::new(1.5, visuals.selection.stroke.color),
            egui::StrokeKind::Inside,
        );
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
    ///
    /// # Overflow: the row WRAPS, and that is a deliberate choice
    ///
    /// This row used to be a plain `ui.horizontal`, which in egui means
    /// "lay everything out left to right and let whatever does not fit
    /// fall off the end". At the default 1100 pt launch width with a
    /// document open, the row ended at Add Text and the Edit Objects,
    /// Measure ▾, Copy ▾, Tools and Shortcuts controls were **entirely
    /// off screen with nothing on screen indicating they existed** —
    /// observed on a running build, not theorised. That is a real defect
    /// and a standing-rule violation (no silent truncation): an operator
    /// hunting for the dimensioning tool was told, in effect, that pdfce
    /// does not have one.
    ///
    /// Switching every control to a ~16 pt icon shrinks the row by
    /// several hundred points and hides the symptom, but does not fix
    /// the bug — a narrow enough window still overflows, and a future
    /// Pass adding two more tools would reintroduce it silently. So the
    /// row now **wraps** onto as many lines as it needs.
    ///
    /// The three candidates and why wrapping won:
    ///
    /// * **Overflow menu (a `»` chevron revealing the clipped tail).**
    ///   The conventional answer, and the one that reads best on a wide
    ///   window. Rejected because it needs to know which controls did not
    ///   fit *before* laying them out, and in an immediate-mode GUI that
    ///   means either measuring a frame late (the menu's contents lag the
    ///   window by one frame while resizing) or hard-coding a "these are
    ///   the low-priority ones" list that silently rots as Passes add
    ///   controls. It also still hides controls behind a second click.
    /// * **Horizontal scroll with an indicator.** Keeps one line, but a
    ///   scroll affordance is a weak cue — it is exactly the "there might
    ///   be more" signal operators miss — and it makes reaching a control
    ///   a gesture rather than a click.
    /// * **Wrapping (chosen).** The only option under which *no control
    ///   is ever hidden at all*, so the "never unreachable without a
    ///   visible cue that more exist" requirement is satisfied
    ///   structurally rather than by an affordance the operator has to
    ///   notice. The cost is that the toolbar grows taller on a narrow
    ///   window, which is honest, visible, and immediately reversible by
    ///   widening the window. Group separators survive wrapping, so the
    ///   grouping the module docs describe is still legible on two lines.
    ///
    /// The status summary keeps its right-hand pin: the row is a
    /// right-to-left layout containing the summary and then a nested
    /// wrapping left-to-right layout holding every control. Without the
    /// nesting the summary would join the wrap flow and drift leftward
    /// as controls were added, which is the exact drift the original
    /// right-alignment comment was written to prevent.
    fn toolbar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let summary = self.status_summary();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(summary);
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                    |ui| self.toolbar_controls(ui, actions),
                );
            });
        });
    }

    /// Every toolbar control, in group order, laid into the wrapping row
    /// [`Self::toolbar`] builds.
    ///
    /// Split out from [`Self::toolbar`] purely so the layout scaffolding
    /// and the control list can each be read without the other; the
    /// grouping, ordering and `ui.separator()` convention are unchanged
    /// from before the icon swap (the ui-spec explicitly does not
    /// reorder or regroup anything — it only assigns an image to each
    /// existing control).
    fn toolbar_controls(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        // Never let a control's OWN label wrap.
        //
        // Without this, a wrapping row hands each widget only the width
        // left on the current line, and egui's default `Wrap` mode makes
        // the widget honour it — so at ~640 pt the "Measure ▾" button
        // rendered its label one character per line, as a tall vertical
        // column that inflated the whole toolbar and pushed the History
        // and utility groups out of the panel. Observed on a running
        // build; it is the wrap fix's own failure mode and would have
        // been a worse defect than the clipping it replaced.
        //
        // `Extend` makes every widget report its full natural width, so
        // the wrap decision is taken at the CONTROL boundary — the whole
        // button moves to the next line, intact — which is the only
        // sensible unit to break a toolbar on.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        {
            // Group: file.
            if ui
                .add(Self::icon_text(
                    ui,
                    icons::Icon::Open,
                    ui_text::open_button(),
                ))
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
                    .add(Self::icon_text(
                        ui,
                        icons::Icon::Save,
                        ui_text::save_button(),
                    ))
                    .on_hover_text(ui_text::save_tooltip())
                    .clicked()
            {
                actions.push(Action::Save);
            }
            ui.separator();

            // Group: view (rail toggle, annotation-visibility). These
            // govern what is on screen rather than the document, per the
            // placement taxonomy (view-state → toolbar view group).
            if Self::icon_button(ui, icons::Icon::Sidebar, ui_text::rail_toggle_tooltip()).clicked()
            {
                actions.push(Action::ToggleRail);
            }
            // Annotation-visibility toggle (Pass 6.0). A `SelectableLabel`
            // rather than a plain button: on a lightly-annotated page,
            // flipping it can produce no visible canvas change, so the
            // control must itself carry and announce its on/off state
            // (the ui-specialist's Rule-6 note) — the highlight, the
            // state-stating tooltip and, since the icon swap left no text
            // to embolden (P1-1), the bold glyph + outline ring
            // [`Self::icon_toggle`] adds are the non-colour cues. It
            // honours the same click-target minimum as every other
            // icon-only control (P0-6) and carries an explicit accessible
            // name (P1-6), which matters MORE after the swap: an image
            // button publishes no usable name of its own. Shown only with
            // a document open: unlike the rail, it acts on the current
            // page's canvas.
            if let Status::Open(doc) = &self.status {
                let visible = doc.annotations_visible;
                let tooltip = if visible {
                    ui_text::annotations_toggle_tooltip_shown()
                } else {
                    ui_text::annotations_toggle_tooltip_hidden()
                };
                if Self::icon_toggle(ui, icons::Icon::Comment, visible, tooltip).clicked() {
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
                    if Self::icon_button(ui, icons::Icon::ChevronLeft, ui_text::prev_page_tooltip())
                        .clicked()
                    {
                        actions.push(Action::PrevPage);
                    }
                });
                ui.label(ui_text::page_nav_label(current, count));
                ui.add_enabled_ui(current < count, |ui| {
                    if Self::icon_button(
                        ui,
                        icons::Icon::ChevronRight,
                        ui_text::next_page_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::NextPage);
                    }
                });
                ui.separator();

                if Self::icon_button(ui, icons::Icon::ZoomOut, ui_text::zoom_out_tooltip())
                    .clicked()
                {
                    actions.push(Action::ZoomOut);
                }
                ui.label(ui_text::zoom_percent_label(doc.view.zoom_percent()));
                if Self::icon_button(ui, icons::Icon::ZoomIn, ui_text::zoom_in_tooltip()).clicked()
                {
                    actions.push(Action::ZoomIn);
                }
                // Fit modes are shown as selectable, because they are
                // modes: the operator can see at a glance whether the
                // view is currently *being kept* fitted or is pinned.
                let fit_page = doc.view.fit == FitMode::Page;
                if Self::icon_text_toggle(
                    ui,
                    icons::Icon::FitPage,
                    fit_page,
                    ui_text::fit_page_button(),
                    ui_text::fit_page_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::Fit(FitMode::Page));
                }
                let fit_width = doc.view.fit == FitMode::Width;
                if Self::icon_text_toggle(
                    ui,
                    icons::Icon::FitWidth,
                    fit_width,
                    ui_text::fit_width_button(),
                    ui_text::fit_width_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::Fit(FitMode::Width));
                }
                // Deliberately the ONE un-iconified control (ui-spec
                // §3.2): "100%" read at a glance already says "I am at
                // exactly true size", and every candidate glyph (a
                // magnifier with a "1" badge, a "1:1" pictograph) adds a
                // decode step a bare percentage does not need. Iconifying
                // it to satisfy "icons for all features" would be worse,
                // not better, so it stays plain text.
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
                    if Self::icon_button(ui, icons::Icon::RotateCcw, ui_text::rotate_left_tooltip())
                        .clicked()
                    {
                        actions.push(Action::RotateLeft);
                    }
                    if Self::icon_button(ui, icons::Icon::RotateCw, ui_text::rotate_right_tooltip())
                        .clicked()
                    {
                        actions.push(Action::RotateRight);
                    }
                });
                // Decision 017 §8.3 keeps this control and its shortcut as
                // the Properties entry point and changes only where they
                // lead. Its selected state is now DERIVED from the dock —
                // "the dock is open AND Properties is its front tab" — so
                // the toggle reports what is on screen rather than a
                // separate boolean that could disagree with it.
                if Self::icon_text_toggle(
                    ui,
                    icons::Icon::Properties,
                    self.tools_open && dock::panel_is_active(&self.dock, DockPanel::Properties),
                    ui_text::properties_button(),
                    ui_text::properties_tooltip(),
                )
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
                    Self::menu_button_labeled(
                        ui,
                        icons::Icon::Markup,
                        ui_text::markup_menu_button().to_owned(),
                        |ui| {
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
                                // Icon + text, never icon alone: a menu row is
                                // read, not scanned, so the words stay the
                                // primary label and the glyph is a recognition
                                // aid beside them (ui-spec §3.3).
                                let row = (
                                    icons::image(ui, kind.icon()),
                                    egui::RichText::new(kind.label()),
                                );
                                if ui.add(egui::Button::new(row)).clicked() {
                                    actions.push(Action::AddMarkupShape(kind));
                                }
                            }
                        },
                    )
                    .response
                    .on_hover_text(ui_text::markup_menu_tooltip());
                });

                // Pass 6.2 text-bearing authoring — the same edit-group,
                // same minimal-affordance approach as Markup. A menu opens
                // the text-entry popup; the actual authoring happens on
                // confirm (see the popup below). A full canvas text editor
                // is the named follow-up slice.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    Self::menu_button_labeled(
                        ui,
                        icons::Icon::Text,
                        ui_text::text_menu_button().to_owned(),
                        |ui| {
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
                                let row = (
                                    icons::image(ui, kind.icon()),
                                    egui::RichText::new(kind.label()),
                                );
                                if ui.add(egui::Button::new(row)).clicked() {
                                    actions.push(Action::OpenTextEntry(kind));
                                }
                            }
                        },
                    )
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
                    let response = Self::icon_text_toggle(
                        ui,
                        icons::Icon::EditText,
                        active,
                        ui_text::edit_text_tool_button(),
                        ui_text::edit_text_tool_tooltip(),
                    );
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
                    let response = Self::icon_text_toggle(
                        ui,
                        icons::Icon::AddText,
                        active,
                        ui_text::add_text_tool_button(),
                        ui_text::add_text_tool_tooltip(),
                    );
                    if response.clicked() {
                        actions.push(Action::SelectCanvasTool(if active {
                            None
                        } else {
                            Some(CanvasTool::AddText)
                        }));
                    }
                });

                // Pass 9c-min Edit Objects — a bare toggle for the vector-edit
                // tool (move / drag-node / delete). Same widget/sizing as the
                // page-text toggles; greyed (not hidden) with no pages. The
                // tooltip names the three gestures and the "not redaction"
                // caveat for delete (decision 011 §2.5).
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    let active = doc.active_tool == Some(CanvasTool::VectorEdit);
                    let response = Self::icon_text_toggle(
                        ui,
                        icons::Icon::EditObjects,
                        active,
                        ui_text::vector_edit_tool_button(),
                        ui_text::vector_edit_tool_tooltip(),
                    );
                    if response.clicked() {
                        actions.push(Action::SelectCanvasTool(if active {
                            None
                        } else {
                            Some(CanvasTool::VectorEdit)
                        }));
                    }
                });

                // Pass 12.M2 Measure ▾ — a menu (not four toolbar icons) for the
                // three dimension tools (ui-spec §1.2, rule 3: dimensioning is
                // used in short deliberate bursts, so it earns a menu, not
                // primary-icon creep). The widget is Markup ▾'s `menu_button`,
                // but the dispatch is Edit Text/Add Text's `SelectCanvasTool`
                // toggle (a NEW combination, ui-spec §1.2). The label is dynamic
                // so the active tool is never hidden by the closed menu.
                ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                    let active_name = match doc.active_tool {
                        Some(CanvasTool::MeasureLinear) => {
                            Some(ui_text::measure_tool_name_linear())
                        }
                        Some(CanvasTool::MeasureCircular) => {
                            Some(ui_text::measure_tool_name_circular())
                        }
                        Some(CanvasTool::MeasureScale) => Some(ui_text::measure_tool_name_scale()),
                        _ => None,
                    };
                    let label = match active_name {
                        Some(name) => ui_text::measure_menu_active_label(name),
                        None => ui_text::measure_menu_button().to_owned(),
                    };
                    // The menu BUTTON's glyph goes bold whenever any
                    // measure sub-tool is active, so the active state is
                    // carried by weight as well as by the dynamic label —
                    // the same "never colour alone" discipline the
                    // icon-only toggles get, applied to a menu.
                    Self::menu_button_atoms(
                        ui,
                        icons::toggle_image(ui, icons::Icon::Measure, active_name.is_some()),
                        Self::toggle_label(active_name.is_some(), &label),
                        &label,
                        |ui| {
                            let mut row = |ui: &mut egui::Ui, tool: CanvasTool, text: &str| {
                                let is_active = doc.active_tool == Some(tool);
                                if ui.selectable_label(is_active, text).clicked() {
                                    actions.push(Action::SelectCanvasTool(if is_active {
                                        None
                                    } else {
                                        Some(tool)
                                    }));
                                    ui.close();
                                }
                            };
                            row(
                                ui,
                                CanvasTool::MeasureLinear,
                                ui_text::measure_linear_menu_item(),
                            );
                            row(
                                ui,
                                CanvasTool::MeasureCircular,
                                ui_text::measure_circular_menu_item(),
                            );
                            row(
                                ui,
                                CanvasTool::MeasureScale,
                                ui_text::measure_set_scale_menu_item(),
                            );
                            ui.separator();
                            // "Manage Dimension Groups…" — opens the §5 modeless
                            // window; does NOT change active_tool (ui-spec §1.2).
                            if ui
                                .button(ui_text::measure_manage_groups_menu_item())
                                .clicked()
                            {
                                actions.push(Action::ToggleDimensionGroups);
                                ui.close();
                            }
                        },
                    )
                    .response
                    .on_hover_text(ui_text::measure_menu_tooltip());
                });

                // Pass 8.1 redaction (ui-spec §3.1) — the entry point to the
                // dock's Redact panel.
                //
                // ## Placement, and how it reconciles two rules that pull
                // opposite ways
                //
                // Standing rule 3 names redaction as an example of what
                // should stay OFF the primary toolbar (progressive
                // disclosure). Rule 7 wants destructive actions
                // DISCOVERABLE — and a security feature that is too well
                // hidden fails its own purpose in a specific, documented
                // way: an operator who cannot find how to redact improvises
                // with the Highlight tool, which is the overlay-only
                // false-redaction failure this whole feature exists to
                // prevent.
                //
                // One icon+label control at the END of the edit group is the
                // minimum weight that satisfies both: present, but not a new
                // group and not a menu. The edit group is its correct home —
                // it acts on the open document's own bytes, which is the
                // group's organising question, and the Properties toggle
                // above it already establishes that a panel toggle belongs
                // here when the panel is about the open document.
                //
                // The ui-spec argued for an UNGROUPED control instead. That
                // argument was against putting Redact in the Tools dock's
                // "files outside the one you have open" list, and it is
                // honoured — Redact is its own dock panel, not a Batch-Tools
                // row. What it did not anticipate is that the dock became a
                // general panel host with per-panel tabs, which removes the
                // framing collision the ungrouped placement was avoiding.
                //
                // Selected state is DERIVED from the dock (dock open AND
                // Redact the front tab), never a boolean of our own, so the
                // toggle cannot disagree with what is on screen.
                if Self::icon_text_toggle(
                    ui,
                    icons::Icon::Redact,
                    self.tools_open && dock::panel_is_active(&self.dock, DockPanel::Redact),
                    ui_text::redact_button(),
                    ui_text::redact_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::ToggleRedactPanel);
                }
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
                        icons::Icon::Undo,
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
                        icons::Icon::Redo,
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
                Self::menu_button_labeled(
                    ui,
                    icons::Icon::Copy,
                    ui_text::copy_text_button().to_owned(),
                    |ui| {
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
                    },
                )
                .response
                .on_hover_text(ui_text::copy_text_tooltip());
            }

            // The whole of Pass 3.2's toolbar growth: ONE toggle. Every
            // other new capability lives on the thumbnails (page-scoped)
            // or in the dock this opens (file-scoped). The toolbar is
            // capped at its existing six groups plus this.
            if ui
                .add(Self::icon_text(
                    ui,
                    icons::Icon::Tools,
                    ui_text::tools_button(),
                ))
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
            if Self::icon_button(ui, icons::Icon::Keyboard, ui_text::shortcuts_tooltip()).clicked()
            {
                self.shortcuts_open = !self.shortcuts_open;
            }
            // The status summary is NOT emitted here — it is pinned to
            // the row's right edge by [`Self::toolbar`]'s outer
            // right-to-left layout, so that it cannot join the wrap flow
            // and drift leftward as future Passes append tool groups.
        }
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

    // -- the panel dock (decision 017 + Amendment A) -----------------

    /// Draw the whole right-hand dock: a one-line header, then the
    /// `egui_tiles` tree.
    ///
    /// ## The borrow dance, and why it is written exactly this way
    ///
    /// `Tree::ui` takes `&mut dyn Behavior<Pane>`, and pdfce's behaviour
    /// needs `&mut PdfceApp` to draw panel bodies — but the tree lives *in*
    /// `PdfceApp`, so passing both at once is two mutable borrows of the
    /// same value. The standard escape is to move the tree out for the
    /// duration.
    ///
    /// **`std::mem::take` does not compile here.** `egui_tiles::Tree`
    /// derives only `Clone, PartialEq` — not `Default` — a fact decision 017
    /// §6.2 recorded in advance precisely so this Pass would not spend time
    /// rediscovering it. [`dock::swap_tree`] supplies the stand-in for
    /// `std::mem::replace` instead.
    ///
    /// While the swap is in place, `self.dock` is an EMPTY tree. Nothing
    /// reachable from a panel body may read or write it — a panel that wants
    /// to change the layout pushes an [`Action`], which is applied after the
    /// real tree is restored. The restore is unconditional (no early return
    /// between the replace and the put-back) so a panic-free path cannot
    /// leave the app dockless.
    fn dock_body(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        self.dock_header(ui, actions);

        // Snapshotted BEFORE the tree draws: `Behavior` callbacks have no
        // other way to learn which tile is its container's current tab, and
        // R84 needs that to give the active tab a weight cue rather than
        // leaving colour to carry the state alone.
        let active = self.dock.active_tiles();

        let mut tree = std::mem::replace(&mut self.dock, dock::swap_tree());
        let mut behavior = dock::DockBehavior {
            app: self,
            actions,
            active,
        };
        tree.ui(&mut behavior, ui);
        self.dock = tree;
    }

    /// The dock's header strip: the layout-reset command and the
    /// session-only disclosure.
    ///
    /// Both are here rather than on a tab bar for the same reason: they are
    /// properties of the DOCK, not of any one panel, and `egui_tiles`'
    /// per-tab-bar hook would have repeated them once per group.
    ///
    /// **Reset ships in the same Pass as the dragging** (decision 017 §8.12,
    /// promoted by A.4 #6 from "nice to have" to necessary): a draggable
    /// layout can be wrecked in ways a fixed one cannot — a pane dragged to
    /// a two-pixel sliver, a group nested inside a group inside a group —
    /// and shipping the wreckage without the undo would be shipping a trap.
    ///
    /// **The disclosure is visible text, not a tooltip** (§7 / A.6 / R82).
    /// An operator will arrange panels, close pdfce, and reopen it; finding
    /// the arrangement gone with no prior warning is exactly the surprise
    /// decision 012 set the precedent against for the font-folders setting.
    fn dock_header(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            if ui
                .button(ui_text::dock_reset_layout_button())
                .on_hover_text(ui_text::dock_reset_layout_tooltip())
                .clicked()
            {
                actions.push(Action::ResetPanelLayout);
            }
        });
        ui.label(
            egui::RichText::new(ui_text::dock_layout_session_only_note())
                .small()
                .weak(),
        );
        ui.separator();
    }

    /// **The one panel-body dispatcher** (decision 017 §8.1 / A.4 #1;
    /// standing rule R80).
    ///
    /// Every dockable surface is reached through here and nowhere else,
    /// which is what makes "no panel is reachable ONLY as a floating window"
    /// a structural property instead of a convention someone has to
    /// remember. §8.1 predicted this function would "survive verbatim if the
    /// §6 trigger ever fires" — it did fire, and it did.
    ///
    /// Each body gets its OWN scroll area with its OWN `id_salt`. Sharing
    /// one salt across panels is a real egui immediate-mode footgun: scroll
    /// state is keyed by id, so two panels sharing a salt would inherit each
    /// other's scroll offset when the operator switched tabs — the object
    /// tree scrolled to row 900 would leave the properties form scrolled off
    /// its own top.
    fn panel_body(&mut self, panel: DockPanel, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        match panel {
            DockPanel::Objects => egui::ScrollArea::vertical()
                .id_salt("dock-objects")
                .show(ui, |ui| self.objects_panel(ui)),
            DockPanel::Properties => egui::ScrollArea::vertical()
                .id_salt("dock-properties")
                .show(ui, |ui| self.properties_panel(ui, actions)),
            DockPanel::BatchTools => egui::ScrollArea::vertical()
                .id_salt("dock-batch-tools")
                .show(ui, |ui| self.tools_dock(ui, actions)),
            DockPanel::Redact => egui::ScrollArea::vertical()
                .id_salt("dock-redact")
                .show(ui, |ui| self.redact_panel(ui, actions)),
        };
    }

    /// The document-properties panel — the body that used to live in the
    /// floating `properties_window` (decision 017 §8.3 / A.4 #2).
    ///
    /// The migration is a **move, not a copy**: there is deliberately no
    /// float-OR-dock dual mode, because two code paths for the same content
    /// would each need their own open-state, position/size and focus
    /// handling, and would drift. The toolbar's Properties control and its
    /// shortcut are unchanged as the *entry point* — only their effect moved
    /// (see [`Action::ToggleProperties`]), so the muscle memory survives and
    /// only the destination changed.
    ///
    /// Never blank: with nothing open it states the precondition rather than
    /// showing an empty form, because a blank region is indistinguishable
    /// from a broken one.
    fn properties_panel(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Status::Open(doc) = &mut self.status else {
            ui.label(ui_text::properties_dock_no_document_hint());
            return;
        };
        if doc.properties_lossy {
            // Honesty over tidiness: some stored bytes could not be decoded
            // with certainty, so the operator is told before they overwrite
            // them (the code that decides this is
            // `pdfce_core::edit::decode_text_string`).
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
                    // Per-field lossy marking, carried from the Pass 3.1
                    // review: the panel-level warning says SOMETHING here is
                    // uncertain, which leaves the operator guessing which box.
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
        // Apply and Revert are greyed out when the draft already matches the
        // document: a live control that provably does nothing trains the
        // operator to distrust the panel. Carried from the Pass 3.1 review.
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
                // P1-5: the disabled Revert now explains itself the way the
                // disabled Apply beside it already does.
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
    }

    /// The object/layer tree panel (`docs/ui_specs/pass-17-dock-and-layer-
    /// tree.md` §B — the section the dock reversal left standing).
    ///
    /// ## What it is FOR — the operator's own words
    ///
    /// > *"I'd like to have a layer tree there for the document that I can
    /// > also click on to select objects. at least that way we can
    /// > troubleshoot better what I am clicking on in the GUI area."*
    ///
    /// So its first job is answering **"what am I clicking on"**, and every
    /// design choice below is subordinate to that. It is a diagnostic
    /// instrument first and a navigation aid second.
    ///
    /// ## A flat list, not a hierarchy — and why that is not a shortcut
    ///
    /// §B.1: `pdfce_core::vector::PageObjects` has **no optional-content-
    /// group (OCG) membership for page content at all** —
    /// `VectorObject::{Path,Text,Image}` carry no `/OC`. There is therefore
    /// no real grouping to render, and inventing one would be a lie about
    /// the document's structure. (The Pass 12.M2 dimension-group OCGs are a
    /// different mechanism entirely — annotation-layer visibility — and have
    /// their own panel; do not conflate them.) A middle grouping level
    /// becomes possible only once `decompose_page` tracks `BDC`/`EMC`
    /// optional-content membership. Deferred, not overlooked.
    ///
    /// ## Front-most FIRST — justified, not merely conventional (§B.2)
    ///
    /// The list is drawn in REVERSE paint order: the last-painted (topmost)
    /// object is the first row. Two reasons, in priority order:
    ///
    /// 1. **It matches what a click does.** `hit_test_point` resolves
    ///    overlapping candidates topmost-first, so the object the operator
    ///    most likely just hit — the one they are confused about — is at the
    ///    top of the list, not scrolled to the bottom of a thousand rows.
    ///    For a panel whose whole purpose is "what did I just click", any
    ///    other order buries the answer.
    /// 2. It is the prevailing convention for layer/object panels (top of
    ///    list = top of z-order), cited strictly as a metaphor-level
    ///    convention, never as a copied GUI structure.
    ///
    /// The row's visible `#n` is the **paint-order** index, NOT the display
    /// position. That is deliberate: `#n` is the number
    /// `pdfce-cli object-list` prints as `index=`, and the number
    /// `object-move`/`object-delete`/`node-move` take as an operand. A
    /// display-position number would look equally authoritative and address
    /// a different object.
    ///
    /// ## Row detail is honestly incomplete for Text and Image (§B.3/§B.4)
    ///
    /// Path rows are complete — paint disposition, colour, node count — all
    /// of it already in the model. Text and Image rows are not, and cannot
    /// be: `TextObject` carries a bbox, `approximate`, and token/byte spans
    /// — **no string, no font, no size** — and `ImageObject` carries no
    /// pixel dimensions or colourspace. The spec's illustrative
    /// `Text · "Section A-A" · Helvetica 10pt` row is not buildable today.
    /// Shipping the honest lesser row beats blocking the panel, and beats a
    /// fabricated one; §B.4's two core extensions are named as owed.
    ///
    /// This gap bites hardest exactly where it matters most — Text is the
    /// object kind most likely to be the "box over nothing" culprit, because
    /// its bbox is never measured glyph ink: it is laid out from the font's
    /// advance widths and designed ascent/descent where those are readable,
    /// and falls back to a coarse em box around the run's start where they
    /// are not. The row says which, in words, and the full sentence in the
    /// readout explains it.
    ///
    /// ## Virtualized, never silently truncated (§B.6)
    ///
    /// A complex drawing can decompose to tens of thousands of objects.
    /// `ScrollArea::show_rows` lays out only the rows actually on screen, so
    /// the list stays cheap at any size and **no cap is applied** — there is
    /// nothing to disclose because nothing is hidden. If a future cost
    /// (row-string precompute, memory) ever forces a cap, §B.6 binds it to
    /// be visible text naming both numbers, never a quietly shortened list.
    fn objects_panel(&mut self, ui: &mut egui::Ui) {
        let Status::Open(doc) = &mut self.status else {
            ui.label(ui_text::objects_dock_no_document_hint());
            return;
        };
        // The dock is added BEFORE the CentralPanel, so on the first frame
        // after a page change or an edit the canvas has not yet rebuilt the
        // provider. Calling it here makes the tree correct on that frame
        // instead of one frame stale; it is idempotent and costs a single
        // index compare in the steady state, and — critically — it is the
        // SAME `ensure_object_provider`, so the tree and the canvas share
        // ONE decomposition rather than each building their own (the Z2
        // "two decompositions quietly diverge" failure decision 011 warns
        // against).
        doc.ensure_object_provider();

        let Some(provider) = doc.object_model.as_ref() else {
            // `None` here means the page's content could not be decoded.
            // Stated in words rather than shown as an empty list, because a
            // failure state must never be visually indistinguishable from a
            // success state that happens to have no content.
            ui.label(ui_text::objects_dock_decompose_failed_hint());
            return;
        };
        let objects = &provider.page_objects().objects;
        let total = objects.len();
        if total == 0 {
            ui.label(ui_text::objects_dock_empty_page_hint());
            return;
        }

        ui.label(ui_text::objects_dock_intro());
        ui.label(ui_text::objects_dock_summary(
            total,
            doc.canvas_selection.len(),
        ));
        ui.separator();

        // The first selected target, in paint order. Drives both the
        // scroll-reveal edge trigger and nothing else — multi-select
        // highlights every matching row independently (§B.5).
        let first_selected = doc.canvas_selection.iter().next().copied();
        let reveal_row = (first_selected != doc.objects_revealed)
            .then(|| first_selected.map(|t| display_row_for_target(t, total)))
            .flatten();
        doc.objects_revealed = first_selected;

        // `Button::selectable` is not `small()` by default, so its height
        // floor is `interact_size.y`; declaring the same value as the row
        // height is what keeps `show_rows`' virtual scroll arithmetic in
        // step with what is actually painted.
        let row_height = ui.spacing().interact_size.y;
        let mut scroll = egui::ScrollArea::vertical().id_salt("objects-tree-rows");
        if let Some(row) = reveal_row {
            // Reveal by SCROLL OFFSET rather than `Response::scroll_to_me`:
            // under virtualization the selected row may not have been laid
            // out at all this frame, so there would be no response to scroll
            // to. The offset is computed from the same row geometry
            // `show_rows` uses, so it lands on the row regardless.
            let spacing = ui.spacing().item_spacing.y;
            scroll = scroll.vertical_scroll_offset(row as f32 * (row_height + spacing));
        }

        let mut clicked: Option<(TargetId, bool)> = None;
        scroll.show_rows(ui, row_height, total, |ui, rows| {
            for row in rows {
                // Front-most first: display row 0 is the LAST-painted
                // object (see the "Front-most FIRST" section above).
                let index = total - 1 - row;
                let Some(object) = objects.get(index) else {
                    continue;
                };
                let target = TargetId(index as u64);
                let selected = doc.canvas_selection.contains(&target);
                // ONE description path (`object_summary::describe_object`),
                // shared with the status-bar selection readout and the canvas
                // overlay's type badge. Two independently-written descriptions
                // of the same object is the divergence pattern decision 011
                // warns about, one layer above the decomposition it warns
                // about it for; this is the structural answer.
                let label = ui_text::object_row(index, &describe_object(object));
                // R84: selected state is never colour alone. The background
                // fill is `Button::selectable`'s; the BOLD is this project's
                // standing second cue, and survives greyscale.
                let text = Self::toggle_label(selected, &label);
                let response = ui.add_sized(
                    egui::vec2(ui.available_width(), row_height),
                    // `Atom::grow()` after the text pushes the label to the
                    // LEFT edge; a centred label in a list of rows reads as
                    // a column of buttons, not as a list.
                    egui::Button::selectable(selected, (text, egui::Atom::grow())).small(),
                );
                if response
                    .on_hover_text(ui_text::objects_dock_row_tooltip())
                    .clicked()
                {
                    clicked = Some((target, ui.input(|i| i.modifiers.shift)));
                }
            }
        });

        // Applied after the loop so the selection is not mutated while the
        // rows are still reading it (and so one click cannot cascade into
        // the rows drawn after it within the same frame).
        if let Some((target, shift)) = clicked {
            // §B.5: the EXACT function the canvas click path calls
            // (`main.rs`'s object-selection branch), never a second,
            // divergent selection path. Plain click replaces, Shift+click
            // toggles membership — the canvas's own convention, mirrored
            // rather than reinvented, so an operator who learned one has
            // already learned the other.
            doc.canvas_selection =
                canvas::selection_after_click(&doc.canvas_selection, Some(target), shift);
            // A tree-driven selection must not then yank the tree's own
            // scroll: the operator is already looking at the row they
            // clicked. Recording it as "already revealed" suppresses the
            // edge trigger on the next frame.
            doc.objects_revealed = doc.canvas_selection.iter().next().copied();
        }
    }

    /// The keyboard-shortcuts reference window (P1-2).
    ///
    /// Modeless: reading a shortcut
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
    ///
    /// # BASE READ ON PURPOSE (Pass 17.1 audit, decision 018 §8)
    ///
    /// `session.document().version()` is one of the two sites decision 018
    /// marks *"legitimately base reads — leave alone."* Do not change it to
    /// `session.view()`. The header version is a fact about **the file as
    /// loaded** (§7.5.2's `%PDF-n.m` header and the catalog's `/Version`
    /// override), and no edit in this session can raise it —
    /// `EditSession::view()`'s own doc comment records that it simply
    /// forwards the base's version, *"and if one ever can, this is the line
    /// that has to learn about it."* Routing this through the view would
    /// therefore change nothing today and quietly become a lie the day a
    /// version-raising edit exists, because it would report the base's
    /// version under a session-shaped name.
    ///
    /// The other two facts on this line are already session-aware and must
    /// stay so: `doc.pages.len()` comes from `EditSession::pages()` (the
    /// overlay walk, so a deleted page really disappears from the count) and
    /// `is_modified()` is the unsaved-changes flag.
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

        // The selection readout sits ABOVE the render diagnostics and above
        // the `page_texture` early-return, deliberately: it must survive a
        // frame where the page has not rasterized yet, and it is the line the
        // operator is looking for when they ask "what did I just click?".
        // Its whole reason for being in the status bar rather than only in the
        // dock is that the dock is not open by default (ui-spec §C.5).
        selection_readout(doc, ui, &mut self.selection_notes_expanded);

        let Some(texture) = &doc.page_texture else {
            ui.label(ui_text::diagnostics_no_document());
            return;
        };
        let d = &texture.diagnostics;
        // Document-scoped /NeedAppearances disclosure (R51) — computed
        // from the current document each frame (a cheap catalog lookup),
        // since it is not part of the per-page render diagnostics.
        //
        // SESSION READ (Pass 17.1 audit, decision 018 §8). This used to
        // pass `session.document()` — the base revision — so the flag
        // described the file as it was OPENED. That is wrong for a
        // disclosure the operator reads as "the state of the document in
        // front of me": form-field work in this session can set (or, on
        // undo, unset) `/AcroForm /NeedAppearances`, and a stale banner
        // either warns about a condition that is gone or stays silent
        // about one just created. `session.graph()` is the base with this
        // session's overlay applied — a plain dictionary lookup, no
        // stream bytes involved, which is why the graph suffices and the
        // heavier `view()` is not needed here.
        let need_appearances = pdfce_core::annot::need_appearances(&doc.session.graph());

        // Pass 8 (R52 / ui-spec §GUI, the ONE non-negotiable redaction GUI
        // item): a PERSISTENT disclosure of UNAPPLIED /Redact marks,
        // computed from the document's own annotations every frame — never a
        // session counter — so a marked-but-not-applied document can never be
        // mistaken for a redacted one (the #1 real-world redaction failure:
        // saving a marked file believing the content is gone).
        //
        // SESSION READ (Pass 17.1 audit) — the CONFIRMED bug decision 018
        // §8 names. Passing `session.document()` counted only marks that
        // were already in the file when it was opened, so a `/Redact` mark
        // placed THIS session — precisely the one an operator is most
        // likely to place and then forget to apply — was not disclosed at
        // all. Note this is still "computed from the document's own
        // annotations, never a session counter": `session.graph()` is a
        // census of annotation objects, not a tally the edit paths
        // increment, so it remains immune to a miscount and still survives
        // save/reload unchanged. A mark added and then undone counts zero,
        // because the overlay holds the base value again.
        let pending_redactions = pdfce_core::redact::count_redaction_marks(&doc.session.graph());
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
        // `contents_streams_unresolved` joins the headline for the same
        // reason `images_unsupported` does: it is a thing pdfce did not
        // draw. It is arguably the most important member of the set — the
        // others leave a page that is missing an element, this one can
        // leave a page that is missing EVERYTHING — so it must never be
        // the one counter that only shows up when the expander is opened.
        let unsupported = d.fonts_unsupported
            + d.deferred_ops
            + d.unknown_ops
            + d.images_unsupported
            + d.contents_streams_unresolved
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
            // First in the detail list, ahead of every font/image line: if
            // a page's content stream is missing from the file, that fact
            // explains the page better than anything below it can.
            if d.contents_streams_unresolved > 0 {
                ui.label(ui_text::diagnostics_contents_unresolved(
                    d.contents_streams_unresolved,
                ));
            }
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
                // Same icons as the toolbar's single-page rotate, because
                // it is the same operation over a wider scope; the
                // tooltip (which names the page count) is what carries
                // the difference.
                if Self::icon_button(
                    ui,
                    icons::Icon::RotateCcw,
                    ui_text::batch_rotate_left_tooltip(selected_count),
                )
                .clicked()
                {
                    actions.push(Action::RotateSelection(-90));
                }
                if Self::icon_button(
                    ui,
                    icons::Icon::RotateCw,
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
                // Still bare Unicode glyphs: the ui-spec's icon mapping
                // covers the toolbar and the Tools dock, and assigns
                // nothing to the rail's reorder arrows. Rather than
                // These were the LAST two text-glyph controls. They kept
                // their U+25B2/U+25BC triangles when the icon set shipped
                // because no icon had been drawn for them — and observation
                // on 2026-08-03 showed those glyphs render as EMPTY BOXES in
                // egui's default font chain, exactly like the menu `▾` did.
                // Being glyph-only, that left them with no visible identity
                // at all. Now real chevrons; the accessible names are
                // unchanged because both entry points always shared
                // the same wrapper.
                if Self::icon_button(
                    ui,
                    icons::Icon::ChevronUp,
                    ui_text::move_selection_up_tooltip(),
                )
                .clicked()
                {
                    actions.push(Action::MoveSelection(-1));
                }
                if Self::icon_button(
                    ui,
                    icons::Icon::ChevronDown,
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
                                            // SESSION READ (Pass 17.1 audit,
                                            // decision 018 §8 — "thumbnails
                                            // need a read fix, not a key").
                                            // Until this Pass the rail built
                                            // from `session.document().view()`
                                            // — the base revision wearing the
                                            // new parameter type — so the page
                                            // rail showed the file AS OPENED
                                            // while the canvas beside it showed
                                            // the file as EDITED. Two pictures
                                            // of the same page, disagreeing, is
                                            // worse than the original defect:
                                            // it invites the operator to trust
                                            // the wrong one.
                                            //
                                            // No generation/cache key is needed
                                            // to make this correct.
                                            // `refresh_pages` already resets
                                            // `ThumbnailCache` wholesale on
                                            // every edit, undo and redo, so a
                                            // stale picture cannot survive a
                                            // commit; only the READ was wrong.
                                            // (Cost: a full rail re-render per
                                            // edit, which is what already
                                            // happened — the pictures were
                                            // simply rebuilt identical.)
                                            //
                                            // `session.view()`, not
                                            // `.graph()`: a thumbnail is a
                                            // raster, so it needs the stream
                                            // BYTES of authored appearance
                                            // streams too, which only the
                                            // R45 `StreamSource::Split` form
                                            // can resolve.
                                            doc.thumbnails.build(
                                                &ctx,
                                                &doc.session.view(),
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
        // Pass 9a: keep the current page's object-model provider fresh, so
        // click-select, marquee, and the selection outline all query the
        // right page's geometry (rebuilt lazily on page change / edit).
        doc.ensure_object_provider();

        let tool_active = doc.active_tool.is_some();
        // Pass 9a interaction decision (the marquee-vs-pan disambiguation the
        // Pass 12.0 substrate deferred to this Pass, spec §4.2): with NO tool
        // active the canvas is in object-selection mode, where a drag is a
        // rubber-band MARQUEE, not a pan — the Inkscape/Illustrator
        // convention (R61). Panning moves to the mouse wheel and the
        // scrollbars (both unchanged). With a tool active, the tool's own
        // suppression rule (`canvas_suppresses_pan`) applies as before. (A UX
        // review by `pdfce-ui-specialist` is owed on this default; it was
        // decided here to satisfy the Pass 9a marquee acceptance criterion.)
        let selection_mode = !tool_active;
        let suppress_pan = selection_mode || canvas::canvas_suppresses_pan(tool_active, None);
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
        // `ScrollSource.drag` is the knob).
        //
        // CORRECTED 2026-08-04. This said: "Always `DragScroll::Always` this
        // Pass, since `suppress_pan` is always `false`." It asserted the
        // exact opposite of what the code does.
        //
        //     suppress_pan = selection_mode || canvas_suppresses_pan(t, None)
        //                  = !t            || (t && true)
        //                  = !t || t
        //                  = true, in every state
        //
        // So `drag` is set to `Never` unconditionally and DRAG-TO-PAN IS OFF
        // EVERYWHERE — with a tool active and without one. That is what Pass
        // 9a decided in effect (its own comment above says panning "moves to
        // the mouse wheel and the scrollbars"), so the BEHAVIOUR is
        // intentional; what was wrong was this comment insisting the opposite,
        // while the module doc at the top of this file still says "Drag pans,
        // via the scroll area's own drag-to-scroll" — also now false.
        //
        // The `||` is a tautology and reads as a live decision. Left as-is
        // deliberately: making it `= tool_active` would be a no-op today and
        // would quietly change behaviour the moment the no-tool branch is
        // revisited, which is precisely the review the operator's
        // navigation request has now triggered. It should be resolved THERE,
        // as a decision, not tidied away here as a cleanup.
        //
        // Found because the operator asked for middle-drag panning; the
        // reason they asked is that plain-drag panning has not existed since
        // Pass 9a, and nothing said so.
        let mut scroll_source = egui::scroll_area::ScrollSource::ALL;
        if suppress_pan {
            scroll_source.drag = egui::scroll_area::DragScroll::Never;
        }
        // Zoom to cursor, half two: a wheel step was seen last frame and the
        // new zoom is now known (post-clamp), so solve for the offset that
        // keeps the anchored page point under the pointer and force it onto
        // the area before it lays out. Taken, not peeked — one wheel step
        // moves the view once.
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt("page-canvas")
            .scroll_source(scroll_source);
        if let Some(anchor) = doc.zoom_anchor.take() {
            let (x, y) = canvas::zoom_anchor_offset(
                anchor.offset_before,
                anchor.display_before,
                (display_size.x, display_size.y),
                anchor.viewport,
                anchor.frac,
            );
            scroll_area = scroll_area.scroll_offset(egui::vec2(x, y));
        } else {
            // Middle-drag pans the page — the CAD/Inkscape/Illustrator/browser
            // convention the operator asked for on 2026-08-04 ("middle click -
            // drag to move the page around on screen").
            //
            // Pointer-drag panning has not existed since Pass 9a, which gave
            // the canvas a marquee instead and moved panning "to the mouse
            // wheel and the scrollbars". That decision is left standing for the
            // PRIMARY button: a left-drag on a page of drawing objects should
            // rubber-band. The middle button was simply never assigned, so this
            // takes nothing away from anything.
            //
            // Implemented against the scroll offset directly rather than by
            // re-enabling `ScrollSource.drag`, because that knob is
            // button-agnostic — turning it on would restore left-drag panning
            // and destroy the marquee. Panning subtracts the pointer delta:
            // the content follows the hand, so the page moves WITH the pointer
            // rather than under it.
            //
            // Gated on the pointer being over the canvas so a middle-drag that
            // began on the thumbnail rail or a dock panel does not yank the
            // page sideways.
            let pan = ui.input(|i| {
                let over = i
                    .pointer
                    .latest_pos()
                    .is_some_and(|p| ui.max_rect().contains(p));
                if i.pointer.middle_down() && over {
                    i.pointer.delta()
                } else {
                    egui::Vec2::ZERO
                }
            });
            if pan != egui::Vec2::ZERO {
                let vp = ui.available_size();
                let (x, y) = canvas::pan_offset(
                    (doc.last_scroll_offset.x, doc.last_scroll_offset.y),
                    (pan.x, pan.y),
                    (display_size.x, display_size.y),
                    (vp.x, vp.y),
                );
                scroll_area = scroll_area.scroll_offset(egui::vec2(x, y));
                // The gesture has to look like what it is. Without a cursor
                // change a middle-drag that hits the end of the scroll range
                // is indistinguishable from a middle-drag that is not working.
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        let scroll_output = scroll_area.show(ui, |ui| {
            // Centre the page MANUALLY rather than with
            // `ui.centered_and_justified`, because that helper returns the
            // JUSTIFIED CONTAINER rect — the whole available area — while
            // drawing the image centred inside it. Taking that rect as
            // `image_rect` made every page↔screen mapping wrong by the
            // centring margin whenever the page was smaller than the
            // viewport.
            //
            // The symptom was severe and specific: at "Fit page" on a page
            // narrower/shorter than the canvas, selection outlines drew
            // offset from the object they outlined (~105 px on one
            // measured case — exactly the vertical margin), and clicking
            // directly ON a visible object MISSED it. At high zoom, where
            // the page exceeds the viewport and the margin is zero, the
            // same click landed perfectly. That is the giveaway: the error
            // scaled with the margin, not with the zoom.
            //
            // Worst of all, it is worst at exactly the zoom an operator
            // uses to see a whole page. This is a THIRD distinct cause of
            // the operator's 2026-08-02 "I don't seem to be able to click
            // on objects", after the zoom-inverted select tolerance
            // (`SELECT_SCREEN_TOLERANCE_PX`) and the object-edit tool
            // drawing no selection outline at all.
            //
            // So: reserve `max(page, viewport)` so the ScrollArea still
            // scrolls when the page is larger AND there is a margin to
            // centre within when it is smaller, then place the image at an
            // explicit centred rect. `Ui::put`/`allocate_rect` return a
            // Response whose `.rect` IS that rect, so `image_rect` is the
            // page's true drawn rect by construction rather than by
            // coincidence.
            let avail = ui.available_size();
            let outer = egui::vec2(display_size.x.max(avail.x), display_size.y.max(avail.y));
            let (outer_rect, _) = ui.allocate_exact_size(outer, egui::Sense::hover());
            let page_rect = egui::Rect::from_center_size(outer_rect.center(), display_size);
            let response = if let Some(texture) = texture {
                ui.put(
                    page_rect,
                    egui::Image::from_texture(&texture)
                        .fit_to_exact_size(display_size)
                        .sense(canvas_sense),
                )
            } else {
                // First frame after an open: the texture is made at the
                // end of this frame. Reserve the page's space (same rect,
                // same sense) so nothing jumps when it arrives — and so
                // the substrate's canonical canvas response exists even
                // before the first raster.
                ui.allocate_rect(page_rect, canvas_sense)
            };
            // `avail` rides out with the response because it is the
            // viewport the zoom-to-cursor solve needs, and it is only
            // knowable in here — the same `avail` that decided `outer`
            // above, so the margin the solve reconstructs is the margin
            // this frame actually drew.
            (response, avail)
        });

        let (image_response, viewport_size) = scroll_output.inner;
        // The offset the area settled on THIS frame, i.e. the `offset_before`
        // of any zoom step the operator starts now, and the base the next
        // frame's middle-drag pan moves from.
        let scroll_offset = scroll_output.state.offset;
        doc.last_scroll_offset = scroll_offset;

        // This `image_response` — not the outer ScrollArea's — is the
        // substrate's canonical canvas response; its `.rect` is the
        // `image_rect` every §2 geometry call takes this frame.
        let image_rect = image_response.rect;

        // The trace that answers "did the canvas widget see the click at all",
        // which no amount of reading the dispatch below can answer. Emitted
        // only on a frame where the pointer actually did something, so a run
        // produces a handful of lines rather than one per idle frame.
        if diag::enabled() {
            let (down, pressed, released, zoom_delta) = ui.input(|i| {
                (
                    i.pointer.any_down(),
                    i.pointer.any_pressed(),
                    i.pointer.any_released(),
                    i.zoom_delta(),
                )
            });
            if pressed || released || down || (zoom_delta - 1.0).abs() > f32::EPSILON {
                let p = ui.ctx().pointer_latest_pos();
                diag::trace(|| {
                    format!(
                        "canvas tool={:?} rect={:?} zoom={zoom} hovered={} clicked={} \
                         drag_started={} dragged={} drag_stopped={} pointer={p:?} \
                         interact={:?} down={down} pressed={pressed} released={released} \
                         zdelta={zoom_delta} off={scroll_offset:?} provider={} sel={}",
                        doc.active_tool,
                        image_rect,
                        image_response.hovered(),
                        image_response.clicked(),
                        canvas::primary_drag_started(&image_response),
                        canvas::primary_dragged(&image_response),
                        canvas::primary_drag_stopped(&image_response),
                        image_response.interact_pointer_pos(),
                        doc.object_model.is_some(),
                        doc.canvas_selection.len(),
                    )
                });
            }
        }

        // §1.4: clicking the canvas (or, once a tool drags, drag-starting it)
        // requests focus for the canvas's own id, making it a genuine Tab
        // stop (§6.2) rather than an inert image.
        if image_response.clicked() || canvas::primary_drag_started(&image_response) {
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
        } else if canvas::tool_builds_measure(doc.active_tool) {
            // Pass 12.M2: a measure tool is selected. The on-canvas snap-pick
            // authoring gesture is the documented follow-up UI slice; this build
            // surfaces the selected tool honestly (a status overlay) while the
            // full authoring path is available today via pdfce-cli. The tool
            // suppresses the object-selection click so a measure-mode click is
            // not silently repurposed as a selection (ui-spec §1.1).
            run_measure_tool(doc, ui, &image_response, image_rect, extent, zoom);
        } else if canvas::tool_builds_vector_edit(doc.active_tool) {
            // Pass 9c-min (decision 011 §2.5): the object-edit tool owns the
            // canvas — click selects, drag moves the selected object (or a
            // grabbed node), Delete removes it, each committing one undoable
            // `EditSession` command. It suppresses the plain object-selection
            // marquee so a drag is unambiguously an edit gesture, not a
            // rubber-band.
            run_vector_edit_tool(doc, ui, &image_response, image_rect, extent, zoom);
        } else {
            if (image_response.clicked() || image_response.double_clicked())
                && let Some(screen_pos) = image_response.interact_pointer_pos()
            {
                let canvas_pos = viewer::screen_to_page(screen_pos, image_rect, extent, zoom);
                let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
                let tol =
                    canvas::screen_tolerance_to_page(canvas::SELECT_SCREEN_TOLERANCE_PX, zoom);
                // Depth first: a double-click descends into the object under
                // the pointer, and a click inside an entered object picks one
                // of its subpaths. `true` means the click was consumed at that
                // level, so the object-level selection below is skipped —
                // otherwise picking a line inside a view would re-select the
                // whole view in the same frame and visually undo the descent.
                let consumed_inside =
                    doc.apply_click_depth(canvas_pos, tol, image_response.double_clicked());
                diag::trace(|| {
                    format!(
                        "depth-click canvas={canvas_pos:?} double={} consumed={consumed_inside} \
                         entered={:?}",
                        image_response.double_clicked(),
                        doc.entered
                    )
                });
                // The ALL-hits query, always — even for a plain click, whose
                // outcome is unchanged (the head of the list). The extra
                // information is what the readout discloses as "1 of 3 at
                // this point", which is how the operator finds out there is
                // anything underneath to Alt+click to (ui-spec §C.3).
                //
                // Skipped entirely when the click was consumed inside an
                // entered object. Computing it and discarding it would be
                // merely wasteful; TRACING it and discarding it was actively
                // misleading — the log read `newsel=1` for a selection that was
                // never applied, which is the kind of evidence that sends the
                // next investigation down a false path (R93).
                let hits = if consumed_inside {
                    Vec::new()
                } else {
                    doc.target_provider()
                        .map(|p| p.hit_test_all(doc.view.page_index, canvas_pos, tol))
                        .unwrap_or_default()
                };
                let (selection, cycle) = canvas::selection_and_cycle_after_click(
                    &hits,
                    &doc.canvas_selection,
                    doc.click_cycle,
                    doc.view.page_index,
                    canvas_pos,
                    shift,
                    alt,
                );
                // Applied — and traced — only when the click was NOT consumed
                // inside an entered object: picking a line within a view must
                // not also re-select the whole view, which would visually undo
                // the descent in the same frame it happened.
                if !consumed_inside {
                    diag::trace(|| {
                        format!(
                            "plain-click screen={screen_pos:?} canvas={canvas_pos:?} tol={tol} \
                             hits={} first={:?} newsel={}",
                            hits.len(),
                            hits.first(),
                            selection.len()
                        )
                    });
                    doc.canvas_selection = selection;
                    doc.click_cycle = cycle;
                }
            }

            // §4.2 marquee — Pass 9a's rubber-band selection. A drag on the
            // canvas (which now suppresses pan in selection mode) records its
            // start in canvas space, draws the in-progress rectangle, and on
            // release asks the provider which objects it fully encloses,
            // folding them into the selection (plain = replace, Shift = add).
            if canvas::primary_drag_started(&image_response) {
                doc.marquee_start = image_response
                    .interact_pointer_pos()
                    .map(|s| viewer::screen_to_page(s, image_rect, extent, zoom));
            }
            if let Some(start_canvas) = doc.marquee_start {
                match image_response.interact_pointer_pos() {
                    Some(cur_screen) => {
                        let cur_canvas =
                            viewer::screen_to_page(cur_screen, image_rect, extent, zoom);
                        let canvas_rect = egui::Rect::from_two_pos(start_canvas, cur_canvas);
                        // Draw the marquee outline (a 1px shape, projected
                        // back to the screen), never a re-raster.
                        let painter = ui.painter_at(image_rect);
                        let min_s =
                            viewer::page_to_screen(canvas_rect.min, image_rect, extent, zoom);
                        let max_s =
                            viewer::page_to_screen(canvas_rect.max, image_rect, extent, zoom);
                        painter.rect_stroke(
                            egui::Rect::from_two_pos(min_s, max_s),
                            0.0,
                            egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                            egui::StrokeKind::Inside,
                        );
                        if canvas::primary_drag_stopped(&image_response) {
                            let shift = ui.input(|i| i.modifiers.shift);
                            let hits = doc
                                .target_provider()
                                .map(|p| p.hit_test_rect(doc.view.page_index, canvas_rect))
                                .unwrap_or_default();
                            doc.canvas_selection = canvas::selection_after_marquee(
                                &doc.canvas_selection,
                                &hits,
                                shift,
                            );
                            doc.marquee_start = None;
                        }
                    }
                    // The pointer left the window mid-drag: abandon the marquee
                    // rather than commit a rectangle with no end point.
                    None => doc.marquee_start = None,
                }
            }

            // §5.1 live-preview overlay: painted above the raster via the
            // painter, NEVER a re-raster. A selection outline is a 2px SHAPE,
            // not a tint (rule 6): a real boundary.
            draw_selection_outlines(doc, ui, image_rect, extent, zoom);
        } // end: non-text-edit-tool object-selection path (Pass 14.3 gate)

        // Pass 12.M2b ui-spec §5: the modeless "Dimension Groups" panel. Drawn
        // here (inside the open-doc scope) so it is available with or without a
        // measure tool active — a scale can be set / a layer toggled by typing,
        // no line required (ui-spec §7.2). A no-op when closed.
        run_dimension_groups_panel(doc, ui);

        // Ctrl+wheel over the canvas: multiply the zoom. Gated on hover
        // so a ctrl+wheel aimed at the thumbnail rail does not zoom the
        // page out from under the operator.
        if image_response.hovered() {
            let factor = ui.ctx().input(|i| i.zoom_delta());
            if (factor - 1.0).abs() > f32::EPSILON {
                // Zoom to cursor, half one: remember WHERE on the page the
                // pointer is before the zoom lands. Anchoring on the viewport
                // centre instead (which is what happens when nothing records
                // this) drags the detail being inspected out from under the
                // operator, worse the further off-centre they point — reported
                // as "jarring" on 2026-08-04. Solved next frame, once the
                // clamped zoom is known; see `ZoomAnchor`.
                //
                // Guarded on a positive drawn size because `frac` divides by
                // it, and on a known pointer because a zoom gesture can also
                // arrive from a trackpad pinch with the pointer off-window.
                let pointer = ui.ctx().pointer_latest_pos();
                if let Some(p) = pointer
                    && display_size.x > 0.0
                    && display_size.y > 0.0
                {
                    doc.zoom_anchor = Some(ZoomAnchor {
                        frac: (
                            (p.x - image_rect.min.x) / display_size.x,
                            (p.y - image_rect.min.y) / display_size.y,
                        ),
                        offset_before: (scroll_offset.x, scroll_offset.y),
                        display_before: (display_size.x, display_size.y),
                        viewport: (viewport_size.x, viewport_size.y),
                    });
                }
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
///
/// # SESSION READ (Pass 17.1 audit, decision 018 §8)
///
/// The parameter used to be `&Document` and its one caller passed
/// `session.document()` — the base revision. That made the family list
/// **stale after any edit that adds a font resource to the page**, which is
/// not a hypothetical: `EditSession::format_text`'s font-family change and
/// `EditSession::add_text` both add a font dictionary to the page's
/// `/Resources /Font` (Pass 16.0 / R79). The operator would change a run to
/// a new family, see it correctly on the canvas after Pass 17.0, and then
/// find that same family missing from the ComboBox that just applied it —
/// the list disagreeing with the page it describes.
///
/// It is now generic over [`ObjectGraph`](pdfce_core::graph::ObjectGraph) so
/// the caller can pass `&session.graph()`. Generic rather than
/// `&DocumentView` for the same reason as
/// [`pdfce_core::redact::count_redaction_marks`]: this reads font
/// **dictionaries** only and never touches a stream's bytes, so an object
/// graph is exactly the capability it needs and nothing more.
///
/// Note `page.resources` itself is the `refresh_pages` snapshot (decision
/// 018 §10 hazard 2). That is consistent, not a second staleness: every
/// commit path funnels through `refresh_pages`, so the snapshot and the
/// graph describe the same revision.
fn page_font_entries<G: pdfce_core::graph::ObjectGraph + ?Sized>(
    base: &G,
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
        // Pass 19.3: one short "what would lift it" hint per refusal the new
        // rows can provoke, joined to core's verbatim `Display` text by
        // `refusal_with_hint` — the pattern the R-INV hints already establish.
        FormatError::ConflictingRise => ui_text::conflicting_rise_hint(),
        FormatError::RealFaceAvailable { .. } => ui_text::real_face_available_hint(),
        FormatError::ShearUnsupported(_) => ui_text::shear_unsupported_hint(),
        FormatError::AmbientUnrestorable(_) => ui_text::ambient_unrestorable_hint(),
        FormatError::BadHorizScale(_) => ui_text::bad_h_scale_hint(),
        // Pass 19.4. The panel does not draw an Apply button for word
        // spacing on a composite run (R83), so this refusal should be
        // unreachable from the GUI — a hint is provided anyway, because
        // "unreachable" is a claim about today's panel and a refusal with
        // no next step is a dead end whichever path produced it.
        FormatError::WordSpacingComposite { .. } => ui_text::word_spacing_composite_hint(),
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
        MetricSpec, NewFill, ReflowEngine, ReflowRequest, ScriptPosition, StyleResolution,
        StyleSynthesis, TextPosition, reflow_recognition_options,
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
        if canvas::primary_drag_started(image_response) {
            // §4.1: press sets BOTH ends to the start caret; the drag then moves
            // the focus end (the `dragged()` arm below) while the anchor holds.
            if let Some(sp) = image_response.interact_pointer_pos() {
                let hit = hit_at(sp);
                click_result = Some((hit, hit));
            }
        } else if canvas::primary_dragged(image_response) {
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
    // Pass 19.3 property-bar intents — one per Apply button, same shape.
    let mut apply_char_spacing: Option<MetricSpec> = None;
    let mut apply_word_spacing: Option<MetricSpec> = None;
    let mut apply_h_scale: Option<f64> = None;
    let mut apply_script: Option<ScriptPosition> = None;
    let mut apply_rise: Option<MetricSpec> = None;
    let mut apply_synthetic: Option<StyleSynthesis> = None;
    // A locally-refused Apply (the mixed real-face/synthesis case), routed
    // into the SAME refusal strip as every core refusal rather than into a
    // second disclosure surface.
    let mut local_refusal: Option<String> = None;
    // Pass 15.2 reflow intents, resolved after the property-bar/status closures.
    let mut enter_reflow = false;
    let mut do_accept_reflow = false;
    let mut do_reject_reflow = false;
    let mut reflow_changed = false;
    // The page cropbox for building overflow-aware reflow requests (§0.2/§6),
    // captured here (disjoint from the `&mut text_edit` borrow below).
    let reflow_page_crop = doc.pages.get(page_index).map(|page| page.crop_box);

    // Font list for the property bar (needs the object graph + page; disjoint
    // from the `&mut text_edit` borrow below since they are separate fields).
    //
    // SESSION READ (Pass 17.1 audit): `session.graph()`, not
    // `session.document()`. A font-family format edit and `add_text` both add
    // a font dict to the page's `/Resources /Font`; reading the base made the
    // ComboBox omit the very family the operator had just applied. See
    // `page_font_entries`' own doc comment.
    let font_entries = doc
        .pages
        .get(page_index)
        .map(|page| page_font_entries(&doc.session.graph(), page, font_env))
        .unwrap_or_default();

    // Pass 19.3 §1.1 "Option B": what WOULD a synthetic-style request resolve
    // to for the caret's run — a real face, or synthesis?
    //
    // Computed here, before the `&mut text_edit` borrow, because it needs
    // `doc.session` (an immutable read of a DIFFERENT field) and the two
    // borrows cannot overlap. It is a genuinely read-only core query
    // (`EditSession::preview_style_resolution`), so ordering it before the
    // mutation phase costs nothing.
    //
    // IMMEDIATE-MODE COST CONTROL (ui-spec §6.4): the query walks the page's
    // `/Font` resources, so it must NOT run every frame. It runs only when
    // `(run, bold, italic)` differs from what the cached answer was computed
    // for — which, since ticking a checkbox happens during the draw, makes
    // the caption at most one repaint late. The draw requests that repaint
    // explicitly rather than waiting for the caret blink to trigger one.
    type StylePreviewRefresh = (Option<Result<StyleResolution, String>>, (usize, bool, bool));
    let style_preview_refresh: Option<StylePreviewRefresh> = doc
        .text_edit
        .as_ref()
        .and_then(|s| {
            // The click has not been applied to `state` yet this frame, so
            // use the caret the click produced where there was one.
            let caret = click_result.map_or(s.caret, |(c, _)| c)?;
            let key = (caret.run, s.prop_bold, s.prop_italic);
            (s.style_preview_key != Some(key)).then_some((s, key))
        })
        .map(|(s, key)| {
            let want = StyleSynthesis::new(s.prop_bold, s.prop_italic);
            if want.is_none() {
                // Nothing ticked: no core call at all, and the cache key is
                // still recorded so this branch is taken once, not per frame.
                return (None, key);
            }
            let resolved = caret_run_text.as_ref().map(|(_, text)| {
                doc.session
                    .preview_style_resolution(page_index, text, pinned_span, want)
                    .map_err(|e| e.to_string())
            });
            (resolved, key)
        });

    if let Some(state) = doc.text_edit.as_mut() {
        if let Some((resolved, key)) = style_preview_refresh {
            state.style_preview = resolved;
            state.style_preview_key = Some(key);
        }
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

        // Pass 19.3: refresh the caret run's ambient text state and, on a move
        // to a different run, re-seed the spacing/baseline fields from it.
        // Placed AFTER the click and the arrow navigation have both landed, so
        // the panel always describes the caret the operator can see.
        seed_spacing_props(state);

        // Property bar (§7): a floating top panel, appearing with the tool.
        egui::Area::new(egui::Id::new("pdfce-text-edit-propbar"))
            .order(egui::Order::Foreground)
            .movable(true)
            .default_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::TopLeft,
                8.0,
            ))
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
                        // ---- Pass 19.3: the spacing & style rows ----
                        //
                        // Collapsed by default, for the same reason Pass 18.4's
                        // status readout put its detail behind one: Size/Colour/
                        // Font are always relevant, these are occasionally
                        // relevant, and permanently growing the panel for the
                        // occasional ones is the ribbon-overload failure mode.
                        // `CollapsingHeader`'s open state is egui `Id`-keyed and
                        // persists across frames with no state of ours.
                        egui::CollapsingHeader::new(ui_text::format_spacing_section_title())
                            .id_salt("pdfce-te-spacing")
                            .show(ui, |ui| {
                                // The caret run's ambient state (Copy), read once.
                                // `None` means "no provenance for this run", which
                                // is said out loud rather than shown as a zero.
                                let amb = state.prop_ambient;

                                // -- Row 1: character spacing (Tc) --
                                ui.label(ui_text::format_char_spacing_label())
                                    .on_hover_text(ui_text::format_char_spacing_tooltip());
                                ui.label(match amb {
                                    Some(a) => {
                                        let mut c = ui_text::format_ambient_caption(
                                            &ui_text::format_ambient_char_spacing_value(
                                                a.per_mille(a.char_spacing),
                                                a.char_spacing,
                                            ),
                                        );
                                        if a.tc_at_default {
                                            c.push_str(ui_text::format_ambient_default_suffix());
                                        }
                                        c
                                    }
                                    None => no_ambient_caption(state.caret.is_some()).to_owned(),
                                });
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut state.prop_char_spacing)
                                            .speed(0.05),
                                    );
                                    // Switching the unit RE-DERIVES the number so
                                    // it keeps meaning what its visible unit says
                                    // — it never silently reinterprets the digits
                                    // already in the box under a new meaning.
                                    if let Some(unit) = unit_toggle(ui, state.prop_tc_unit)
                                        && unit != state.prop_tc_unit
                                    {
                                        if let Some(a) = amb {
                                            state.prop_char_spacing = match unit {
                                                MetricUnit::Absolute => {
                                                    a.per_mille_to_operand(state.prop_char_spacing)
                                                }
                                                MetricUnit::Relative => {
                                                    a.per_mille(state.prop_char_spacing)
                                                }
                                            };
                                        }
                                        state.prop_tc_unit = unit;
                                    }
                                    if ui.button(ui_text::format_apply_char_spacing()).clicked() {
                                        apply_char_spacing = Some(metric_spec(
                                            state.prop_tc_unit,
                                            state.prop_char_spacing,
                                        ));
                                    }
                                });

                                // -- Row 2: horizontal scaling (Tz) --
                                ui.label(ui_text::format_h_scale_label())
                                    .on_hover_text(ui_text::format_h_scale_tooltip());
                                ui.label(match amb {
                                    Some(a) => {
                                        let mut c = ui_text::format_ambient_caption(
                                            &ui_text::format_ambient_h_scale_value(a.h_scale),
                                        );
                                        if a.tz_at_default {
                                            c.push_str(ui_text::format_ambient_default_suffix());
                                        }
                                        c
                                    }
                                    None => no_ambient_caption(state.caret.is_some()).to_owned(),
                                });
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut state.prop_h_scale)
                                            .range(1.0..=1000.0)
                                            .speed(0.5)
                                            .suffix(ui_text::percent_suffix()),
                                    );
                                    if ui.button(ui_text::format_apply_h_scale()).clicked() {
                                        apply_h_scale = Some(state.prop_h_scale);
                                    }
                                });

                                // -- Row 3: baseline — ONE control, four
                                // positions, exactly one live. This is where the
                                // super/subscript-vs-free-rise mutual exclusion
                                // lives: both write `Ts`, and core refuses a
                                // request carrying both (`ConflictingRise`), so
                                // the UI is built so that request cannot be
                                // spelled rather than catching it at runtime.
                                ui.label(ui_text::format_baseline_label())
                                    .on_hover_text(ui_text::format_baseline_tooltip());
                                ui.label(match amb {
                                    Some(a) if a.rise.abs() < f64::EPSILON => {
                                        let mut c = ui_text::format_ambient_caption(
                                            ui_text::format_ambient_baseline_normal(),
                                        );
                                        if a.rise_at_default {
                                            c.push_str(ui_text::format_ambient_default_suffix());
                                        }
                                        c
                                    }
                                    Some(a) => ui_text::format_ambient_caption(
                                        &ui_text::format_ambient_baseline_value(
                                            a.rise,
                                            a.per_mille(a.rise),
                                        ),
                                    ),
                                    None => no_ambient_caption(state.caret.is_some()).to_owned(),
                                });
                                ui.horizontal(|ui| {
                                    for (choice, label) in [
                                        (BaselineChoice::Normal, ui_text::format_baseline_normal()),
                                        (
                                            BaselineChoice::Superscript,
                                            ui_text::format_baseline_superscript(),
                                        ),
                                        (
                                            BaselineChoice::Subscript,
                                            ui_text::format_baseline_subscript(),
                                        ),
                                        (BaselineChoice::Custom, ui_text::format_baseline_custom()),
                                    ] {
                                        let sel = state.prop_baseline == choice;
                                        // R84: NOT a bare `selectable_value` —
                                        // egui's only built-in selected signal is
                                        // a background fill, i.e. colour alone.
                                        // Pairing with `toggle_label` makes the
                                        // live option BOLD as well.
                                        if ui
                                            .selectable_label(
                                                sel,
                                                PdfceApp::toggle_label(sel, label),
                                            )
                                            .clicked()
                                        {
                                            state.prop_baseline = choice;
                                        }
                                    }
                                });
                                // The free-form field is HIDDEN, not disabled,
                                // for the other three positions: there is no
                                // capability to combine a script position with a
                                // custom rise, so R83 says draw no control that
                                // implies there is.
                                if state.prop_baseline == BaselineChoice::Custom {
                                    ui.horizontal(|ui| {
                                        ui.label(ui_text::format_rise_label());
                                        ui.add(
                                            egui::DragValue::new(&mut state.prop_rise).speed(0.1),
                                        );
                                        if let Some(unit) = unit_toggle(ui, state.prop_rise_unit)
                                            && unit != state.prop_rise_unit
                                        {
                                            if let Some(a) = amb {
                                                state.prop_rise = match unit {
                                                    MetricUnit::Absolute => {
                                                        a.per_mille_to_operand(state.prop_rise)
                                                    }
                                                    MetricUnit::Relative => {
                                                        a.per_mille(state.prop_rise)
                                                    }
                                                };
                                            }
                                            state.prop_rise_unit = unit;
                                        }
                                    });
                                }
                                if ui.button(ui_text::format_apply_baseline()).clicked() {
                                    // EITHER a script position OR a free rise —
                                    // never both, by construction.
                                    match state.prop_baseline {
                                        BaselineChoice::Normal => {
                                            apply_script = Some(ScriptPosition::Normal);
                                        }
                                        BaselineChoice::Superscript => {
                                            apply_script = Some(ScriptPosition::Superscript);
                                        }
                                        BaselineChoice::Subscript => {
                                            apply_script = Some(ScriptPosition::Subscript);
                                        }
                                        BaselineChoice::Custom => {
                                            apply_rise = Some(metric_spec(
                                                state.prop_rise_unit,
                                                state.prop_rise,
                                            ));
                                        }
                                    }
                                }

                                // -- Row 4: synthetic bold / italic (R90) --
                                ui.label(ui_text::format_style_label())
                                    .on_hover_text(ui_text::format_style_tooltip());
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut state.prop_bold, ui_text::format_style_bold());
                                    ui.checkbox(
                                        &mut state.prop_italic,
                                        ui_text::format_style_italic(),
                                    );
                                });
                                // The pre-Apply resolution, from the core query
                                // computed at the top of this frame. When the
                                // checkboxes just changed, the cached answer is
                                // for the previous combination — so ask for one
                                // more frame rather than showing a stale caption.
                                let key_now = state
                                    .caret
                                    .map(|c| (c.run, state.prop_bold, state.prop_italic));
                                let fresh = key_now.is_some() && state.style_preview_key == key_now;
                                let row = if fresh {
                                    match state.style_preview.as_ref() {
                                        Some(Ok(res)) => style_row_text(res),
                                        // The query could not answer. Say so,
                                        // in the same ✖ shape as any refusal,
                                        // rather than rendering nothing and
                                        // letting Apply look unremarkable.
                                        Some(Err(e)) => {
                                            Some((ui_text::refusal_line(e), Some(e.clone())))
                                        }
                                        None => None,
                                    }
                                } else {
                                    ui.ctx().request_repaint();
                                    None
                                };
                                if let Some((caption, _)) = row.as_ref() {
                                    ui.label(caption);
                                }
                                if ui.button(ui_text::format_apply_style()).clicked() {
                                    let want =
                                        StyleSynthesis::new(state.prop_bold, state.prop_italic);
                                    match row.as_ref().and_then(|(_, r)| r.clone()) {
                                        // A combination pdfce refuses locally
                                        // (a real face exists for one axis but
                                        // not the other). Refuse BY NAME instead
                                        // of submitting something core would
                                        // accept but that would discard a real
                                        // face the operator can have.
                                        Some(refusal) => local_refusal = Some(refusal),
                                        None if !want.is_none() => {
                                            apply_synthetic = Some(want);
                                        }
                                        None => {}
                                    }
                                }
                                // -- Row 5: word spacing (Tw) — LIVE on a simple
                                // font, a read-only disclosure on a composite one
                                // (Pass 19.4).
                                //
                                // This is the one row in the panel whose SHAPE
                                // depends on the run, and R83 is why: §9.3.3 makes
                                // Tw void for multi-byte codes, so on a composite
                                // run there is no capability and therefore no
                                // affordance — not even a greyed-out spinner,
                                // which is still an affordance. The value is shown
                                // either way (an inert number with no explanation
                                // invites "this looks broken") and the reason is
                                // stated by name.
                                //
                                // The gate reads `AmbientSnapshot::composite`,
                                // which comes straight from
                                // `GlyphProvenance::composite` — the SAME
                                // `ExtractFont::is_simple` answer core's own R91
                                // refusal uses. Nothing about font models is
                                // re-derived here (R74), so the affordance the GUI
                                // draws and the request core accepts cannot
                                // disagree.
                                ui.separator();
                                ui.label(ui_text::format_word_spacing_label())
                                    .on_hover_text(ui_text::format_word_spacing_tooltip());
                                match amb {
                                    None => {
                                        ui.label(no_ambient_caption(state.caret.is_some()));
                                    }
                                    Some(a) if a.composite => {
                                        ui.colored_label(
                                            ui.visuals().weak_text_color(),
                                            ui_text::format_word_spacing_readonly(a.word_spacing),
                                        );
                                        ui.label(
                                            ui_text::format_word_spacing_explanation_composite(),
                                        );
                                    }
                                    Some(a) => {
                                        let mut c = ui_text::format_ambient_caption(
                                            &ui_text::format_ambient_word_spacing_value(
                                                a.per_mille(a.word_spacing),
                                                a.word_spacing,
                                            ),
                                        );
                                        if a.tw_at_default {
                                            c.push_str(ui_text::format_ambient_default_suffix());
                                        }
                                        ui.label(c);
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut state.prop_word_spacing)
                                                    .speed(0.5),
                                            );
                                            // Same re-derive-on-unit-switch rule as
                                            // the tracking row: the digits in the
                                            // box keep meaning what their visible
                                            // unit says.
                                            if let Some(unit) = unit_toggle(ui, state.prop_tw_unit)
                                                && unit != state.prop_tw_unit
                                            {
                                                state.prop_word_spacing = match unit {
                                                    MetricUnit::Absolute => a.per_mille_to_operand(
                                                        state.prop_word_spacing,
                                                    ),
                                                    MetricUnit::Relative => {
                                                        a.per_mille(state.prop_word_spacing)
                                                    }
                                                };
                                                state.prop_tw_unit = unit;
                                            }
                                            if ui
                                                .button(ui_text::format_apply_word_spacing())
                                                .clicked()
                                            {
                                                apply_word_spacing = Some(metric_spec(
                                                    state.prop_tw_unit,
                                                    state.prop_word_spacing,
                                                ));
                                            }
                                        });
                                    }
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
            .fixed_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::BottomLeft,
                8.0,
            ))
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
                                .add(
                                    egui::Button::new(ui_text::accept_edit())
                                        .min_size(ICON_BUTTON_SIZE),
                                )
                                .clicked()
                            {
                                do_accept = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(ui_text::reject_edit())
                                        .min_size(ICON_BUTTON_SIZE),
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
                                .add(
                                    egui::Button::new(ui_text::reflow_accept())
                                        .min_size(ICON_BUTTON_SIZE),
                                )
                                .clicked()
                            {
                                do_accept_reflow = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(ui_text::reflow_reject())
                                        .min_size(ICON_BUTTON_SIZE),
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
    // Pass 19.3: a refusal pdfce decided WITHOUT calling core (the mixed
    // real-face/synthesis request) lands in the same strip as every core
    // refusal — one disclosure surface, not two.
    if let Some(msg) = local_refusal
        && let Some(state) = doc.text_edit.as_mut()
    {
        state.last_refusal = Some(msg);
        return;
    }
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
            FormatOp::CharSpacing(spec) => r.char_spacing(spec),
            FormatOp::WordSpacing(spec) => r.word_spacing(spec),
            FormatOp::HScale(pct) => r.h_scale(pct),
            FormatOp::Script(pos) => r.script(pos),
            FormatOp::Rise(spec) => r.rise(spec),
            FormatOp::Synthetic(s) => r.synthetic(s),
        })
    };
    // One Apply per frame — the operator clicked one button. The order below
    // is the panel's own top-to-bottom order and is never reached with two
    // intents set, because two buttons cannot be clicked in one frame.
    let chosen = if let Some(pt) = apply_size {
        Some(FormatOp::Size(pt))
    } else if let Some(f) = apply_fill {
        Some(FormatOp::Fill(f))
    } else if let Some(sel) = apply_font {
        Some(FormatOp::Font(sel))
    } else if let Some(spec) = apply_char_spacing {
        Some(FormatOp::CharSpacing(spec))
    } else if let Some(spec) = apply_word_spacing {
        Some(FormatOp::WordSpacing(spec))
    } else if let Some(pct) = apply_h_scale {
        Some(FormatOp::HScale(pct))
    } else if let Some(pos) = apply_script {
        Some(FormatOp::Script(pos))
    } else if let Some(spec) = apply_rise {
        Some(FormatOp::Rise(spec))
    } else {
        apply_synthetic.map(FormatOp::Synthetic)
    };
    if let Some(op) = chosen
        && let Some(req) = format_req(op)
    {
        match doc.session.format_text(&req, &FormatOptions::default()) {
            Ok(report) => {
                doc.refresh_pages();
                doc.build_text_edit_state();
                if let Some(state) = doc.text_edit.as_mut() {
                    // Core's own sentences, verbatim (§8.1) — plus, when a
                    // justified line's slack was invalidated, one GUI sentence
                    // pointing at the Reflow control that is already three
                    // rows below in this same panel. Core says WHY (and names
                    // the width delta, per decision 019 Amendment B.1); the
                    // GUI adds only WHAT TO DO, which is the half core cannot
                    // know because it does not know a panel is open.
                    let justify = report.justify_slack_invalidated;
                    state.last_disclosures = report.disclosures;
                    if justify {
                        state
                            .last_disclosures
                            .push(ui_text::format_justify_invalidated_hint().to_owned());
                    }
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
        if canvas::primary_drag_started(image_response) {
            state.draft = None;
            state.drag_anchor = image_response.interact_pointer_pos();
        } else if canvas::primary_drag_stopped(image_response) {
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
        if canvas::primary_dragged(image_response)
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
            .movable(true)
            .default_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::TopLeft,
                8.0,
            ))
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
                            .add(
                                egui::Button::new(ui_text::add_text_place_box_button())
                                    .min_size(ICON_BUTTON_SIZE),
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
                        .add(
                            egui::Button::new(ui_text::add_text_place_point_button())
                                .min_size(ICON_BUTTON_SIZE),
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
            .fixed_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::BottomLeft,
                8.0,
            ))
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
///
/// One variant per Apply button, deliberately: the panel keeps the shipped
/// one-control-family-per-commit granularity, so each accepted change is one
/// undo step and one disclosure set. Note that [`Self::Script`] and
/// [`Self::Rise`] are separate variants — that is the type-level half of the
/// baseline control's mutual exclusion (§9.3.7: both write `Ts`, and asking
/// for both is `FormatError::ConflictingRise`). There is no way to spell
/// "both" here.
enum FormatOp {
    Size(f64),
    Fill(pdfce_core::text_edit::NewFill),
    Font(String),
    /// `Tc`, in whichever unit the operator's toggle named.
    CharSpacing(pdfce_core::text_edit::MetricSpec),
    /// `Tw`, in whichever unit the operator's toggle named (Pass 19.4).
    /// Only ever built on a simple-font run — the row draws no Apply button
    /// on a composite one (R83), and core refuses one anyway (R91).
    WordSpacing(pdfce_core::text_edit::MetricSpec),
    /// `Tz`, percent.
    HScale(f64),
    /// The coarse super/subscript toggle (or `Normal`, which flattens).
    Script(pdfce_core::text_edit::ScriptPosition),
    /// A free-form `Ts`.
    Rise(pdfce_core::text_edit::MetricSpec),
    /// Synthetic bold and/or italic (R90).
    Synthetic(pdfce_core::text_edit::StyleSynthesis),
}

/// The caption for a row that has no ambient value to show — and the reason
/// it has none, which is not always the same reason.
///
/// With no caret there is simply nothing selected yet (the ordinary state at
/// tool entry, and again immediately after an accepted change, which rebuilds
/// the page model and clears the caret). With a caret but no provenance,
/// pdfce genuinely could not read the run's state. Reporting the first as the
/// second would claim a limitation pdfce does not have.
fn no_ambient_caption(has_caret: bool) -> &'static str {
    if has_caret {
        ui_text::format_ambient_unknown()
    } else {
        ui_text::format_ambient_no_caret()
    }
}

/// Draw the two-way absolute/relative unit selector and report the operator's
/// pick.
///
/// # Why this is not a `selectable_value` pair
///
/// Standing rule R84: a selection's state is never carried by colour alone.
/// egui's `selectable_value` signals "selected" with a background fill and
/// nothing else, which fails that outright for anyone who cannot separate the
/// fill from the surround. Pairing `selectable_label` with
/// [`PdfceApp::toggle_label`] adds **weight** — the live option is bold — so
/// there are two independent cues.
///
/// Two rows in this same panel (the reflow alignment picker, the colour-model
/// picker) still use the bare form. They predate R84 and are grandfathered;
/// R84 explicitly binds new selection surfaces, which is what this is.
///
/// Returns the clicked unit, or `None` if neither was clicked this frame.
fn unit_toggle(ui: &mut egui::Ui, current: MetricUnit) -> Option<MetricUnit> {
    let mut picked = None;
    for (unit, label, tip) in [
        (
            MetricUnit::Relative,
            ui_text::format_unit_relative(),
            ui_text::format_unit_relative_tooltip(),
        ),
        (
            MetricUnit::Absolute,
            ui_text::format_unit_absolute(),
            ui_text::format_unit_absolute_tooltip(),
        ),
    ] {
        let sel = current == unit;
        if ui
            .selectable_label(sel, PdfceApp::toggle_label(sel, label))
            .on_hover_text(tip)
            .clicked()
        {
            picked = Some(unit);
        }
    }
    picked
}

/// Build the core `MetricSpec` the operator's unit choice names.
///
/// The single place the GUI's unit tag becomes core's discriminated one, so
/// the mapping cannot drift between the two call sites (tracking and rise).
fn metric_spec(unit: MetricUnit, value: f64) -> pdfce_core::text_edit::MetricSpec {
    use pdfce_core::text_edit::MetricSpec;
    match unit {
        MetricUnit::Absolute => MetricSpec::Absolute(value),
        MetricUnit::Relative => MetricSpec::Relative(value),
    }
}

/// Turn the core's read-only [`StyleResolution`] into the pair of strings the
/// style row needs: the live caption shown **before** Apply, and — when the
/// combination is one pdfce refuses locally — the refusal to record if the
/// operator clicks Apply anyway.
///
/// # Why the refusal is built here and not in `pdfce-core`
///
/// The *decision* is core's: `StyleResolution::is_mixed()` and the per-axis
/// probes come from `gate_synthesis` itself, so no matching rule is
/// re-derived in the GUI (R74). What is local is only the **policy** of not
/// submitting a request core would accept — pdfce's core will happily
/// synthesize both axes, which is exactly the behaviour that would silently
/// pass over an available real face. Declining to ask is a UI choice; knowing
/// there is something to decline is not.
///
/// Returns `None` when nothing is ticked (no caption, no refusal).
fn style_row_text(
    res: &pdfce_core::text_edit::StyleResolution,
) -> Option<(String, Option<String>)> {
    use pdfce_core::text_edit::{StyleOutcome, StyleSynthesis};
    let combined = res.combined.as_ref()?;
    // The PLAIN style name, not `StyleSynthesis::label()` — that one reads
    // "synthetic bold", which would produce "a real synthetic bold face".
    let style = match res.want {
        StyleSynthesis::Bold => ui_text::format_style_bold(),
        StyleSynthesis::Italic => ui_text::format_style_italic(),
        _ => ui_text::format_style_bold_italic(),
    };
    // A real face covering the WHOLE request: core refuses, and the caption
    // says so plainly rather than promising a font switch pdfce will not do.
    if let StyleOutcome::RealFaceResolves {
        real_font,
        resource,
    } = combined
    {
        return Some((
            ui_text::format_style_preview_real_face(style, real_font, resource),
            None,
        ));
    }
    // No single face covers the request. Two sub-cases, and the difference
    // matters to the operator's next action.
    let bold_real = match res.bold_axis.as_ref() {
        Some(StyleOutcome::RealFaceResolves {
            real_font,
            resource,
        }) => Some((real_font.as_str(), resource.as_str())),
        _ => None,
    };
    let italic_real = match res.italic_axis.as_ref() {
        Some(StyleOutcome::RealFaceResolves {
            real_font,
            resource,
        }) => Some((real_font.as_str(), resource.as_str())),
        _ => None,
    };
    match (bold_real, italic_real) {
        (Some((bf, br)), Some((itf, itr))) => Some((
            ui_text::format_style_preview_both_real(bf, br, itf, itr),
            Some(ui_text::format_style_both_real_refusal(bf, br, itf, itr)),
        )),
        (Some((f, r)), None) => Some((
            ui_text::format_style_preview_mixed(
                ui_text::format_style_bold(),
                f,
                r,
                ui_text::format_style_italic(),
            ),
            Some(ui_text::format_style_mixed_refusal(
                ui_text::format_style_bold(),
                f,
                r,
                ui_text::format_style_italic(),
            )),
        )),
        (None, Some((f, r))) => Some((
            ui_text::format_style_preview_mixed(
                ui_text::format_style_italic(),
                f,
                r,
                ui_text::format_style_bold(),
            ),
            Some(ui_text::format_style_mixed_refusal(
                ui_text::format_style_italic(),
                f,
                r,
                ui_text::format_style_bold(),
            )),
        )),
        // The ordinary case: nothing real anywhere, so synthesis is the
        // genuine fallback it is meant to be.
        (None, None) => Some((ui_text::format_style_preview_synthesize(style), None)),
    }
}

/// Refresh the caret run's ambient snapshot, and re-seed the spacing/style
/// fields whenever the caret has landed on a **different** run.
///
/// # Why this exists at all (the gap this Pass closes)
///
/// `TextEditState::prop_size`/`prop_model`/`prop_font` are seeded once, to a
/// fixed default, and never re-seeded when the caret moves. For those three
/// that is tolerable — the operator can see that the glyphs are 12 pt, are
/// that colour, are that face — so the panel can get away with meaning "what
/// to apply next" rather than "what is true now".
///
/// **That tolerance does not transfer to `Tc`/`Tz`/`Ts`.** A `Tc` of 0.24 is
/// invisible at reading zoom; a `Tz` of 95% is barely perceptible; a small
/// `Ts` is subtle by design. A panel seeded from a blind default would show
/// `0` beside a run carrying `0.24`, i.e. it would state something false
/// about the document, and an operator who then clicked Apply would silently
/// stomp a value they were never shown. That is a rule-4 failure specific to
/// this control family.
///
/// # Why it keys on the RUN and not on the caret
///
/// Ambient text state is a property of the run, so a caret moving within one
/// run must NOT re-seed — that would fight an operator part-way through
/// typing a number. Moving to a new run re-seeds, because the panel is now
/// describing something else.
///
/// # Ordering note (immediate mode)
///
/// Called in phase B **after** the click/arrow navigation has been applied,
/// so the snapshot always describes the caret the operator can see rather
/// than the one phase A rendered. A caret that carries no provenance (a
/// derived-whitespace run, or an extraction without provenance capture)
/// clears the snapshot rather than leaving a stale one on screen.
fn seed_spacing_props(state: &mut TextEditState) {
    let Some(caret) = state.caret else {
        state.prop_ambient = None;
        state.props_seeded_for = None;
        return;
    };
    let ambient = state
        .page_text
        .runs
        .get(caret.run)
        .and_then(|r| r.glyphs.first())
        .and_then(|g| g.provenance.as_ref())
        .map(AmbientSnapshot::from_provenance);
    state.prop_ambient = ambient;

    let Some(a) = ambient else {
        // No provenance to seed from. Leave the fields alone and clear the
        // key so the next run WITH provenance re-seeds; the captions render
        // as "unknown" rather than as a confident zero.
        state.props_seeded_for = None;
        return;
    };
    if state.props_seeded_for == Some(caret.run) {
        return;
    }
    state.props_seeded_for = Some(caret.run);

    state.prop_char_spacing = match state.prop_tc_unit {
        MetricUnit::Absolute => a.char_spacing,
        MetricUnit::Relative => a.per_mille(a.char_spacing),
    };
    // Pass 19.4: seeded from the file's own ambient `Tw` on EVERY run,
    // including a composite one whose row is read-only. The value is what
    // is in force either way, and a panel that showed 0 beside a run
    // carrying 2 would be stating a falsehood about the document — which
    // is the whole reason `prop_ambient` exists.
    state.prop_word_spacing = match state.prop_tw_unit {
        MetricUnit::Absolute => a.word_spacing,
        MetricUnit::Relative => a.per_mille(a.word_spacing),
    };
    state.prop_h_scale = a.h_scale;
    // A non-zero ambient rise seeds the CUSTOM position and its number, not a
    // guessed superscript. pdfce cannot tell "the producer applied a
    // superscript" from "the producer applied a rise of 4.08" — the bytes are
    // the same — so it shows the number it can prove instead of inferring an
    // intent it cannot.
    if a.rise.abs() < f64::EPSILON {
        state.prop_baseline = BaselineChoice::Normal;
        state.prop_rise = 0.0;
    } else {
        state.prop_baseline = BaselineChoice::Custom;
        state.prop_rise = match state.prop_rise_unit {
            MetricUnit::Absolute => a.rise,
            MetricUnit::Relative => a.per_mille(a.rise),
        };
    }
    // The style checkboxes are a REQUEST, not a reading of the run, so they
    // reset to "asking for nothing" on a new run rather than claiming to
    // report the run's weight. (Detecting an existing synthesis is
    // `synth::detect`'s job and a separate, unbuilt badge — §9 of the
    // ui-spec, P1.)
    state.prop_bold = false;
    state.prop_italic = false;
    state.style_preview = None;
    state.style_preview_key = None;
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
/// The status bar's **selection readout** — the line that says what is
/// selected, and why a selection box may be sitting over apparently-empty
/// paper (ui-spec §C.5/§C.6).
///
/// # Why this line exists
///
/// The operator's report was *"sometimes I click and get a box highlighting on
/// the screen that doesn't seem to correspond to anything."* Three separate
/// causes of that turned out to be real bugs and are fixed (a zoom-inverted
/// catch radius, an object-edit tool that drew no outline at all, and a
/// page-centring coordinate offset). What was left is the half no bug fix
/// reaches: a selection can be entirely CORRECT and still enclose blank paper
/// — because the object is a text bbox inflated around glyph origins, or a
/// clip path that paints nothing, or a zero-height rule. Before this line the
/// app had no way whatsoever to tell the operator which of those it was.
///
/// # Why the STATUS BAR and not only the dock
///
/// The dock is not open by default, and the canvas is where the confusion
/// happens. A readout that requires first discovering a panel is not a readout
/// for the moment the question is asked. The status bar is always on screen,
/// is already this app's narrator channel (`status_bar_body`'s own doc
/// comment), and — unlike the canvas raster, which remains
/// screen-reader-illegible — is made of real text widgets. That last point is
/// the accessibility argument as well as the discoverability one: routing the
/// facts through text here NARROWS the practical impact of the canvas's
/// AccessKit gap rather than widening it.
///
/// # Contract
///
/// - Nothing selected → **no line at all.** The status bar's other lines are
///   non-suppressible disclosures about the document; this one is about a
///   transient the operator created, and an ever-present "Selected: nothing"
///   would be noise crowding out disclosures that matter.
/// - Exactly one object, resolvable → **one headline line**
///   ([`ui_text::selection_readout_single`]), which already carries the SHORT
///   form of the leading disclosure, plus — when any disclosure applies — a
///   collapsing expander holding ONE full sentence per applicable
///   [`object_summary::ObjectNote`]. Those sentences are the deliverable; the
///   expander is what keeps them from costing the canvas four lines of height
///   every time something is selected (see
///   [`PdfceApp::selection_notes_expanded`] for why that matters more than it
///   sounds). An object with nothing to explain gets a plain label, never an
///   expander that opens onto nothing.
/// - More than one, all resolvable → a per-kind census
///   ([`ui_text::selection_readout_multi`]). Orientation, not detail; the
///   Objects panel's highlighted rows are where per-object detail lives.
/// - Any target the current page model cannot resolve → the honest
///   ([`ui_text::selection_readout_unresolved`]) line for the WHOLE selection
///   rather than a partial census, because a census that silently omits the
///   unresolvable ones would print a count that disagrees with the number of
///   boxes on screen. This is a one-frame transient after an edit, before
///   `prune_canvas_selection` runs.
///
/// Every description comes from `object_summary::describe_object` — the same
/// call the Objects panel's rows and the canvas type badges make, so the three
/// surfaces cannot drift into describing one object three ways.
fn selection_readout(doc: &OpenDoc, ui: &mut egui::Ui, expanded: &mut bool) {
    // Being INSIDE an object is a selection state in its own right and gets
    // said out loud. Two things need stating that an outline cannot: that the
    // operator is at the second level at all (so the different-coloured box is
    // explained rather than merely noticed), and how many parts the object has
    // — which is the number that explains why clicking a line selected an
    // entire view in the first place.
    if let Some(entered) = doc.entered {
        let parts = doc.object_model.as_ref().and_then(|p| {
            match p.page_objects().objects.get(entered.object) {
                Some(pdfce_core::vector::VectorObject::Path(path)) => Some(path.subpaths.len()),
                _ => None,
            }
        });
        ui.label(ui_text::entered_object_readout(
            entered.object,
            entered.subpath,
            parts,
        ))
        .on_hover_text(ui_text::entered_object_tooltip());
    }

    let selected = doc.canvas_selection.len();
    if selected == 0 {
        return;
    }
    let summaries: Vec<ObjectSummary> = doc
        .object_model
        .as_ref()
        .map(|provider| {
            let objects = &provider.page_objects().objects;
            doc.canvas_selection
                .iter()
                .filter_map(|target| objects.get(usize::try_from(target.0).ok()?))
                .map(describe_object)
                .collect()
        })
        .unwrap_or_default();

    if summaries.len() != selected {
        ui.label(ui_text::selection_readout_unresolved(selected))
            .on_hover_text(ui_text::selection_readout_tooltip());
        return;
    }
    let Some([only]) = (summaries.len() == 1).then_some(summaries.as_slice()) else {
        ui.label(ui_text::selection_readout_multi(census(
            summaries.iter().map(|s| s.kind),
        )))
        .on_hover_text(ui_text::selection_readout_tooltip());
        return;
    };

    // The click-through disclosure (ui-spec §C.3 / rule 4): "2 of 3 at this
    // point". Guarded by `describes` so it is shown only while the recorded
    // cycle is still the live explanation of what is selected — after a tree
    // click, a marquee or an edit it is not, and a stale "2 of 3" would be
    // worse than none.
    let cycle = doc
        .click_cycle
        .filter(|c| c.describes(doc.view.page_index, &doc.canvas_selection))
        .map(|c| (c.position(), c.total));
    let headline = ui_text::selection_readout_single(only, cycle);
    if only.notes.is_empty() {
        // Nothing needs explaining: an ordinary, visible object. A collapsing
        // header with an empty body would be an affordance that leads nowhere,
        // which R83 forbids as firmly as a capability with no affordance.
        ui.label(headline)
            .on_hover_text(ui_text::selection_readout_tooltip());
        return;
    }
    // Same shape as the render-diagnostics header directly below it — a
    // one-line summary with an expander — because it is the same kind of
    // thing: a headline the operator always sees, and a detailed disclosure
    // they open when the headline is not enough. Reusing that pattern rather
    // than inventing a second one means an operator who has learned the
    // diagnostics expander has already learned this.
    let response = egui::CollapsingHeader::new(headline)
        .id_salt("selection-readout")
        .open(Some(*expanded))
        .show(ui, |ui| {
            // One sentence per disclosure, each its own label so it wraps as
            // a paragraph rather than running into the line above. These are
            // explanations, not warnings, so they take the narrator's ordinary
            // colour: an operator who selected a clip path did nothing wrong.
            for note in &only.notes {
                ui.label(ui_text::object_note(*note));
            }
        });
    response
        .header_response
        .clone()
        .on_hover_text(ui_text::selection_readout_tooltip());
    if response.header_response.clicked() {
        *expanded = !*expanded;
    }
}

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

// ===================================================================
// Pass 12.M2b — the on-canvas dimension-authoring gesture handlers
// (`docs/ui_specs/pass-12.M2-dimension-tools.md`, decision 011 §2.3/§2.4)
// ===================================================================
//
// These wire the ALREADY-SHIPPED `pdfce-core::dimension` engine (12.M2) +
// snapping (12.M1) to the canvas. The testable authoring-state logic lives in
// `measure_tool` (headless unit tests); these handlers are the thin,
// compile-and-launch-only egui frame that drives it: the snap query + indicator
// (12.M1), the live preview, the property/status bars, and the Phase-C commit
// through the SAME `EditSession::{add_dimension, set_group_scale}` path the CLI
// uses — so a canvas-authored dimension is byte-identical to `dimension-add`
// for the same picks (measure_tool's equivalence tests pin the shared `kind`).

/// The measure tools' per-frame handler (ui-spec §§2–4): resolve the snapped
/// pick, draw the indicator + live preview, render the property/status bars,
/// and commit an accepted dimension / scale as ONE undoable `EditSession`
/// command. A free function (like `run_add_text_tool`) so `doc.pages`
/// (immutable, coordinate bridge), `doc.object_model` (immutable, the ONE
/// decomposition — snap + fit), and `doc.measure` (mutable, tool state) are
/// borrowed as disjoint fields across the frame; `doc.session`/`refresh_pages`
/// are touched only in the Phase-C commit, after those borrows drop.
#[allow(
    clippy::too_many_lines,
    reason = "one tool family = one handler; the pick/preview/propbar/status/commit phases are tightly coupled and splitting them would need shared owned scratch structs that obscure more than they clarify" // ui-text-exempt: clippy lint justification, never displayed
)]
/// Pass 9c-min (decision 011 §2.5): the on-canvas object-edit gesture.
///
/// Click selects the object under the pointer (reusing the 9a hit-test);
/// **dragging** a selected object translates it, and dragging its anchor
/// relocates that node (a node grab is decided by
/// [`vector_edit_tool::classify_drag`], snapped via the 12.M1 engine), each
/// showing a live preview before it commits on release to one undoable
/// `EditSession::{move_object, move_node}` command. Delete is routed
/// separately (`delete_selected_object`) from the `DeleteSelection` action.
///
/// GUI glue only: every geometry decision is a headless-tested
/// `vector_edit_tool`/`canvas`/`vector` helper; the surgery is `pdfce-core`.
///
/// **Known limitation (documented, not a defect):** after one committed
/// vector edit on a page, the core refuses a second same-session edit with
/// `VectorEditNeedsReopen` (rule 4) — save and reopen to continue editing
/// that page. The correctness of each individual edit is proven headlessly
/// in `pdfce-core`/`pdfce-render`.
///
/// ## Corrected at Pass 17.0 (decision 018)
///
/// The stated *reason* used to be *"the object provider is rebuilt from the
/// base document (`session.document()`), so after one committed vector edit
/// the base-relative object indices no longer match the session-current
/// content."* Half of that is no longer true and the other half moved:
///
/// - The provider is now rebuilt from **`session.view()`**
///   (`OpenDoc::ensure_object_provider`), so its indices describe the
///   content the operator is actually looking at.
/// - The refusal now comes entirely from the core:
///   `EditSession::vector_surgery` decomposes the **base** on purpose — so
///   that `object_index` means the same thing to every caller — and
///   therefore refuses any page whose first `/Contents` object this session
///   has already rewritten, rather than risk misindexing it.
///
/// The two agree on every page that has *not* been rewritten this session
/// (base content and session content are the same bytes there), and on a
/// page that *has* been, the edit is refused before any index is used. So
/// the limitation is unchanged in effect, and is now a deliberate core
/// refusal rather than a GUI read-path accident. Lifting it is a scoped
/// core change (session-relative object indices), not a GUI one.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
/// Stroke a 2px outline around every currently-selected canvas object.
///
/// # Why this is a shared function and not inline code
///
/// It used to be inline in the plain-selection branch of `canvas`, which meant
/// **the object-edit tool drew no selection feedback at all**: that branch is
/// an `else`, and `run_vector_edit_tool` returns long before it. Clicking an
/// object with the Obj tool selected it, armed Delete, and armed drag-to-move —
/// while showing the operator absolutely nothing.
///
/// That is the second half of the operator's 2026-08-02 report, *"If I click
/// the one to edit objects, I don't seem to be able to click on objects."* The
/// first half was a real hit-testing bug (a canvas-space tolerance shrinking to
/// sub-pixel on screen as you zoom out, fixed in `SELECT_SCREEN_TOLERANCE_PX`).
/// This is the other half, and it is the more deceptive of the two: hit-testing
/// worked, selection worked, the state was correct — and the tool looked
/// completely dead because nothing was painted. A correct action with no
/// feedback is indistinguishable from a broken one.
///
/// Every tool that owns the canvas and supports object selection must call
/// this. Painting is a `painter` overlay above the raster, never a re-raster.
/// Where an object sits in the Objects panel's **display** order, given its
/// paint-order [`TargetId`] and the page's object count (ui-spec §B.2).
///
/// The panel lists front-most first, so display row = `total - 1 - index`.
/// Pulled out of [`PdfceApp::objects_panel`] as a free function for exactly
/// one reason: it is the only arithmetic in that panel that can be wrong in
/// a way a human would notice (the scroll-reveal landing on the wrong row,
/// or one row off the end), and this crate's standing split is that wiring
/// gets reviewed while arithmetic gets a test.
///
/// Out-of-range input is clamped rather than refused. A [`TargetId`] can
/// outlive the object it named — an edit shortens the list and
/// `prune_canvas_selection` has not run yet — and the worst honest outcome
/// of a stale id is scrolling to the wrong row for one frame. Panicking, or
/// refusing to draw the list, would both be worse answers to a transient.
fn display_row_for_target(target: TargetId, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    let last = total - 1;
    let index = usize::try_from(target.0).unwrap_or(last).min(last);
    last - index
}

/// The side of the square type-badge chip drawn at a selection's top-left
/// corner, in egui logical points.
///
/// Big enough for a legible capital at the badge font size, small enough that
/// it does not swamp the outline of a small object.
const SELECTION_BADGE_SIZE: f32 = 15.0;

/// How many selected objects may carry a type badge before badges are
/// suppressed and only outlines are drawn.
///
/// Not a silent cap on information: the status-bar readout states the true
/// selection count and its per-kind census whatever this number is, so nothing
/// becomes unknowable. It is a legibility cap — past a few dozen objects the
/// chips overlap into a smear that answers nothing, and each one costs a text
/// galley per frame. The badge exists to answer "what is THIS?", a question
/// that only has an answer while the selection is small enough to point at.
const MAX_SELECTION_BADGES: usize = 48;

/// The dash and gap lengths, in egui logical points, of the outline drawn
/// around an object whose bounds are an APPROXIMATION rather than a
/// measurement (today: every text object).
const APPROXIMATE_OUTLINE_DASH: (f32, f32) = (6.0, 4.0);

fn draw_selection_outlines(
    doc: &OpenDoc,
    ui: &egui::Ui,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
) {
    // The entered object's selected subpath, drawn FIRST and in its own hue.
    //
    // Without this the second selection level is invisible: the operator
    // double-clicks a drawing view, pdfce descends into it, and the screen
    // looks exactly as it did — which is indistinguishable from the
    // double-click having done nothing. R83's sibling problem: an affordance
    // that works but shows nothing is as good as absent.
    //
    // A different colour from the object accent because it means something
    // different — "inside this object, this part" rather than "this object" —
    // and drawn before the object outlines so an object outline is never
    // hidden beneath it.
    if let Some(entered) = doc.entered
        && let Some(sp) = entered.subpath
        && let Some(provider) = doc.object_model.as_ref()
        && let Some(b) = provider.subpath_bounds_canvas(entered.object, sp)
    {
        let painter = ui.painter_at(image_rect);
        let min = viewer::page_to_screen(b.min, image_rect, extent, zoom);
        let max = viewer::page_to_screen(b.max, image_rect, extent, zoom);
        // The same degenerate-box treatment the object outlines get: most
        // subpaths of a CAD drawing ARE single straight lines, so a
        // zero-height box is the common case here rather than the exception.
        let rect = canvas::visible_outline_rect(
            egui::Rect::from_two_pos(min, max),
            canvas::MIN_OUTLINE_EXTENT_PX,
        );
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, SUBPATH_OUTLINE_COLOR),
            egui::StrokeKind::Outside,
        );
    }

    let outlines = canvas::selection_outline_bounds(
        &doc.canvas_selection,
        doc.target_provider(),
        doc.view.page_index,
    );
    if outlines.is_empty() {
        return;
    }
    // The concrete provider (not the opaque trait) is what can name the
    // objects behind the targets. Absent it — an undecodable page — the
    // overlay still draws every box, just without a kind-specific treatment:
    // an unlabelled box beats no box at all, which is the state that started
    // this whole line of work.
    let objects = doc
        .object_model
        .as_ref()
        .map(|p| p.page_objects().objects.as_slice());
    let painter = ui.painter_at(image_rect);
    let accent = ui.visuals().selection.stroke.color;
    let stroke = egui::Stroke::new(2.0, accent);
    let badges = outlines.len() <= MAX_SELECTION_BADGES;

    for (target, canvas_bounds) in outlines {
        let min = viewer::page_to_screen(canvas_bounds.min, image_rect, extent, zoom);
        let max = viewer::page_to_screen(canvas_bounds.max, image_rect, extent, zoom);
        // The degenerate-outline fix. A zero-height rule's box strokes
        // literally nothing without this, so a correct selection looked like
        // a dead click. `visible_outline_rect` grows it about its own centre
        // and the status readout states the object's true size, so the
        // enlargement is legible AND disclosed rather than quietly wrong.
        let rect = canvas::visible_outline_rect(
            egui::Rect::from_two_pos(min, max),
            canvas::MIN_OUTLINE_EXTENT_PX,
        );
        let summary = objects
            .and_then(|objs| objs.get(usize::try_from(target.0).ok()?))
            .map(describe_object);

        // Per-kind treatment, R84-compliant: the cue that distinguishes an
        // approximate box from a measured one is the DASH PATTERN — a shape
        // property that survives greyscale and colour-vision deficiency —
        // never a second accent colour. A solid box claims "the object is
        // exactly here"; a dashed box claims "the object is somewhere in
        // here", which for a text bbox inflated around glyph origins is the
        // literal truth and the single likeliest explanation for a box that
        // appears to surround nothing.
        if summary
            .as_ref()
            .is_some_and(ObjectSummary::bounds_are_approximate)
        {
            let (dash, gap) = APPROXIMATE_OUTLINE_DASH;
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
                rect.left_top(),
            ];
            painter.extend(egui::Shape::dashed_line(&corners, stroke, dash, gap));
        } else {
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }

        if badges && let Some(summary) = &summary {
            draw_selection_badge(&painter, image_rect, rect, accent, ui, summary);
        }
    }
}

/// Draw the type badge at a selection outline's top-left corner (ui-spec
/// §C.1) — a filled chip carrying a single letter naming the object kind.
///
/// ## Why a letter and not an icon
///
/// `icons::Icon` has no glyph for a path, an image or a form XObject, and its
/// `Text` glyph names the text *tool*, not a text object — reusing it would
/// assert an affordance that does not exist (R83). §C.1 anticipated exactly
/// this and named a letter badge as the honest interim, with the badge's
/// POSITION and EXISTENCE as the durable part of the design; when the icon set
/// grows object-kind glyphs, only this function changes.
///
/// ## Why it is not the only cue
///
/// R84 forbids colour-alone state. The badge is a filled SHAPE carrying a
/// LETTER, the outline beside it already distinguishes approximate from
/// measured bounds by dash pattern, and the full sentence is in the status
/// readout. The canvas raster remains screen-reader-illegible (the standing
/// gap in `main.rs`'s accessibility notes) — which is precisely why the
/// readout, not this badge, is the load-bearing disclosure, and why the badge
/// is allowed to be terse.
///
/// The chip is clamped into `image_rect`, so an object selected at the very
/// top-left of the page still shows its badge instead of painting it into the
/// panel gutter where `painter_at` would clip it away.
fn draw_selection_badge(
    painter: &egui::Painter,
    image_rect: egui::Rect,
    outline: egui::Rect,
    accent: egui::Color32,
    ui: &egui::Ui,
    summary: &ObjectSummary,
) {
    let size = egui::vec2(SELECTION_BADGE_SIZE, SELECTION_BADGE_SIZE);
    let wanted = egui::Rect::from_min_size(outline.left_top(), size);
    // Translate rather than intersect: a clipped chip would be a half-letter,
    // which reads as a rendering fault rather than as a label.
    let dx =
        (image_rect.min.x - wanted.min.x).max(0.0) + (image_rect.max.x - wanted.max.x).min(0.0);
    let dy =
        (image_rect.min.y - wanted.min.y).max(0.0) + (image_rect.max.y - wanted.max.y).min(0.0);
    let chip = wanted.translate(egui::vec2(dx, dy));
    painter.rect_filled(chip, 3.0, accent);
    painter.text(
        chip.center(),
        egui::Align2::CENTER_CENTER,
        ui_text::object_kind_badge(summary.kind),
        egui::FontId::proportional(SELECTION_BADGE_SIZE * 0.72),
        // The window's own extreme background, which is near-white under a
        // light theme and near-black under a dark one — so the letter stays
        // legible against the accent fill in both, without this function
        // having to know which theme is live.
        ui.visuals().extreme_bg_color,
    );
}

fn run_vector_edit_tool(
    doc: &mut OpenDoc,
    ui: &mut egui::Ui,
    image_response: &egui::Response,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
) {
    use pdfce_core::vector::{Point, SnapConfig, snap_candidates};

    let page_index = doc.view.page_index;
    if doc.pages.get(page_index).is_none() {
        return;
    }
    let selected: Option<usize> = doc.canvas_selection.iter().next().map(|t| t.0 as usize);

    // The gesture's outcome for this frame, decided in the read/preview block
    // and applied after the read borrows end (so the &mut-self commit and the
    // provider rebuild do not fight the page/object-model borrows).
    enum Commit {
        None,
        Move { idx: usize, dx: f64, dy: f64 },
        Node { idx: usize, node: usize, to: Point },
    }
    let mut commit = Commit::None;
    let mut new_selection: Option<std::collections::BTreeSet<TargetId>> = None;
    let mut new_cycle: Option<Option<canvas::ClickCycle>> = None;
    let mut new_drag: Option<Option<vector_edit_tool::VectorDrag>> = None;
    // A click's effect on the selection DEPTH, deferred like every other
    // outcome in this function: `apply_click_depth` needs `&mut doc`, and the
    // read borrows of the page and the object model are still live inside the
    // block below.
    let mut depth_request: Option<(egui::Pos2, f64, bool)> = None;

    {
        let page = &doc.pages[page_index];
        let painter = ui.painter_at(image_rect);
        let preview_color = egui::Color32::from_rgb(210, 90, 40);
        let snap_color = ui.visuals().selection.stroke.color;

        let to_pdf = |sp: egui::Pos2| -> Option<Point> {
            viewer::canvas_to_pdf_space(viewer::screen_to_page(sp, image_rect, extent, zoom), page)
                .map(|p| Point::new(f64::from(p.x), f64::from(p.y)))
        };
        let to_screen = |pt: Point| -> Option<egui::Pos2> {
            let p = egui::pos2(pt.x as f32, pt.y as f32);
            viewer::pdf_space_to_canvas(p, page)
                .map(|c| viewer::page_to_screen(c, image_rect, extent, zoom))
        };

        // Snap a page-space query to the nearest 12.M1 candidate (node drag
        // only — a whole-object move uses the raw delta so it never snaps to
        // the object it is dragging). The snap indicator is drawn pre-commit.
        let objects = doc
            .object_model
            .as_ref()
            .map(object_provider::ObjectModelProvider::page_objects);
        let snap = |q: Point| -> (Point, bool) {
            if let Some(objs) = objects {
                let tol = canvas::screen_tolerance_to_page(canvas::SNAP_SCREEN_TOLERANCE_PX, zoom);
                let cands = snap_candidates(q, &SnapConfig::new(tol), objs);
                if let Some(c) = cands.first() {
                    return (c.point, true);
                }
            }
            (q, false)
        };

        // A plain/Shift/Alt click selects the object under the pointer (9a),
        // through the SAME resolver the plain-selection path uses — a second
        // cycling rule for the object-edit tool would be a divergence the
        // operator would experience as the tool "cycling differently".
        if (image_response.clicked() || image_response.double_clicked())
            && let Some(sp) = image_response.interact_pointer_pos()
        {
            let canvas_pos = viewer::screen_to_page(sp, image_rect, extent, zoom);
            let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
            let tol = canvas::screen_tolerance_to_page(canvas::SELECT_SCREEN_TOLERANCE_PX, zoom);
            // The SAME level navigation the no-tool path has. Recorded here and
            // applied below rather than duplicated — the whole reason
            // `apply_click_depth` is one method on `OpenDoc` (R92). Wiring it
            // into only one of the two paths, which is what shipped first, made
            // double-click descend with no tool and do nothing with the object
            // tool armed: exactly the invisible divergence the shared method
            // was written to prevent.
            depth_request = Some((canvas_pos, tol, image_response.double_clicked()));
            let hits = doc
                .object_model
                .as_ref()
                .map(|p| p.hit_test_all(page_index, canvas_pos, tol))
                .unwrap_or_default();
            let (selection, cycle) = canvas::selection_and_cycle_after_click(
                &hits,
                &doc.canvas_selection,
                doc.click_cycle,
                page_index,
                canvas_pos,
                shift,
                alt,
            );
            diag::trace(|| {
                format!(
                    "vector-click screen={sp:?} canvas={canvas_pos:?} tol={tol} hits={} \
                     first={:?} newsel={}",
                    hits.len(),
                    hits.first(),
                    selection.len()
                )
            });
            new_selection = Some(selection);
            new_cycle = Some(cycle);
        }

        // Drag start: classify as a node grab (near a selected object's
        // anchor) or a whole-object move.
        if canvas::primary_drag_started(image_response)
            && let Some(sp) = image_response.interact_pointer_pos()
            && let Some(start) = to_pdf(sp)
            && let Some(idx) = selected
        {
            let anchors = doc
                .object_model
                .as_ref()
                .map(|p| p.object_sample_points(idx))
                .unwrap_or_default();
            new_drag = Some(Some(vector_edit_tool::classify_drag(idx, start, &anchors)));
        }

        // Live preview + commit-on-release for an in-flight drag.
        if let Some(drag) = doc.vector_drag
            && let Some(sp) = image_response.interact_pointer_pos()
            && let Some(ptr) = to_pdf(sp)
        {
            if let Some(node) = drag.node {
                // Node drag: snap the target and draw the snap marker (shown
                // pre-commit — fuzzy-never-sneaky) plus a preview handle.
                let (target, snapped) = snap(ptr);
                if let Some(s) = to_screen(target) {
                    if snapped {
                        painter.extend(canvas::snap_marker_shapes(
                            s,
                            pdfce_core::vector::SnapKind::Node,
                            snap_color,
                            5.0,
                        ));
                    }
                    painter.circle_stroke(s, 4.0, egui::Stroke::new(1.5, preview_color));
                }
                if canvas::primary_drag_stopped(image_response) {
                    commit = Commit::Node {
                        idx: drag.object_index,
                        node,
                        to: target,
                    };
                }
            } else {
                // Whole-object move: preview the object's bbox offset by the
                // raw page-space delta (no snap — a move never snaps to the
                // object it is dragging).
                let (dx, dy) = drag.delta(ptr);
                if let Some(prov) = doc.object_model.as_ref()
                    && let Some(r) = prov.bounds(page_index, TargetId(drag.object_index as u64))
                {
                    // The page-space delta as a screen-space vector: map the
                    // delta and the origin through the same transform and
                    // subtract (a pure translation + flip at this scale).
                    let d_screen = to_screen(Point::new(dx, dy))
                        .zip(to_screen(Point::new(0.0, 0.0)))
                        .map(|(a, b)| a - b)
                        .unwrap_or(egui::Vec2::ZERO);
                    let min = viewer::page_to_screen(r.min, image_rect, extent, zoom) + d_screen;
                    let max = viewer::page_to_screen(r.max, image_rect, extent, zoom) + d_screen;
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(min, max),
                        0.0,
                        egui::Stroke::new(2.0, preview_color),
                        egui::StrokeKind::Inside,
                    );
                }
                if canvas::primary_drag_stopped(image_response) {
                    commit = Commit::Move {
                        idx: drag.object_index,
                        dx,
                        dy,
                    };
                }
            }
        }
    }

    // Apply the frame's outcome (no read borrows held now).
    //
    // Depth FIRST: when the click was consumed inside an entered object, the
    // object-level selection computed above is discarded, or picking one part
    // of a view would re-select the whole view in the same frame and visually
    // undo the descent.
    if let Some((pos, tol, double)) = depth_request
        && doc.apply_click_depth(pos, tol, double)
    {
        new_selection = None;
        new_cycle = None;
    }
    if let Some(sel) = new_selection {
        doc.canvas_selection = sel;
    }
    if let Some(cycle) = new_cycle {
        doc.click_cycle = cycle;
    }
    if let Some(d) = new_drag {
        doc.vector_drag = d;
    }
    match commit {
        Commit::None => {
            // A drag that released without a committable target drops its state.
            if canvas::primary_drag_stopped(image_response) {
                doc.vector_drag = None;
            }
        }
        // Both arms call `refresh_pages` before rebuilding the provider —
        // the decision 018 §10 hazard 2 audit (Pass 17.0) found these two
        // sites, plus `delete_selected_object`, committing a real content
        // rewrite WITHOUT it. They did their own
        // `ensure_object_provider` + `prune_canvas_selection`, which looks
        // equivalent and is not:
        //
        // 1. `ensure_object_provider` early-returns while `provider_page`
        //    still equals the current page, so the provider was never
        //    actually rebuilt — only `refresh_pages` nulls that key;
        // 2. nothing dropped `page_texture`, so even with Pass 17.0's
        //    session-aware read path the moved geometry would not repaint
        //    until some unrelated event invalidated the cached raster.
        //
        // Before Pass 17.0 neither omission was observable, because the
        // canvas rendered the base revision no matter what. `refresh_pages`
        // is a strict superset of what these arms did (page list, texture,
        // thumbnails, provider key, marquee, selection prune), so the
        // explicit `ensure_object_provider` that follows only pulls the
        // rebuild into THIS frame rather than the next.
        Commit::Move { idx, dx, dy } => {
            let outcome = doc.session.move_object(page_index, idx, dx, dy);
            diag::trace(|| format!("commit-move idx={idx} dx={dx} dy={dy} -> {outcome:?}"));
            doc.vector_drag = None;
            doc.refresh_pages();
            doc.ensure_object_provider();
        }
        Commit::Node { idx, node, to } => {
            let outcome = doc.session.move_node(page_index, idx, node, to);
            diag::trace(|| format!("commit-node idx={idx} node={node} to={to:?} -> {outcome:?}"));
            doc.vector_drag = None;
            doc.refresh_pages();
            doc.ensure_object_provider();
        }
    }

    // Show what is selected. Drawn LAST, after the frame's selection change and
    // any commit have been applied, so the outline reflects the state the
    // operator just produced rather than the previous frame's. Without this the
    // object-edit tool selects silently — see `draw_selection_outlines`.
    draw_selection_outlines(doc, ui, image_rect, extent, zoom);
}

fn run_measure_tool(
    doc: &mut OpenDoc,
    ui: &mut egui::Ui,
    image_response: &egui::Response,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
) {
    use pdfce_core::dimension::{
        DEFAULT_GROUP_ID, DimensionKind, ScaleState, Unit, format_measurement,
    };
    use pdfce_core::vector::{AxisConstraint, Point, SnapConfig, SnapKind, snap_candidates};

    let active = doc.active_tool;
    let page_index = doc.view.page_index;

    // Repoint on page navigation while the tool stays active (ui-spec §1.3):
    // the picks/fit/scale of page N have no meaning on page N+1.
    match doc.measure.as_mut() {
        Some(st) if st.page_index != page_index => {
            st.page_index = page_index;
            st.clear_gesture();
            st.last_disclosures.clear();
        }
        Some(_) => {}
        None => return,
    }
    if doc.pages.get(page_index).is_none() {
        return;
    }

    // The authoritative model (owned clone — no lingering `doc.session` borrow):
    // the active group's scale/format for the live readout + the group picker.
    let model = doc.session.dimension_model();

    // Intents captured in the UI closures, applied in Phase C.
    let mut do_accept = false;
    let mut do_reject = false;
    let mut open_groups = false;

    {
        let page = &doc.pages[page_index];
        let painter = ui.painter_at(image_rect);
        let preview_color = egui::Color32::from_rgb(210, 90, 40);
        let snap_color = ui.visuals().selection.stroke.color;
        let text_color = ui.visuals().text_color();
        let warn_color = ui.visuals().warn_fg_color;

        // Coordinate bridges — the 14.3/16.2 canvas↔PDF bridge (rotation/zoom
        // correct), never `screen_to_page` alone.
        let to_screen = |pt: Point| -> Option<egui::Pos2> {
            #[allow(clippy::cast_possible_truncation)]
            let p = egui::pos2(pt.x as f32, pt.y as f32);
            viewer::pdf_space_to_canvas(p, page)
                .map(|c| viewer::page_to_screen(c, image_rect, extent, zoom))
        };
        let to_pdf = |sp: egui::Pos2| -> Option<Point> {
            viewer::canvas_to_pdf_space(viewer::screen_to_page(sp, image_rect, extent, zoom), page)
                .map(|p| Point::new(f64::from(p.x), f64::from(p.y)))
        };

        // The ONE page decomposition (shared with selection, ui-spec §3.3).
        let objects = doc.object_model.as_ref().map(|p| p.page_objects());

        let Some(st) = doc.measure.as_mut() else {
            return;
        };

        // Tab cycles the tied snap candidates; Alt suppresses snapping for this
        // pick (ui-spec §2.4). Consuming Tab keeps egui focus traversal off it
        // while the tool is active.
        let tab = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
        let alt = ui.input(|i| i.modifiers.alt);

        let pointer_pdf = image_response.hover_pos().and_then(to_pdf);
        let snap_on = canvas::snap_query_enabled(st.snap_master, alt);
        let cands = match (snap_on, pointer_pdf, objects) {
            (true, Some(q), Some(objs)) => {
                let tol = canvas::screen_tolerance_to_page(canvas::SNAP_SCREEN_TOLERANCE_PX, zoom);
                snap_candidates(q, &SnapConfig::new(tol), objs)
            }
            _ => Vec::new(),
        };
        if tab {
            st.snap_cycle = canvas::next_snap_index(st.snap_cycle, cands.len());
        }
        let active_cand = canvas::active_snap_candidate(&cands, st.snap_cycle);
        let active_is_derived = active_cand.is_some_and(|c| c.kind.is_derived());
        // The effective pick point: the snapped candidate, else the raw pointer.
        let effective = active_cand.map(|c| c.point).or(pointer_pdf);

        // The snap indicator (12.M1 primitive: shape distinguishes kind, rule 6)
        // + its type label, drawn BEFORE the click commits (fuzzy-never-sneaky).
        if let Some(c) = active_cand
            && let Some(sp) = to_screen(c.point)
        {
            painter.extend(canvas::snap_marker_shapes(sp, c.kind, snap_color, 5.0));
            painter.text(
                sp + egui::vec2(9.0, -2.0),
                egui::Align2::LEFT_CENTER,
                ui_text::snap_indicator_label(c.kind),
                egui::FontId::proportional(11.0),
                text_color,
            );
        }

        // A click resolves against the active candidate (derived ⇒ two-click
        // confirm, ui-spec §2.3) → advance the active tool's state machine.
        if image_response.clicked()
            && let Some(pick) = effective
            && let measure_tool::ClickOutcome::Commit(point) =
                st.resolve_click(pick, active_is_derived)
        {
            st.snap_cycle = 0;
            if canvas::tool_builds_measure_linear(active) {
                if st.pending.is_none()
                    && let Some(kind) = st.linear.commit_point(point)
                {
                    st.pending = Some(kind);
                }
            } else if canvas::tool_builds_measure_scale(active) {
                st.scale.commit_point(point);
            } else if canvas::tool_builds_measure_circular(active)
                && let Some(sp) = image_response.interact_pointer_pos()
            {
                // Toggle the clicked object into the fit set (ui-spec §3.1) —
                // its page-space anchors feed the SAME `fit_circle_taubin`.
                let canvas_pos = viewer::screen_to_page(sp, image_rect, extent, zoom);
                let tol =
                    canvas::screen_tolerance_to_page(canvas::SELECT_SCREEN_TOLERANCE_PX, zoom);
                if let Some(provider) = doc.object_model.as_ref()
                    && let Some(target) = provider.hit_test(page_index, canvas_pos, tol)
                {
                    let idx = target.0 as usize;
                    let samples = provider.object_sample_points(idx);
                    st.circular.toggle_object(idx, samples);
                }
            }
        }

        // Live preview (dashed-preview colour, ui-spec §2.5/§3.4 — display only).
        let draw_seg = |a: Point, b: Point| {
            if let (Some(sa), Some(sb)) = (to_screen(a), to_screen(b)) {
                painter.line_segment([sa, sb], egui::Stroke::new(1.5, preview_color));
            }
        };
        if canvas::tool_builds_measure_linear(active) {
            if let Some(DimensionKind::Linear { a, b, .. }) = st.pending {
                draw_seg(a, b);
            } else if let Some(ptr) = pointer_pdf
                && let Some((a, b)) = st.linear.preview_segment(ptr)
            {
                draw_seg(a, b);
            }
        } else if canvas::tool_builds_measure_scale(active) {
            if let Some(ptr) = pointer_pdf
                && let Some((a, b)) = st.scale.line.preview_segment(ptr)
            {
                draw_seg(a, b);
            }
        } else if canvas::tool_builds_measure_circular(active) {
            // Outline every object currently in the fit set (ui-spec §3.4 —
            // the picked sources are visible), reusing the ONE decomposition's
            // canvas-space bounds.
            if let Some(provider) = doc.object_model.as_ref() {
                for idx in st.circular.object_indices() {
                    if let Some(r) = provider.bounds(page_index, TargetId(idx as u64)) {
                        let min = viewer::page_to_screen(r.min, image_rect, extent, zoom);
                        let max = viewer::page_to_screen(r.max, image_rect, extent, zoom);
                        painter.rect_stroke(
                            egui::Rect::from_two_pos(min, max),
                            0.0,
                            egui::Stroke::new(1.0, snap_color),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
            // The live best-fit circle (dashed-preview colour) + centre glyph,
            // with its residual surfaced in the status strip (fuzzy, §3.4).
            if let Some(fit) = st.circular.fit()
                && let Some(c) = to_screen(fit.center)
            {
                #[allow(clippy::cast_possible_truncation)]
                let rad = fit.radius as f32 * zoom;
                painter.circle_stroke(c, rad, egui::Stroke::new(1.5, preview_color));
                painter.extend(canvas::snap_marker_shapes(
                    c,
                    SnapKind::Center,
                    preview_color,
                    5.0,
                ));
            }
        }

        // -- Property bar (ui-spec §2.5/§2.6/§3.4/§4.2): a floating top panel;
        //    every control a REAL egui widget (accesskit). --
        // MOVABLE and CLOSABLE (operator request, 2026-08-04).
        //
        // It was `.fixed_pos(...)`, pinned to the page's top-left corner with
        // no way to shift it or dismiss it. On a drawing whose dimensions sit
        // under that corner, the box covers exactly the geometry the operator
        // is trying to pick — and the only escape was to switch tools, which
        // also throws away the gesture in progress.
        //
        // `default_pos` + `movable` keeps the same opening position (nothing
        // moves for anyone who was happy with it) while letting it be dragged
        // off the work. Close puts the TOOL away, not merely the box: leaving
        // a tool armed with its controls hidden would keep canvas clicks
        // doing something the operator can no longer see the settings for.
        let mut close_tool = false;
        egui::Area::new(egui::Id::new("pdfce-measure-propbar"))
            .order(egui::Order::Foreground)
            .default_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::TopLeft,
                8.0,
            ))
            .movable(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(440.0);
                    ui.horizontal(|ui| {
                        // Right-aligned so it cannot be hit while reaching for
                        // the tool's own first control.
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(ui_text::tool_panel_close())
                                .on_hover_text(ui_text::tool_panel_close_tooltip())
                                .clicked()
                            {
                                close_tool = true;
                            }
                        });
                    });
                    if canvas::tool_builds_measure_linear(active) {
                        ui.label(ui_text::measure_linear_menu_item());
                        ui.label(ui_text::measure_linear_hint());
                    } else if canvas::tool_builds_measure_circular(active) {
                        ui.label(ui_text::measure_circular_menu_item());
                        ui.label(ui_text::measure_circular_hint());
                    } else {
                        ui.label(ui_text::measure_set_scale_menu_item());
                        ui.label(ui_text::measure_scale_hint());
                    }
                    ui.separator();
                    // Active-group picker + the group-panel opener (ui-spec §2.6).
                    ui.horizontal(|ui| {
                        ui.label(ui_text::measure_group_label());
                        let cur = model
                            .group(st.group)
                            .map_or_else(String::new, |g| g.name.clone());
                        egui::ComboBox::from_id_salt("pdfce-measure-group")
                            .selected_text(cur)
                            .show_ui(ui, |ui| {
                                for g in model.groups() {
                                    ui.selectable_value(&mut st.group, g.id, g.name.clone());
                                }
                            });
                        if ui.button(ui_text::measure_open_groups_button()).clicked() {
                            open_groups = true;
                        }
                    });
                    ui.checkbox(&mut st.snap_master, ui_text::snap_toggle_label())
                        .on_hover_text(ui_text::snap_toggle_tooltip());
                    // H/V/aligned constraint (linear + scale, ui-spec §2.5).
                    if canvas::tool_builds_measure_linear(active)
                        || canvas::tool_builds_measure_scale(active)
                    {
                        ui.horizontal(|ui| {
                            ui.label(ui_text::measure_alignment_label());
                            for c in [
                                AxisConstraint::Aligned,
                                AxisConstraint::Horizontal,
                                AxisConstraint::Vertical,
                            ] {
                                let label = ui_text::axis_constraint_label(c);
                                if canvas::tool_builds_measure_linear(active) {
                                    ui.selectable_value(&mut st.linear.constraint, c, label);
                                } else {
                                    ui.selectable_value(&mut st.scale.line.constraint, c, label);
                                }
                            }
                        });
                    }
                    // Radius/diameter display toggle (circular, ui-spec §3.4).
                    if canvas::tool_builds_measure_circular(active) {
                        ui.horizontal(|ui| {
                            ui.label(ui_text::measure_display_label());
                            ui.selectable_value(
                                &mut st.circular.show_diameter,
                                false,
                                ui_text::measure_radius_option(),
                            );
                            ui.selectable_value(
                                &mut st.circular.show_diameter,
                                true,
                                ui_text::measure_diameter_option(),
                            );
                        });
                    }
                    // The scale-entry dialog once the reference line is drawn
                    // (ui-spec §4.2), via the SHARED scale-entry widget.
                    if canvas::tool_builds_measure_scale(active) && st.scale.dialog_open() {
                        ui.separator();
                        let drawn = st.scale.drawn_pdf_length;
                        if let Some(len) = drawn {
                            ui.label(ui_text::scale_entry_drawn_length(len));
                        }
                        scale_entry_widget(ui, &mut st.scale.fields, drawn);
                    }
                });
            });

        // Closing the box puts the TOOL away, and the in-progress pick with
        // it. Keeping a half-finished two-point gesture alive behind a
        // dismissed panel would leave the next canvas click completing a
        // measurement the operator thought they had cancelled.
        if close_tool {
            doc.active_tool = None;
            return;
        }

        // -- Status / disclosure strip + Accept/Reject (ui-spec §2.6/§6). --
        let gscale = model
            .group(st.group)
            .map_or(ScaleState::NeverSet, |g| g.scale);
        let gformat = model
            .group(st.group)
            .map_or_else(|| Unit::Millimeter.default_format(), |g| g.format);
        egui::Area::new(egui::Id::new("pdfce-measure-status"))
            .order(egui::Order::Foreground)
            .fixed_pos(canvas::tool_strip_anchor(
                ui.max_rect(),
                canvas::StripCorner::BottomLeft,
                8.0,
            ))
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(480.0);
                    let mut can_accept = false;

                    if canvas::tool_builds_measure_linear(active) {
                        let raw = st
                            .pending
                            .map(|k| k.measured_points())
                            .or_else(|| pointer_pdf.and_then(|ptr| st.linear.measured(ptr)));
                        if let Some(raw) = raw {
                            let d = format_measurement(raw, gscale, gformat);
                            ui.label(ui_text::measure_length_readout(
                                raw,
                                &d.text,
                                d.raw_page_units,
                            ));
                            if d.raw_page_units {
                                ui.label(pdfce_core::dimension::NO_SCALE_DISCLOSURE);
                            }
                        }
                        can_accept = st.pending.is_some();
                    } else if canvas::tool_builds_measure_circular(active) {
                        if let Some(fit) = st.circular.fit() {
                            ui.label(ui_text::best_fit_circle_disclosure(
                                st.circular.object_count(),
                                fit.radius,
                                fit.residual,
                            ));
                            if fit.residual > fit.radius * 0.1 {
                                ui.colored_label(warn_color, ui_text::best_fit_residual_high());
                            }
                            let raw = if st.circular.show_diameter {
                                2.0 * fit.radius
                            } else {
                                fit.radius
                            };
                            let d = format_measurement(raw, gscale, gformat);
                            ui.label(ui_text::measure_length_readout(
                                raw,
                                &d.text,
                                d.raw_page_units,
                            ));
                            if d.raw_page_units {
                                ui.label(pdfce_core::dimension::NO_SCALE_DISCLOSURE);
                            }
                            can_accept = true;
                        }
                    } else if canvas::tool_builds_measure_scale(active) {
                        if let Some(prev) = st.scale.preview() {
                            ui.label(ui_text::scale_entry_preview(&prev.ratio_label));
                        }
                        can_accept = st.scale.commit().is_some();
                    }

                    // The derived-centerline confirm (fuzzy inference, §2.3.1).
                    if active_is_derived {
                        ui.colored_label(warn_color, ui_text::measure_confirm_derived_centerline());
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(can_accept, egui::Button::new(ui_text::measure_accept()))
                            .clicked()
                        {
                            do_accept = true;
                        }
                        if ui
                            .add(egui::Button::new(ui_text::measure_reject()))
                            .clicked()
                        {
                            do_reject = true;
                        }
                    });

                    // The last Accept's disclosures, verbatim (ui-spec §6).
                    if !st.last_disclosures.is_empty() {
                        ui.separator();
                        for d in &st.last_disclosures {
                            ui.label(ui_text::disclosure_bullet(d));
                        }
                    }
                });
            });
    }

    // ---- Phase C: the session mutation (one undoable command) ----
    if do_reject {
        if let Some(st) = doc.measure.as_mut() {
            st.clear_gesture();
        }
        return;
    }
    if open_groups {
        doc.dimension_groups_open = true;
    }
    if !do_accept {
        return;
    }
    let group = doc.measure.as_ref().map_or(DEFAULT_GROUP_ID, |s| s.group);
    let group_name = model
        .group(group)
        .map_or_else(String::new, |g| g.name.clone());

    if canvas::tool_builds_measure_linear(active) || canvas::tool_builds_measure_circular(active) {
        // Both author a dimension via the SAME `add_dimension` path the CLI
        // uses (byte-identical output for the same kind — measure_tool's
        // equivalence tests). Linear commits its `pending`; circular its fit.
        let kind = if canvas::tool_builds_measure_linear(active) {
            doc.measure.as_ref().and_then(|s| s.pending)
        } else {
            doc.measure.as_ref().and_then(|s| s.circular.author())
        };
        if let Some(kind) = kind {
            match doc.session.add_dimension(page_index, group, kind) {
                Ok(_) => {
                    doc.refresh_pages(); // the page's annots changed
                    if let Some(st) = doc.measure.as_mut() {
                        st.pending = None;
                        st.circular.clear();
                        st.last_disclosures =
                            vec![ui_text::measure_dimension_authored(&group_name)];
                    }
                }
                Err(err) => {
                    if let Some(st) = doc.measure.as_mut() {
                        st.last_disclosures = vec![ui_text::refusal_line(&err.to_string())];
                    }
                }
            }
        }
    } else if canvas::tool_builds_measure_scale(active)
        && let Some((scale, format)) = doc.measure.as_ref().and_then(|s| s.scale.commit())
    {
        {
            match doc.session.set_group_scale(group, scale, format) {
                Ok(updated) => {
                    doc.refresh_pages(); // members' /AP regenerated
                    if let Some(st) = doc.measure.as_mut() {
                        st.scale.clear();
                        st.last_disclosures =
                            vec![ui_text::measure_scale_applied(&group_name, updated)];
                    }
                }
                Err(err) => {
                    if let Some(st) = doc.measure.as_mut() {
                        st.last_disclosures = vec![ui_text::refusal_line(&err.to_string())];
                    }
                }
            }
        }
    }
}

/// The SHARED scale-entry sub-form (ui-spec §4.2 / §5.2) — the ONE scale-entry
/// UI in the whole app, driven by both the MeasureScale dialog and the
/// group-panel inline editor. Renders the two co-equal paths (real-length
/// recommended when a line exists, direct ratio otherwise) + the live preview,
/// mutating `fields` in place. `drawn` is the drawn reference length (points)
/// when a line exists (enables the real-length path); `None` ⇒ ratio only.
fn scale_entry_widget(
    ui: &mut egui::Ui,
    fields: &mut measure_tool::ScaleEntryFields,
    drawn: Option<f64>,
) {
    use pdfce_core::dimension::Unit;
    // Real-length path — only offered when a reference line was drawn.
    if drawn.is_some() {
        ui.selectable_value(
            &mut fields.use_real_length,
            true,
            ui_text::scale_entry_real_length_label(),
        );
        if fields.use_real_length {
            // A TEXT field, not a numeric spinner. The point of this workflow
            // is to type the dimension exactly as the drawing prints it —
            // `55 5/8"`, `4'-7 1/2"` — so that reading a number off a drawing
            // and entering it are the same action. A spinner made the
            // operator convert to a decimal and set the unit by hand: two
            // opportunities to enter something plausible and wrong, in a
            // field that silently rescales every dimension in the group.
            let mut parse_err = None;
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut fields.real_length_text)
                        .desired_width(120.0)
                        .hint_text(ui_text::scale_entry_real_length_hint()),
                );
                // Re-read every frame, not only on `changed()`. The unit
                // dropdown below can move independently, and a bare number
                // means "whatever the dropdown says" — so the parsed value
                // has to follow a dropdown change too, not just typing.
                let _ = resp;
                parse_err = fields.sync_real_length();
                egui::ComboBox::from_id_salt("pdfce-scale-real-unit")
                    .selected_text(ui_text::unit_dropdown_label(fields.unit))
                    .show_ui(ui, |ui| {
                        for u in Unit::all() {
                            ui.selectable_value(
                                &mut fields.unit,
                                u,
                                ui_text::unit_dropdown_label(u),
                            );
                        }
                    });
            });
            // Show the reading back, or say why there isn't one. Rule 4: the
            // parser accepts several notations and takes the unit from the
            // text, so it must show what it understood BEFORE the operator
            // commits — a calibration silently rescales every dimension in
            // the group, and "it accepted my input" is not the same as "it
            // read it the way I meant".
            match parse_err {
                None => {
                    ui.label(ui_text::scale_entry_real_length_echo(
                        fields.real_length,
                        ui_text::unit_dropdown_label(fields.unit),
                    ));
                }
                Some(e) => {
                    ui.colored_label(ui.visuals().warn_fg_color, e.to_string());
                }
            }
        }
    } else {
        fields.use_real_length = false; // no line ⇒ ratio only (ui-spec §7.2)
    }
    // Direct-ratio path (needs no line).
    ui.selectable_value(
        &mut fields.use_real_length,
        false,
        ui_text::scale_entry_ratio_label(),
    );
    if !fields.use_real_length {
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut fields.ratio_paper)
                    .range(0.0001..=1.0e9)
                    .speed(0.1),
            );
            ui.label(ui_text::ratio_colon());
            ui.add(
                egui::DragValue::new(&mut fields.ratio_real)
                    .range(0.0..=1.0e9)
                    .speed(0.5),
            );
            egui::ComboBox::from_id_salt("pdfce-scale-basis-unit")
                .selected_text(ui_text::unit_dropdown_label(fields.basis))
                .show_ui(ui, |ui| {
                    for u in Unit::all() {
                        ui.selectable_value(&mut fields.basis, u, ui_text::unit_dropdown_label(u));
                    }
                });
        });
    }
    // The paper-unit basis is ALWAYS shown, even when the ratio path is not
    // selected (ui-spec §4.2 — the operator learns the basis exists first).
    ui.label(ui_text::scale_entry_paper_basis_caption());
    // Live preview of the resulting scale, BEFORE Accept (ui-spec §4.2).
    if let Some(prev) = fields.preview(drawn) {
        ui.label(ui_text::scale_entry_preview(&prev.ratio_label));
    }
}

/// The modeless "Dimension Groups" panel (ui-spec §5): create groups, set a
/// group's scale + units, and toggle its layer — each mapping onto exactly ONE
/// shipped `EditSession` command (one undo step, ui-spec §5.4). Available with
/// or without a measure tool active (ui-spec §7.2). A no-op when closed.
fn run_dimension_groups_panel(doc: &mut OpenDoc, ui: &mut egui::Ui) {
    if !doc.dimension_groups_open {
        return;
    }
    use pdfce_core::dimension::{DEFAULT_GROUP_ID, Unit};
    let model = doc.session.dimension_model();
    // Engine intents + editor-close captured in the closure, applied after it
    // (so `doc.session`/`refresh_pages` are touched with no field borrow live).
    let mut actions: Vec<measure_tool::GroupAction> = Vec::new();
    let mut close_editor = false;
    let mut open = doc.dimension_groups_open;

    egui::Window::new(ui_text::group_manager_title())
        .open(&mut open)
        .resizable(true)
        .default_width(520.0)
        .show(ui.ctx(), |ui| {
            // -- New group (ui-spec §5.2). --
            ui.horizontal(|ui| {
                ui.label(ui_text::group_new_group_name_label());
                ui.text_edit_singleline(&mut doc.group_new_name);
                egui::ComboBox::from_id_salt("pdfce-newgroup-unit")
                    .selected_text(ui_text::unit_dropdown_label(doc.group_new_unit))
                    .show_ui(ui, |ui| {
                        for u in Unit::all() {
                            ui.selectable_value(
                                &mut doc.group_new_unit,
                                u,
                                ui_text::unit_dropdown_label(u),
                            );
                        }
                    });
                if ui.button(ui_text::group_new_group_button()).clicked()
                    && !doc.group_new_name.trim().is_empty()
                {
                    actions.push(measure_tool::GroupAction::Create {
                        name: doc.group_new_name.trim().to_owned(),
                        unit: doc.group_new_unit,
                    });
                    doc.group_new_name.clear();
                }
            });
            ui.separator();

            // -- One row per group (ui-spec §5.2). --
            for g in model.groups() {
                let summary = ui_text::group_scale_summary(g.scale, g.unit());
                let mut label =
                    ui_text::group_row_summary(&g.name, &summary, model.member_count(g.id));
                if !g.visible {
                    label.push(' ');
                    label.push_str(ui_text::group_hidden_suffix());
                }
                // Greyed when hidden — rule 6: never the eye glyph alone.
                if g.visible {
                    ui.label(label);
                } else {
                    ui.weak(label);
                }
                ui.horizontal(|ui| {
                    // Layer visibility toggle (default group un-hideable — the
                    // engine enforces it, ui-spec §5.3; disabled here too).
                    let is_default = g.id == DEFAULT_GROUP_ID;
                    if ui
                        .add_enabled(
                            !is_default,
                            egui::Button::new(ui_text::group_visibility_button(g.visible)),
                        )
                        .clicked()
                    {
                        actions.push(measure_tool::GroupAction::ToggleLayer {
                            group: g.id,
                            visible: !g.visible,
                        });
                    }
                    if ui.button(ui_text::group_set_scale_button()).clicked() {
                        doc.group_scale_edit =
                            Some((g.id, measure_tool::ScaleEntryFields::for_group_panel()));
                    }
                });
                // The inline scale editor for the expanded group (ui-spec §5.2).
                if let Some((gid, fields)) = doc.group_scale_edit.as_mut()
                    && *gid == g.id
                {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        scale_entry_widget(ui, fields, None);
                        ui.horizontal(|ui| {
                            if ui.button(ui_text::group_apply_button()).clicked() {
                                if let Some((scale, format)) = fields.commit(None) {
                                    actions.push(measure_tool::GroupAction::SetScale {
                                        group: g.id,
                                        scale,
                                        format,
                                    });
                                }
                                close_editor = true;
                            }
                            if ui.button(ui_text::group_cancel_button()).clicked() {
                                close_editor = true;
                            }
                        });
                    });
                }
                ui.separator();
            }
        });

    doc.dimension_groups_open = open;
    if close_editor {
        doc.group_scale_edit = None;
    }
    // Apply the captured engine intents — each ONE undoable command.
    for action in actions {
        match action {
            measure_tool::GroupAction::Create { name, unit } => {
                let _ = doc.session.add_dimension_group(&name, unit);
            }
            measure_tool::GroupAction::SetScale {
                group,
                scale,
                format,
            } => {
                if doc.session.set_group_scale(group, scale, format).is_ok() {
                    doc.refresh_pages();
                }
            }
            measure_tool::GroupAction::ToggleLayer { group, visible } => {
                if doc.session.toggle_dimension_layer(group, visible).is_ok() {
                    doc.refresh_pages();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Pass 19.3: the spacing & style property surface ----

    /// The GUI's unit tag maps onto core's discriminated one and nowhere
    /// else. If these two ever disagreed, a "scales with size" number would
    /// be written into the file as an absolute operand — silently wrong, and
    /// invisible until the run was resized.
    #[test]
    fn the_unit_toggle_maps_onto_the_core_metric_spec() {
        use pdfce_core::text_edit::MetricSpec;
        assert_eq!(
            metric_spec(MetricUnit::Absolute, 0.24),
            MetricSpec::Absolute(0.24)
        );
        assert_eq!(
            metric_spec(MetricUnit::Relative, 20.0),
            MetricSpec::Relative(20.0)
        );
    }

    fn snapshot(font_size: f64) -> AmbientSnapshot {
        AmbientSnapshot {
            char_spacing: 0.0,
            h_scale: 100.0,
            rise: 0.0,
            word_spacing: 0.0,
            font_size,
            composite: false,
            tc_at_default: true,
            tw_at_default: true,
            tz_at_default: true,
            rise_at_default: true,
        }
    }

    /// Switching the unit must RE-DERIVE the number, never reinterpret the
    /// digits already in the box. 20‰ at 12 pt is 0.24 text-space units — the
    /// same physical spacing, spelled two ways — and the round trip must not
    /// drift, or repeated toggling would walk the value.
    #[test]
    fn switching_units_preserves_the_physical_quantity() {
        let a = snapshot(12.0);
        assert!((a.per_mille_to_operand(20.0) - 0.24).abs() < 1e-12);
        assert!((a.per_mille(0.24) - 20.0).abs() < 1e-9);
        // …and it is the same relationship core's own resolver applies.
        assert!(
            (pdfce_core::text_edit::MetricSpec::Relative(20.0).resolve(12.0) - 0.24).abs() < 1e-12
        );
    }

    /// A run with no `Tf` size must not make the caption an infinity or a
    /// NaN. It shows zero, which is wrong-but-harmless, rather than rendering
    /// garbage into the panel.
    #[test]
    fn a_zero_font_size_does_not_produce_an_infinite_caption() {
        let a = snapshot(0.0);
        assert_eq!(a.per_mille(0.24), 0.0);
    }

    /// Load a fixture, wrap it in a session, and ask the read-only query.
    fn preview_on_fixture(
        name: &str,
        find: &str,
        want: pdfce_core::text_edit::StyleSynthesis,
    ) -> pdfce_core::text_edit::StyleResolution {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic/textedit")
            .join(name);
        let bytes = std::fs::read(path).expect("fixture");
        let doc = pdfce_core::document::Document::from_bytes(bytes).expect("parse");
        let session = pdfce_core::edit::EditSession::new(doc);
        session
            .preview_style_resolution(0, find, None, want)
            .expect("preview")
    }

    /// The ordinary case: the page has no Times-Italic, so the caption tells
    /// the operator — BEFORE they click — that Apply will synthesize, and
    /// there is no local refusal.
    #[test]
    fn the_style_caption_announces_synthesis_before_the_click() {
        use pdfce_core::text_edit::StyleSynthesis;
        let res = preview_on_fixture("format_family.pdf", "hello world", StyleSynthesis::Italic);
        let (caption, refusal) = style_row_text(&res).expect("a style was requested");
        assert!(refusal.is_none(), "nothing to refuse: {caption}");
        assert!(caption.contains("synthesize"), "{caption}");
        assert!(caption.contains("Italic"), "{caption}");
    }

    /// The real-face case: `format_family.pdf` carries a real `Times-Bold`
    /// beside its `Times-Roman` run, so the caption must name that face AND
    /// say the Apply will be refused — it must NOT promise a font switch,
    /// because pdfce does not perform one.
    #[test]
    fn the_style_caption_names_the_real_face_and_does_not_promise_a_switch() {
        use pdfce_core::text_edit::StyleSynthesis;
        let res = preview_on_fixture("format_family.pdf", "hello world", StyleSynthesis::Bold);
        let (caption, refusal) = style_row_text(&res).expect("a style was requested");
        assert!(refusal.is_none(), "core refuses this one, not the GUI");
        assert!(caption.contains("Times-Bold"), "{caption}");
        assert!(caption.contains("REFUSED"), "{caption}");
        assert!(
            caption.contains("Font control"),
            "it points at the control that DOES switch fonts: {caption}"
        );
    }

    /// **The mixed case.** The page has a real `Times-Bold` but no
    /// `Times-Italic`. Core's gate is all-or-nothing, so submitting
    /// Bold+Italic would synthesize BOTH and quietly pass over the real Bold.
    /// The panel refuses it by name instead, says which axis has the real
    /// face and which does not, and gives the two-step route that works.
    #[test]
    fn a_mixed_bold_italic_request_is_refused_by_name_with_both_axes_named() {
        use pdfce_core::text_edit::StyleSynthesis;
        let res = preview_on_fixture(
            "format_family.pdf",
            "hello world",
            StyleSynthesis::BoldItalic,
        );
        assert!(res.is_mixed(), "the fixture is the mixed shape");
        let (caption, refusal) = style_row_text(&res).expect("a style was requested");
        let refusal = refusal.expect("the GUI refuses this one locally");

        for text in [&caption, &refusal] {
            assert!(text.contains("Bold"), "the covered axis is named: {text}");
            assert!(
                text.contains("Italic"),
                "the uncovered axis is named: {text}"
            );
            assert!(
                text.contains("Times-Bold"),
                "the real face is named: {text}"
            );
        }
        assert!(
            caption.contains("Font control"),
            "the working route is offered: {caption}"
        );
        assert!(
            refusal.contains("Nothing was applied"),
            "the refusal states the outcome, like every core refusal: {refusal}"
        );
    }

    /// Nothing ticked, nothing said. The style row must not editorialize when
    /// the operator has not asked for anything.
    #[test]
    fn no_style_request_produces_no_caption() {
        use pdfce_core::text_edit::StyleSynthesis;
        let res = preview_on_fixture("format_family.pdf", "hello world", StyleSynthesis::None);
        assert!(style_row_text(&res).is_none());
    }

    /// Zooming must never discard an in-progress tool gesture.
    ///
    /// The regression this pins: picking point A of a linear dimension and
    /// then ctrl+scrolling to zoom in for an accurate point B silently threw
    /// point A away, because the interrupt rule treated every action except
    /// tool-select/cancel as a reason to discard. Zoom changes the camera,
    /// not the document, and gestures are stored in page space — so a zoom
    /// cannot invalidate one.
    #[test]
    fn view_only_actions_preserve_an_in_progress_gesture() {
        for action in [
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomBy(1.1),
            Action::ZoomActualSize,
            Action::Fit(FitMode::Page),
            Action::Fit(FitMode::Width),
            Action::SelectCanvasTool(None),
            Action::CancelToolGesture,
        ] {
            assert!(
                PdfceApp::action_preserves_gesture(action),
                "{action:?} must not discard an in-progress gesture"
            );
        }
    }

    /// The other half of the contract: actions that change the SUBJECT or
    /// touch the document still discard. A gesture anchored to page N is
    /// meaningless on page M, so page navigation deliberately stays
    /// interrupting — this test exists so a future "just allow-list
    /// everything harmless" edit has to argue with it.
    #[test]
    fn subject_changing_and_document_touching_actions_still_discard() {
        for action in [
            Action::Undo,
            Action::Redo,
            Action::NextPage,
            Action::PrevPage,
            Action::ToggleDimensionGroups,
        ] {
            assert!(
                !PdfceApp::action_preserves_gesture(action),
                "{action:?} must discard an in-progress gesture"
            );
        }
    }

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

    // ---- Pass 18.4 dock + object tree ------------------------------------

    /// A fixture path, resolved from this crate's manifest directory so the
    /// test does not depend on the working directory the runner chose.
    fn fixture(rel: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic")
            .join(rel)
    }

    /// **The regression test for decision 017 §8.4 / A.4 #3** — the
    /// prerequisite bugfix the Properties migration was not allowed to ship
    /// without.
    ///
    /// Before the fix, `open_path` seeded `properties_draft` only if the
    /// floating properties window happened to be open, so a freshly opened
    /// document normally carried an EMPTY draft. That was survivable while
    /// Properties was a window an operator explicitly opened (opening it
    /// seeded the draft on the way in). It is not survivable for a
    /// persistently-mounted dock panel, which can be drawn against a
    /// document nobody ever "opened" it for — the operator would see an
    /// empty metadata form, and typing into it and pressing Apply would
    /// write that emptiness over the document's real metadata.
    ///
    /// The assertion is deliberately about the draft's SHAPE, not its
    /// content: one entry per `InfoField`, present without anyone touching
    /// the panel. That is exactly the property the old code lacked, and it
    /// holds for a document whose `/Info` is absent or empty — which is the
    /// case that used to look identical to the bug.
    #[test]
    fn opening_a_document_seeds_the_properties_draft_without_the_panel_being_open() {
        let mut app = PdfceApp::default();
        app.open_path(fixture("hello.pdf"));
        let Status::Open(doc) = &app.status else {
            panic!("the fixture did not open");
        };
        assert_eq!(
            doc.properties_draft.len(),
            InfoField::all().len(),
            "the properties draft was not seeded on open — the dock panel \
             would render an empty metadata form (decision 017 §8.4)"
        );
        for (field, _) in &doc.properties_draft {
            assert!(
                InfoField::all().contains(field),
                "the draft holds a field that is not an InfoField"
            );
        }
    }

    /// Opening a SECOND document must reseed, not leave the first
    /// document's draft in place — the "stale" half of the same bug. A dock
    /// row showing the previous file's author is a quieter failure than an
    /// empty one and a worse one, because it looks correct.
    #[test]
    fn opening_a_second_document_replaces_the_previous_drafts_content() {
        let mut app = PdfceApp::default();
        app.open_path(fixture("hello.pdf"));
        // Simulate half-typed, uncommitted edits against document one.
        if let Status::Open(doc) = &mut app.status {
            for (_, text) in &mut doc.properties_draft {
                text.push_str("stale");
            }
        }
        app.open_path(fixture("vector/mixed.pdf"));
        let Status::Open(doc) = &app.status else {
            panic!("the second fixture did not open");
        };
        assert!(
            doc.properties_draft
                .iter()
                .all(|(_, t)| !t.contains("stale")),
            "the previous document's draft survived an Open"
        );
    }

    /// The Objects panel lists front-most first (ui-spec §B.2), so display
    /// row 0 is the LAST-painted object. Getting this backwards would put
    /// the object a click most likely hit at the bottom of a long list —
    /// which is the one thing the panel exists to avoid.
    #[test]
    fn the_object_tree_lists_the_topmost_object_first() {
        // Five objects, paint order 0..4. #4 was painted last, so it is
        // topmost, so it is display row 0.
        assert_eq!(display_row_for_target(TargetId(4), 5), 0);
        assert_eq!(display_row_for_target(TargetId(3), 5), 1);
        assert_eq!(display_row_for_target(TargetId(0), 5), 4);
        // A single-object page: the only object is the only row.
        assert_eq!(display_row_for_target(TargetId(0), 1), 0);
    }

    /// A stale target id (an edit shortened the list, and
    /// `prune_canvas_selection` has not run yet) must clamp, never panic and
    /// never index past the end — the worst honest outcome is one frame
    /// scrolled to the wrong row.
    #[test]
    fn a_stale_target_clamps_instead_of_panicking() {
        assert_eq!(display_row_for_target(TargetId(99), 5), 0);
        assert_eq!(display_row_for_target(TargetId(0), 0), 0);
        assert_eq!(display_row_for_target(TargetId(u64::MAX), 3), 0);
    }

    /// Every object kind produces a row that carries its paint-order index
    /// and names its kind — the two facts the panel exists to supply. Text
    /// and image rows are honestly thinner than the ui-spec's illustrative
    /// examples (§B.3/§B.4: the core model carries no text string, font or
    /// pixel size), and this asserts the floor they must still clear.
    #[test]
    fn every_object_kind_gets_a_row_naming_its_index_and_its_kind() {
        use pdfce_core::content::ContentStream;
        use pdfce_core::vector::{Matrix, NoXObjects, decompose};

        // A stroked line, a text object and an inline image, in paint order.
        let src = b"10 20 m 100 20 l S BT /F1 12 Tf 40 40 Td (Hi) Tj ET";
        let cs = ContentStream::parse(src.to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        assert!(objects.objects.len() >= 2, "fixture stream decomposed thin");

        for (index, object) in objects.objects.iter().enumerate() {
            let row = ui_text::object_row(index, &describe_object(object));
            assert!(
                row.contains(&format!("#{index}")),
                "row {row:?} does not carry its paint-order index"
            );
            assert!(
                row.len() > 8,
                "row {row:?} is too thin to identify anything"
            );
        }
        // The path row must name what it paints and how many nodes it has:
        // "it paints nothing" is the direct answer to "why is there a
        // selection box over blank paper?" (§C.2).
        let path_row = ui_text::object_row(0, &describe_object(&objects.objects[0]));
        assert!(path_row.contains("Path"), "{path_row:?}");
        assert!(path_row.contains("node"), "{path_row:?}");
    }

    /// A no-paint (`n`) path must say so in words. This is the single
    /// cheapest answer to the operator's "a box highlighting that doesn't
    /// correspond to anything" — a clip or discarded path is a real,
    /// selectable object that marks no pixels.
    #[test]
    fn a_path_that_paints_nothing_says_so() {
        use pdfce_core::vector::{FillRule, PaintStyle};
        let invisible = PaintStyle {
            fill: None,
            stroke: false,
        };
        assert!(invisible.is_invisible());
        let label = ui_text::paint_style_label(invisible);
        assert!(
            label.contains("nothing"),
            "an n-op path's label must say it paints nothing, got {label:?}"
        );
        // Every disposition gets a DISTINCT label, or the row cannot
        // distinguish a filled shape from a stroked one.
        let mut seen = BTreeSet::new();
        for fill in [None, Some(FillRule::NonZero), Some(FillRule::EvenOdd)] {
            for stroke in [false, true] {
                let label = ui_text::paint_style_label(PaintStyle { fill, stroke });
                assert!(seen.insert(label), "duplicate paint label {label:?}");
            }
        }
    }

    /// Every disclosure in the catalog must be a real, distinct EXPLANATION.
    ///
    /// A note whose sentence was copy-pasted from its neighbour, or left as a
    /// two-word label, would ship as a disclosure that discloses nothing —
    /// worse than no note, because it looks like the app answered the
    /// question. The floor asserted here: a long form that is a sentence
    /// rather than a label, a short form for the one-line row, the two not
    /// identical, and no duplicates in either set.
    #[test]
    fn every_selection_disclosure_is_a_distinct_explanation() {
        use object_summary::ObjectNote;
        let mut longs = BTreeSet::new();
        let mut shorts = BTreeSet::new();
        for note in ObjectNote::ALL {
            let long = ui_text::object_note(note);
            let short = ui_text::object_note_short(note);
            assert!(
                long.len() > 80,
                "{note:?}'s explanation is a label, not an explanation: {long:?}"
            );
            assert!(!short.is_empty(), "{note:?} has no short form");
            assert_ne!(long, short, "{note:?}'s two forms are the same string");
            assert!(longs.insert(long), "duplicate explanation on {note:?}");
            assert!(shorts.insert(short), "duplicate short form on {note:?}");
        }
    }

    /// Every object kind must be nameable and badge-able, with distinct names
    /// — a kind that shares another's name is a kind the operator cannot tell
    /// apart, which defeats the point of the readout.
    #[test]
    fn every_object_kind_has_a_distinct_name_and_a_badge() {
        use object_summary::ObjectKind;
        let mut names = BTreeSet::new();
        for kind in ObjectKind::ALL {
            let name = ui_text::object_kind_label(kind);
            let badge = ui_text::object_kind_badge(kind);
            assert!(!name.is_empty(), "{kind:?} has no name");
            // A badge is one or two characters — anything longer will not fit
            // the chip drawn at a selection's corner.
            assert!(
                (1..=2).contains(&badge.chars().count()),
                "{kind:?}'s badge {badge:?} will not fit the chip"
            );
            assert!(names.insert(name), "duplicate kind name on {kind:?}");
        }
    }

    /// The end-to-end answer to the operator's report: selecting a text object
    /// must produce a readout that SAYS the box is approximate and why.
    ///
    /// This is the case §0.2 of the ui-spec identified as the most likely real
    /// cause of "a box highlighting that doesn't seem to correspond to
    /// anything".
    ///
    /// `decompose` (as opposed to `decompose_page`) resolves no fonts, so
    /// this exercises the [`TextBoundsBasis::EmBox`] fallback deliberately:
    /// the sentence shown for it is the one that must NOT reassure, because
    /// that box really can miss the glyphs it claims to bound.
    #[test]
    fn a_text_selection_readout_explains_its_approximate_box() {
        use object_summary::ObjectNote;
        use pdfce_core::content::ContentStream;
        use pdfce_core::vector::{Matrix, NoXObjects, TextBoundsBasis, decompose};

        let cs = ContentStream::parse(b"BT /F1 12 Tf 40 40 Td (Hi) Tj ET".to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let summary = describe_object(&objects.objects[0]);

        let readout = ui_text::selection_readout_single(&summary, None);
        assert!(readout.contains("Text"), "{readout:?}");
        // The size is stated, so an operator can compare the box on screen
        // against the number and see for themselves that they disagree.
        assert!(readout.contains("pt at ("), "{readout:?}");
        let note_kind = ObjectNote::ApproximateTextBounds(TextBoundsBasis::EmBox);
        assert!(
            summary.notes.contains(&note_kind),
            "{:?}", // ui-text-exempt: test assertion payload, never displayed
            summary.notes
        );
        let note = ui_text::object_note(note_kind);
        // The two facts §E.3 makes binding for the fallback sentence: it
        // names the miss direction, and it does NOT reassure that a click
        // was correct anyway.
        assert!(note.contains("MISS"), "{note:?}");
        assert!(
            !note.contains("selection is correct"),
            "the fallback sentence must not reassure: {note:?}"
        );
    }

    /// The degenerate case found while observing the running app: a horizontal
    /// rule (the only object in `dimension/linear-base.pdf`) selects correctly
    /// and its outline rect is exactly zero high, so it strokes nothing.
    ///
    /// The readout must state the TRUE size — `200.0 × 0.0 pt` — even though
    /// the outline on screen has been thickened to be visible, so the operator
    /// is never left inferring the object's extent from a box that has been
    /// deliberately enlarged.
    #[test]
    fn a_zero_height_rule_reports_its_true_size_and_explains_the_thickened_box() {
        use object_summary::{Degeneracy, ObjectNote};
        use pdfce_core::content::ContentStream;
        use pdfce_core::vector::{Matrix, NoXObjects, decompose};

        let cs = ContentStream::parse(b"100 200 m 300 200 l S".to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let summary = describe_object(&objects.objects[0]);
        assert_eq!(summary.size(), Some((200.0, 0.0)));

        let readout = ui_text::selection_readout_single(&summary, None);
        assert!(readout.contains("200.0 × 0.0 pt"), "{readout:?}");
        let note = ui_text::object_note(ObjectNote::DegenerateBounds(Degeneracy::HorizontalRule));
        assert!(note.contains("zero height"), "{note:?}");
        // And the outline the canvas would stroke for it is no longer
        // invisible — the fix, asserted at the seam the overlay calls.
        let grown = canvas::visible_outline_rect(
            egui::Rect::from_min_max(egui::pos2(100.0, 200.0), egui::pos2(300.0, 200.0)),
            canvas::MIN_OUTLINE_EXTENT_PX,
        );
        assert!(grown.height() >= canvas::MIN_OUTLINE_EXTENT_PX);
    }

    /// The single-source-of-truth guarantee, asserted structurally rather than
    /// trusted: the Objects panel's row and the status-bar readout are two
    /// renderings of ONE [`object_summary::ObjectSummary`], so the detail
    /// clause of the row must appear verbatim inside the readout.
    ///
    /// This is the divergence `object_provider.rs`'s own module docs cite
    /// decision 011 about, one layer up. If someone later re-derives either
    /// description from the `VectorObject` instead of the summary, the two
    /// will drift and this fails.
    #[test]
    fn the_objects_row_and_the_status_readout_describe_one_object_identically() {
        use pdfce_core::content::ContentStream;
        use pdfce_core::vector::{Matrix, NoXObjects, decompose};

        let cs = ContentStream::parse(b"0 0 1 rg 2 w 10 10 80 80 re B".to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let summary = describe_object(&objects.objects[0]);

        let row = ui_text::object_row(7, &summary);
        let readout = ui_text::selection_readout_single(&summary, None);
        // The shared clause: kind, paint disposition, colour, width, nodes.
        let clause = row
            .split_once("  ")
            .map(|(_, rest)| rest.to_owned())
            .expect("the row carries its index then its description");
        assert!(
            readout.contains(&clause),
            "row clause {clause:?} is absent from readout {readout:?}"
        );
        // And the facts themselves actually made it through.
        assert!(clause.contains("#0000FF"), "{clause:?}");
        assert!(clause.contains("node"), "{clause:?}");
    }

    /// **ui-spec §B.3's illustrative rows, now actually buildable.** The
    /// spec's own examples were `Text · "Section A-A" · Helvetica 10pt` and
    /// `Image · 640×480`, and §B.4 recorded both as owed core work because
    /// neither was buildable from the model as it stood. This asserts the
    /// debt is paid at the surface the operator reads — and, just as
    /// importantly, that it is paid with the FILE's values rather than the
    /// spec's illustrative ones.
    #[test]
    fn the_text_and_image_rows_now_carry_the_detail_the_ui_spec_asked_for() {
        let mut app = PdfceApp::default();
        app.open_path(fixture("vector/mixed.pdf"));
        let Status::Open(doc) = &mut app.status else {
            panic!("the fixture did not open");
        };
        doc.ensure_object_provider();
        let objects = &doc
            .object_model
            .as_ref()
            .expect("the fixture page decomposes")
            .page_objects()
            .objects;

        let rows: Vec<String> = objects
            .iter()
            .enumerate()
            .map(|(index, object)| ui_text::object_row(index, &describe_object(object)))
            .collect();

        // Row 1 is the text object: its string and its typeface, both from
        // the file. Before §B.4's core work this row could only say "Text".
        assert!(rows[1].contains("Text"), "{rows:?}");
        assert!(
            rows[1].contains('"'),
            "the text row carries no quoted string: {rows:?}"
        );
        assert!(
            rows[1].contains("Helvetica"),
            "the text row does not name its typeface: {rows:?}"
        );
        assert!(rows[1].contains("pt"), "{rows:?}");

        // Row 2 is the image XObject: its sample count (§8.9.5 Table 89).
        // The fixture's image is 2x2 DeviceGray.
        assert!(rows[2].contains("Image"), "{rows:?}");
        assert!(
            rows[2].contains("2 × 2 px"),
            "the image row does not carry its pixel dimensions: {rows:?}"
        );
    }

    /// The single-source-of-truth guarantee, extended to the NEW detail: a
    /// text row's string/font clause must appear verbatim in the status
    /// readout, exactly as the path clause already does. Two renderings of
    /// one `ObjectSummary`, never two descriptions.
    #[test]
    fn the_text_row_detail_also_appears_verbatim_in_the_readout() {
        let mut app = PdfceApp::default();
        app.open_path(fixture("vector/mixed.pdf"));
        let Status::Open(doc) = &mut app.status else {
            panic!("the fixture did not open");
        };
        doc.ensure_object_provider();
        let objects = &doc
            .object_model
            .as_ref()
            .expect("decomposes")
            .page_objects()
            .objects;
        let summary = describe_object(&objects[1]);
        assert_eq!(summary.kind, object_summary::ObjectKind::Text);

        let row = ui_text::object_row(1, &summary);
        let readout = ui_text::selection_readout_single(&summary, None);
        let clause = row
            .split_once("  ")
            .map(|(_, rest)| rest.to_owned())
            .expect("the row carries its index then its description");
        assert!(
            readout.contains(&clause),
            "row clause {clause:?} is absent from readout {readout:?}"
        );
    }

    /// **The cycling disclosure (ui-spec §C.3, rule 4).** A click into a
    /// stack of overlapping objects must SAY that it was a stack, so a
    /// repeat click that cycled is distinguishable from a mis-hit — which is
    /// the exact confusion this whole line of work exists to end.
    ///
    /// Also pins the negative: a lone object gets no "1 of 1" noise on a
    /// status bar that is one line by design.
    #[test]
    fn the_readout_discloses_that_a_click_landed_in_a_stack() {
        use pdfce_core::content::ContentStream;
        use pdfce_core::vector::{Matrix, NoXObjects, decompose};

        let cs =
            ContentStream::parse(b"40 40 20 20 re f 0 0 100 100 re f".to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let summary = describe_object(&objects.objects[1]);

        // A plain click into a 3-deep stack reports where it landed…
        let readout = ui_text::selection_readout_single(&summary, Some((1, 2)));
        assert!(readout.contains("1 of 2"), "{readout:?}");
        // …and names the gesture that reaches the rest, because a modifier
        // nobody discovers is not a feature.
        assert!(readout.contains("Alt+click"), "{readout:?}");

        // A cycled click says which one it is now on.
        let cycled = ui_text::selection_readout_single(&summary, Some((2, 2)));
        assert!(cycled.contains("2 of 2"), "{cycled:?}");

        // A lone object: nothing to disclose, nothing added.
        let lone = ui_text::selection_readout_single(&summary, Some((1, 1)));
        assert_eq!(lone, ui_text::selection_readout_single(&summary, None));
        assert_eq!(ui_text::hit_cycle_clause(1, 1), None);
    }

    /// Colours are rendered as `#RRGGBB`, and out-of-range components are
    /// clamped at DISPLAY time rather than repaired in the model — the
    /// decomposition records what the content stream actually said.
    #[test]
    fn colours_render_as_clamped_hex() {
        use pdfce_core::vector::Rgb;
        assert_eq!(ui_text::rgb_hex(Rgb::BLACK), "#000000");
        assert_eq!(
            ui_text::rgb_hex(Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0
            }),
            "#FFFFFF"
        );
        // Out of gamut in both directions: clamped, never wrapped.
        assert_eq!(
            ui_text::rgb_hex(Rgb {
                r: 2.0,
                g: -1.0,
                b: 0.5
            }),
            "#FF0080"
        );
    }

    /// **The Objects panel agrees with `pdfce-cli object-list`.**
    ///
    /// There is already a headless oracle for exactly the data this panel
    /// displays: `object-list` prints the same `decompose_page` walk with the
    /// same paint-order indices, and its own doc comment names it a
    /// "diagnostic oracle for GUI selection". This test pins the agreement so
    /// a future change to either side fails here rather than being discovered
    /// by an operator comparing two answers.
    ///
    /// The expected values are the literal output of
    ///
    /// ```text
    /// pdfce-cli object-list fixtures/synthetic/vector/mixed.pdf --page 1
    /// object page=1 index=0 kind=path  bbox=20,20,280,20  subpaths=1 anchors=2 …
    /// object page=1 index=1 kind=text  bbox=30,147.102,70.46,160.052  approximate=1 bounds=font-metrics
    /// object page=1 index=2 kind=image bbox=30,250,90,290  source=xobject
    /// object-list … objects=3 paths=1 text=1 images=1 forms=0
    /// ```
    ///
    /// The text row's bbox is the accumulated advances of `(Vector)` in
    /// Helvetica 14 from `30 150 Td` — 40.46 pt wide, 10.05 above the
    /// baseline and 2.90 below. It read `16,136,44,164` (a 28 × 28 pt square
    /// centred on the pen start) before the advances were accumulated.
    ///
    /// run against this fixture. The panel reaches the model through
    /// `ObjectModelProvider`, which the CLI does not use — but both call the
    /// SAME `decompose_page`, which is why they can be compared at all and
    /// why a divergence would mean something had grown a second walk (the Z2
    /// failure decision 011 warns against).
    #[test]
    fn the_object_tree_agrees_with_the_object_list_oracle() {
        let mut app = PdfceApp::default();
        app.open_path(fixture("vector/mixed.pdf"));
        let Status::Open(doc) = &mut app.status else {
            panic!("the fixture did not open");
        };
        doc.ensure_object_provider();
        let provider = doc
            .object_model
            .as_ref()
            .expect("the fixture page decomposes");
        let objects = &provider.page_objects().objects;

        assert_eq!(objects.len(), 3, "object-list reports objects=3");
        // Paint order, index by index, exactly as the oracle prints it.
        assert!(matches!(
            objects[0],
            pdfce_core::vector::VectorObject::Path(_)
        ));
        assert!(matches!(
            objects[1],
            pdfce_core::vector::VectorObject::Text(_)
        ));
        assert!(matches!(
            objects[2],
            pdfce_core::vector::VectorObject::Image(_)
        ));

        // The text object's bbox, to the same numbers the oracle prints —
        // the value that would silently diverge first if either side ever
        // grew its own text-layout arithmetic.
        let pdfce_core::vector::VectorObject::Text(text) = &objects[1] else {
            panic!("index 1 is the text object");
        };
        assert_eq!(
            text.bounds_basis,
            pdfce_core::vector::TextBoundsBasis::FontMetrics
        );
        assert!((text.page_bbox.min.x - 30.0).abs() < 1e-6, "{text:?}");
        assert!((text.page_bbox.max.x - 70.46).abs() < 1e-3, "{text:?}");
        assert!((text.page_bbox.min.y - 147.102).abs() < 1e-3, "{text:?}");
        assert!((text.page_bbox.max.y - 160.052).abs() < 1e-3, "{text:?}");

        // The rows the panel would draw, top to bottom. Front-most first, so
        // the image (painted last) heads the list and the stroked line
        // (painted first) is at the bottom — the reverse of the oracle's
        // print order, by design (§B.2), with the SAME index numbers.
        let rows: Vec<String> = (0..objects.len())
            .map(|row| {
                let index = objects.len() - 1 - row;
                ui_text::object_row(index, &describe_object(&objects[index]))
            })
            .collect();
        assert!(
            rows[0].starts_with("#2") && rows[0].contains("Image"),
            "{rows:?}"
        );
        assert!(
            rows[1].starts_with("#1") && rows[1].contains("Text"),
            "{rows:?}"
        );
        assert!(
            rows[2].starts_with("#0") && rows[2].contains("Path"),
            "{rows:?}"
        );
        // The oracle says `paint=stroke` for index 0; the row must say the
        // same thing in the panel's own words, not a different one.
        assert!(rows[2].contains("stroked"), "{rows:?}");

        // And the display-row arithmetic the scroll-reveal uses agrees with
        // the ordering the rows were built with.
        assert_eq!(display_row_for_target(TargetId(2), 3), 0);
        assert_eq!(display_row_for_target(TargetId(0), 3), 2);
    }

    /// A tree row click and a canvas click must produce the SAME selection —
    /// they are the same function (`canvas::selection_after_click`), and this
    /// pins that they stay so. A second, divergent selection path is the
    /// specific thing ui-spec §B.5 forbids.
    #[test]
    fn tree_selection_and_canvas_selection_are_one_operation() {
        let empty = BTreeSet::new();
        let plain = canvas::selection_after_click(&empty, Some(TargetId(2)), false);
        assert_eq!(plain, BTreeSet::from([TargetId(2)]));
        // Shift adds...
        let added = canvas::selection_after_click(&plain, Some(TargetId(0)), true);
        assert_eq!(added, BTreeSet::from([TargetId(0), TargetId(2)]));
        // ...and Shift on an already-selected row removes, which is the
        // canvas's own toggle convention, mirrored rather than reinvented.
        let removed = canvas::selection_after_click(&added, Some(TargetId(2)), true);
        assert_eq!(removed, BTreeSet::from([TargetId(0)]));
    }

    /// The toolbar's Properties control must report what is ON SCREEN.
    ///
    /// Its selected state is `tools_open && Properties is the front tab`,
    /// derived from the dock rather than from a flag of its own — the
    /// retired `properties_open` boolean could disagree with the screen, and
    /// a toggle that lies about its own state is worse than no toggle.
    #[test]
    fn the_properties_toggle_reports_what_is_on_screen() {
        let mut app = PdfceApp::default();
        // Dock closed: Properties is not on screen, whatever the tree says.
        assert!(!app.tools_open);
        assert!(dock::panel_is_active(&app.dock, DockPanel::Properties));
        let showing =
            |a: &PdfceApp| a.tools_open && dock::panel_is_active(&a.dock, DockPanel::Properties);
        assert!(!showing(&app));

        app.tools_open = true;
        assert!(showing(&app));

        // Bringing Batch Tools forward hides Properties behind it, and the
        // toggle must follow — this is the exact disagreement the old
        // boolean was capable of.
        dock::activate(&mut app.dock, DockPanel::BatchTools);
        assert!(!showing(&app));
        // ...while the object tree above the split stays visible throughout.
        assert!(dock::panel_is_active(&app.dock, DockPanel::Objects));
    }

    /// "Reset panel layout" must restore the DEFAULT arrangement, not merely
    /// some arrangement — including the Objects-above-Properties split that
    /// decision 017 A.3 makes the point of the whole default.
    #[test]
    fn resetting_the_layout_restores_the_default_arrangement() {
        let mut app = PdfceApp::default();
        dock::activate(&mut app.dock, DockPanel::BatchTools);
        assert!(!dock::panel_is_active(&app.dock, DockPanel::Properties));

        app.dock = dock::default_tree();
        assert!(dock::panel_is_active(&app.dock, DockPanel::Objects));
        assert!(dock::panel_is_active(&app.dock, DockPanel::Properties));
    }

    // ---- Pass 19.4: the word-spacing row ----

    /// The R83 gate the word-spacing row's SHAPE depends on comes from
    /// provenance, not from anything the shell works out for itself.
    ///
    /// This is the R74 assertion for this slice: if the GUI ever grew its
    /// own notion of "is this run composite", the panel could offer a
    /// control on a run core would refuse — or, worse, withhold one on a run
    /// where `Tw` works. Asserting the flag arrives from
    /// `GlyphProvenance::composite` keeps the two answers the same answer.
    #[test]
    fn the_word_spacing_gate_reads_the_published_composite_flag() {
        use pdfce_core::text_extract::{ExtractOptions, extract_page};

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic/textedit/format_color.pdf");
        let bytes = std::fs::read(path).expect("fixture");
        let doc = pdfce_core::document::Document::from_bytes(bytes).expect("parse");
        let pages = pdfce_core::page_tree::pages(&doc).expect("page tree");
        let page = pages.first().expect("one page");
        let page_text = extract_page(
            &doc,
            page,
            0,
            &ExtractOptions::default().with_provenance(true),
        )
        .expect("extract");
        let prov = page_text
            .runs
            .first()
            .and_then(|r| r.glyphs.first())
            .and_then(|g| g.provenance.as_ref())
            .expect("provenance is captured");

        let a = AmbientSnapshot::from_provenance(prov);
        assert_eq!(
            a.composite, prov.composite,
            "the snapshot must CARRY the published flag, not recompute one"
        );
        assert!(
            !a.composite,
            "this fixture's /Calibri is a simple font, so the row is live here"
        );
    }

    /// The word-spacing field's unit switch preserves the physical quantity,
    /// exactly as the tracking field's does — the two rows share
    /// `per_mille`/`per_mille_to_operand` rather than each carrying their
    /// own conversion, and this pins that they agree with core's resolver.
    #[test]
    fn the_word_spacing_unit_switch_preserves_the_quantity() {
        let a = snapshot(12.0);
        // 200‰ of a 12 pt em is 2.4 unscaled text-space units.
        assert!((a.per_mille_to_operand(200.0) - 2.4).abs() < 1e-12);
        assert!((a.per_mille(2.4) - 200.0).abs() < 1e-9);
        assert!(
            (pdfce_core::text_edit::MetricSpec::Relative(200.0).resolve(12.0) - 2.4).abs() < 1e-12
        );
    }

    /// The GUI's word-spacing intent reaches core as a word-spacing request
    /// and not as some neighbouring one. Cheap, and it is the exact class of
    /// mistake a five-arm `match` invites.
    #[test]
    fn the_word_spacing_op_builds_a_word_spacing_request() {
        use pdfce_core::text_edit::{FormatRequest, MetricSpec};
        let r = FormatRequest::new(0, "a b").word_spacing(MetricSpec::Relative(200.0));
        assert_eq!(r.set_word_spacing, Some(MetricSpec::Relative(200.0)));
        assert!(
            r.set_char_spacing.is_none(),
            "word spacing must not land in the tracking field"
        );
    }

    /// Every refusal gets a next step (§7.3's rule), including the one the
    /// panel is built not to be able to provoke — and this one's next step
    /// must point at reflow, the mechanism that actually works on a
    /// composite run.
    #[test]
    fn the_composite_word_spacing_refusal_offers_reflow_as_the_next_step() {
        let err = pdfce_core::text_edit::FormatError::WordSpacingComposite {
            base_font: "ABCDEF+NotoSans".to_owned(),
        };
        let hint = format_refusal_hint(&err);
        assert!(hint.contains("Reflow"), "the remedy is named: {hint}");
        assert_ne!(
            hint,
            ui_text::edit_generic_hint(),
            "this refusal must not fall through to the generic hint"
        );
    }
}
