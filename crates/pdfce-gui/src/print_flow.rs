//! The print flow: the dialog, its preview, and the spool call.
//!
//! # Printing is the one action in pdfce with no undo
//!
//! Everything else this application does can be reverted, closed without
//! saving, or corrected before a save. A print marks paper, occupies a
//! device somebody else may share, and cannot be taken back. That single
//! fact decides most of what is in this file: why the dialog is its own
//! stationary surface rather than a dock pane, why the preview shows the
//! printable RECTANGLE and not just the sheet, why Enter does not commit,
//! and why no keyboard chord spools.
//!
//! # The dialog IS the confirmation
//!
//! The CLI defaults to a dry run and requires `--send`. That is right for
//! a scriptable tool whose operator is not watching, and wrong here: a
//! GUI whose premise is that the operator is looking at the settings does
//! not also need them to confirm the settings. Decision 024 §4.4 records
//! what that friction felt like from the other side.
//!
//! What replaces a second gate is disclosure with teeth — the clip count
//! is in the BUTTON'S OWN LABEL, so the uncertainty is stated in the
//! disclosure rather than implied by a confirm step existing (rule 4).

use crate::{Action, PdfceApp, Status, ui_text};
use eframe::egui;

/// Width of the preview column, in egui points.
///
/// # ★ A CONSTANT, and that is the whole point
///
/// The dialog body lives inside a [`egui::ScrollArea::both`], and a
/// horizontally scrollable area has no bounded width to report: anything
/// laid out from `ui.available_width()` inside one is being sized from a
/// number that the scroll area is itself deriving from the content. Two
/// fixed columns break that circle — the content width is a constant, so
/// the horizontal scrollbar has something stable to measure and the
/// operator gets a scrollbar instead of a column that grows to meet it.
const PREVIEW_COLUMN_WIDTH_PTS: f32 = 340.0;

/// Width of the options column, in egui points. Same reasoning as
/// [`PREVIEW_COLUMN_WIDTH_PTS`]; sized to hold the longest radio label in
/// the three tabs without wrapping.
const OPTIONS_COLUMN_WIDTH_PTS: f32 = 400.0;

/// The dialog body's natural content width, in egui points: both columns
/// plus the separator and the two item gaps around it.
///
/// Stated as a constant rather than measured from the laid-out row,
/// because it is what the scrolling body is told to be — see the
/// `set_width` call in `print_dialog` for why measuring instead produces
/// a body that fits by squeezing a column rather than by scrolling.
const BODY_CONTENT_WIDTH_PTS: f32 = PREVIEW_COLUMN_WIDTH_PTS + OPTIONS_COLUMN_WIDTH_PTS + 24.0;

/// Height of the fixed strip under the preview canvas, in egui points.
///
/// # ★ FIXED, so the canvas can never be shrunk by its own caption
///
/// The canvas height is computed as `available − this constant`. Reading
/// the strip's ACTUAL laid-out height instead would reproduce R128's
/// feedback loop exactly: the clip caption wraps to two lines on a narrow
/// window, the strip grows, the canvas shrinks, the sheet is refitted
/// smaller — and the operator watches the preview settle over several
/// frames for no reason they can see. Subtracting a constant means the
/// strip's content cannot reach the canvas at all.
///
/// The window-resize coupling is the DESIRED one and is unaffected: a
/// taller window still means a taller canvas.
const PREVIEW_STRIP_HEIGHT_PTS: f32 = 68.0;

/// Smallest preview canvas, in egui points. Below this the sheet outline
/// stops being a picture and becomes a smudge, so the column scrolls
/// rather than shrinking further.
const PREVIEW_CANVAS_MIN_HEIGHT_PTS: f32 = 160.0;

/// Largest preview canvas, in egui points.
///
/// A ceiling rather than a preference. `ui.available_height()` inside a
/// scroll area is a value this code does not own; clamping both ends
/// means a surprising answer from egui produces a preview that is merely
/// the wrong size rather than one that allocates a screen-sized rect.
const PREVIEW_CANVAS_MAX_HEIGHT_PTS: f32 = 1400.0;

/// Height reserved under the scrolling body for the Cancel/Print row.
///
/// The footer is drawn AFTER the scroll area, so the scroll area must be
/// told not to eat the whole window. Reserved as a constant for the same
/// reason [`PREVIEW_STRIP_HEIGHT_PTS`] is: the commit button's position
/// must not depend on how much the body happens to contain this frame.
const FOOTER_HEIGHT_PTS: f32 = 46.0;

/// Resolution the preview bitmap is rendered at, in DPI.
///
/// Chosen against what the preview is FOR — checking that fine print
/// clears the unprintable margin — rather than against the size it is
/// first drawn at. At fit the bitmap is heavily downsampled, and that
/// headroom is what lets the operator zoom in and still see type rather
/// than a mosaic. It is deliberately NOT the job's own render DPI: the
/// job renders at up to 2400 DPI and a preview does not need a 500 MB
/// pixmap to answer a margin question.
const PRINT_PREVIEW_TARGET_DPI: f32 = 150.0;

/// Ceiling on the preview bitmap's longest side, in pixels.
///
/// The DPI figure alone is not a bound: an ISO A0 sheet is 3370 pt on its
/// long side, which at 150 DPI is 7020 px and 190 MB of RGBA. This clamp
/// holds the worst case near 20 MB regardless of page size, which matters
/// because large-format CAD sheets are exactly the document population
/// this project's operator prints.
///
/// **Set ABOVE the office page sizes on purpose.** The first value tried
/// was 1600, which is below US Legal (2100 px) and below US Letter's own
/// 1650 — so the "ceiling for exotic sheets" silently became the scale
/// for every ordinary document, quietly costing preview sharpness on the
/// common case to bound the rare one. 2200 leaves A4 (1754), Letter
/// (1650) and Legal (2100) at the full target DPI and binds only where it
/// was meant to. `a_letter_page_previews_at_the_target_resolution` is the
/// test that caught it and is what keeps the two constants in step.
const PRINT_PREVIEW_MAX_SIDE_PX: f32 = 2200.0;

/// Smallest and largest preview zoom, as a multiple of the fit scale.
///
/// Bounded on BOTH sides because zoom is driven by a wheel: an unbounded
/// multiplier reached by a flick leaves the operator staring at one white
/// pixel with no way back except the Fit button they may not have found.
const PREVIEW_ZOOM_MIN: f32 = 0.25;
/// See [`PREVIEW_ZOOM_MIN`].
const PREVIEW_ZOOM_MAX: f32 = 40.0;

/// Which group of settings the dialog is showing.
///
/// # Why tabs at all
///
/// The dialog grew past what one column can hold, and the previous answer
/// was a `CollapsingHeader` labelled "More options" holding orientation,
/// duplex, copies, collation, reverse, subset, tray selection, annotation
/// scope and the DPI override. That is not progressive disclosure, it is
/// a drawer: a control's location told the operator nothing about what it
/// did, everything below the fold was equally invisible, and the header's
/// label promised no more than "there is additional stuff".
///
/// Three tabs replace it, each named for the QUESTION it answers, so a
/// control's location is itself a hint. The printer selector deliberately
/// stays OUTSIDE the tabs: it gates whether the duplex and tray controls
/// exist at all, so hiding it behind a tab would let the operator change
/// the device without seeing which one they had changed it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PrintTab {
    /// Range, subset, sizing, orientation — which pages, and how each
    /// lands on the sheet.
    #[default]
    PagesLayout,
    /// Copies, collation, reverse order, duplex, tray selection — how
    /// many sheets come out and in what state.
    CopiesFinishing,
    /// Annotation scope and rendering resolution — what is painted onto
    /// each page, and how finely.
    CommentsResolution,
}

impl PrintTab {
    /// Every tab, in the order the strip draws them.
    ///
    /// An array rather than three literal calls at the draw site, so that
    /// adding a tab is one edit and cannot leave the strip and the
    /// content branch disagreeing about how many there are.
    pub(crate) const ALL: [Self; 3] = [
        Self::PagesLayout,
        Self::CopiesFinishing,
        Self::CommentsResolution,
    ];

    /// The tab's label.
    fn label(self) -> &'static str {
        match self {
            Self::PagesLayout => ui_text::print_tab_pages_layout(),
            Self::CopiesFinishing => ui_text::print_tab_copies_finishing(),
            Self::CommentsResolution => ui_text::print_tab_comments_resolution(),
        }
    }

    /// The tab's hover text.
    fn tooltip(self) -> &'static str {
        match self {
            Self::PagesLayout => ui_text::print_tab_pages_layout_tooltip(),
            Self::CopiesFinishing => ui_text::print_tab_copies_finishing_tooltip(),
            Self::CommentsResolution => ui_text::print_tab_comments_resolution_tooltip(),
        }
    }
}

/// What a cached preview bitmap is a picture OF.
///
/// # Every field here is something that changes the pixels
///
/// A cache key is a claim: "if these are equal, re-rendering would
/// produce the same image." Getting it wrong in the lax direction is the
/// bug class `PageTexture`'s five staleness keys were each added to close
/// — a control that changes the render, does not change the key, and
/// therefore silently does nothing.
///
/// **Orientation is deliberately absent, and the REASON changed even
/// though the conclusion did not.** It used to be justified by
/// "`plan_job` never reads [`pdfce_print::DeviceSettings`]" — which was
/// true, and was the defect: the sheet turned in the driver while
/// planning stayed upright, so a landscape job printed at about 77% of
/// correct size. Orientation now reaches planning through
/// [`pdfce_print::DeviceGeometry::for_orientation`], so it DOES change
/// [`pdfce_print::Placement::scale`] and the rectangle the preview draws.
///
/// It still changes no pixel of THIS bitmap. The texture is rasterised at
/// [`preview_raster_scale`], which is derived from the page's own size
/// and the preview's target DPI and never from the placement — the
/// placement scales the drawn rectangle, not the raster. Nothing here
/// rotates page content either: the driver turns the sheet, pdfce does
/// not turn the page. So the key stays as it is, and putting orientation
/// in it would throw the cache away on every radio click for nothing.
///
/// Everything the preview needs that is NOT the dialog's own state.
///
/// # Why a struct rather than eight parameters
///
/// The preview reads from four different places — the open document, the
/// application's font and colour settings, the device's capabilities, and
/// the job plan — and the alternative is an eight-argument function with
/// an `#[allow(clippy::too_many_arguments)]` on it. Grouping them also
/// makes the borrow situation legible: `print_dialog` holds `&mut
/// self.pending_print` and these are all reads of DISJOINT fields, which
/// is the only reason the call compiles at all.
struct PreviewInputs<'a> {
    /// The open document, for its pages and its edited view.
    doc: &'a crate::OpenDoc,
    /// Operator-supplied faces (decision 012).
    fonts: &'a pdfce_render::FontEnvironment,
    /// `PdfceApp::font_env_generation`, for the cache key.
    font_generation: u64,
    /// The operator's CMYK conversion choice (R169).
    cmyk_intent: pdfce_core::settings::CmykIntent,
    /// Real device geometry TURNED FOR THIS JOB, or `None` when the
    /// driver would not answer.
    ///
    /// ★ Not `PrinterCaps`. The preview drew its sheet, its printable
    /// rectangle and its margins straight from the raw capabilities,
    /// which is what made the Orientation radio appear to do nothing —
    /// [`pdfce_print::printer_caps`] reports the device's DEFAULT
    /// `DEVMODE`, so on a portrait-default printer the preview drew a
    /// portrait sheet no matter what the operator selected. Taking the
    /// same [`pdfce_print::DeviceGeometry`] the job was PLANNED against
    /// is what makes the picture and the paper the same claim.
    geometry: Option<&'a pdfce_print::DeviceGeometry>,
    /// Page sizes in DOCUMENT order — indexed by
    /// [`pdfce_print::PagePlan::index`], never by a position in
    /// [`Self::plans`].
    page_sizes: &'a [(f64, f64)],
    /// The job, in the order it will be sent.
    plans: &'a [pdfce_print::PagePlan],
    /// How many pages of the job will lose content off an edge.
    clipped: usize,
}

/// The pt-to-pixel scale a preview bitmap is rendered at.
///
/// # Two bounds, and the second one is the load-bearing one
///
/// [`PRINT_PREVIEW_TARGET_DPI`] alone would be a scale, not a bound: it
/// says how finely to render a point and says nothing about how many
/// points there are. An ANSI E sheet is 2448 × 3168 pt, which at 150 DPI
/// is 5100 × 6600 px and 134 MB of RGBA for a picture drawn 300 pt wide.
/// [`PRINT_PREVIEW_MAX_SIDE_PX`] holds that near 15 MB, and it binds on
/// exactly the large-format documents this project's operator prints
/// while leaving every office page size at full resolution.
///
/// The result depends only on the page's own size, so it is fully
/// determined by [`PreviewKey::page`] and does not need to be a key field
/// of its own.
/// The preview zoom and pan after multiplying the zoom by `step` while
/// holding the screen point `at` still.
///
/// # ★ The anchor term, derived rather than tuned
///
/// The sheet is drawn at `origin(z) = centre − sheet·fit·z/2 + pan`.
/// Holding the screen point `at` fixed across a zoom from `z0` to
/// `z1 = k·z0` requires `at − (at − origin(z0))·k = origin(z1)`.
/// Substituting both origins collapses every `sheet` and `fit` term:
///
/// ```text
/// pan1 = (at − centre)·(1 − k) + k·pan0
/// ```
///
/// That the page geometry drops out is what makes this correct for a
/// sheet of any size, and it is why the anchor is computed rather than
/// arrived at by nudging the pan until it looked right. Without the
/// anchor, zooming in on the bottom-left corner of a sheet walks it off
/// the canvas and the operator has to hunt it back with a drag.
///
/// A button click passes `at == centre`, which degenerates to
/// `pan1 = k·pan0` — the sheet grows about the middle of the canvas,
/// which is where the operator is looking when they press a button rather
/// than pointing at something.
///
/// # `k` is the EFFECTIVE ratio, after clamping
///
/// Using `step` for the anchor term instead would displace the sheet on a
/// zoom the clamp refused: at maximum zoom, Ctrl+wheel would stop
/// magnifying but keep sliding the page sideways, which reads as the
/// preview drifting on its own.
///
/// A non-finite or non-positive `step` returns the inputs unchanged.
/// egui's `zoom_delta()` is well-behaved, but this is the function a
/// future gesture source would also call, and a `NaN` reaching
/// `preview_pan` poisons every subsequent frame's arithmetic with no way
/// back except closing the dialog.
fn zoomed_view(
    zoom: f32,
    pan: egui::Vec2,
    step: f32,
    at: egui::Pos2,
    centre: egui::Pos2,
) -> (f32, egui::Vec2) {
    if !step.is_finite() || step <= 0.0 || !zoom.is_finite() || zoom <= 0.0 {
        return (zoom, pan);
    }
    let after = (zoom * step).clamp(PREVIEW_ZOOM_MIN, PREVIEW_ZOOM_MAX);
    let k = after / zoom;
    (after, (at - centre) * (1.0 - k) + k * pan)
}

fn preview_raster_scale(page_pt: (f64, f64)) -> f32 {
    let dpi_scale = PRINT_PREVIEW_TARGET_DPI / 72.0;
    let longest = page_pt.0.max(page_pt.1) as f32;
    if !longest.is_finite() || longest <= 0.0 {
        // A degenerate `/MediaBox`. The renderer has its own guards; this
        // only has to avoid handing it a division by zero.
        return dpi_scale;
    }
    dpi_scale.min(PRINT_PREVIEW_MAX_SIDE_PX / longest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreviewKey {
    /// Which document page (0-based).
    page: usize,
    /// Which annotation classes are painted (§12.5).
    scope: pdfce_render::AnnotationScope,
    /// [`PdfceApp::font_env_generation`] — supplying a font folder must
    /// invalidate the preview, exactly as it invalidates the canvas.
    fonts: u64,
    /// The operator's CMYK conversion choice (R169).
    cmyk: pdfce_core::settings::CmykIntent,
}

/// Parse `3`, `1-4`, `5,1-2` into zero-based indices.
///
/// # Deliberately the same syntax the CLI accepts
///
/// Two range parsers would eventually disagree about something like
/// `5,1-2` — whether it reorders, whether it deduplicates — and an
/// operator moving between the GUI and a script would have no way to
/// know which one they were talking to. The syntax is kept identical and
/// the behaviour on malformed input is the same: an unparseable range
/// yields NOTHING rather than a guess, so the Print button disables and
/// says why instead of printing a range nobody asked for.
pub(crate) fn parse_page_range(spec: &str, count: usize) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((a, b)) => {
                let a: usize = a.trim().parse().ok()?;
                let b: usize = b.trim().parse().ok()?;
                if a == 0 || b == 0 || a > b || b > count {
                    return None;
                }
                out.extend((a - 1)..b);
            }
            None => {
                let n: usize = part.parse().ok()?;
                if n == 0 || n > count {
                    return None;
                }
                out.push(n - 1);
            }
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Which pages a print job covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrintRange {
    /// Every page.
    All,
    /// The page on screen.
    Current,
    /// A typed range, parsed with the SAME parser the CLI uses.
    Custom,
}

/// The print dialog's live state.
///
/// # Why a `pending_*` field rather than a panel
///
/// Printing is a single transaction with a start and an end, not
/// something an operator dips in and out of while working — which is
/// what a dock pane is for. It follows `pending_copy` and
/// `pending_redaction_apply`: the same idiom, the third instance, not a
/// new convention.
pub(crate) struct PendingPrint {
    /// Printers as the spooler reported them when the dialog opened.
    ///
    /// Read ONCE rather than per frame. Enumerating printers touches the
    /// spooler, and doing it sixty times a second while a dialog sits
    /// open would be rude to a service other applications share.
    pub(crate) printers: Vec<pdfce_print::Printer>,
    /// Index into [`Self::printers`].
    selected: usize,
    /// Which pages.
    range: PrintRange,
    /// The typed range, live even when [`PrintRange::Custom`] is not
    /// selected, so switching away and back does not lose it.
    range_text: String,
    /// How each page is sized onto the sheet.
    scale: pdfce_print::ScaleMode,
    /// The custom percentage, kept across mode switches for the same
    /// reason as `range_text`.
    custom_percent: u32,
    /// Which classes of annotation print.
    ///
    /// # This was one checkbox until the renderer could back four
    ///
    /// `RenderOptions` carried a single `bool`, so the dialog offered a
    /// single honestly-labelled toggle rather than Acrobat's four-way
    /// selector — a control implying a capability that does not exist is
    /// R83's failure even when the control itself works. The renderer
    /// gained `AnnotationScope`, so the selector is backed and offered.
    ///
    /// Defaulted to `Document` for PRINTING, which differs from the
    /// renderer's own `DocumentAndMarkups` default. Deliberate on both
    /// sides: the canvas should show markup, and a print should not
    /// carry review comments unless asked. Acrobat Pro defaults the
    /// other way and Reader defaults to `Document`; pdfce takes
    /// Reader's here, because a comment reaching paper unasked is the
    /// costlier mistake.
    pub(crate) scope: pdfce_render::AnnotationScope,
    /// Rendering resolution ceiling, in DPI. A memory bound, editable
    /// because the disclosure is worth more as a control than a warning.
    max_dpi: u32,
    /// Driver-level settings: orientation, duplex, tray choice.
    pub(crate) device: pdfce_print::DeviceSettings,
    /// What the device says it can do, read once when the dialog opens.
    pub(crate) features: pdfce_print::DeviceFeatures,
    /// Odd/even filtering.
    pub(crate) subset: pdfce_print::PageSubset,
    /// Print back to front.
    pub(crate) reverse: bool,
    /// Copy count.
    pub(crate) copies: u16,
    /// Copy ordering, as the checkbox holds it.
    pub(crate) uncollated: bool,
    /// Which page the preview shows.
    preview_page: usize,
    /// Which group of settings is on screen.
    ///
    /// Lives on the dialog rather than on the application, so closing the
    /// dialog forgets it. That is the right lifetime: the tab an operator
    /// last used is a fact about the job they were configuring, and
    /// re-opening the dialog for a different job should start where the
    /// dialog's own default says, not where an unrelated job ended.
    pub(crate) active_tab: PrintTab,
    /// Preview magnification, as a multiple of the fit scale. `1.0` is
    /// fit.
    ///
    /// Expressed relative to fit rather than as an absolute pt-per-pt
    /// scale so that resizing the window keeps whatever the operator
    /// chose: at `1.0` a taller window shows a bigger sheet, and at `3.0`
    /// it shows the same detail, bigger. An absolute scale would make the
    /// preview drift out of the canvas every time the window changed.
    pub(crate) preview_zoom: f32,
    /// How far the sheet is displaced from centred, in egui points.
    ///
    /// Applied AFTER centring, so `Vec2::ZERO` always means "centred at
    /// the current zoom" and the Fit button is a two-field reset rather
    /// than a recomputation.
    pub(crate) preview_pan: egui::Vec2,
    /// The rendered page bitmap behind the preview, and what it is a
    /// picture of.
    ///
    /// `None` until the first successful render, and set back to `None`
    /// when a render fails — in which case the preview falls back to the
    /// flat fill it drew before this cache existed, which still shows the
    /// GEOMETRY correctly. A preview that shows the right rectangle and
    /// no content is degraded; one that shows a stale page is wrong.
    preview_texture: Option<(PreviewKey, egui::TextureHandle)>,
    /// The last spool attempt's outcome, once there is one.
    pub(crate) outcome: Option<Result<pdfce_print::SpoolReport, String>>,
    /// The plan the dialog is currently showing, refreshed every frame.
    ///
    /// Held here rather than carried on the action because `Action` is
    /// `Copy` and a plan is a `Vec`. That constraint pushed the design
    /// somewhere better: the action becomes a bare "the operator pressed
    /// Print" signal, and the state that decides WHAT prints has exactly
    /// one home — so there is no way for the action and the dialog to
    /// describe different jobs.
    pub(crate) plans: Vec<pdfce_print::PagePlan>,
    /// The resolved printer name for those plans.
    pub(crate) printer_name: Option<String>,
}

impl PdfceApp {
    /// Open the print dialog.
    ///
    /// Enumerating printers can block briefly on a network spooler, so it
    /// happens here — once, on a deliberate click — rather than inside
    /// the frame loop.
    ///
    /// # ★ Two guards, at the ONE place the dialog is ever built
    ///
    /// `Ctrl+P` (Pass 63.0) made this reachable from a chord as well as
    /// from the ribbon button. The ribbon button is wrapped in
    /// `add_enabled_ui(has_doc, …)` and cannot be pressed twice in a
    /// frame; a chord has neither property. Rather than duplicate the
    /// button's condition at the keymap — where the file's own pattern is
    /// "push the chord blind, gate the effect in dispatch", and where a
    /// third caller would have to remember to copy it — both conditions
    /// are enforced here, which fixes the ribbon and the chord by
    /// construction:
    ///
    /// - **No document, no dialog.** Without this, `Ctrl+P` on an empty
    ///   canvas enumerates the spooler (a blocking call on a network
    ///   printer) to populate a window that [`Self::print_dialog`] closes
    ///   again on its very next frame, because that function returns early
    ///   when the status is not [`Status::Open`].
    /// - **Already open means leave it alone.** This function REBUILDS
    ///   `PendingPrint` from defaults. A second press part-way through
    ///   configuring a job would silently reset the range, the scale, the
    ///   copy count and the annotation scope — the operator's own
    ///   settings, discarded by the shortcut they pressed to look at
    ///   them.
    pub(crate) fn open_print_dialog(&mut self) {
        if !matches!(self.status, Status::Open(_)) {
            return;
        }
        if self.pending_print.is_some() {
            return;
        }
        let printers = pdfce_print::list_printers().unwrap_or_default();
        let selected = printers.iter().position(|p| p.is_default).unwrap_or(0);
        // Read ONCE, here. A duplex control must not appear for a device
        // that cannot duplex (R83), and asking the driver that question
        // sixty times a second while a dialog sits open would be rude to
        // a service other applications share.
        let features = printers
            .get(selected)
            .and_then(|p| pdfce_print::device_features(&p.name).ok())
            .unwrap_or_default();
        let preview_page = match &self.status {
            Status::Open(doc) => doc.view.page_index,
            _ => 0,
        };
        self.pending_print = Some(PendingPrint {
            printers,
            selected,
            range: PrintRange::All,
            range_text: String::new(),
            scale: pdfce_print::ScaleMode::Fit,
            custom_percent: 100,
            scope: pdfce_render::AnnotationScope::Document,
            max_dpi: 300,
            device: pdfce_print::DeviceSettings::default(),
            features,
            subset: pdfce_print::PageSubset::All,
            reverse: false,
            copies: 1,
            uncollated: false,
            preview_page,
            active_tab: PrintTab::default(),
            // Fit, centred. Both are reset here rather than carried over
            // from a previous dialog for the same reason `active_tab` is:
            // a zoom chosen while inspecting page 4 of last week's job
            // says nothing about this one.
            preview_zoom: 1.0,
            preview_pan: egui::Vec2::ZERO,
            preview_texture: None,
            outcome: None,
            plans: Vec::new(),
            printer_name: None,
        });
    }

    /// The print dialog.
    ///
    /// # ★ This dialog IS the confirmation. There is no second gate.
    ///
    /// The CLI defaults to a dry run and requires `--send`, which is
    /// right for a scriptable tool whose operator is not watching. It
    /// does not transfer to a GUI whose whole premise is that they are.
    /// A second confirm in front of settings the operator just configured
    /// and can see in full is the friction decision 024 §4.4 corrected —
    /// and this is a stationary, screen-anchored surface reached by a
    /// deliberate click, not a box whose position moves with the page.
    ///
    /// Two guards are inherited verbatim from the redaction-apply dialog,
    /// because the reasoning is identical and re-deriving it would risk
    /// arriving somewhere else:
    ///
    /// - **Enter does not print.** An operator reading a dialog and
    ///   pressing Enter out of habit must not commit the one action in
    ///   this application with no undo. There is a text field here, which
    ///   makes the habit likelier, not less.
    /// - **No keyboard chord commits.** `Ctrl+P` opens this dialog and
    ///   nothing spools a job. Reversible actions get chords; the
    ///   irreversible one does not.
    ///
    /// The clip warning lives in the BUTTON LABEL rather than beside it,
    /// because rule 4 asks for the uncertainty to be stated in the
    /// disclosure itself rather than implied by a confirm step existing.
    pub(crate) fn print_dialog(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(pending) = self.pending_print.as_mut() else {
            return;
        };
        let Status::Open(doc) = &self.status else {
            self.pending_print = None;
            return;
        };

        // Everything the dialog needs about the device, computed once per
        // frame from the current selection.
        let printer_name = pending
            .printers
            .get(pending.selected)
            .map(|p| p.name.clone());
        let caps = printer_name
            .as_deref()
            .and_then(|n| pdfce_print::printer_caps(n).ok());

        let page_sizes: Vec<(f64, f64)> = doc
            .pages
            .iter()
            .map(|p| {
                let mb = p.media_box;
                ((mb.urx - mb.llx).abs(), (mb.ury - mb.lly).abs())
            })
            .collect();

        let selected_pages: Vec<usize> = match pending.range {
            PrintRange::All => (0..page_sizes.len()).collect(),
            PrintRange::Current => vec![doc.view.page_index],
            // The SAME parser the CLI uses, not a second one — two range
            // parsers would eventually disagree about `5,1-2`, and the
            // operator would have no way to know which they were using.
            PrintRange::Custom => {
                parse_page_range(&pending.range_text, page_sizes.len()).unwrap_or_default()
            }
        };

        let mode = match pending.scale {
            pdfce_print::ScaleMode::Custom(_) => {
                pdfce_print::ScaleMode::Custom(f64::from(pending.custom_percent) / 100.0)
            }
            other => other,
        };
        let spec = pdfce_print::JobSpec {
            pages: selected_pages.clone(),
            mode,
            max_dpi: pending.max_dpi,
            subset: pending.subset,
            reverse: pending.reverse,
            copies: pending.copies,
            collate: if pending.uncollated {
                pdfce_print::Collate::Uncollated
            } else {
                pdfce_print::Collate::Collated
            },
        };
        // ★ The device geometry is TURNED for this job before anything is
        // planned against it. `printer_caps` reports the device's default
        // `DEVMODE`, so on a portrait-default printer it hands back a
        // portrait printable area — and a landscape job prints on a sheet
        // the driver has turned. Planning against the un-turned area
        // under-scales every page to about 77% of correct size with a wide
        // empty margin, and reports no clip, so nothing says it happened.
        //
        // The orientation and the first page come from the same place the
        // `DEVMODE` will: `pending.device.orientation` and the first page
        // the job SENDS (not `pages[0]` — the sequence may be reversed).
        let geometry = caps.as_ref().map(|c| {
            pdfce_print::DeviceGeometry::from_caps(
                c,
                pending.device.orientation,
                spec.first_page_pt(&page_sizes),
            )
        });
        let plans = geometry
            .as_ref()
            .map(|g| pdfce_print::plan_job(g, &page_sizes, &spec))
            .unwrap_or_default();
        let resolution = geometry
            .as_ref()
            .map(|g| pdfce_print::job_resolution(g, &spec));
        let clipped = plans.iter().filter(|p| p.placement.clipped).count();

        // Read here, from the app, so the two columns can be plain
        // associated functions taking exactly what they need. `pending`
        // already holds a `&mut` borrow of `self.pending_print`, and these
        // are disjoint fields — the borrow checker permits it, and keeping
        // the reads together at the top makes that non-obvious fact
        // legible rather than something a future edit trips over.
        let fonts = &self.font_env;
        let font_generation = self.font_env_generation;
        let cmyk_intent = self.settings.cmyk_intent;
        let danger = self.theme.palette.danger;

        let mut open = true;
        let mut do_print = false;
        let window = egui::Window::new(ui_text::print_dialog_title())
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 620.0])
            // A floor, not a preference. `resizable(true)` without one lets
            // the operator drag the window down to a title bar and a
            // scrollbar, which is a state with no way back except closing
            // it — and closing this dialog discards the job they were
            // configuring. The floor is the smallest size at which one
            // column and both scrollbars are still usable.
            .min_size([520.0, 380.0])
            // Anchored to the SCREEN, never to the document. Decision 024
            // §4.4: the operator's objection was to controls whose
            // position is derived from the page and therefore move on
            // every zoom and scroll.
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                if pending.printers.is_empty() {
                    ui.label(ui_text::print_no_printers());
                    return;
                }
                // The body scrolls in BOTH directions, and exactly one
                // scroll area owns it.
                //
                // Both axes because the window is resizable in both and the
                // operator's complaint was about either dimension being too
                // small; a vertical-only area answers half of it and leaves
                // the other half cut off with no indication that anything is
                // missing.
                //
                // ONE area, wrapping the whole body rather than one per
                // column, because a nested scroll area raises "which
                // consumer owns the wheel over this pixel" — a question with
                // no verified answer in this project's egui notes, and the
                // preview deliberately owns no scroll area of its own for
                // the same reason (see `print_preview_column`).
                //
                // `max_height` reserves the footer. With
                // `auto_shrink([false, false])` the area fills whatever it
                // is given, so without the reservation it would take the
                // whole window and push the Print button off the bottom of a
                // dialog whose entire purpose is that button.
                let body_height = (ui.available_height() - FOOTER_HEIGHT_PTS).max(200.0);
                crate::diag::trace(|| {
                    format!(
                        "print-body avail=({:.0},{:.0}) body_h={body_height:.0}",
                        ui.available_width(),
                        ui.available_height()
                    )
                });
                // ★ `max_width` is NOT optional, and leaving it out is the
                // exact failure this Pass measured.
                //
                // A `ScrollArea` decides whether to show a bar by comparing
                // its CONTENT size against its VIEWPORT size, and with
                // `auto_shrink` off on the x axis it takes its viewport
                // width from `ui.available_width()`. Inside an
                // `egui::Window`, `Resize` measures its content by laying it
                // out with generous space first, so on the frame that
                // matters `available_width` is not the window's real width —
                // the area concludes 750 pt of columns fit, shows no
                // horizontal bar, and the window's own max-size clamp
                // (`Window::constrain`, on by default) then CLIPS the
                // options column against the screen edge. Observed at a
                // 700x520 viewport: the third tab's label and the
                // even-pages radio were cut off with no scrollbar anywhere.
                //
                // Pinning both dimensions from the same `ui` makes the
                // comparison honest in both axes. This is the concrete form
                // of the general hazard the design flagged: content sized
                // from `available_width()` inside a horizontally scrollable
                // area is sized from a number the area is itself deriving.
                let body_width = ui.available_width();
                // ★ SOLID SCROLLBARS, not egui's floating default.
                //
                // `ScrollStyle::default()` is `floating()`: a 2 pt sliver
                // that allocates no space and fades out when the pointer is
                // elsewhere. Functionally the body scrolls either way — but
                // the operator's report was that a too-small dialog cuts
                // content off, and a scrollbar nobody can see does not
                // answer it. Measured during this Pass: at a 700x520
                // viewport the body was scrolling correctly in both axes
                // and looked, in a screenshot, exactly like content clipped
                // at the window edge.
                //
                // Scoped to this `ui` rather than set on the application
                // style, because it is an answer to THIS surface's problem —
                // a dialog whose content genuinely does not fit at the
                // sizes it can be dragged to. The canvas and the docks are
                // not in that position and should keep egui's default.
                //
                // `foreground_color` on top of `solid()`, and that second
                // step is not cosmetic either. A solid handle defaults to
                // `widgets.inactive.bg_fill`, which in pdfce's light preset
                // is a near-white against a near-white panel: measured on a
                // capture, the bar was present, opaque, correctly sized —
                // and invisible. `foreground_color` draws the handle from
                // the same visuals' TEXT colour instead, so it inherits
                // whatever contrast the active theme gives its text rather
                // than needing a colour of its own here.
                let mut scroll = egui::style::ScrollStyle::solid();
                scroll.foreground_color = true;
                scroll.bar_width = 10.0;
                ui.style_mut().spacing.scroll = scroll;
                let body = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .max_height(body_height)
                    .max_width(body_width)
                    .id_salt("print-dialog-body")
                    .show(ui, |ui| {
                        // ★ THE HORIZONTAL SCROLLBAR DOES NOT APPEAR
                        // WITHOUT THIS LINE. Measured, not reasoned.
                        //
                        // `allocate_ui_with_layout` clamps its requested
                        // size to whatever space is LEFT
                        // (`Placer::next_space`). So in a viewport narrower
                        // than the two columns, the first column takes its
                        // 340 and the second is silently squeezed into the
                        // remainder — 328 instead of 400. The row then
                        // measures exactly the viewport width, the scroll
                        // area concludes everything fits, no bar is drawn,
                        // and the options column's right-hand controls are
                        // clipped against the window edge with nothing
                        // saying so. Observed at a 700x520 viewport: the
                        // "Comments & Resolution" tab label and the
                        // even-pages radio were cut in half.
                        //
                        // `set_width` grows `max_rect` as well as
                        // `min_rect` (`Placer::set_max_width`), which is
                        // what makes the second column get its real width
                        // and the scroll area see content wider than its
                        // viewport. `max` rather than a bare assignment so
                        // a wide window still fills, rather than leaving a
                        // dead strip to the right of the options.
                        ui.set_width(BODY_CONTENT_WIDTH_PTS.max(body_width));
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(PREVIEW_COLUMN_WIDTH_PTS, body_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    Self::print_preview_column(
                                        ui,
                                        &PreviewInputs {
                                            doc,
                                            fonts,
                                            font_generation,
                                            cmyk_intent,
                                            geometry: geometry.as_ref(),
                                            plans: &plans,
                                            page_sizes: &page_sizes,
                                            clipped,
                                        },
                                        pending,
                                        body_height,
                                    );
                                },
                            );
                            ui.separator();
                            ui.allocate_ui_with_layout(
                                egui::vec2(OPTIONS_COLUMN_WIDTH_PTS, body_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    Self::print_options_column(
                                        ui,
                                        pending,
                                        resolution,
                                        doc.pages.len(),
                                    );
                                },
                            );
                        });
                    });
                crate::diag::trace(|| {
                    format!(
                        "print-body content=({:.0},{:.0}) inner=({:.0},{:.0})",
                        body.content_size.x,
                        body.content_size.y,
                        body.inner_rect.width(),
                        body.inner_rect.height()
                    )
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(ui_text::print_cancel()).clicked() {
                        actions.push(Action::CancelPrint);
                    }
                    let label = if clipped > 0 {
                        ui_text::print_button_clipping(clipped)
                    } else {
                        ui_text::print_button().to_owned()
                    };
                    let enabled = geometry.is_some() && !plans.is_empty();
                    if ui
                        .add_enabled(enabled, egui::Button::new(label))
                        .on_disabled_hover_text(ui_text::print_button_why_disabled())
                        .clicked()
                    {
                        do_print = true;
                    }
                    if let Some(outcome) = &pending.outcome {
                        match outcome {
                            Ok(report) => ui.label(ui_text::print_sent(report.pages)),
                            Err(msg) => ui.label(
                                egui::RichText::new(ui_text::print_failed(msg)).color(danger),
                            ),
                        };
                    }
                });
            });

        // The window's own outer rect, in screen points. Traced for the
        // same reason the preview canvas is (R172): a harness that has to
        // drag a resize grip cannot be told to guess where it is, and the
        // two harness scripts do not share a viewport size, so a corner
        // read off a screenshot is not transferable to the one that
        // injects the drag.
        crate::diag::trace(|| {
            let rect = window.as_ref().map(|w| w.response.rect);
            format!(
                "print-window rect={:?}",
                rect.map(|r| (r.min.x, r.min.y, r.width(), r.height()))
            )
        });
        crate::diag::trace(|| {
            format!(
                "print-plan printer={:?} pages={} clipped={} dpi={:?} orientation={:?} \
                 scale={:?}",
                printer_name,
                plans.len(),
                clipped,
                resolution.map(|r| r.dpi),
                pending.device.orientation,
                // The FIRST plan's scale, which is the number the
                // orientation defect moved. Traced beside the orientation
                // that produced it so the two can be read together: a
                // radio that changes `orientation=` and not `scale=` on a
                // landscape page is the regression, restated.
                plans.first().map(|p| p.placement.scale),
            )
        });
        // The preview's own state, traced separately from the plan because
        // it answers a different question: the plan line says what WILL
        // print, this one says whether the operator can currently SEE it.
        // `tex=0` with a plan present is precisely the regression that
        // would put the flat-fill placeholder back without anything else
        // looking wrong.
        crate::diag::trace(|| {
            format!(
                "print-preview tab={:?} page={} zoom={:.3} pan=({:.1},{:.1}) tex={}",
                pending.active_tab,
                pending.preview_page,
                pending.preview_zoom,
                pending.preview_pan.x,
                pending.preview_pan.y,
                u8::from(pending.preview_texture.is_some()),
            )
        });
        // Refreshed every frame so the action never has to carry them.
        pending.plans = plans;
        pending.printer_name = printer_name;
        if do_print {
            actions.push(Action::SpoolPrint);
        }
        if !open {
            actions.push(Action::CancelPrint);
        }
    }
}

impl PdfceApp {
    /// The preview: what the sheet will actually look like.
    ///
    /// # Why a picture rather than a number
    ///
    /// pdfce diverges from Acrobat here on purpose — Acrobat clips
    /// silently when content falls outside the printable area, and pdfce
    /// says so. That divergence is worth nothing if the GUI reduces it to
    /// a count an operator can look past.
    ///
    /// The whole reason `pdfce-print` reads real device geometry instead
    /// of guessing a bounding box is so this can be exact. Drawing only
    /// the SHEET and not the PRINTABLE AREA would be the naive version
    /// and would show a page fitting that will not.
    ///
    /// # What changed in Pass 63.0, and why it was not a bug fix
    ///
    /// This function computed real device geometry from the first day and
    /// still never drew the page: the "placed" rectangle was a flat fill
    /// in the surface colour. So the preview was not broken — the geometry
    /// was right — it simply answered "where will the page sit" and never
    /// "what is on it", which is the half an operator checking a margin
    /// actually needs. The bitmap is rendered through the SAME
    /// [`PdfceApp::print_render_options`] the spooler uses, because a
    /// preview built from a second, independently-written options builder
    /// is a preview that can be confidently wrong.
    ///
    /// # ★ The preview owns NO scroll area, deliberately
    ///
    /// Zoom is Ctrl+wheel and pan is a primary-button drag. Neither
    /// competes with the dialog's own [`egui::ScrollArea`]: per
    /// `D:\dev\rag\egui\egui_0.35_zoom_with_keyboard_vs_app_zoom_chords.md`
    /// egui splits wheel input at the input-state level, so a wheel event
    /// carrying the zoom modifier surfaces as `zoom_delta()` and
    /// contributes nothing to `smooth_scroll_delta()` — the two cannot
    /// fire from one gesture. A plain wheel over the preview therefore
    /// belongs unambiguously to the dialog, and there is no nested
    /// consumer to race it. Scroll-to-pan was rejected for exactly that
    /// reason: it would have made the preview a scroll consumer and put
    /// the question back.
    fn print_preview_column(
        ui: &mut egui::Ui,
        inputs: &PreviewInputs<'_>,
        pending: &mut PendingPrint,
        column_height: f32,
    ) {
        let (plans, page_sizes, clipped) = (inputs.plans, inputs.page_sizes, inputs.clipped);
        let Some(device) = inputs.geometry else {
            ui.label(ui_text::print_device_unavailable());
            return;
        };
        // Which plan the preview shows. The stepper walks the SELECTED
        // pages, not the document's, because a preview of a page the job
        // does not include would be answering a question nobody asked.
        let shown = pending.preview_page.min(plans.len().saturating_sub(1));
        if plans.is_empty() {
            ui.label(ui_text::print_no_pages_selected());
            return;
        }

        // The canvas takes whatever the column has, MINUS a constant. See
        // `PREVIEW_STRIP_HEIGHT_PTS` for why the constant rather than the
        // strip's measured height, and `PREVIEW_CANVAS_MAX_HEIGHT_PTS` for
        // why the result is clamped at both ends.
        let canvas_height = (column_height - PREVIEW_STRIP_HEIGHT_PTS)
            .clamp(PREVIEW_CANVAS_MIN_HEIGHT_PTS, PREVIEW_CANVAS_MAX_HEIGHT_PTS);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(PREVIEW_COLUMN_WIDTH_PTS, canvas_height),
            // `click_and_drag` rather than `drag`: a click that does not
            // move must still mark the canvas hovered-and-interacted, which
            // is what gates the Ctrl+wheel read below.
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let theme = crate::theme::Theme::of(ui.ctx());

        // Fit the SHEET into the preview box, preserving aspect. This is
        // recomputed every frame from the CURRENT rect on purpose: a
        // taller window should show a bigger sheet, and that coupling is
        // the desired half of R128, not the hazardous half — `rect` is
        // derived from a constant, so nothing the strip draws can feed
        // back into it.
        let sheet = device.physical_pt;
        let fit = (rect.width() / sheet.0 as f32).min(rect.height() / sheet.1 as f32) * 0.92;

        // ---- zoom and pan, before anything is drawn from them ----------
        //
        // Ctrl+wheel, gated on hover so it cannot steal the gesture from a
        // sibling control. Zoom is anchored on the POINTER: without the
        // anchor term, zooming in on the bottom-left corner of a sheet
        // walks it off screen and the operator has to hunt it back with a
        // drag.
        if response.hovered() {
            let step = ui.input(|i| i.zoom_delta());
            if (step - 1.0).abs() > f32::EPSILON {
                let at = response.hover_pos().unwrap_or_else(|| rect.center());
                Self::zoom_preview(pending, step, at, rect.center());
            }
        }
        // ★ `dragged_by(Primary)`, never bare `dragged()`. Per
        // `D:\dev\rag\egui\egui_response_drag_predicates_are_button_agnostic.md`
        // the unqualified predicate fires for middle and right drags too,
        // which would silently claim the right-drag this preview may later
        // want for a context menu.
        if response.dragged_by(egui::PointerButton::Primary) {
            pending.preview_pan += response.drag_delta();
        }

        let s = fit * pending.preview_zoom;
        let sheet_px = egui::vec2(sheet.0 as f32 * s, sheet.1 as f32 * s);
        let origin = rect.center() - sheet_px / 2.0 + pending.preview_pan;
        let sheet_rect = egui::Rect::from_min_size(origin, sheet_px);
        painter.rect_filled(sheet_rect, 2.0, theme.palette.label_backdrop);
        painter.rect_stroke(
            sheet_rect,
            2.0,
            egui::Stroke::new(1.0, theme.palette.outline),
            egui::StrokeKind::Middle,
        );

        // The printable area, inset by the driver's own unprintable
        // margins. This is the rectangle that actually constrains the
        // job.
        let printable = egui::Rect::from_min_size(
            origin + egui::vec2(device.offset_pt.0 as f32 * s, device.offset_pt.1 as f32 * s),
            egui::vec2(
                device.printable_pt.0 as f32 * s,
                device.printable_pt.1 as f32 * s,
            ),
        );
        painter.rect_stroke(
            printable,
            0.0,
            egui::Stroke::new(1.0, theme.palette.guide),
            egui::StrokeKind::Middle,
        );

        // The placed page.
        //
        // ★ `page_sizes` is indexed by `plan.index`, NOT by `shown`.
        // `shown` walks the JOB (which may be a custom range, odd/even
        // filtered, or reversed) and `page_sizes` is in document order, so
        // the two coincide only for a whole-document forward job. Indexing
        // it by `shown` — as this did until Pass 63.0 — drew the placed
        // rectangle at the size of a page the job may not even contain,
        // which on a document mixing sheet sizes is a preview that reports
        // a clip that will not happen or misses one that will.
        if let Some(plan) = plans.get(shown)
            && let Some(&size) = page_sizes.get(plan.index)
        {
            let placed = egui::Rect::from_min_size(
                printable.min
                    + egui::vec2(
                        plan.placement.offset_x_pt as f32 * s,
                        plan.placement.offset_y_pt as f32 * s,
                    ),
                egui::vec2(
                    (size.0 * plan.placement.scale) as f32 * s,
                    (size.1 * plan.placement.scale) as f32 * s,
                ),
            );
            // The rendered page, if one is cached. The fallback is the flat
            // fill this drew before Pass 63.0 — a preview showing the right
            // rectangle and no content is degraded but honest; one showing
            // a stale page would be wrong.
            let texture = Self::preview_texture(ui.ctx(), inputs, pending, plan.index);
            if let Some(texture) = texture {
                // NOT A THEME COLOUR: a pass-through tint for an
                // already-rendered page bitmap. `painter.image` MULTIPLIES
                // the texture by this value, so white means "draw the
                // pixels as rendered". These are document content, not
                // chrome — restyling the application must not restyle the
                // operator's page, and any palette role here would do
                // exactly that.
                painter.image(
                    texture,
                    placed,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    // NOT A THEME COLOUR: pass-through tint, see above.
                    egui::Color32::WHITE,
                );
            } else {
                painter.rect_filled(placed, 0.0, theme.palette.surface);
            }
            painter.rect_stroke(
                placed,
                0.0,
                egui::Stroke::new(1.0, theme.palette.text_muted),
                egui::StrokeKind::Middle,
            );
            // What will be lost, hatched. Pass 8's grammar: a hatch means
            // "this will happen and has not happened yet", which is
            // exactly a pre-print clip. A solid fill would read as
            // something already done.
            if plan.placement.clipped {
                let lost = placed
                    .intersect(egui::Rect::everything_right_of(printable.max.x))
                    .union(placed.intersect(egui::Rect::everything_below(printable.max.y)));
                if lost.is_positive() {
                    let step = 6.0;
                    let mut x = lost.min.x;
                    while x < lost.max.x + lost.height() {
                        painter.line_segment(
                            [
                                egui::pos2(x.min(lost.max.x), lost.min.y),
                                egui::pos2((x - lost.height()).max(lost.min.x), lost.max.y),
                            ],
                            egui::Stroke::new(1.0, theme.palette.preview),
                        );
                        x += step;
                    }
                }
            }
        }

        // ---- the fixed strip ------------------------------------------
        //
        // One row: page stepper on the left, zoom controls on the right.
        // They share a row because they are both "what am I looking at"
        // controls and because two rows plus the clip caption would not
        // fit the fixed strip — and the strip's height is fixed for
        // R128's reason, so the layout has to live inside it rather than
        // the other way round.
        ui.horizontal(|ui| {
            if ui.button(ui_text::find_previous_label()).clicked() {
                pending.preview_page = pending.preview_page.saturating_sub(1);
                // A different page is a different picture, so the zoom and
                // pan the operator chose for the last one no longer mean
                // anything — on a differently-sized sheet they would put
                // the new page off screen. Reset rather than carry.
                Self::reset_preview_view(pending);
            }
            ui.label(ui_text::print_preview_position(shown + 1, plans.len()));
            if ui.button(ui_text::find_next_label()).clicked() && shown + 1 < plans.len() {
                pending.preview_page = shown + 1;
                Self::reset_preview_view(pending);
            }
            ui.separator();
            if ui
                .button(ui_text::print_preview_zoom_fit())
                .on_hover_text(ui_text::print_preview_zoom_fit_tooltip())
                .clicked()
            {
                Self::reset_preview_view(pending);
            }
            // Buttons as well as the wheel gesture, and kept even though
            // Ctrl+wheel exists: the commonest reason to zoom a print
            // preview is checking that fine print clears the margin, which
            // is a deliberate look at a known amount of magnification, not
            // a scrub. A gesture is faster and a button is findable.
            if crate::PdfceApp::icon_button(
                ui,
                crate::icons::Icon::ZoomOut,
                ui_text::print_preview_zoom_out_tooltip(),
            )
            .clicked()
            {
                Self::zoom_preview(pending, 1.0 / 1.25, rect.center(), rect.center());
            }
            if crate::PdfceApp::icon_button(
                ui,
                crate::icons::Icon::ZoomIn,
                ui_text::print_preview_zoom_in_tooltip(),
            )
            .clicked()
            {
                Self::zoom_preview(pending, 1.25, rect.center(), rect.center());
            }
            // Actual size means one PDF point drawn as one egui point, so
            // the multiplier that gets there is `1 / fit` — the number the
            // percentage readout will then show as 100%.
            if ui
                .button(ui_text::print_preview_zoom_actual())
                .on_hover_text(ui_text::print_preview_zoom_actual_tooltip())
                .clicked()
                && fit > 0.0
            {
                Self::zoom_preview(pending, 1.0 / s, rect.center(), rect.center());
            }
        });
        ui.horizontal(|ui| {
            // The scale as a percentage of ACTUAL size, not of the fit —
            // see `ui_text::print_preview_zoom_percent` for why a number
            // that changes when the window is dragged would be useless.
            // Clamped and rounded before the cast: `PREVIEW_ZOOM_MIN` and
            // `PREVIEW_ZOOM_MAX` bound the multiplier and `fit` is a ratio
            // of two positive lengths, so the product cannot be negative
            // or large enough to saturate — but the clamp is written
            // rather than argued, because a degenerate `physical_pt`
            // of zero would make `fit` infinite and a cast of infinity is
            // a silent zero.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let percent = (s * 100.0).round().clamp(0.0, 100_000.0) as u32;
            ui.label(
                egui::RichText::new(ui_text::print_preview_zoom_percent(percent))
                    .small()
                    .weak(),
            );
            ui.label(
                egui::RichText::new(ui_text::print_preview_pan_hint())
                    .small()
                    .weak(),
            );
        });
        // The canvas rectangle, in screen points.
        //
        // Traced because R172 forbids guessing harness coordinates and the
        // two harness scripts use different window sizes, so a point read
        // off a screenshot is not transferable to the one that injects
        // events. This is the only honest source for "where do I aim a
        // Ctrl+wheel to zoom the preview" — and it is also the fastest way
        // to see that the resize coupling is live, since the height moves
        // with the window.
        // ★ `sheet=` and `printable=` are on this line because they are
        // the only honest evidence that the Orientation radio reaches the
        // geometry. The radio changes no pixel of the page bitmap (see
        // `PreviewKey`) and turns a rectangle whose aspect a screenshot
        // can suggest but not measure. A trace of the two rectangles is
        // what lets a harness assert the turn rather than photograph it.
        crate::diag::trace(|| {
            format!(
                "print-preview-rect canvas=[{:.0},{:.0} {:.0}x{:.0}] fit={:.4} scale={:.4} \
                 sheet={:.0}x{:.0} printable={:.0}x{:.0} margin={:.0},{:.0}",
                rect.min.x,
                rect.min.y,
                rect.width(),
                rect.height(),
                fit,
                s,
                device.physical_pt.0,
                device.physical_pt.1,
                device.printable_pt.0,
                device.printable_pt.1,
                device.offset_pt.0,
                device.offset_pt.1,
            )
        });
        // The count, always, for a multi-page job whose clip is on a page
        // the preview is not showing.
        if clipped > 0 {
            ui.label(
                egui::RichText::new(ui_text::print_clip_summary(clipped, plans.len()))
                    .color(crate::theme::Theme::of(ui.ctx()).palette.notice),
            );
        }
    }

    /// Put the preview back to fit, centred.
    ///
    /// Two fields, one place. The Fit button and both page-stepper
    /// buttons need exactly this, and three copies of `zoom = 1.0; pan =
    /// ZERO` is how a fourth caller ends up resetting only one of them.
    fn reset_preview_view(pending: &mut PendingPrint) {
        pending.preview_zoom = 1.0;
        pending.preview_pan = egui::Vec2::ZERO;
    }

    /// Multiply the preview zoom by `step`, keeping the point `at` still.
    ///
    /// A two-field assignment over [`zoomed_view`], which holds the
    /// arithmetic and is where it is tested — `PendingPrint` carries a
    /// spooler's worth of device state that a test of the anchor term has
    /// no business constructing.
    fn zoom_preview(pending: &mut PendingPrint, step: f32, at: egui::Pos2, centre: egui::Pos2) {
        let (zoom, pan) = zoomed_view(pending.preview_zoom, pending.preview_pan, step, at, centre);
        pending.preview_zoom = zoom;
        pending.preview_pan = pan;
    }

    /// The texture for `page`, rendering and caching it if needed.
    ///
    /// Returns the id rather than the handle so the caller holds no borrow
    /// of `pending` past the call — the alternative is a `&TextureHandle`
    /// living across the rest of a function that also wants `pending`
    /// mutably, which compiles only by accident of statement ordering.
    ///
    /// # Never re-rendered on a frame where nothing changed
    ///
    /// Same discipline as the printer enumeration at the top of this file
    /// and as `PageTexture`'s five staleness keys: a preview that
    /// re-rasterises sixty times a second would make an open dialog cost
    /// more than the print. [`PreviewKey`] carries every input that can
    /// change the pixels; see its docs for why orientation is not one of
    /// them.
    ///
    /// A failed render clears the cache and returns `None`, which drops
    /// the preview back to the flat fill. It is not reported as an error:
    /// the same failure will be reported honestly, once, by the spool
    /// attempt, and a preview that turns into an error banner while the
    /// operator is still choosing a page range is noise in front of a
    /// decision they have not made yet.
    fn preview_texture(
        ctx: &egui::Context,
        inputs: &PreviewInputs<'_>,
        pending: &mut PendingPrint,
        page: usize,
    ) -> Option<egui::TextureId> {
        let key = PreviewKey {
            page,
            scope: pending.scope,
            fonts: inputs.font_generation,
            cmyk: inputs.cmyk_intent,
        };
        if let Some((cached, texture)) = &pending.preview_texture
            && *cached == key
        {
            return Some(texture.id());
        }
        let page_obj = inputs.doc.pages.get(page)?;
        let media = page_obj.media_box;
        let size = ((media.urx - media.llx).abs(), (media.ury - media.lly).abs());
        let options =
            PdfceApp::print_render_options(pending.scope, inputs.fonts.clone(), inputs.cmyk_intent);
        let view = inputs.doc.session.view();
        let rendered = pdfce_render::render_page_with_view(
            &view,
            page_obj,
            preview_raster_scale(size),
            &options,
        )
        .ok();
        let Some(rendered) = rendered else {
            pending.preview_texture = None;
            return None;
        };
        // Uploaded through `raster`'s own helper, so the preview cannot
        // acquire a different premultiplied-alpha convention from the
        // canvas — see that module's header for what silently goes wrong
        // when two call sites each pick their own constructor.
        let texture =
            crate::raster::texture_from_pixmap(ctx, "pdfce-print-preview", &rendered.pixmap);
        let id = texture.id();
        pending.preview_texture = Some((key, texture));
        Some(id)
    }

    /// The render options a print job — and its preview — are drawn with.
    ///
    /// # ★ ONE builder, called from both, and that is the point
    ///
    /// This file already carries the argument in `parse_page_range`'s
    /// docs: two independently-written builders eventually disagree about
    /// something, and neither side can tell which one they are looking at.
    /// For a print preview that failure is the whole feature — a preview
    /// exists to say what will come out of the printer, so a preview built
    /// from its own options is a preview that can be confidently wrong.
    /// It was extracted from `spool_print` when the preview started
    /// rendering content (Pass 63.0).
    ///
    /// The three deliberate choices it encodes:
    ///
    /// - **`view_magnification` stays `None`** — the PRINT answer under
    ///   §8.11.4.5, which says a printing application "shall not apply the
    ///   changes based on usage application dictionaries". Inheriting the
    ///   canvas's options would apply the zoom-driven optional-content
    ///   states the operator happens to be looking at.
    /// - **The operator's layer overrides are NOT applied**, for the same
    ///   clause: they are a viewing choice, and §8.11.4.5 puts printing on
    ///   the document's own default configuration.
    /// - **The operator's CMYK intent IS applied** (R169). This is a
    ///   correction: `spool_print` built its options without it, so a
    ///   document proofed on screen under `Calibrated` printed under
    ///   `NeutralBlack` and nothing said so. R169's whole framing is that
    ///   a choice the standard leaves open travels with every render
    ///   rather than being decided at one call site, and a print is a
    ///   render.
    fn print_render_options(
        scope: pdfce_render::AnnotationScope,
        fonts: pdfce_render::FontEnvironment,
        cmyk_intent: pdfce_core::settings::CmykIntent,
    ) -> pdfce_render::RenderOptions {
        let mut options = pdfce_render::RenderOptions::default()
            .with_annotation_scope(scope)
            .with_cmyk_intent(cmyk_intent);
        options.fonts = fonts;
        options
    }

    /// The options column: the printer, then one of three tabs.
    ///
    /// # ★ The printer selector is OUTSIDE the tabs, always visible
    ///
    /// It is not a setting like the others — it is the thing that decides
    /// which of the others exist. `pending.features` is read from the
    /// selected device and gates the duplex radios and the tray checkbox
    /// (R83), so a tab that could hide the printer name would let the
    /// operator change device, watch controls appear and disappear, and
    /// have no way to see what they had changed it to without going
    /// looking.
    ///
    /// # The tab strip reuses the ribbon's widget, deliberately
    ///
    /// `egui::Button::selectable` plus a bold weight on the active one is
    /// what `PdfceApp::toolbar` already draws for the ribbon's own tabs.
    /// Inventing a different tab affordance for the second tabbed surface
    /// in the application would teach the operator that "tab" looks like
    /// two different things. The bold weight is not decoration: R84
    /// forbids state carried by colour alone.
    fn print_options_column(
        ui: &mut egui::Ui,
        pending: &mut PendingPrint,
        resolution: Option<pdfce_print::JobResolution>,
        page_count: usize,
    ) {
        egui::ComboBox::from_id_salt("print-printer")
            .selected_text(
                pending
                    .printers
                    .get(pending.selected)
                    .map_or_else(String::new, |p| p.name.clone()),
            )
            .show_ui(ui, |ui| {
                for (i, p) in pending.printers.iter().enumerate() {
                    ui.selectable_value(&mut pending.selected, i, &p.name);
                }
            });
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            for tab in PrintTab::ALL {
                let selected = tab == pending.active_tab;
                let text = if selected {
                    egui::RichText::new(tab.label()).strong()
                } else {
                    egui::RichText::new(tab.label())
                };
                if ui
                    .add(egui::Button::selectable(selected, text))
                    .on_hover_text(tab.tooltip())
                    .clicked()
                {
                    pending.active_tab = tab;
                }
            }
        });
        ui.separator();

        match pending.active_tab {
            PrintTab::PagesLayout => Self::print_pages_layout_tab(ui, pending, page_count),
            PrintTab::CopiesFinishing => Self::print_copies_finishing_tab(ui, pending),
            PrintTab::CommentsResolution => {
                Self::print_comments_resolution_tab(ui, pending, resolution);
            }
        }
    }

    /// Tab 1 — which pages, and how each one lands on the sheet.
    ///
    /// # Why the odd/even subset is here and not with the copies
    ///
    /// It is a SELECTION question. "Every page / odd only / even only"
    /// narrows the same set the range radios above it narrow, and the two
    /// compose — a custom range of `1-10` plus Odd prints five sheets. Put
    /// under Copies it would sit next to Reverse and read as a delivery
    /// option, which is what an operator hand-feeding a duplex job would
    /// reasonably but wrongly assume it was.
    ///
    /// Orientation is here for the matching reason: it is a statement
    /// about how the page meets the sheet, which is the same question the
    /// sizing radios answer.
    fn print_pages_layout_tab(ui: &mut egui::Ui, pending: &mut PendingPrint, page_count: usize) {
        ui.label(ui_text::print_pages_heading());
        ui.radio_value(
            &mut pending.range,
            PrintRange::All,
            ui_text::print_range_all(page_count),
        );
        ui.radio_value(
            &mut pending.range,
            PrintRange::Current,
            ui_text::print_range_current(),
        );
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut pending.range,
                PrintRange::Custom,
                ui_text::print_range_custom(),
            );
            if ui
                .add(egui::TextEdit::singleline(&mut pending.range_text).desired_width(120.0))
                .changed()
            {
                // Typing in the box means the operator wants that range;
                // making them also click the radio is the kind of
                // second step that reads as the software not listening.
                pending.range = PrintRange::Custom;
            }
        });
        ui.horizontal(|ui| {
            ui.label(ui_text::print_subset_label());
            for (s, label) in [
                (pdfce_print::PageSubset::All, ui_text::print_subset_all()),
                (pdfce_print::PageSubset::Odd, ui_text::print_subset_odd()),
                (pdfce_print::PageSubset::Even, ui_text::print_subset_even()),
            ] {
                if ui.radio(pending.subset == s, label).clicked() {
                    pending.subset = s;
                }
            }
        });
        ui.add_space(8.0);

        ui.label(ui_text::print_sizing_heading());
        // Four modes, not three. `place_page`'s own test exists because
        // collapsing Fit and Shrink is the natural simplification and it
        // silently blows a business card up to fill a Letter sheet.
        for (mode, label) in [
            (pdfce_print::ScaleMode::Fit, ui_text::print_scale_fit()),
            (
                pdfce_print::ScaleMode::ActualSize,
                ui_text::print_scale_actual(),
            ),
            (
                pdfce_print::ScaleMode::ShrinkOversized,
                ui_text::print_scale_shrink(),
            ),
        ] {
            let selected = pending.scale == mode;
            if ui.radio(selected, label).clicked() {
                pending.scale = mode;
            }
        }
        let custom_selected = matches!(pending.scale, pdfce_print::ScaleMode::Custom(_));
        ui.horizontal(|ui| {
            if ui
                .radio(custom_selected, ui_text::print_scale_custom())
                .clicked()
            {
                pending.scale =
                    pdfce_print::ScaleMode::Custom(f64::from(pending.custom_percent) / 100.0);
            }
            ui.add_enabled(
                custom_selected,
                egui::DragValue::new(&mut pending.custom_percent)
                    .range(1..=1000)
                    .suffix(ui_text::print_percent_suffix()),
            );
        });

        ui.add_space(8.0);

        ui.label(ui_text::print_orientation_heading());
        for (o, label) in [
            (
                pdfce_print::Orientation::Auto,
                ui_text::print_orientation_auto(),
            ),
            (
                pdfce_print::Orientation::Portrait,
                ui_text::print_orientation_portrait(),
            ),
            (
                pdfce_print::Orientation::Landscape,
                ui_text::print_orientation_landscape(),
            ),
        ] {
            if ui.radio(pending.device.orientation == o, label).clicked() {
                pending.device.orientation = o;
            }
        }
    }

    /// Tab 2 — how many sheets come out, and in what state.
    ///
    /// # Why Reverse is here and not with the page range
    ///
    /// It is a DELIVERY question, not a selection one: it changes nothing
    /// about which pages print, only the order they land in the tray, and
    /// the reason to want it is a printer that stacks face-up. That is the
    /// same class of question as collation, which is why the two sit
    /// together — an operator fixing "my stack comes out backwards" looks
    /// in one place.
    ///
    /// The tray checkbox is here for the third variant of the same
    /// argument: it is a request to the DRIVER about hardware, like
    /// duplex, not arithmetic pdfce performs.
    fn print_copies_finishing_tab(ui: &mut egui::Ui, pending: &mut PendingPrint) {
        ui.horizontal(|ui| {
            ui.label(ui_text::print_copies_label());
            ui.add(egui::DragValue::new(&mut pending.copies).range(1..=999));
        });
        ui.checkbox(&mut pending.uncollated, ui_text::print_uncollated());
        ui.checkbox(&mut pending.reverse, ui_text::print_reverse());
        ui.add_space(8.0);

        // ★ R83: no duplex control for a device that cannot duplex. pdfce
        // does NOT simulate it by reordering pages and asking the operator
        // to reinsert the stack — that workflow has a documented
        // mis-assembly failure mode, and offering it as though it were
        // duplex would claim a capability the hardware does not have.
        //
        // Absent rather than disabled: a greyed control implies something
        // the operator could turn on, and no setting in this dialog will
        // ever make this printer two-sided.
        //
        // Note what this means for the tab: on a simplex-only device this
        // tab is SHORTER, not emptier-looking. That is the intended
        // reading — the tab still holds copies, collation and reverse, so
        // it never becomes a tab with nothing in it.
        if pending.features.supports_duplex {
            ui.label(ui_text::print_duplex_heading());
            for (d, label) in [
                (pdfce_print::Duplex::Simplex, ui_text::print_duplex_off()),
                (pdfce_print::Duplex::LongEdge, ui_text::print_duplex_long()),
                (
                    pdfce_print::Duplex::ShortEdge,
                    ui_text::print_duplex_short(),
                ),
            ] {
                if ui.radio(pending.device.duplex == d, label).clicked() {
                    pending.device.duplex = d;
                }
            }
            ui.add_space(8.0);
        }

        ui.checkbox(
            &mut pending.device.pick_tray_by_page_size,
            ui_text::print_pick_tray(),
        );
    }

    /// Tab 3 — what is painted onto each page, and how finely.
    ///
    /// Both halves are about the PIXELS rather than about the paper, which
    /// is what makes them one tab: the annotation scope decides what is in
    /// the bitmap and the resolution decides how much of it survives.
    fn print_comments_resolution_tab(
        ui: &mut egui::Ui,
        pending: &mut PendingPrint,
        resolution: Option<pdfce_print::JobResolution>,
    ) {
        ui.label(ui_text::print_comments_heading());
        for (s, label) in [
            (
                pdfce_render::AnnotationScope::Document,
                ui_text::print_scope_document(),
            ),
            (
                pdfce_render::AnnotationScope::DocumentAndMarkups,
                ui_text::print_scope_markups(),
            ),
            (
                pdfce_render::AnnotationScope::DocumentAndStamps,
                ui_text::print_scope_stamps(),
            ),
            (
                pdfce_render::AnnotationScope::FormFieldsOnly,
                ui_text::print_scope_fields_only(),
            ),
        ] {
            if ui.radio(pending.scope == s, label).clicked() {
                pending.scope = s;
            }
        }
        ui.add_space(8.0);
        // Always true, so a static caption rather than a warning. A banner
        // that fires on every job trains an operator to stop reading
        // banners.
        ui.label(
            egui::RichText::new(ui_text::print_raster_note())
                .small()
                .weak(),
        );
        // Conditional, because it is a per-job substitution: pdfce picked
        // a resolution the operator did not.
        if let Some(res) = resolution
            && res.capped
        {
            ui.add_space(4.0);
            ui.label(ui_text::print_dpi_capped(
                res.dpi,
                res.device_dpi,
                res.uncapped_page_mb(),
            ));
            ui.add(
                egui::DragValue::new(&mut pending.max_dpi)
                    .range(36..=2400)
                    .suffix(ui_text::print_dpi_suffix()),
            );
        }
    }
}

impl PdfceApp {
    /// Render the planned pages and hand them to the spooler.
    ///
    /// # ★ The one place in the GUI that starts a print job
    ///
    /// Reached only from [`Action::SpoolPrint`], which is pushed only by
    /// the Print button. Nothing here runs as a side effect of opening,
    /// previewing, saving or rendering.
    ///
    /// # Print-correct render options, not the canvas's
    ///
    /// The options are built fresh rather than reused from whatever is
    /// driving the canvas. `view_magnification` stays `None`, which is
    /// the PRINT answer under §8.11.4.5 — a printing application "shall
    /// not apply the changes based on usage application dictionaries",
    /// and inheriting the canvas's options would apply the zoom-driven
    /// layer states the operator happens to be looking at.
    ///
    /// The operator's own layer overrides are likewise NOT applied: they
    /// are a viewing choice, and §8.11.4.5 puts printing on the
    /// document's own default configuration.
    pub(crate) fn spool_print(&mut self) -> Result<pdfce_print::SpoolReport, String> {
        let Some(pending) = self.pending_print.as_ref() else {
            return Err(ui_text::print_no_document().to_owned());
        };
        let (Some(printer), scope) = (pending.printer_name.clone(), pending.scope) else {
            return Err(ui_text::print_button_why_disabled().to_owned());
        };
        let plans = pending.plans.clone();
        let settings = pending.device;
        let Status::Open(doc) = &self.status else {
            return Err(ui_text::print_no_document().to_owned());
        };
        // The SAME builder the preview calls (Pass 63.0). See
        // `print_render_options` for the three choices it encodes and why
        // a second copy of them here would defeat the preview's purpose.
        let options =
            Self::print_render_options(scope, self.font_env.clone(), self.settings.cmyk_intent);
        let view = doc.session.view();

        let mut bitmaps = Vec::with_capacity(plans.len());
        for plan in &plans {
            let Some(page) = doc.pages.get(plan.index) else {
                continue;
            };
            let mb = page.media_box;
            let size = ((mb.urx - mb.llx).abs(), (mb.ury - mb.lly).abs());
            let rendered = pdfce_render::render_page_with_view(
                &view,
                page,
                plan.render_scale as f32,
                &options,
            )
            .map_err(|e| e.to_string())?;
            bitmaps.push(pdfce_print::PageBitmap {
                width: rendered.pixmap.width(),
                height: rendered.pixmap.height(),
                rgba: rendered.pixmap.data().to_vec(),
                placement: plan.placement,
                page_pt: size,
            });
        }
        // The orientation page is passed explicitly, and it is the FIRST
        // PLANNED page — the same one `print_dialog` turned the device
        // geometry for. Taking it from `plans` rather than from the
        // document keeps that guarantee even when the job is reversed or
        // range-filtered, which is exactly when `pages[0]` would be the
        // wrong page.
        let first_page_pt = bitmaps
            .first()
            .map_or(pdfce_print::US_LETTER_PORTRAIT_PT, |b| b.page_pt);
        pdfce_print::spool(
            &printer,
            &bitmaps,
            pdfce_print::DryRun::No,
            None,
            settings,
            first_page_pt,
        )
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PREVIEW_ZOOM_MAX, PREVIEW_ZOOM_MIN, PRINT_PREVIEW_MAX_SIDE_PX, PRINT_PREVIEW_TARGET_DPI,
        preview_raster_scale, zoomed_view,
    };
    use eframe::egui;

    /// The screen position a sheet point lands at, for a given view.
    ///
    /// Mirrors `print_preview_column`'s own `origin` computation so the
    /// anchor tests below assert the property that matters — "this point
    /// did not move" — rather than re-stating the formula they are meant
    /// to be checking.
    fn on_screen(
        sheet_pt: egui::Vec2,
        fit: f32,
        zoom: f32,
        pan: egui::Vec2,
        centre: egui::Pos2,
        point_in_sheet: egui::Vec2,
    ) -> egui::Pos2 {
        let s = fit * zoom;
        let origin = centre - (sheet_pt * s) / 2.0 + pan;
        origin + point_in_sheet * s
    }

    /// ★ The point under the pointer does not move when you zoom on it.
    ///
    /// This is the whole reason the anchor term exists, and it is the one
    /// property a reader can check without re-deriving the algebra. Asserted
    /// on an OFF-CENTRE point, because every wrong version of this formula —
    /// including simply omitting the term — is correct at the centre.
    #[test]
    fn ctrl_wheel_zoom_holds_the_point_under_the_pointer_still() {
        // US Letter, fitted into a 340 x 400 canvas at the same 0.92 margin
        // factor the preview uses.
        let sheet = egui::vec2(612.0, 792.0);
        let fit = (340.0_f32 / sheet.x).min(400.0 / sheet.y) * 0.92;
        let centre = egui::pos2(170.0, 200.0);
        // A point near the sheet's bottom-right, which is where an operator
        // checking a margin actually looks.
        let target_in_sheet = egui::vec2(560.0, 730.0);

        let (zoom0, pan0) = (1.0_f32, egui::Vec2::ZERO);
        let at = on_screen(sheet, fit, zoom0, pan0, centre, target_in_sheet);

        let (zoom1, pan1) = zoomed_view(zoom0, pan0, 2.5, at, centre);
        let after = on_screen(sheet, fit, zoom1, pan1, centre, target_in_sheet);

        assert!(
            (after - at).length() < 0.001,
            "the anchored point moved from {at:?} to {after:?} — without the \
             (at - centre)(1 - k) term, zooming in on a corner walks the sheet \
             off the canvas"
        );
        assert!(
            (zoom1 - 2.5).abs() < 1e-6,
            "the zoom itself must still be applied; got {zoom1}"
        );
    }

    /// A button press anchors on the canvas centre, which is the degenerate
    /// case `pan1 = k * pan0` — the sheet grows about the middle rather than
    /// about wherever the pointer happened to be resting.
    #[test]
    fn a_button_zoom_scales_the_existing_pan_about_the_centre() {
        let centre = egui::pos2(170.0, 200.0);
        let (zoom, pan) = zoomed_view(2.0, egui::vec2(30.0, -12.0), 1.25, centre, centre);
        assert!((zoom - 2.5).abs() < 1e-6);
        assert!((pan.x - 37.5).abs() < 1e-4, "pan.x was {}", pan.x);
        assert!((pan.y + 15.0).abs() < 1e-4, "pan.y was {}", pan.y);
    }

    /// ★ A zoom the clamp refuses must not pan either.
    ///
    /// The bug this pins is subtle and would look like a hardware fault:
    /// at maximum zoom the wheel stops magnifying but keeps sliding the
    /// sheet sideways, so the preview appears to drift on its own. It comes
    /// from using the REQUESTED step for the anchor term instead of the
    /// effective, post-clamp ratio.
    #[test]
    fn a_refused_zoom_leaves_the_pan_exactly_where_it_was() {
        let pan = egui::vec2(21.0, -8.0);
        let (zoom, after) = zoomed_view(
            PREVIEW_ZOOM_MAX,
            pan,
            4.0,
            egui::pos2(300.0, 40.0),
            egui::pos2(170.0, 200.0),
        );
        assert!(
            (zoom - PREVIEW_ZOOM_MAX).abs() < 1e-6,
            "clamped at the ceiling"
        );
        assert!(
            (after - pan).length() < 1e-4,
            "a refused zoom moved the sheet from {pan:?} to {after:?}"
        );

        // The same at the floor.
        let (zoom, after) = zoomed_view(
            PREVIEW_ZOOM_MIN,
            pan,
            0.1,
            egui::pos2(300.0, 40.0),
            egui::pos2(170.0, 200.0),
        );
        assert!((zoom - PREVIEW_ZOOM_MIN).abs() < 1e-6);
        assert!((after - pan).length() < 1e-4);
    }

    /// A hostile or degenerate step is a no-op rather than a `NaN` that
    /// poisons every later frame's pan arithmetic.
    #[test]
    fn a_non_finite_or_negative_step_changes_nothing() {
        let pan = egui::vec2(3.0, 4.0);
        let centre = egui::pos2(0.0, 0.0);
        for step in [f32::NAN, f32::INFINITY, 0.0, -1.5] {
            let (zoom, after) = zoomed_view(2.0, pan, step, egui::pos2(10.0, 10.0), centre);
            assert!(
                (zoom - 2.0).abs() < 1e-6 && (after - pan).length() < 1e-6,
                "step {step} must be ignored, got zoom {zoom} pan {after:?}"
            );
        }
    }

    /// An ordinary page renders at the target DPI — the pixel ceiling does
    /// not bind, and must not quietly downgrade every normal preview.
    #[test]
    fn a_letter_page_previews_at_the_target_resolution() {
        let scale = preview_raster_scale((612.0, 792.0));
        assert!(
            (scale - PRINT_PREVIEW_TARGET_DPI / 72.0).abs() < 1e-6,
            "a Letter page must not be capped; got {scale}"
        );
    }

    /// ★ A large-format sheet is capped by PIXELS, not by DPI.
    ///
    /// The bound that matters. An ANSI E sheet at the target DPI would be
    /// 5100 x 6600 px and about 134 MB of RGBA for a picture drawn 300 pt
    /// wide — and CAD sheets are exactly the population this project's
    /// operator prints, so this is the common case, not the exotic one.
    #[test]
    fn a_large_format_sheet_is_capped_by_pixels() {
        let sheet = (2448.0, 3168.0); // ANSI E, 34 x 44 inches.
        let scale = preview_raster_scale(sheet);
        let longest = sheet.0.max(sheet.1) as f32 * scale;
        assert!(
            longest <= PRINT_PREVIEW_MAX_SIDE_PX + 0.5,
            "the long side rendered to {longest} px, over the {PRINT_PREVIEW_MAX_SIDE_PX} ceiling"
        );
        assert!(
            scale < PRINT_PREVIEW_TARGET_DPI / 72.0,
            "the cap must actually bind on this size; got {scale}"
        );
    }

    /// A degenerate `/MediaBox` must not divide by zero. Real files carry
    /// them — the renderer has its own guards, and this only has to hand it
    /// a finite number.
    #[test]
    fn a_zero_sized_page_yields_a_finite_scale() {
        let scale = preview_raster_scale((0.0, 0.0));
        assert!(scale.is_finite() && scale > 0.0, "got {scale}");
    }
}
