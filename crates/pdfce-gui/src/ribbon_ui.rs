//! The ribbon: tab strip, groups, and the Quick Access Toolbar.
//!
//! # One command, one place
//!
//! The rule this module exists to hold is R123: a command lives in
//! exactly one location. A command reachable from two places is two
//! mental models of the same application, and the operator has to learn
//! both to be sure they are equivalent.
//!
//! That rule has already cost real work here and is worth restating
//! where the code is. Undo/Redo and the zoom and page controls used to
//! sit in ribbon groups on the Edit and View tabs — and the ribbon emits
//! only the ACTIVE tab's band, so an operator working on the Measure tab
//! had no undo, no zoom and no page control without leaving the tab they
//! were working in. The fix was to MOVE them (to the Quick Access
//! Toolbar and the status bar), never to mirror them.
//!
//! # A group's caption is not decoration
//!
//! `ribbon_group_ui` draws a group's controls in a row with its caption
//! beneath and a rule separating it from the next. The caption is what
//! says which QUESTION a group answers — `Panels` governs what is shown
//! beside the page, `Show` governs what is drawn over it — and a band of
//! uncaptioned controls has previously read as a rendering fault rather
//! than as a toolbar.
//!
//! `every_ribbon_group_is_gated_to_a_widget` in `main.rs` checks that
//! each declared group actually reaches a widget: a group declared in
//! `ribbon.rs` and gated to nothing renders a caption with nothing under
//! it, which no type in the system prevents.
//!
//! # Stage 3 of the `main.rs` split, and the last of the planned ones
//!
//! A move: no logic, no signatures, no behaviour. The `pub(crate)`
//! markers are the compiler's requirement for the calls `main.rs` makes
//! back up into this module, and nothing is wider than the crate.

use crate::{
    Action, FitMode, PdfceApp, RIBBON_CAPTION_GAP, Status, canvas, diag, icons, ribbon, ui_text,
};
use crate::{CopyScope, GuiMarkupKind, GuiTextKind};
use canvas::CanvasTool;
use eframe::egui;
use ribbon::RibbonGroup as RG;

impl PdfceApp {
    /// Lay one ribbon group out as Office lays one out: its controls in a
    /// row, its **caption beneath them**, and a vertical rule separating it
    /// from the next group (Pass 47.2).
    ///
    /// # What was wrong with the shape this replaces
    ///
    /// The predecessor was a `-> bool` predicate that emitted the caption
    /// *inline, to the LEFT of* the group's controls, in a single flat row.
    /// Captured from the running application on 2026-08-08 that produces:
    ///
    /// ```text
    /// File [Open…] [Save a copy…] Document [Properties] Clipboard [Copy this page's text] …
    /// ```
    ///
    /// — a ~26 px strip in which the captions read as just more small controls
    /// and **the grouping is invisible**. The whole band parses as an
    /// undifferentiated toolbar with a tab strip above it, which is precisely
    /// the operator's *"doesn't look like Office"* complaint: the one
    /// structural cue a ribbon has — a labelled block of related controls —
    /// was the thing being dropped.
    ///
    /// # Why a closure rather than the `-> bool` gate it replaces
    ///
    /// Immediate mode. To put the caption *under* the controls, the controls
    /// must be emitted inside a vertical container that is still open when the
    /// caption is written; a predicate returning `bool` has already returned
    /// before the body runs. That is the whole reason for the signature
    /// change — the group set, the ordering and the separator convention are
    /// unchanged.
    ///
    /// # It also closes a class of defect by construction
    ///
    /// Two sites previously bypassed the predicate and therefore drew no
    /// caption at all: `LayoutReset` used a bare `tab.shows(..)`, and `Show`
    /// and `Panels` shared one `shows(A) || shows(B)` block. Both were visible
    /// in the 2026-08-08 capture as unlabelled floating controls. With the
    /// body handed to this function there is no longer a code path that shows
    /// a group without captioning it.
    pub(crate) fn ribbon_group_ui(
        ui: &mut egui::Ui,
        tab: ribbon::RibbonTab,
        group: ribbon::RibbonGroup,
        body: impl FnOnce(&mut egui::Ui),
    ) {
        if !tab.shows(group) {
            return;
        }
        // Separator BEFORE the caption rather than after, so the band never
        // ends with a trailing rule and the first group never starts with one.
        if tab.groups().first() != Some(&group) {
            ui.separator();
        }
        ui.vertical(|ui| {
            // The controls row FIRST, and its measured width is what the
            // caption is then centred within. Office centres a group caption
            // under its group, and doing that here needs the row's width,
            // which in immediate mode only exists after the row is emitted —
            // hence measure-then-allocate rather than a `vertical_centered`
            // wrapper, which would justify to the whole remaining band and
            // scatter the captions across the window.
            let row = ui.horizontal(|ui| body(ui)).response.rect;
            ui.add_space(RIBBON_CAPTION_GAP);
            ui.allocate_ui_with_layout(
                egui::vec2(row.width(), 0.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new(group.caption()).weak().small());
                },
            );
        });
    }

    /// The **Quick Access Toolbar** — Open, Save a copy, Undo, Redo — drawn
    /// left of the tab strip and emitted on EVERY frame regardless of which
    /// tab is active (Pass 47.1; R125's scope note).
    ///
    /// # Why this exists, and why its absence was a real defect
    ///
    /// Decision 024 §3.5(d) specified a QAT in these words: *"pdfce's
    /// zoom/navigation controls are used constantly and must not end up
    /// behind a tab switch."* It was never built. What shipped instead put
    /// Undo/Redo in `RibbonGroup::History` on **Edit** and page navigation and
    /// zoom in `Navigate`/`Zoom` on **View** — and R125 emits only the active
    /// tab's band.
    ///
    /// The consequence, confirmed by capturing each tab of the running
    /// application on 2026-08-08: an operator on **Measure** — this operator's
    /// own stated primary activity — had **no undo, no zoom and no page
    /// control** without first leaving the tab they were working in. That is
    /// precisely the outcome decision 024 wrote the QAT to prevent.
    ///
    /// # These controls MOVED; they were not duplicated (R123)
    ///
    /// `History`, `Navigate` and `Zoom` are gone from the ribbon rather than
    /// mirrored here. A command reachable from two places is the
    /// two-mental-models failure `ribbon.rs`'s own module docs name, and R123
    /// is explicit that each command lives in exactly one location.
    ///
    /// Page navigation and zoom do not live here either — they went to the
    /// **status bar**, where every other PDF reader puts them. See
    /// [`Self::status_view_controls`].
    pub(crate) fn quick_access_toolbar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
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
        // ★ SAVE KEEPS ITS TEXT LABEL, and that is not a styling preference.
        //
        // Office's QAT is conventionally icon-only, and Undo/Redo below follow
        // it — those two glyphs are universally read. Save cannot: the label
        // is **"Save a copy…"**, and the wording is load-bearing because pdfce
        // never overwrites the open file unasked. A bare disk glyph says
        // "Save" — i.e. "overwrite what I opened" — which is the small lie
        // `ui_text::save_button`'s own doc comment was written to forbid.
        // Convention loses to not misleading anyone.
        //
        // Hidden rather than disabled with no document open: there is nothing
        // to discover about saving when nothing is open. Same posture the
        // ribbon's File group used before this Pass moved the control here.
        // §7.6: an encrypted document cannot be written yet, so the control
        // is DISABLED rather than left live to fail on click.
        //
        // The refusal in `writer::save_*` stays as the safety net — this is
        // not a replacement for it. But a button that predictably produces a
        // refusal is worse than a visibly-disabled one: it costs a click, it
        // can carry the operator as far as a file-picker before the answer
        // arrives, and "it let me try" reads as a bug rather than a scope.
        let encrypted = matches!(&self.status, Status::Open(doc)
            if doc.session.document().encryption().is_some());
        if matches!(self.status, Status::Open(_)) {
            let button = ui.add_enabled(
                !encrypted,
                Self::icon_text(ui, icons::Icon::Save, ui_text::save_button()),
            );
            let button = if encrypted {
                // The tooltip states the reason. A disabled control with no
                // explanation is the same dead end as a failing one.
                button.on_disabled_hover_text(ui_text::save_disabled_encrypted_tooltip())
            } else {
                button.on_hover_text(ui_text::save_tooltip())
            };
            if button.clicked() {
                actions.push(Action::Save);
            }
        }
        if let Status::Open(doc) = &self.status {
            ui.separator();
            // Disabled rather than hidden: the absence of an Undo control and
            // a greyed-out one say different things, and the second confirms
            // there is nothing to undo — which is information. The tooltips
            // name the specific operation, which is why `EditSession` hands
            // out a structured `CommandKind` at all.
            let (can_undo, can_redo) = (doc.session.can_undo(), doc.session.can_redo());
            let (undo_kind, redo_kind) = (doc.session.undo_kind(), doc.session.redo_kind());
            ui.add_enabled_ui(can_undo, |ui| {
                if Self::icon_button(ui, icons::Icon::Undo, ui_text::undo_tooltip_for(undo_kind))
                    .clicked()
                {
                    actions.push(Action::Undo);
                }
            });
            ui.add_enabled_ui(can_redo, |ui| {
                if Self::icon_button(ui, icons::Icon::Redo, ui_text::redo_tooltip_for(redo_kind))
                    .clicked()
                {
                    actions.push(Action::Redo);
                }
            });
        }
        ui.separator();
    }

    /// Page navigation and zoom, pinned to the **right of the status bar**
    /// (Pass 47.1).
    ///
    /// # Why the status bar and not the QAT
    ///
    /// Decision 024 §3.5(d) listed these among the Quick Access controls.
    /// This Pass diverges, and the divergence is the operator's own ruling on
    /// decision 033 §7 Q2: a QAT is conventionally three or four items, and
    /// page-nav plus zoom is six or more. Every mainstream PDF reader —
    /// Acrobat, PDF-XChange, Edge, Chrome — puts them **bottom-right**, which
    /// is the position an operator's hand already knows.
    ///
    /// Emitted every frame, like the QAT, and for the same reason: these are
    /// the controls decision 024 said must never sit behind a tab switch.
    ///
    /// Hidden entirely with no document open — there is nothing to discover
    /// about a page control when no pages exist, and R124 (as re-amended by
    /// the operator's *"no placeholders"* ruling) says an unavailable
    /// capability shows nothing rather than a disabled stub.
    pub(crate) fn status_view_controls(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        let count = doc.pages.len();
        let current = doc.view.page_index + 1;
        let (zoom_percent, fit) = (doc.view.zoom_percent(), doc.view.fit);
        let at_first = doc.view.page_index == 0;

        // Laid out right-to-left, so the cluster stays pinned to the window's
        // right edge as the narrator text on the left grows. Read in reverse:
        // the LAST thing added here is the LEFTMOST thing on screen.
        // Find lives here rather than on a ribbon tab for the reason the
        // zoom and page controls do: the status bar is always on screen,
        // so it never sits behind a tab switch, and Ctrl+F alone is not
        // DISCOVERABLE — an operator who does not know the chord has no
        // way to learn the feature exists.
        //
        // Selectable, like the fit modes, because Find is a MODE: the bar
        // is either showing or it is not, and the control should say
        // which.
        if Self::icon_text_toggle(
            ui,
            icons::Icon::Search,
            self.find_open,
            ui_text::find_open_button(),
            ui_text::find_open_tooltip(),
        )
        .clicked()
        {
            actions.push(Action::ToggleFind);
        }
        if ui
            .button(ui_text::zoom_100_button())
            .on_hover_text(ui_text::zoom_100_tooltip())
            .clicked()
        {
            actions.push(Action::ZoomActualSize);
        }
        // Fit modes are selectable because they are MODES: the operator can
        // see at a glance whether the view is being kept fitted or is pinned.
        if Self::icon_text_toggle(
            ui,
            icons::Icon::FitWidth,
            fit == FitMode::Width,
            ui_text::fit_width_button(),
            ui_text::fit_width_tooltip(),
        )
        .clicked()
        {
            actions.push(Action::Fit(FitMode::Width));
        }
        if Self::icon_text_toggle(
            ui,
            icons::Icon::FitPage,
            fit == FitMode::Page,
            ui_text::fit_page_button(),
            ui_text::fit_page_tooltip(),
        )
        .clicked()
        {
            actions.push(Action::Fit(FitMode::Page));
        }
        if Self::icon_button(ui, icons::Icon::ZoomIn, ui_text::zoom_in_tooltip()).clicked() {
            actions.push(Action::ZoomIn);
        }
        ui.label(ui_text::zoom_percent_label(zoom_percent));
        if Self::icon_button(ui, icons::Icon::ZoomOut, ui_text::zoom_out_tooltip()).clicked() {
            actions.push(Action::ZoomOut);
        }
        ui.separator();
        ui.add_enabled_ui(current < count, |ui| {
            if Self::icon_button(ui, icons::Icon::ChevronRight, ui_text::next_page_tooltip())
                .clicked()
            {
                actions.push(Action::NextPage);
            }
        });
        ui.label(ui_text::page_nav_label(current, count));
        ui.add_enabled_ui(!at_first, |ui| {
            if Self::icon_button(ui, icons::Icon::ChevronLeft, ui_text::prev_page_tooltip())
                .clicked()
            {
                actions.push(Action::PrevPage);
            }
        });
    }

    /// Every toolbar control, in group order, laid into the wrapping row
    /// [`Self::toolbar`] builds.
    ///
    /// Split out from [`Self::toolbar`] purely so the layout scaffolding and
    /// the control list can each be read without the other.
    ///
    /// **Pass 47.2 changed the shape of every call site here** — each group
    /// now hands its body to [`Self::ribbon_group_ui`] as a closure instead of
    /// sitting behind an `if` predicate, so the caption can be drawn beneath
    /// the controls rather than inline before them. The group set and the
    /// ordering are unchanged; what moved is where the caption goes and
    /// which code path is capable of omitting it (none).
    pub(crate) fn toolbar_controls(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let tab = self.ribbon_tab;
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
            // RG::FileOps is GONE from the ribbon — Open and Save moved to
            // the Quick Access Toolbar (Pass 47.1). Moved, not mirrored: a
            // command in two places is R123's two-mental-models failure.

            // ★ SPLIT INTO ITS TWO DECLARED GROUPS (Pass 47.2).
            //
            // This was ONE `tab.shows(A) || tab.shows(B)` block covering two
            // declared groups, so NEITHER caption was ever emitted — on the
            // View tab the four leftmost controls floated with no label of any
            // kind, confirmed in the 2026-08-08 capture. `Panels` governs which
            // DOCKS are visible; `Show` governs what is drawn OVER the page.
            // Two different questions, and the captions are what say so.
            Self::ribbon_group_ui(ui, tab, RG::Panels, |ui| {
                if Self::icon_button(ui, icons::Icon::Sidebar, ui_text::rail_toggle_tooltip())
                    .clicked()
                {
                    actions.push(Action::ToggleRail);
                }
                // ★ THREE PANELS THAT HAD NO WAY IN.
                //
                // Bookmarks, Layers and Signatures each shipped with a
                // `PaneSubject`, a panel body and a `diag` step — and no
                // operator-reachable control at all. Their ONLY callers
                // were the harness step handlers, so all three were
                // unreachable in a real build while being reported as
                // working, and a build note told the operator to open one.
                //
                // The diag step made them look driveable, which is exactly
                // how the gap survived: the harness could reach them, so
                // every verification passed. A `PaneSubject` variant is not
                // a feature until something an operator can click sets it.
                //
                // Here rather than on their own tab: the View tab's Panels
                // group is already "what is shown beside the page", and
                // R123 forbids a second entry point for one command.
                //
                // Disabled-and-explained with no document (R83), never
                // hidden — a control that vanishes teaches nothing.
                let has_doc = matches!(self.status, Status::Open(_));
                ui.add_enabled_ui(has_doc, |ui| {
                    if Self::icon_text_toggle(
                        ui,
                        icons::Icon::Bookmarks,
                        self.pane_subject == ribbon::PaneSubject::Bookmarks,
                        ui_text::activities_bookmarks_label(),
                        ui_text::bookmarks_open_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::ShowBookmarks);
                    }
                    if Self::icon_text_toggle(
                        ui,
                        icons::Icon::Layers,
                        self.pane_subject == ribbon::PaneSubject::Layers,
                        ui_text::activities_layers_label(),
                        ui_text::layers_open_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::ShowLayers);
                    }
                    if Self::icon_text_toggle(
                        ui,
                        icons::Icon::Signatures,
                        self.pane_subject == ribbon::PaneSubject::Signatures,
                        ui_text::activities_signatures_label(),
                        ui_text::signatures_open_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::ShowSignatures);
                    }
                });
                // Traced because the compile-time gate proves the CALL exists,
                // and only a running frame proves the CONTROL was drawn. Those
                // are different claims, and the panels this replaces were
                // reported working on the strength of the weaker one.
                diag::trace(|| {
                    format!(
                        "ribbon-panel-toggles drawn=3 enabled={has_doc} subject={:?}",
                        self.pane_subject
                    )
                });
                // THE OBJECT-TREE SIDEBAR'S OWN TOGGLE (2026-08-06, operator
                // instruction: *"these can be activated from the view menu"*).
                //
                // Until now the right dock had NO toggle of its own — it was
                // opened only as a side effect of other commands, and after
                // `ToggleTools` was repurposed to show Batch Tools it had no
                // route at all. A panel reachable by accident is the R80
                // defect one step short of a panel reachable nowhere.
                let b = ui
                    .add(Self::icon_text(
                        ui,
                        icons::Icon::EditObjects,
                        ui_text::objects_sidebar_toggle(),
                    ))
                    .on_hover_text(ui_text::objects_sidebar_toggle_tooltip());
                diag::trace(|| format!("objects-sidebar-toggle rect={:?}", b.rect));
                if b.clicked() {
                    actions.push(Action::ToggleObjectsSidebar);
                }
            });
            Self::ribbon_group_ui(ui, tab, RG::Show, |ui| {
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
                    // "Show points" (Pass 36.3), beside the annotation toggle
                    // because it is the same kind of control: it changes what is
                    // drawn over the page, never the page. A `SelectableLabel` for
                    // the same Rule-6 reason — on an object whose points are all
                    // off-screen, flipping it can produce no visible change, so the
                    // control has to carry its own state.
                    let points_on = doc.show_all_points;
                    let points_tooltip = if points_on {
                        ui_text::show_points_toggle_tooltip_on()
                    } else {
                        ui_text::show_points_toggle_tooltip_off()
                    };
                    if Self::icon_toggle(ui, icons::Icon::ShowPoints, points_on, points_tooltip)
                        .clicked()
                    {
                        actions.push(Action::ToggleShowPoints);
                    }
                }
            });

            // RG::Navigate and RG::Zoom are GONE from the ribbon — page
            // navigation and zoom moved to the STATUS BAR (Pass 47.1), where
            // every mainstream PDF reader puts them and where they are
            // reachable from every tab. See `status_view_controls`.
            if let Status::Open(doc) = &self.status {
                Self::ribbon_group_ui(ui, tab, RG::Pages, |ui| {
                    // Group: edit. A new group rather than additions to an
                    // existing one, exactly as the module docs anticipated:
                    // rotation acts on the document, whereas everything to
                    // its left acts on the view, and mixing the two would
                    // make "does this button change my file?" unanswerable
                    // at a glance.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        if Self::icon_button(
                            ui,
                            icons::Icon::RotateCcw,
                            ui_text::rotate_left_tooltip(),
                        )
                        .clicked()
                        {
                            actions.push(Action::RotateLeft);
                        }
                        if Self::icon_button(
                            ui,
                            icons::Icon::RotateCw,
                            ui_text::rotate_right_tooltip(),
                        )
                        .clicked()
                        {
                            actions.push(Action::RotateRight);
                        }
                    });
                    // Decision 017 §8.3 keeps this control and its shortcut as
                    // the Properties entry point and changes only where they
                    // lead. Its selected state is now DERIVED from the dock —
                });
                Self::ribbon_group_ui(ui, tab, RG::DocumentProperties, |ui| {
                    // "the dock is open AND Properties is its front tab" — so
                    // the toggle reports what is on screen rather than a
                    // separate boolean that could disagree with it.
                    if Self::icon_text_toggle(
                        ui,
                        icons::Icon::Properties,
                        // Properties is always visible now, so the toggle's
                        // "selected" state is just whether the rail is open.
                        self.rail_expanded,
                        ui_text::properties_button(),
                        ui_text::properties_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::ToggleProperties);
                    }
                });
                Self::ribbon_group_ui(ui, tab, RG::Markup, |ui| {
                    // Pass 6.1 markup authoring — an edit tool, so it lives in
                    // the toolbar edit group per the settled placement taxonomy
                    // (edit → toolbar; ARCHITECTURE.md §12 continuation-23). A
                    // menu rather than one button per subtype, because four
                    // (eventually ten) shape tools would swamp the group. The
                    // canvas drawing tools of the ui-spec are a follow-up slice;
                    // this minimal affordance authors a default-placed shape on
                    // the current page through the same command path.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        // Pass 24.4: FLATTENED out of a `Markup ▾` menu into
                        // one button per shape plus the pen swatch.
                        //
                        // Icon + text, never icon alone. That rule came from
                        // ui-spec §3.3 for a MENU ROW — "a menu row is read,
                        // not scanned, so the words stay the primary label" —
                        // and it survives the move for a different reason: four
                        // outline glyphs (square, circle, line, band) are
                        // genuinely hard to tell apart at 16 px, so the word is
                        // still doing the identifying work here.
                        for kind in [
                            GuiMarkupKind::Square,
                            GuiMarkupKind::Circle,
                            GuiMarkupKind::Line,
                            GuiMarkupKind::Highlight,
                        ] {
                            let row = (
                                icons::image(ui, kind.icon()),
                                egui::RichText::new(kind.label()),
                            );
                            if ui
                                .add(egui::Button::new(row))
                                .on_hover_text(ui_text::markup_menu_hint())
                                .clicked()
                            {
                                actions.push(Action::AddMarkupShape(kind));
                            }
                        }
                        // The pen colour. Changing it is NOT an edit (ui-spec
                        // §1.1) — only authoring is — so it sits beside the
                        // shapes rather than being mistaken for one of them.
                        ui.label(ui_text::markup_color_label());
                        ui.color_edit_button_srgba(&mut self.markup_color);
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::CommentsList, |ui| {
                    // The entry point to the Comments pane. Disabled with a
                    // reason when there are no pages (R83) — a control that
                    // vanishes teaches nothing, a disabled one teaches what
                    // would enable it.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::EditText,
                            self.pane_subject == ribbon::PaneSubject::Comments,
                            ui_text::comments_open_button(),
                            ui_text::comments_open_tooltip(),
                        );
                        if response.clicked() {
                            actions.push(Action::ToggleCommentsPanel);
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::Notes, |ui| {
                    // Pass 6.2 text-bearing authoring — the same edit-group,
                    // same minimal-affordance approach as Markup. A menu opens
                    // the text-entry popup; the actual authoring happens on
                    // confirm (see the popup below). A full canvas text editor
                    // is the named follow-up slice.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        // Pass 24.4: FLATTENED out of a `Text ▾` menu.
                        //
                        // The pen-colour control does NOT come with it. It was
                        // duplicated into this menu by P1-3b so a FreeText box
                        // authored without ever opening Markup would not take
                        // an unexplained default colour — a fix for the menu
                        // being the only place the colour was visible. In a
                        // band, the Markup group's swatch is two groups away on
                        // the same tab and permanently on screen, so the
                        // duplicate has nothing left to fix, and two colour
                        // pickers editing ONE `markup_color` would be a
                        // control that appears to be per-kind and is not.
                        //
                        // `text_menu_color_note` (which said the colour applies
                        // to the box only, and that sticky notes and stamps
                        // ignore it) moves onto the FreeText button's own
                        // tooltip — the one place it is actually actionable.
                        for kind in [
                            GuiTextKind::FreeText,
                            GuiTextKind::Sticky,
                            GuiTextKind::Stamp,
                        ] {
                            let row = (
                                icons::image(ui, kind.icon()),
                                egui::RichText::new(kind.label()),
                            );
                            let hint = if matches!(kind, GuiTextKind::FreeText) {
                                ui_text::text_menu_color_note()
                            } else {
                                ui_text::text_menu_hint()
                            };
                            if ui.add(egui::Button::new(row)).on_hover_text(hint).clicked() {
                                actions.push(Action::OpenTextEntry(kind));
                            }
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::ContentTools, |ui| {
                    // THE MASTER EDIT SWITCH, first in the group because it
                    // governs everything after it. The operator: "should have
                    // one toggle to turn all edits on or off."
                    //
                    // Its OFF state is a review mode — the document can be
                    // read, selected, navigated and searched, and no gesture
                    // anywhere in the application can change it by accident.
                    // Selected-state shows EDITING ON, so the lit button means
                    // "this document is editable", which is the fact an
                    // operator wants at a glance before they start clicking.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let on = doc.editing_enabled;
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::EditObjects,
                            on,
                            ui_text::editing_enabled_button(on),
                            ui_text::editing_enabled_tooltip(on),
                        );
                        // Rect traced so the harness can drive the master
                        // switch — the technique used throughout this session
                        // rather than guessing a screen point.
                        diag::trace(|| {
                            format!("master-edit-toggle on={on} rect={:?}", response.rect)
                        });
                        if response.clicked() {
                            actions.push(Action::ToggleEditingEnabled);
                        }
                    });
                    ui.separator();
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
                        let active = doc.tool_enabled(CanvasTool::TextEdit);
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::EditText,
                            active,
                            ui_text::edit_text_tool_button(),
                            ui_text::edit_text_tool_tooltip(),
                        );
                        if response.clicked() {
                            actions.push(Action::SelectCanvasTool(Some(CanvasTool::TextEdit)));
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
                        let active = doc.tool_enabled(CanvasTool::AddText);
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::AddText,
                            active,
                            ui_text::add_text_tool_button(),
                            ui_text::add_text_tool_tooltip(),
                        );
                        if response.clicked() {
                            actions.push(Action::SelectCanvasTool(Some(CanvasTool::AddText)));
                        }
                    });

                    // Pass 9c-min Edit Objects — a bare toggle for the vector-edit
                    // tool (move / drag-node / delete). Same widget/sizing as the
                    // page-text toggles; greyed (not hidden) with no pages. The
                    // tooltip names the three gestures and the "not redaction"
                    // caveat for delete (decision 011 §2.5).
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let active = doc.tool_enabled(CanvasTool::VectorEdit);
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::EditObjects,
                            active,
                            ui_text::vector_edit_tool_button(),
                            ui_text::vector_edit_tool_tooltip(),
                        );
                        if response.clicked() {
                            actions.push(Action::SelectCanvasTool(Some(CanvasTool::VectorEdit)));
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::MeasureTools, |ui| {
                    // Pass 12.M2 Measure ▾ — a menu (not four toolbar icons) for the
                    // three dimension tools (ui-spec §1.2, rule 3: dimensioning is
                    // used in short deliberate bursts, so it earns a menu, not
                    // primary-icon creep). The widget is Markup ▾'s `menu_button`,
                    // but the dispatch is Edit Text/Add Text's `SelectCanvasTool`
                    // toggle (a NEW combination, ui-spec §1.2). The label is dynamic
                    // so the active tool is never hidden by the closed menu.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let active_name = match doc.active_tool() {
                            Some(CanvasTool::MeasureLinear) => {
                                Some(ui_text::measure_tool_name_linear())
                            }
                            Some(CanvasTool::MeasureCircular) => {
                                Some(ui_text::measure_tool_name_circular())
                            }
                            Some(CanvasTool::MeasureScale) => {
                                Some(ui_text::measure_tool_name_scale())
                            }
                            _ => None,
                        };
                        let label = match active_name {
                            Some(name) => ui_text::measure_menu_active_label(name),
                            None => ui_text::measure_menu_button().to_owned(),
                        };
                        // Pass 24.4: FLATTENED out of a `Measure ▾` menu into
                        // three toggles plus the group command.
                        //
                        // `docs/ui_specs/pass-12.M2-dimension-tools.md` §1.2
                        // chose the menu deliberately — "dimensioning is used
                        // in short deliberate bursts, so it earns a menu, not
                        // primary-icon creep" — and that was correct GIVEN A
                        // FLAT TOOLBAR WITH NO ROOM. A tab has room, and
                        // "short deliberate bursts" is exactly what a tab is
                        // for: you go to it, you work, you leave. The prior
                        // reasoning is not overturned; its premise is, which
                        // is the same argument decision 024 §3.2 used to give
                        // Measure a tab of its own in the first place.
                        //
                        // Each is a SELECTABLE toggle carrying its own active
                        // state, so the operator can see WHICH sub-tool is
                        // armed without reading a dynamic button label — the
                        // thing the menu could only do by rewriting its own
                        // caption.
                        let _ = &label;
                        for (tool, text) in [
                            (
                                CanvasTool::MeasureLinear,
                                ui_text::measure_linear_menu_item(),
                            ),
                            (
                                CanvasTool::MeasureCircular,
                                ui_text::measure_circular_menu_item(),
                            ),
                            (
                                CanvasTool::MeasureScale,
                                ui_text::measure_set_scale_menu_item(),
                            ),
                        ] {
                            let is_active = doc.tool_enabled(tool);
                            if ui
                                .add(egui::Button::selectable(
                                    is_active,
                                    Self::toggle_label(is_active, text),
                                ))
                                .clicked()
                            {
                                actions.push(Action::SelectCanvasTool(Some(tool)));
                            }
                        }
                        // "Manage Dimension Groups…" — opens the §5 modeless
                        // window; does NOT change `active_tool` (ui-spec §1.2).
                        if ui
                            .button(ui_text::measure_manage_groups_menu_item())
                            .clicked()
                        {
                            actions.push(Action::ToggleDimensionGroups);
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::Forms, |ui| {
                    // The entry point to the Forms pane. Disabled with a
                    // reason when the document has no pages (R83), for the
                    // same reason every other document-scoped control here is:
                    // a control that vanishes teaches nothing, a disabled one
                    // with a tooltip teaches what would enable it.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::EditText,
                            self.pane_subject == ribbon::PaneSubject::Forms,
                            ui_text::forms_open_button(),
                            ui_text::forms_open_tooltip(),
                        );
                        if response.clicked() {
                            actions.push(Action::ToggleFormsPanel);
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::FormsAuthor, |ui| {
                    // Decision 020 F5's Create Field tool. A toggle, the same
                    // widget as every other tool arming control, and disabled
                    // with a reason when there are no pages (R83) rather than
                    // hidden — a control that vanishes teaches nothing.
                    //
                    // Its own group, adjacent to Forms: see
                    // `RibbonGroup::FormsAuthor`'s doc comment for why filling
                    // and authoring cannot share a band.
                    ui.add_enabled_ui(!doc.pages.is_empty(), |ui| {
                        let active = doc.tool_enabled(CanvasTool::PlaceField);
                        let response = Self::icon_text_toggle(
                            ui,
                            icons::Icon::FormField,
                            active,
                            ui_text::create_field_button(),
                            ui_text::create_field_button_tooltip(),
                        );
                        diag::trace(|| {
                            format!("create-field-toggle on={active} rect={:?}", response.rect)
                        });
                        if response.clicked() {
                            actions.push(Action::SelectCanvasTool(Some(CanvasTool::PlaceField)));
                        }
                    });
                });

                Self::ribbon_group_ui(ui, tab, RG::Protect, |ui| {
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
                        self.rail_expanded && self.pane_subject == ribbon::PaneSubject::Redact,
                        ui_text::redact_button(),
                        ui_text::redact_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::ToggleRedactPanel);
                    }
                    ui.separator();
                });

                // RG::History is GONE from the ribbon — Undo and Redo moved to
                // the Quick Access Toolbar (Pass 47.1). This was the single
                // worst instance of the tab-gating problem: undo lived on
                // Edit, so an operator measuring on the Measure tab could not
                // undo without leaving their work.
            }

            Self::ribbon_group_ui(ui, tab, RG::Clipboard, |ui| {
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
                //
                // Pass 24.4: FLATTENED out of a `Copy ▾` menu into two
                // buttons. The menu existed because the operator must choose a
                // scope and a single Copy button would have had to guess —
                // which is still true, and is now expressed by there being two
                // buttons rather than one behind a menu. A menu of two items
                // costs a click to reveal what a band has room to show.
                if self.status_is_open() {
                    if ui
                        .add(Self::icon_text(
                            ui,
                            icons::Icon::Copy,
                            ui_text::copy_page_text_menu_item(),
                        ))
                        .on_hover_text(ui_text::copy_page_text_tooltip())
                        .clicked()
                    {
                        actions.push(Action::CopyText(CopyScope::Page));
                    }
                    if ui
                        .add(Self::icon_text(
                            ui,
                            icons::Icon::Copy,
                            ui_text::copy_document_text_menu_item(),
                        ))
                        .on_hover_text(ui_text::copy_document_text_tooltip())
                        .clicked()
                    {
                        actions.push(Action::CopyText(CopyScope::Document));
                    }
                }
            });

            // Export ▸ DXF (Pass 52.2). Beside Clipboard, because both are
            // "get content out" — and a group of its own, because what the
            // operator ends up holding is a file for another application
            // rather than characters on the clipboard. See
            // `RibbonGroup::Export` for the full placement argument.
            //
            // Hidden rather than disabled with no document open, matching
            // Save's posture in the QAT and R124's "no placeholders"
            // ruling: there is nothing to discover about exporting a page
            // when no page exists.
            Self::ribbon_group_ui(ui, tab, RG::Export, |ui| {
                if self.status_is_open()
                    && ui
                        .button(ui_text::export_dxf_button())
                        .on_hover_text(ui_text::export_dxf_tooltip())
                        .clicked()
                {
                    actions.push(Action::OpenDxfExport);
                }
            });

            Self::ribbon_group_ui(ui, tab, RG::Batch, |ui| {
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
            });

            // Fonts — Tools ▸ Fonts (Pass 24.1 follow-up).
            //
            // `RibbonGroup::Fonts` was declared on the Tools tab and gated to
            // NO widget, so the tab promised a group that rendered nothing.
            // That is the same shape as R151 (a capability with no caller) and
            // R152 (a caller that confirms nothing), one layer out: a
            // declaration with no implementation, and the ribbon's own tests
            // could not catch it — they assert that every group has exactly
            // one owning TAB, which says nothing about whether any widget
            // asks for it. Found by reading the gate list against the
            // taxonomy, not by a test, which is worth knowing.
            //
            // Opens the panel with Font folders already selected, rather than
            // opening it on whatever the operator last had open: a control
            // named "Font folders…" that lands somewhere else is a control
            // that lied.
            Self::ribbon_group_ui(ui, tab, RG::Fonts, |ui| {
                if ui
                    .add(Self::icon_text(
                        ui,
                        icons::Icon::FontFolders,
                        ui_text::tool_font_folders_label(),
                    ))
                    .on_hover_text(ui_text::font_folders_intro())
                    .clicked()
                {
                    actions.push(Action::OpenFontFolders);
                }
            });
            // Reset layout… — File ▸ Layout (Pass 24.1). Opens the chooser
            // rather than acting immediately: it is the one control here whose
            // effect the operator cannot see until after it happens, and
            // "reset the layout" now means more than it used to.
            // ★ THIS SITE USED A BARE `tab.shows(..)` AND THEREFORE NEVER
            // DREW ITS CAPTION OR ITS SEPARATOR — visible in the 2026-08-08
            // capture as a "Reset layout…" button floating unlabelled between
            // Clipboard and Help. Routing it through the helper like every
            // other group fixes it BY CONSTRUCTION: there is no longer a code
            // path that shows a group without captioning it, which is the
            // difference between a ribbon and a toolbar someone filtered.
            Self::ribbon_group_ui(ui, tab, RG::Print, |ui| {
                // Disabled-and-explained with no document (R83), never
                // hidden — and the ellipsis says it opens something
                // rather than acting, which for the one irreversible
                // command in this application is worth the character.
                let has_doc = matches!(self.status, Status::Open(_));
                ui.add_enabled_ui(has_doc, |ui| {
                    if Self::icon_text_toggle(
                        ui,
                        icons::Icon::Stamp,
                        false,
                        ui_text::print_open_label(),
                        ui_text::print_open_tooltip(),
                    )
                    .clicked()
                    {
                        actions.push(Action::OpenPrintDialog);
                    }
                });
            });
            Self::ribbon_group_ui(ui, tab, RG::LayoutReset, |ui| {
                if ui
                    .button(ui_text::reset_layout_button())
                    .on_hover_text(ui_text::reset_layout_tooltip())
                    .clicked()
                {
                    actions.push(Action::OpenResetLayout);
                }
            });
            Self::ribbon_group_ui(ui, tab, RG::Settings, |ui| {
                // Its own group, not a button inside LayoutReset: that
                // group is about the WINDOW, these are about how pdfce
                // reads and writes DOCUMENTS. Always shown — every one of
                // these choices is document-independent, so gating it on a
                // file being open would hide it exactly when a new user
                // goes looking for it.
                if ui
                    .button(ui_text::settings_button())
                    .on_hover_text(ui_text::settings_tooltip())
                    .clicked()
                {
                    actions.push(Action::OpenSettings);
                }
            });
            Self::ribbon_group_ui(ui, tab, RG::Help, |ui| {
                // Keyboard-shortcuts reference (P1-2), the other ungrouped
                // utility control: a disclosure surface, not an edit or a
                // document-scoped tool, so it sits beside Tools rather than in
                // any group. Shown always (its content is document-independent
                // — the chords work the same with or without a file open).
                if Self::icon_button(ui, icons::Icon::Keyboard, ui_text::shortcuts_tooltip())
                    .clicked()
                {
                    self.shortcuts_open = !self.shortcuts_open;
                }
            });
            // The status summary is NOT emitted here — it is pinned to
            // the row's right edge by [`Self::toolbar`]'s outer
            // right-to-left layout, so that it cannot join the wrap flow
            // and drift leftward as future Passes append tool groups.
        }
    }

    // -- document properties -----------------------------------------
}
