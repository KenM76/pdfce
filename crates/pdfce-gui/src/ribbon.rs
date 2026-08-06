//! # ribbon — the tabbed command surface (Pass 24.1)
//!
//! ## Why a ribbon, and why these tabs
//!
//! The operator, 2026-08-05: *"I want you to work on the ribbon area as it is
//! a mess of controls and buttons. Needs organized tabs … File (including
//! reset layout option with checkboxes of what to reset), Edit, Tools (Batch
//! tools should be here, not where they appear now), View (controls for the
//! layout), Review, etc. Organize where you think they go best."*
//!
//! Before this, every control lived in ONE wrapping row divided by
//! `ui.separator()`. The row held file commands, view toggles, page
//! navigation, zoom, page rotation, document properties, four authoring
//! menus, four tool toggles, redaction, undo/redo, clipboard, the batch-tools
//! dock toggle and the shortcuts button — thirty-odd controls across nine
//! unrelated activities, wrapping onto two or three lines at ordinary window
//! widths. "A mess" is a fair description.
//!
//! ## The organising question, which is NOT another product's menu
//!
//! Standing rule **R123** (and CLAUDE.md rule 12, and R61 for Inkscape)
//! forbid deriving this structure from Acrobat's or Inkscape's. The axis used
//! instead already existed in this codebase before any ribbon did: `main.rs`
//! separates view-state commands, which *"govern what is on screen rather
//! than the document"*, from edit commands, because *"mixing the two would
//! make 'does this button change my file?' unanswerable at a glance."* Each
//! tab below answers one question of that kind.
//!
//! The operator named five tabs (File, Edit, Tools, View, Review) and left the
//! rest to judgement. All five are here, with one addition — Measure — argued
//! at [`RibbonTab::Measure`].
//!
//! ## What this module does and does not own
//!
//! It owns the **taxonomy**: which tabs exist, what each is for, and which
//! groups belong to which tab. It does not own the widgets — those stay in
//! `main.rs`, which gates each existing group on
//! [`RibbonTab::shows`]. That split is deliberate for one Pass: moving thirty
//! controls and re-tabbing them at the same time would make a regression in
//! either indistinguishable from a regression in the other.
//!
//! ## The customization the operator asked to keep possible
//!
//! *"we might want to make these customizable in the future like you can with
//! solidworks and ms office."* That is not built here, and this module is
//! shaped so it can be: [`RibbonGroup`] is a **stable identity**, not a source
//! position, so a future operator-defined layout is a `Vec<(RibbonTab,
//! Vec<RibbonGroup>)>` that replaces [`RibbonTab::groups`] without touching a
//! single widget. What is deliberately NOT built: reorder/hide UI, and
//! persistence. `ui_text::dock_layout_session_only_note` already tells the
//! operator the dock layout does not survive a restart; a customizable ribbon
//! that also forgot itself would be worse than one that cannot be customized.

use crate::ui_text;

/// One band of related commands within a tab.
///
/// A **stable identity**, deliberately: the future customization surface has
/// to name a group across sessions and across pdfce versions, and a position
/// in a source file cannot be named. Adding a variant is how a new group
/// arrives; renaming one is a breaking change to any saved layout, which is
/// the honest cost and the reason the names are chosen to describe the
/// *activity* rather than the current membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RibbonGroup {
    /// Open / Save a copy.
    FileOps,
    /// Document metadata (`/Info`).
    DocumentProperties,
    /// Get content OUT — copy text to the clipboard.
    Clipboard,
    /// Reset the panel arrangement, with a choice of what to reset.
    LayoutReset,
    /// Keyboard-shortcut reference.
    Help,
    /// Undo / Redo.
    History,
    /// Whole-page operations — rotation today.
    Pages,
    /// Move between pages.
    Navigate,
    /// The tools that change what is drawn: Edit Text, Add Text, Obj.
    ContentTools,
    /// Interactive-form (AcroForm) filling.
    ///
    /// # Why its own group and not a member of `ContentTools`
    ///
    /// `ContentTools` arms canvas TOOLS — each of its entries sets
    /// `doc.active_tool` and takes over pointer input. Filling a form does
    /// neither: the Forms panel is a list of the document's fields that works
    /// with no tool armed and never touches the canvas gesture state. Putting
    /// it in that group would promise a mode change that does not happen.
    ///
    /// # Why `Edit` and not `Tools` (where `Protect`/Redact lives)
    ///
    /// Redact sits under `Tools` because of what it IS — an irreversible,
    /// document-wide operation that deserves distance from routine editing.
    /// That reasoning does not transfer: filling a field is reversible before
    /// save, low-stakes, and is exactly the routine document editing the
    /// `Edit` tab is for.
    Forms,
    /// Annotation authoring: shapes, highlights.
    Markup,
    /// Annotation authoring: notes, free text, stamps.
    Notes,
    /// The three ce-dimension measure tools.
    ///
    /// # There is deliberately no `MeasureGroups` beside this
    ///
    /// One existed, briefly, and was declared with no widget gated to it —
    /// the Measure tab promised a "Groups" band that rendered nothing, and
    /// `every_ribbon_group_is_gated_to_a_widget` caught it on that test's
    /// first run.
    ///
    /// It was deleted rather than given a widget, because the widget already
    /// exists somewhere better. The ce-dimension group picker and "Groups…"
    /// button live in the Tool Options pane, which is where Pass 34.1 put
    /// every armed tool's live controls. Adding a second copy to the ribbon
    /// would be exactly the two-mental-models duplication decision 024 §1.3
    /// item 6 names — and the same objection that made 024 reject a Tools tab
    /// holding batch operations.
    ///
    /// The division that came out of this arc, stated once here because it is
    /// the thing to check the next time a group is proposed: **the ribbon
    /// picks the activity; the sidebar holds that activity's controls.**
    MeasureTools,
    /// Batch operations across whole files.
    Batch,
    /// Operator-supplied font folders.
    Fonts,
    /// Redaction — irreversible, and grouped apart for that reason.
    Protect,
    /// Zoom and fit.
    Zoom,
    /// What is drawn over the page: annotations, editable points.
    Show,
    /// Panel visibility.
    Panels,
}

impl RibbonGroup {
    /// Every group.
    ///
    /// Exists so a sweep cannot silently miss a variant — the list the
    /// module's own test walks, and the one `main.rs` walks to check that
    /// every declared group is gated to a widget. A second hand-written list
    /// would drift from this one, which is the failure both tests exist to
    /// prevent.
    #[allow(
        dead_code,
        reason = "the group enumeration; swept by this module's taxonomy test and by main.rs's gated-widget test, and the list any future group-picker must read rather than re-derive" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub const ALL: [Self; 19] = [
        Self::FileOps,
        Self::DocumentProperties,
        Self::Clipboard,
        Self::LayoutReset,
        Self::Help,
        Self::History,
        Self::Pages,
        Self::Navigate,
        Self::ContentTools,
        Self::Forms,
        Self::Markup,
        Self::Notes,
        Self::MeasureTools,
        Self::Batch,
        Self::Fonts,
        Self::Protect,
        Self::Zoom,
        Self::Show,
        Self::Panels,
    ];

    /// The caption printed under the group (R1: through the catalog).
    #[must_use]
    pub fn caption(self) -> &'static str {
        match self {
            Self::FileOps => ui_text::ribbon_group_file_ops(),
            Self::DocumentProperties => ui_text::ribbon_group_document_properties(),
            Self::Clipboard => ui_text::ribbon_group_clipboard(),
            Self::LayoutReset => ui_text::ribbon_group_layout_reset(),
            Self::Help => ui_text::ribbon_group_help(),
            Self::History => ui_text::ribbon_group_history(),
            Self::Pages => ui_text::ribbon_group_pages(),
            Self::Navigate => ui_text::ribbon_group_navigate(),
            Self::ContentTools => ui_text::ribbon_group_content_tools(),
            Self::Forms => ui_text::ribbon_group_forms(),
            Self::Markup => ui_text::ribbon_group_markup(),
            Self::Notes => ui_text::ribbon_group_notes(),
            Self::MeasureTools => ui_text::ribbon_group_measure_tools(),
            Self::Batch => ui_text::ribbon_group_batch(),
            Self::Fonts => ui_text::ribbon_group_fonts(),
            Self::Protect => ui_text::ribbon_group_protect(),
            Self::Zoom => ui_text::ribbon_group_zoom(),
            Self::Show => ui_text::ribbon_group_show(),
            Self::Panels => ui_text::ribbon_group_panels(),
        }
    }
}

/// A fixed ribbon tab.
///
/// Six, each answering one question. The count is a real constraint rather
/// than an aesthetic: a tab strip that wraps is a tab strip with hidden tabs,
/// which is the failure this whole Pass exists to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RibbonTab {
    /// *"What do I do with the file as a whole, or with pdfce itself?"*
    ///
    /// A TAB, not a menu button. Decision 024 §3.5 proposed a menu, on the
    /// convention that File is where an application's own commands live
    /// rather than the document's. The operator listed it among the tabs, so
    /// it is a tab — and his reason is visible in what he put in it: *"reset
    /// layout option with checkboxes of what to reset"* is a settings surface,
    /// and a settings surface behind a menu is a settings surface nobody
    /// finds.
    #[default]
    File,
    /// *"What am I CHANGING about what is already there?"*
    ///
    /// Page rotation lives here rather than on a Pages tab of its own: it
    /// changes the document, and an operator asking "how do I turn this page"
    /// is asking an editing question. Undo/Redo sits alongside because it is
    /// the same question answered backwards.
    Edit,
    /// *"What am I adding for someone else to read?"*
    ///
    /// The operator named this tab. Decision 024 §3.2 rejected a Review tab —
    /// but it rejected the *lifecycle* reading (Review/Prepare/Share as
    /// workflow stages, which groups by a stage the operator may not be in).
    /// This is not that. It is the object-based reading: annotations are
    /// content added ON TOP of the document for a reader, which is a
    /// different kind of thing from content that IS the document. That
    /// distinction is one pdfce's own model already draws — annotations are a
    /// separate object space from page content — so the tab is derived from
    /// the file format, not from a product's workflow.
    Review,
    /// *"What am I measuring, and in what units?"*
    ///
    /// Not named by the operator, and added anyway — the one addition that
    /// needs defending. ce dimensions are technically annotations (`/Line` +
    /// `/IT /LineDimension`), so Review could hold them. Two reasons not to:
    /// measuring is this operator's stated primary activity on CAD drawings,
    /// and the measure surface is not one button but a tool trio plus a group
    /// picker plus scale management. Burying that under a menu on a tab about
    /// sticky notes would reproduce, one level up, exactly the crowding this
    /// Pass is undoing.
    Measure,
    /// *"What do I run ACROSS files, or configure once?"*
    ///
    /// The operator: *"Batch tools should be here, not where they appear
    /// now."* Decision 024 §3.2 explicitly rejected a Tools tab holding the
    /// batch operations — but read its reason: *"duplicating their entry
    /// points is exactly the two-mental-models problem."* The objection was to
    /// DUPLICATION. This is a MOVE: `DockPanel::BatchTools` goes away and its
    /// commands live here alone. The objection is answered rather than
    /// overruled.
    Tools,
    /// *"What is on my screen?"*
    ///
    /// The operator: *"View (controls for the layout)."* Everything here
    /// changes what is displayed and nothing here changes the document —
    /// which is the same line `action_preserves_gesture` draws for deciding
    /// what may safely interrupt an in-progress edit. The tab and that
    /// predicate should agree, and where they disagree one of them is wrong.
    View,
}

impl RibbonTab {
    /// Every tab, in strip order.
    pub const ALL: [Self; 6] = [
        Self::File,
        Self::Edit,
        Self::Review,
        Self::Measure,
        Self::Tools,
        Self::View,
    ];

    /// The tab's label (R1).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::File => ui_text::ribbon_tab_file(),
            Self::Edit => ui_text::ribbon_tab_edit(),
            Self::Review => ui_text::ribbon_tab_review(),
            Self::Measure => ui_text::ribbon_tab_measure(),
            Self::Tools => ui_text::ribbon_tab_tools(),
            Self::View => ui_text::ribbon_tab_view(),
        }
    }

    /// The tab's purpose tooltip — the organising QUESTION it answers, never
    /// a restatement of its label or a list of its contents.
    ///
    /// A list would go stale the first time a group moved; the question is
    /// what lets an operator work out where an unfamiliar command lives
    /// without hunting, which is the entire point of having tabs.
    #[must_use]
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::File => ui_text::ribbon_tab_file_tooltip(),
            Self::Edit => ui_text::ribbon_tab_edit_tooltip(),
            Self::Review => ui_text::ribbon_tab_review_tooltip(),
            Self::Measure => ui_text::ribbon_tab_measure_tooltip(),
            Self::Tools => ui_text::ribbon_tab_tools_tooltip(),
            Self::View => ui_text::ribbon_tab_view_tooltip(),
        }
    }

    /// The groups this tab shows, in band order.
    ///
    /// **The single definition of which groups belong to which tab.** A
    /// future operator-customized layout replaces the result of this function
    /// and nothing else — no widget moves, because `main.rs` asks
    /// [`Self::shows`] rather than knowing which tab it is drawing.
    ///
    /// # The ORDER here is descriptive, not yet authoritative — read this
    ///
    /// Membership is enforced (`shows` is the only gate, and a test asserts
    /// every group has exactly one owner). Order is **not**: the band renders
    /// in the order the widget blocks appear in `main.rs`, so this list is
    /// maintained to MATCH that rather than to drive it. It was already wrong
    /// once — File was declared FileOps/Clipboard/Document and renders
    /// FileOps/Document/Clipboard — and a screenshot caught it, which is the
    /// honest reason this warning exists rather than a claim that order is
    /// authoritative.
    ///
    /// The registry Pass that moves the widgets behind stable command ids is
    /// what makes this list drive order too. Until then, changing the order
    /// here changes nothing on screen, and a reader must not assume otherwise.
    #[must_use]
    pub fn groups(self) -> &'static [RibbonGroup] {
        match self {
            Self::File => &[
                RibbonGroup::FileOps,
                RibbonGroup::DocumentProperties,
                RibbonGroup::Clipboard,
                RibbonGroup::LayoutReset,
                RibbonGroup::Help,
            ],
            Self::Edit => &[
                RibbonGroup::History,
                RibbonGroup::ContentTools,
                RibbonGroup::Forms,
                RibbonGroup::Pages,
            ],
            Self::Review => &[RibbonGroup::Markup, RibbonGroup::Notes],
            Self::Measure => &[RibbonGroup::MeasureTools],
            Self::Tools => &[RibbonGroup::Batch, RibbonGroup::Fonts, RibbonGroup::Protect],
            Self::View => &[
                RibbonGroup::Navigate,
                RibbonGroup::Zoom,
                RibbonGroup::Show,
                RibbonGroup::Panels,
            ],
        }
    }

    /// Whether `group` belongs to this tab.
    ///
    /// The one question `main.rs` asks. Every widget block is gated on it, so
    /// a group's tab assignment is changed HERE and nowhere else — which is
    /// what makes the future customization a data change rather than a
    /// widget-moving exercise.
    #[must_use]
    pub fn shows(self, group: RibbonGroup) -> bool {
        self.groups().contains(&group)
    }
}

/// What a layout reset puts back, chosen by the operator (Pass 24.1).
///
/// # Why checkboxes rather than one button
///
/// The operator asked for *"reset layout option with checkboxes of what to
/// reset"*, and the request exposes a real defect in what existed. The old
/// `Action::ResetPanelLayout` rebuilt `self.dock` — the RIGHT dock — and
/// nothing else. Pass 34.1 added a second, LEFT dock, and the reset never
/// learned about it: an operator who had dragged the left dock into a state
/// they disliked had no way back at all, and the control that claimed to
/// reset the layout silently did half the job.
///
/// So this is not only a nicer affordance; it is the fix. And the checkboxes
/// are what make it safe to widen: "reset the layout" now means more than it
/// did, and an operator who wanted only the right dock back should not lose
/// their left one to a broadened definition.
///
/// All fields default to `true` — the common case is "put it all back" — but
/// the choice is visible before it is applied, which is rule 4's requirement
/// on anything pdfce would otherwise decide for the operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetScope {
    /// The RIGHT dock's pane arrangement (Objects / Properties / Redact).
    pub right_panels: bool,
    /// The LEFT dock's pane arrangement (Pages / Tool Options) — the half the
    /// old reset could not reach.
    pub left_panels: bool,
    /// Whether the two docks are open at all.
    pub visibility: bool,
}

impl Default for ResetScope {
    fn default() -> Self {
        Self {
            right_panels: true,
            left_panels: true,
            visibility: true,
        }
    }
}

impl ResetScope {
    /// Whether this scope would do anything at all.
    ///
    /// Used to disable the confirm control rather than let an operator press
    /// a button that is defined to have no effect — R83's rule that an
    /// affordance must be real, applied to a control the operator can empty
    /// out themselves by clearing every box.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.right_panels && !self.left_panels && !self.visibility
    }
}

/// What the Tool Options pane is currently showing (Pass 24.3).
///
/// # Why this exists
///
/// The operator, 2026-08-05: *"the only thing that should be visible in the
/// right side panel is the Objects tree. Clicking for those other items should
/// bring up the options and dialogus in the Tool Options tab."*
///
/// Before this, the right dock held four panes — Objects, Properties, Batch
/// Tools, Redact — which made it a second, competing home for controls. That
/// split the answer to "where do I configure the thing I just clicked" across
/// two sides of the window, which is the two-mental-models failure decision
/// 024 §1.3 item 6 names.
///
/// The rule the whole arc converged on, now applied to the last exception:
/// **the ribbon picks the activity; the sidebar holds its controls.** The
/// right dock keeps exactly one job — *what is on this page* — and everything
/// that is a control moves left.
///
/// `ActiveTool` is the default and the one that follows `doc.active_tool`;
/// the others are pinned by a ribbon command until something else replaces
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaneSubject {
    /// Batch operations across whole files, and the font folders.
    ///
    /// The DEFAULT, because the Activities compartment is always on screen
    /// now and therefore always showing something: it needs a subject that is
    /// useful to land on rather than one that is a mode. Redact and Forms are
    /// both destinations an operator goes to deliberately; Batch Tools is the
    /// closest thing here to a neutral resting state.
    #[default]
    BatchTools,
    /// The redaction review surface.
    Redact,
    /// The interactive-form (AcroForm) field list — see
    /// `docs/ui_specs/forms-panel.md`.
    ///
    /// A fifth subject rather than a section of [`Self::Properties`]: that
    /// pane is about "the selected thing", and a form fill shares no state
    /// with a canvas selection. It is also not [`Self::ActiveTool`], which is
    /// structurally tied to `doc.active_tool` — a value the Forms panel
    /// correctly never sets, because filling a field arms no canvas tool.
    Forms,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every group belongs to exactly one tab.
    ///
    /// Two tabs claiming one group would put the same controls in two places,
    /// which is the "two mental models" failure decision 024 §1.3 item 6
    /// names — and the specific objection that made it reject a Tools tab.
    /// Zero tabs claiming one would make it unreachable, which is worse.
    #[test]
    fn every_group_belongs_to_exactly_one_tab() {
        for group in RibbonGroup::ALL {
            let owners: Vec<RibbonTab> = RibbonTab::ALL
                .into_iter()
                .filter(|t| t.shows(group))
                .collect();
            assert_eq!(
                owners.len(),
                1,
                "{group:?} is claimed by {owners:?}, expected exactly one tab"
            );
        }
    }

    /// No tab is empty — an empty tab is a promise of contents that are not
    /// there.
    #[test]
    fn no_tab_is_empty() {
        for tab in RibbonTab::ALL {
            assert!(!tab.groups().is_empty(), "{tab:?} has no groups");
        }
    }

    /// An emptied reset scope reports itself empty, so the confirm can be
    /// disabled rather than doing nothing.
    #[test]
    fn a_cleared_reset_scope_is_empty() {
        let none = ResetScope {
            right_panels: false,
            left_panels: false,
            visibility: false,
        };
        assert!(none.is_empty());
        assert!(!ResetScope::default().is_empty());
    }
}
