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
    /// Whether annotation appearances print.
    ///
    /// ONE flag, deliberately. `RenderOptions::annotations` is a single
    /// `bool`, so a Comments-and-Forms taxonomy with separate markup,
    /// stamp and form-field entries would be a control implying a
    /// capability that does not exist — R83's failure, even though the
    /// toggle itself is real. Per-kind filtering is a `pdfce-render`
    /// change, not a GUI arrangement.
    annotations: bool,
    /// Rendering resolution ceiling, in DPI. A memory bound, editable
    /// because the disclosure is worth more as a control than a warning.
    max_dpi: u32,
    /// Odd/even filtering.
    pub(crate) subset: pdfce_print::PageSubset,
    /// Print back to front.
    pub(crate) reverse: bool,
    /// Copy count.
    pub(crate) copies: u16,
    /// Copy ordering.
    pub(crate) collate: pdfce_print::Collate,
    /// Which page the preview shows.
    preview_page: usize,
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
    pub(crate) fn open_print_dialog(&mut self) {
        let printers = pdfce_print::list_printers().unwrap_or_default();
        let selected = printers.iter().position(|p| p.is_default).unwrap_or(0);
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
            annotations: true,
            max_dpi: 300,
            subset: pdfce_print::PageSubset::All,
            reverse: false,
            copies: 1,
            collate: pdfce_print::Collate::Collated,
            preview_page,
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
            collate: pending.collate,
        };
        let geometry = caps.as_ref().map(pdfce_print::DeviceGeometry::from);
        let plans = geometry
            .as_ref()
            .map(|g| pdfce_print::plan_job(g, &page_sizes, &spec))
            .unwrap_or_default();
        let resolution = geometry
            .as_ref()
            .map(|g| pdfce_print::job_resolution(g, &spec));
        let clipped = plans.iter().filter(|p| p.placement.clipped).count();

        let mut open = true;
        let mut do_print = false;
        egui::Window::new(ui_text::print_dialog_title())
            .collapsible(false)
            .resizable(true)
            .default_size([780.0, 580.0])
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
                ui.columns(2, |cols| {
                    Self::print_preview_column(
                        &mut cols[0],
                        caps.as_ref(),
                        &plans,
                        &page_sizes,
                        pending,
                        clipped,
                    );
                    Self::print_options_column(&mut cols[1], pending, resolution, doc.pages.len());
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
                    let enabled = caps.is_some() && !plans.is_empty();
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
                                egui::RichText::new(ui_text::print_failed(msg))
                                    .color(self.theme.palette.danger),
                            ),
                        };
                    }
                });
            });

        crate::diag::trace(|| {
            format!(
                "print-plan printer={:?} pages={} clipped={} dpi={:?}",
                printer_name,
                plans.len(),
                clipped,
                resolution.map(|r| r.dpi)
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
    fn print_preview_column(
        ui: &mut egui::Ui,
        caps: Option<&pdfce_print::PrinterCaps>,
        plans: &[pdfce_print::PagePlan],
        page_sizes: &[(f64, f64)],
        pending: &mut PendingPrint,
        clipped: usize,
    ) {
        let Some(caps) = caps else {
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

        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 340.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        let theme = crate::theme::Theme::of(ui.ctx());

        // Fit the SHEET into the preview box, preserving aspect.
        let sheet = caps.physical_pt;
        let s = (rect.width() / sheet.0 as f32).min(rect.height() / sheet.1 as f32) * 0.92;
        let sheet_px = egui::vec2(sheet.0 as f32 * s, sheet.1 as f32 * s);
        let origin = rect.center() - sheet_px / 2.0;
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
            origin + egui::vec2(caps.offset_pt.0 as f32 * s, caps.offset_pt.1 as f32 * s),
            egui::vec2(
                caps.printable_pt.0 as f32 * s,
                caps.printable_pt.1 as f32 * s,
            ),
        );
        painter.rect_stroke(
            printable,
            0.0,
            egui::Stroke::new(1.0, theme.palette.guide),
            egui::StrokeKind::Middle,
        );

        // The placed page.
        if let (Some(plan), Some(&size)) = (plans.get(shown), page_sizes.get(shown)) {
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
            painter.rect_filled(placed, 0.0, theme.palette.surface);
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

        ui.horizontal(|ui| {
            if ui.button(ui_text::find_previous_label()).clicked() {
                pending.preview_page = pending.preview_page.saturating_sub(1);
            }
            ui.label(ui_text::print_preview_position(shown + 1, plans.len()));
            if ui.button(ui_text::find_next_label()).clicked() && shown + 1 < plans.len() {
                pending.preview_page = shown + 1;
            }
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

    /// The options column.
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
        egui::CollapsingHeader::new(ui_text::print_more_options())
            .default_open(false)
            .show(ui, |ui| {
                ui.checkbox(
                    &mut pending.annotations,
                    ui_text::print_include_annotations(),
                );
                ui.add_space(4.0);
                // Always true, so a static caption rather than a warning.
                // A banner that fires on every job trains an operator to
                // stop reading banners.
                ui.label(
                    egui::RichText::new(ui_text::print_raster_note())
                        .small()
                        .weak(),
                );
                // Conditional, because it is a per-job substitution:
                // pdfce picked a resolution the operator did not.
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
            });
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
        let (Some(printer), annotations) = (pending.printer_name.clone(), pending.annotations)
        else {
            return Err(ui_text::print_button_why_disabled().to_owned());
        };
        let plans = pending.plans.clone();
        let Status::Open(doc) = &self.status else {
            return Err(ui_text::print_no_document().to_owned());
        };
        let mut options = pdfce_render::RenderOptions::default().with_annotations(annotations);
        options.fonts = self.font_env.clone();
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
        pdfce_print::spool(&printer, &bitmaps, pdfce_print::DryRun::No, None)
            .map_err(|e| e.to_string())
    }
}
