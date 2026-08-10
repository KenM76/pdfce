//! Canvas overlays — everything pdfce draws ON TOP of the rendered page.
//!
//! # What belongs here
//!
//! Selection outlines, type badges, vector-node marks, form-field chrome
//! and the disclosure text that goes with them. Everything in this module
//! paints over a page that has already been rastered; none of it renders
//! page content, and none of it can change a document.
//!
//! That is the boundary worth stating, because it is what makes the file
//! safe to reason about in isolation: a defect here can draw the wrong
//! marks on the screen, and cannot corrupt a file.
//!
//! # Why it was moved out of `main.rs`
//!
//! `main.rs` was 27,647 lines — the ribbon, every panel, the canvas
//! overlays and the whole application state machine in one file. The
//! operator asked whether the interface could be changed without a
//! refactor and a crop of new bugs, and the honest answer was that the
//! size itself was the obstacle: not because large files are untidy, but
//! because there was no way to open "the overlay drawing" without
//! opening everything else with it.
//!
//! This is the first stage of splitting it, done as its own commit so it
//! is separately revertable. It is a MOVE: no logic changed, no
//! signature changed, and the same tests cover it before and after. The
//! only edits are the visibility markers the compiler required to see
//! these items from another module, and those are `pub(crate)` — the
//! narrowest thing that works, never `pub`, because nothing outside this
//! binary has any business calling them.
//!
//! # The overlay vocabulary is semantic, and lives in `theme`
//!
//! Every colour drawn here is a named role in [`crate::theme`]: the node
//! mark, the subpath outline, the preview, the guide, the field chrome.
//! Those hues are not decoration — they distinguish "a point is here"
//! from "this run is one subpath", and a proposal from committed
//! document state (rule 4). `check-theme-colors.sh` will refuse a raw
//! colour added to this file.

use crate::describe_object;
use crate::object_summary::ObjectSummary;
use crate::{
    OpenDoc, canvas, diag, node_mark_color, node_mark_fill, ribbon, subpath_outline_color, theme,
    ui_text, vector_edit_tool, viewer,
};
use canvas::CanvasTool;
use eframe::egui;

/// The side of the square type-badge chip drawn at a selection's top-left
/// corner, in egui logical points.
///
/// Big enough for a legible capital at the badge font size, small enough that
/// it does not swamp the outline of a small object.
pub(crate) const SELECTION_BADGE_SIZE: f32 = 15.0;

/// How many selected objects may carry a type badge before badges are
/// suppressed and only outlines are drawn.
///
/// Not a silent cap on information: the status-bar readout states the true
/// selection count and its per-kind census whatever this number is, so nothing
/// becomes unknowable. It is a legibility cap — past a few dozen objects the
/// chips overlap into a smear that answers nothing, and each one costs a text
/// galley per frame. The badge exists to answer "what is THIS?", a question
/// that only has an answer while the selection is small enough to point at.
pub(crate) const MAX_SELECTION_BADGES: usize = 48;

/// The dash and gap lengths, in egui logical points, of the outline drawn
/// around an object whose bounds are an APPROXIMATION rather than a
/// measurement (today: every text object).
pub(crate) const APPROXIMATE_OUTLINE_DASH: (f32, f32) = (6.0, 4.0);

/// Surface the disclosures a vector surgery owes into the narrator.
///
/// # Why a note and not silence
///
/// Dragging one corner of a rectangle out of square, or moving a shape whose
/// start point the file never wrote down, forces pdfce to change HOW the shape
/// is written in order to do what was asked (`re` cannot spell a
/// non-rectangle; a subpath cannot be moved off a start it inherits). The
/// picture is unchanged, so nothing on screen would tell the operator — and
/// dragging back does not restore the original bytes. Rule 4 (fuzzy, never
/// sneaky) applies to representation, not just to geometry.
///
/// Routed through `pending_note` because this runs under the `&mut OpenDoc`
/// borrow, where `self.edit_note` is unreachable; the app drains it once the
/// borrow ends. Its neighbours' habit of discarding an outcome to compile is
/// exactly what that channel exists to stop.
///
/// Multiple disclosures are joined rather than overwritten: keeping only the
/// last is the silent-truncation failure, and no current surgery emits more
/// than one anyway.
pub(crate) fn disclose_vector_edit(doc: &mut OpenDoc, disclosures: &[String]) {
    if disclosures.is_empty() {
        return;
    }
    // ui-text-exempt: a separator between core-authored sentences, not a message
    doc.pending_note = Some(disclosures.join(" "));
}

/// Draw the **editor chrome** for every form-field widget on the current page
/// — a dashed, tinted outline and a type letter — so a field the document
/// gives no visible border can still be found (Pass 47.3, standing rule
/// **R167**).
///
/// # The defect this exists for, observed rather than argued
///
/// `fixtures/synthetic/forms/demo-form.pdf` has two fields. Opened in pdfce on
/// 2026-08-08, the canvas showed **one** — the check box, whose appearance is
/// vector artwork pdfce draws. The text field was not faint or unstyled; it
/// was **absent**, and the status bar said so in the application's own words:
///
/// ```text
/// 1 annotation(s) have no appearance stream pdfce can paint, so nothing was
/// drawn for them (pdfce never invents a look): Widget ×1.
/// ```
///
/// Meanwhile the Forms panel listed *"Full name (p. 1)"* with a working text
/// box. **The operator could type a value into a field they could not
/// locate.** For a form editor that is close to disqualifying, and it is the
/// direct cause of the operator's *"feels incomplete"* report.
///
/// # R43 is NOT the bug, and is not changed
///
/// R43 says pdfce renders an annotation from its `/AP` or not at all — it
/// never synthesises appearance from `/MK`. That is a real spec-fidelity
/// property worth protecting: a border pdfce invented would be a border the
/// document does not have, and would then differ from every other viewer.
///
/// The bug was that **nothing ever filled the gap R43 deliberately leaves.**
/// R167 is the companion rule that authorises this layer, on three conditions,
/// all met here:
///
/// 1. **Never written.** This paints into the frame and touches no object. No
///    `/AP`, no `/MK`, no dirty bit — the document is byte-identical whether
///    this ran or not.
/// 2. **Unmistakably chrome.** A DASHED stroke in a distinct hue. The dash is
///    the load-bearing half: no PDF producer emits a dashed hairline as a
///    field border by convention, so the outline cannot be mistaken for the
///    document's own artwork the way a solid rectangle could.
/// 3. **Editing context only.** Gated on [`form_context_active`], so a plain
///    viewing session shows exactly what R43 already guarantees and nothing
///    more.
///
/// # Why the type letter, and why not an icon
///
/// A rectangle says *"a field is here"* and not *"which kind"*, and the kinds
/// behave so differently that confusing them wastes the operator's time — a
/// check box is a two-state toggle, a choice field cannot be filled at all
/// without options. A single letter is legible at the size a widget actually
/// occupies (often 18×18 pt) where a glyph would be a smudge, and it costs no
/// icon-atlas lookup per widget on a page that may carry hundreds.
/// Whether a **form-editing context** is active, which is what gates the
/// field chrome (Pass 47.3, R167 condition 3).
///
/// Two states qualify, and both mean the operator is working ON the form
/// rather than reading the document:
///
/// - the **Forms panel** is the visible pane subject — they are looking at a
///   list of fields and need to know which row is which rectangle;
/// - the **Create Field** tool is armed — they are placing fields and need to
///   see the ones already there, if only to avoid landing on top of one.
///
/// Outside those, nothing is drawn and a plain viewing session shows exactly
/// what R43 guarantees. That gate is the whole reason R167 can authorise this
/// layer without weakening R43: the chrome is never present when someone is
/// simply reading a PDF.
pub(crate) fn form_context_active(app_subject: ribbon::PaneSubject, doc: &OpenDoc) -> bool {
    app_subject == ribbon::PaneSubject::Forms || doc.active_tool() == Some(CanvasTool::PlaceField)
}

pub(crate) fn draw_form_field_chrome(
    doc: &OpenDoc,
    ui: &egui::Ui,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
    highlight: Option<pdfce_core::object::ObjId>,
) {
    let Some(page) = doc.pages.get(doc.view.page_index) else {
        return;
    };
    let Some(form) = pdfce_core::forms::parse_acroform(&doc.session.graph()) else {
        return;
    };
    let painter = ui.painter_at(image_rect);
    let base = ui.visuals().selection.stroke.color;
    // A hue of its own, distinct from the object-selection accent, because it
    // means something different: "a control lives here", not "this is
    // selected". R84 — the dash carries the distinction too, not colour alone.
    let chrome = theme::Theme::of(ui.ctx()).palette.field_chrome;

    let page_index = doc.view.page_index;
    let Some(page_id) = doc.pages.get(page_index).map(|p| p.id) else {
        return;
    };
    for field in &form.fields {
        for widget in &field.widgets {
            // Only this page's widgets. `/P` is authoritative when present;
            // without it the widget is skipped rather than guessed onto the
            // current page, which would draw a box over a field that is
            // somewhere else entirely.
            // `/P` is authoritative when present; without it the widget is
            // SKIPPED rather than guessed onto the current page, which would
            // draw a box over a field that lives somewhere else entirely.
            // Same identity the Forms panel's "(p. N)" suffix resolves.
            if widget.page != Some(page_id) {
                continue;
            }
            let Some(rect) = widget.rect else {
                continue;
            };
            let to_screen = |x: f64, y: f64| -> Option<egui::Pos2> {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "page coordinates are f32 on the canvas side already" // ui-text-exempt: clippy lint justification, never displayed
                )]
                let p = egui::pos2(x as f32, y as f32);
                viewer::pdf_space_to_canvas(p, page)
                    .map(|c| viewer::page_to_screen(c, image_rect, extent, zoom))
            };
            let (Some(a), Some(b)) = (to_screen(rect.llx, rect.lly), to_screen(rect.urx, rect.ury))
            else {
                continue;
            };
            let r = egui::Rect::from_two_pos(a, b);
            let highlighted = highlight == Some(field.id);
            let colour = if highlighted { base } else { chrome };
            let width = if highlighted { 2.0 } else { 1.0 };
            // Dashed, per R167 condition (2). Drawn as four dashed segments
            // rather than a stroked rect because egui has no dashed-rect
            // primitive.
            let dash = 4.0;
            let gap = 3.0;
            for (p0, p1) in [
                (r.left_top(), r.right_top()),
                (r.right_top(), r.right_bottom()),
                (r.right_bottom(), r.left_bottom()),
                (r.left_bottom(), r.left_top()),
            ] {
                painter.extend(egui::Shape::dashed_line(
                    &[p0, p1],
                    egui::Stroke::new(width, colour),
                    dash,
                    gap,
                ));
            }
            // The type letter, at the widget's top-left, only when the box is
            // big enough to hold it without covering the field's own content.
            if r.width() > 14.0 && r.height() > 12.0 {
                painter.text(
                    r.left_top() + egui::vec2(2.0, 1.0),
                    egui::Align2::LEFT_TOP,
                    ui_text::form_field_chrome_letter(field.field_type, field.button_kind),
                    egui::FontId::monospace(9.0),
                    colour,
                );
            }
        }
    }
}

pub(crate) fn draw_selection_outlines(
    doc: &OpenDoc,
    ui: &egui::Ui,
    image_rect: egui::Rect,
    extent: (f32, f32),
    zoom: f32,
    // The pointer in PDF page space, when it is over the canvas (Pass 26.2).
    // Used to HIGHLIGHT the node the operator is about to grab, BEFORE they
    // press — *"they should highlight when I hover the mouse over them."*
    // Without it, the only feedback that a press will hit a node is the
    // geometry moving afterwards, which arrives one step too late to help.
    hover_pdf: Option<pdfce_core::vector::Point>,
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
        && let Some(b) = provider.part_bounds_canvas(entered.object, sp)
        && let Some(page) = doc.pages.get(doc.view.page_index)
    {
        let painter = ui.painter_at(image_rect);
        // PDF page space -> canvas space -> screen (Pass 36.3).
        //
        // # The bug this closure replaces, at three call sites
        //
        // `subpath_node_points` and `subpath_handle_points` return PDF **page
        // space** (y-up, origin bottom-left). `subpath_bounds_canvas`, three
        // lines below, returns **canvas space** (y-down, origin top-left), and
        // `page_to_screen` takes canvas space. All three point-drawing sites
        // fed their PDF-space points straight into `page_to_screen`, skipping
        // the flip — carrying a comment that asserted "the outline above uses
        // the same conversion for its corners", which was exactly the mistake:
        // the outline goes through `_canvas`, these did not.
        //
        // The marks were therefore drawn at the VERTICALLY MIRRORED position.
        // Measured on 2026-08-05 with a real screenshot: on a 400 pt page, the
        // anchors of a line at PDF y=340 were painted ~680 screen px below it,
        // near the bottom of the page. Nothing was missing and nothing was
        // invisible — the marks were somewhere else, which is why the operator
        // reported "there is no visual cue on screen that shows me the nodes"
        // even for a part whose points were being drawn.
        //
        // Hit-testing was never affected: `nearest_node` converts the CLICK
        // into PDF space and compares there, which is correct. So the pointer
        // and the paint disagreed about where a node was, and only the paint
        // was wrong — the reason descending onto a node worked whenever the
        // operator happened to aim at the true position rather than the drawn
        // one, and the reason this survived every headless check.
        let pdf_to_screen = |p: pdfce_core::vector::Point| -> Option<egui::Pos2> {
            #[allow(clippy::cast_possible_truncation)]
            let q = egui::pos2(p.x as f32, p.y as f32);
            viewer::pdf_space_to_canvas(q, page)
                .map(|c| viewer::page_to_screen(c, image_rect, extent, zoom))
        };
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
            egui::Stroke::new(2.0, subpath_outline_color(ui.ctx())),
            egui::StrokeKind::Outside,
        );

        // ---- The part's anchors, drawn as squares ----------------------
        //
        // # Pass 36.3: this used to require `entered.node.is_some()`
        //
        // That is, the point marks appeared only AFTER a point had been
        // selected — and the only way to select one is to double-click within
        // grab range of it. The marks that tell you where the points are were
        // gated behind having already hit one. A bootstrapping deadlock:
        // aim blind, and you are rewarded with the aiming aid.
        //
        // The operator, 2026-08-05: *"node editing is still very hard to
        // accomplish. there is no visual cue on screen that shows me the nodes
        // when I've clicked down to individual features."*
        //
        // The original reasoning was that drawing them at the Part rung would
        // put up to 6,681 marks on a CAD object "with no gesture to use them".
        // Both halves were wrong by the time they were written. The count is
        // what `MAX_DRAWN_NODES` already caps — with a disclosure, not a
        // silent first-N — so the CAD case was handled here and nowhere else
        // needed to be. And the gesture that uses them is the descending
        // double-click itself, which is performed FROM the Part rung: the
        // points have to be visible exactly one rung before the code was
        // showing them.
        //
        // It also inverted the principle it cited. Decision 028 §Q1 says the
        // points you can grab are the points you can see; the grab happens at
        // the Part rung, so that is where they must be seen.
        //
        // SQUARE, and sized in SCREEN pixels. Square so that the node/handle
        // distinction is carried by shape rather than by colour alone (R84).
        // Screen-sized so a node is the same target at every zoom — matching
        // `NODE_GRAB_SCREEN_TOLERANCE_PX`, or the mark and the grab radius
        // would disagree about where a point is.
        {
            let nodes = provider.subpath_node_points(entered.object, sp);
            // Which node the pointer is over, resolved with the SAME radius the
            // grab uses (Pass 26.2). If the highlight and the grab disagreed,
            // the highlight would be a promise the press does not keep — which
            // is worse than no highlight, because the operator would learn to
            // distrust it.
            let hover_index = hover_pdf.and_then(|h| {
                let tol = canvas::screen_tolerance_to_page(
                    vector_edit_tool::NODE_GRAB_SCREEN_TOLERANCE_PX,
                    zoom,
                );
                nodes
                    .iter()
                    .filter(|(_, p)| p.is_finite())
                    .map(|(i, p)| (*i, p.distance(h)))
                    .filter(|(_, d)| *d <= tol)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(i, _)| i)
            });
            // Pass 36.3 observability. "Are the point marks on screen" is the
            // exact question the operator's report turned on, and it is not
            // answerable from the depth trace: `node: None` is the state BOTH
            // when the marks were never drawn (the defect) and when they are
            // drawn and simply none is selected (the fix). One line tells the
            // two apart without a screenshot.
            diag::trace(|| {
                format!(
                    "node-marks part={sp} count={} drawn={} selected={:?}",
                    nodes.len(),
                    nodes.len() <= canvas::MAX_DRAWN_NODES,
                    entered.node
                )
            });
            if nodes.len() > canvas::MAX_DRAWN_NODES {
                // Never a silent first-N: an operator shown 300 of 1,200
                // points would reasonably believe the part has 300.
                //
                // Painted ON the part rather than sent to the status line,
                // because it is a statement about what this outline does and
                // does not contain — putting it where the operator is already
                // looking beats a note elsewhere that competes with every
                // other note. (Drawing must not mutate session state either;
                // this function holds `doc` by shared reference.)
                painter.text(
                    rect.left_top() + egui::vec2(4.0, -18.0),
                    egui::Align2::LEFT_TOP,
                    ui_text::subpath_node_view_off(nodes.len(), canvas::MAX_DRAWN_NODES),
                    egui::FontId::proportional(12.0),
                    subpath_outline_color(ui.ctx()),
                );
            } else {
                // ---- Bézier handles, drawn UNDER the node marks ----------
                //
                // Under, because a handle can sit almost on top of its own
                // node when the curve is nearly flat there, and in that case
                // the node is the thing that must stay legible — it is the
                // point the curve actually passes through.
                //
                // Shown for EVERY node of the entered part that has one, not
                // only the selected node: handles are how an operator decides
                // WHICH node to pick, so hiding them until after the pick
                // inverts the order the information is needed in
                // (decision 028 §Q2).
                for (index, side, h) in provider.subpath_handle_points(entered.object, sp) {
                    let Some(hc) = pdf_to_screen(h) else {
                        continue;
                    };
                    // The node this handle belongs to, so the arm can be drawn
                    // between them. Skipped rather than guessed if the node is
                    // not in the drawn set.
                    let Some(anchor) = nodes.iter().find(|(i, _)| *i == index).map(|(_, p)| *p)
                    else {
                        continue;
                    };
                    let Some(ac) = pdf_to_screen(anchor) else {
                        continue;
                    };
                    // EVERY selected anchor draws as selected, not only the
                    // primary. `selected_nodes` holds the complete set and
                    // `entered.node` names the primary within it; the fallback
                    // to `entered.node` covers the ordinary single-node case
                    // before any additive click has populated the set.
                    let selected =
                        doc.selected_nodes.contains(&index) || entered.node == Some(index);
                    // A DASHED arm ties the handle to its node. Dashing is
                    // already this project's signal for "this line is not a
                    // measured edge of the drawing" (APPROXIMATE_OUTLINE_DASH),
                    // so it reuses a meaning the operator has already met
                    // rather than inventing a fourth (decision 028 §Q2).
                    painter.add(egui::Shape::dashed_line(
                        &[ac, hc],
                        egui::Stroke::new(1.0, subpath_outline_color(ui.ctx())),
                        3.0,
                        3.0,
                    ));
                    let r = if selected {
                        canvas::HANDLE_MARK_SELECTED_PX
                    } else {
                        canvas::HANDLE_MARK_PX
                    } / 2.0;
                    // CIRCLE, where a node is a SQUARE — the node/handle
                    // distinction is carried by shape, so it survives for an
                    // operator who cannot separate the two colours (R84).
                    if selected {
                        painter.circle_filled(hc, r, ui.visuals().selection.stroke.color);
                    } else {
                        painter.circle_stroke(
                            hc,
                            r,
                            egui::Stroke::new(1.5, subpath_outline_color(ui.ctx())),
                        );
                    }
                    let _ = side;
                }
                for (index, p) in &nodes {
                    let (index, p) = (*index, *p);
                    let Some(c) = pdf_to_screen(p) else {
                        continue;
                    };
                    // Screen positions of the drawn anchors, so the scripted
                    // harness can CLICK a node instead of guessing a pixel.
                    diag::trace(|| format!("node-mark index={index} screen={c:?}"));
                    // EVERY selected anchor draws as selected, not only the
                    // primary. `selected_nodes` holds the complete set and
                    // `entered.node` names the primary within it; the fallback
                    // to `entered.node` covers the ordinary single-node case
                    // before any additive click has populated the set.
                    let selected =
                        doc.selected_nodes.contains(&index) || entered.node == Some(index);
                    // Pass 26.2: HOVER is a third state, between unselected and
                    // selected, and it is drawn at the SELECTED size so the
                    // operator sees the target grow under the pointer before
                    // they commit to pressing. It keeps the unselected FILL, so
                    // the three states stay distinguishable: small+pale,
                    // large+pale (about to be grabbed), large+accent (grabbed).
                    // Size and fill, never colour alone (R84).
                    let hovered = hover_index == Some(index) && !selected;
                    let half = if selected || hovered {
                        canvas::NODE_MARK_SELECTED_PX
                    } else {
                        canvas::NODE_MARK_PX
                    } / 2.0;
                    let r = egui::Rect::from_center_size(c, egui::vec2(half * 2.0, half * 2.0));
                    // FILLED in both states, differing in fill colour and size
                    // — never fill-vs-outline (Pass 36.3). An outline-only mark
                    // is the line it sits on showing through its middle, which
                    // on a 1 px CAD stroke reads as a slightly thicker bit of
                    // line rather than as a handle. Filling makes the square a
                    // square; size and fill colour then carry selection, and
                    // both survive greyscale (R84).
                    painter.rect_filled(
                        r,
                        0.0,
                        if selected {
                            ui.visuals().selection.stroke.color
                        } else {
                            node_mark_fill(ui.ctx())
                        },
                    );
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.5, node_mark_color(ui.ctx())),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }

        // ---- "Show points": the OTHER parts' anchors (Pass 36.3) --------
        //
        // The block above draws the SELECTED part's points, which is what an
        // operator needs to pick one within that part. It leaves every other
        // part of the same object dark — and on a CAD export where the object
        // is a whole drawing view, that is still working through a keyhole:
        // you cannot see where the points of the next line are until you have
        // selected that line.
        //
        // Drawn smaller and stroke-only, never filled, so the two populations
        // are told apart by SIZE at a glance and the selected part keeps
        // visual priority. That is a shape/size cue, not a colour one (R84),
        // which matters because these share the part outline's hue.
        //
        // The budget is shared with, not additional to, `MAX_DRAWN_NODES`: the
        // question the cap answers is "how many marks can be on screen before
        // they stop being marks", and that does not get a second allowance
        // because a different code path is drawing them. Parts are taken in
        // order until the budget is spent, and the remainder is DISCLOSED
        // rather than silently dropped — the same rule the single-part path
        // follows, for the same reason.
        if doc.show_all_points
            && let Some(provider) = doc.object_model.as_ref()
        {
            let painter = ui.painter_at(image_rect);
            // Kind-aware (`Pass 32.0`): a text object's parts are its runs.
            let part_count = provider.part_count(entered.object);
            diag::trace(|| format!("show-points object={} parts={part_count}", entered.object));
            let mut budget = canvas::MAX_DRAWN_NODES;
            let mut undrawn = 0usize;
            for other in 0..part_count {
                if other == sp {
                    continue; // already drawn, at full size, above
                }
                let pts = provider.subpath_node_points(entered.object, other);
                if pts.len() > budget {
                    undrawn = undrawn.saturating_add(pts.len());
                    continue;
                }
                budget -= pts.len();
                for (_, p) in &pts {
                    let Some(c) = pdf_to_screen(*p) else {
                        continue;
                    };
                    let half = canvas::NODE_MARK_OTHER_PART_PX / 2.0;
                    let r = egui::Rect::from_center_size(c, egui::vec2(half * 2.0, half * 2.0));
                    // Same fill-then-stroke treatment as the selected part's
                    // marks, for the same legibility reason; smaller, so the
                    // two populations are told apart by SIZE rather than by
                    // presence-of-fill (which is what selection means).
                    painter.rect_filled(r, 0.0, node_mark_fill(ui.ctx()));
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.0, node_mark_color(ui.ctx())),
                        egui::StrokeKind::Middle,
                    );
                }
            }
            if undrawn > 0 {
                let b = provider
                    .part_bounds_canvas(entered.object, sp)
                    .map_or(image_rect, |bb| {
                        egui::Rect::from_two_pos(
                            viewer::page_to_screen(bb.min, image_rect, extent, zoom),
                            viewer::page_to_screen(bb.max, image_rect, extent, zoom),
                        )
                    });
                painter.text(
                    b.left_top() + egui::vec2(4.0, -32.0),
                    egui::Align2::LEFT_TOP,
                    ui_text::other_parts_points_not_drawn(undrawn),
                    egui::FontId::proportional(12.0),
                    subpath_outline_color(ui.ctx()),
                );
            }
        }
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
pub(crate) fn draw_selection_badge(
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
