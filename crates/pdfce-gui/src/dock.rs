//! # dock — the right-hand panel dock, built on `egui_tiles`
//!
//! The tiling/tabbing layout engine for pdfce's right-hand panel host, and
//! the one place the `egui_tiles` dependency is touched. Implements
//! `docs/decisions/017-tabbed-dockable-panel-system.md` **as amended by
//! AMENDMENT A** (2026-08-02), which is what governs where this record and
//! its own §§1/3/8 disagree.
//!
//! ## What the operator asked for, and what that decided
//!
//! Decision 017 originally hand-rolled a two-compartment vertical row list
//! and pre-approved `egui_tiles` behind ONE named trigger (§6.1): *does the
//! panel system own only the narrow right-hand dock, or eventually the whole
//! content area?* The operator answered in the widest direction available —
//! *"Use egui_tiles. You're building something to compete with Acrobat and is
//! open source, and has the flexibal docking that works as well as
//! inkscape's"* — firing the trigger. Amendment A records the adoption; this
//! module is its implementation.
//!
//! ## The requirement that survived the reversal (A.3) — read this before
//! changing the default layout
//!
//! §3's two-compartment design solved a requirement that is INDEPENDENT of
//! which engine draws the tabs:
//!
//! > **Layers and Properties must be visible SIMULTANEOUSLY.** In every
//! > vector editor you select an object in the layer tree and edit its
//! > properties without losing sight of the tree. If Layers and Properties
//! > are mutually exclusive tabs, the tool is *worse* than the flat list it
//! > replaced.
//!
//! Under `egui_tiles` that becomes a **vertical split container**, and
//! Amendment A is explicit that it must ship **in the default layout** — not
//! be something the operator discovers by dragging a pane out. That is
//! exactly what [`default_tree`] builds: Objects on top, a
//! Properties/Batch-Tools tab group below. Do not "simplify" it into one tab
//! group.
//!
//! ## Narrow-column overflow is mitigated, not solved (A.3)
//!
//! §4's arithmetic still holds: `egui_tiles` draws **horizontal** tab bars
//! only, and 0.16.0's answer to a tab bar that does not fit is scroll arrows
//! — i.e. it *hides* tabs. Amendment A's mitigations are honoured here:
//! do not pack many panels into one narrow tab group by default (the lower
//! group holds two), and the dock's default width was raised from the
//! historic 320 pt now that it hosts real content
//! ([`crate::DOCK_DEFAULT_WIDTH_PTS`]). Treat the appearance of tab-bar
//! scroll arrows in the DEFAULT layout as a defect report, not as normal.
//!
//! ## Persistence: session-only, and disclosed (§7, A.6, R82)
//!
//! Nothing here is written to disk. eframe's `persistence` feature is NOT
//! enabled (it writes to a *platform* app-data directory, contradicting
//! decision 003's single-folder-portable posture and R15's requirement that
//! user state live in a named partition of the distribution folder), and
//! `egui_tiles`' own `serde` feature is NOT enabled either (see the
//! `default-features = false` note in `Cargo.toml`). The dock header states
//! the session-only scope in visible text rather than letting it pass
//! silently — the precedent decision 012 set for the font-folders setting.
//!
//! When R15 lands, serializing [`DockTree`] is the natural mechanism, and it
//! is governed by §7's **fail-soft** contract: a missing file, a parse
//! failure, an unknown [`DockPanel`] variant, or an absent mandatory panel
//! all fall back to [`default_tree`], noted in the status surface — never an
//! error dialog, never a lost document session. A *draggable* layout makes
//! that contract more load-bearing, not less: the operator can now build a
//! layout a later build cannot represent.
//!
//! ## Two gotchas decision 017 §6.2 paid for in advance — do not rediscover
//!
//! 1. `egui_tiles::Tree<Pane>` derives only `Clone, PartialEq` — **not
//!    `Default`** — so `std::mem::take` will not compile for the borrow
//!    dance in [`crate::PdfceApp::dock_body`]. Use
//!    `std::mem::replace(&mut self.dock, Tree::empty(SWAP_TREE_ID))`.
//! 2. `SimplificationOptions::default()` sets `prune_single_child_tabs:
//!    true` with `all_panes_must_have_tabs: false`, which makes **the tab
//!    bar vanish when only one panel is open** — a panel with no tab is a
//!    panel with no visible name and no drag handle. [`DockBehavior`]
//!    overrides `all_panes_must_have_tabs: true`, which the crate documents
//!    as winning over `prune_single_child_tabs`.
//!
//! ## Accessibility (A.4 #5, R84) — what is supplied and what is still a gap
//!
//! `egui_tiles` 0.16.0 ships its tab bars **unnamed to AccessKit**: a source
//! search of the pinned release finds zero occurrences of `widget_info` or
//! `accesskit`, and its default `tab_ui` allocates a bare
//! `ui.interact(rect, id, Sense::click_and_drag())`. Because that sense sets
//! egui's `FOCUSABLE` bit, the tabs *are* Tab-reachable and
//! keyboard-activatable — and announce nothing, which is the worst case. The
//! fix landed only on the crate's `main` branch, i.e. after 0.16.0, so this
//! build falls on the **unfixed** side and must supply the names itself.
//! [`DockBehavior::on_tab_button`] does that, reusing
//! [`crate::PdfceApp::labeled_icon_button`]'s `WidgetInfo` pattern: every tab
//! gets an accessible NAME plus a purpose tooltip saying WHEN to reach for
//! the panel, not a restatement of its label.
//!
//! **Still a tracked gap, stated honestly rather than implied away** (the
//! same convention `main.rs`'s accessibility doc-comment already uses for the
//! canvas): egui 0.35's [`egui::WidgetType`] has no `Tab`/`TabList` member,
//! so these controls are announced as selectable labels with the right name
//! and the right selected state, but **not** with a tab ROLE. Only the name
//! is supplied; the role cannot be, short of an upstream change.
//!
//! R84 (selected state is never colour alone) is honoured by
//! [`DockBehavior::tab_title_for_tile`], which renders the ACTIVE tab's title
//! **bold** — a weight cue paired with `egui_tiles`' own background fill.
//! The active set is snapshotted once per frame before the tree draws,
//! because a `Behavior` callback cannot otherwise learn whether the tile it
//! is titling is its parent's current tab.

use eframe::egui;
use egui_tiles::{SimplificationOptions, Tile, TileId, Tree, UiResponse};

use crate::{Action, PdfceApp, ui_text};

/// The `egui::Id` of the live dock tree.
///
/// A stable string rather than a derived one: `egui_tiles` stores per-tile
/// drag/resize state under ids derived from this, so changing it between
/// frames would silently reset every in-flight interaction.
const DOCK_TREE_ID: &str = "pdfce-dock";

/// The id given to the throwaway tree that stands in while the real one is
/// moved out for the borrow dance (module docs, gotcha 1).
///
/// It never draws — it exists for exactly the span of one
/// `std::mem::replace`, because `Tree` is not `Default` and `mem::take`
/// therefore does not compile.
const SWAP_TREE_ID: &str = "pdfce-dock-swap";

/// The dock's pane tree.
///
/// A type alias rather than a newtype so `egui_tiles`' whole `Tree` surface
/// (drag, split, `make_active`, `active_tiles`) stays available without a
/// forwarding layer that would have to grow a method per upstream feature.
pub type DockTree = Tree<DockPanel>;

/// One dockable surface (decision 017 §8.1 / A.4 #1, standing rule R80).
///
/// **Every dockable surface is a variant here, reached through ONE
/// dispatcher** ([`PdfceApp::panel_body`]). R80's second half — *no panel is
/// reachable ONLY as a floating window* — is why Pass 18.4 retired
/// `properties_window`'s floating form rather than shipping a float-OR-dock
/// dual mode: two code paths for the same content, each duplicating
/// open-state, position/size and focus handling, for zero operator benefit
/// at this scale.
///
/// ## Why this must stay extensible (decision 017 §10 Q1, A.4 #1)
///
/// A payload-carrying `Document(DocId)` variant — the wide model, where the
/// engine owns the canvas region too and each open document is a pane — must
/// remain a **non-breaking addition**. It is, structurally: the pane payload
/// travels inside [`DockTree`], so a variant with a field changes no type
/// signature anywhere. [`DockBehavior`] takes `&DockPanel`/`&mut DockPanel`
/// and never assumes `Copy` at an API boundary, and [`PdfceApp::panel_body`]
/// dispatches by `match` — which the compiler will point at, exhaustively,
/// the moment a variant lands. Under the operator's chosen wide model that
/// variant is *expected*, not hypothetical, so do not add a `Copy` bound, a
/// `[DockPanel; N]` array, or an index-based encoding that would make the
/// field expensive to introduce.
///
/// `PartialEq` is required by `egui_tiles::Tiles::find_pane`; `Hash`/`Eq`
/// let a set of panels be addressed cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockPanel {
    /// The page's object/layer tree (`docs/ui_specs/pass-17-dock-and-layer-
    /// tree.md` §B — the one section of that spec the dock reversal left
    /// standing). Answers "what am I clicking on", which is the operator's
    /// own stated purpose for it.
    Objects,
    /// The document's `/Info` metadata form (§14.3.3) — the body that used
    /// to live in the floating `properties_window`.
    Properties,
    /// The historic Tools dock body: Combine / Split / Insert pages / Font
    /// folders.
    ///
    /// Named "Batch Tools" in the UI, deliberately, per decision 017 §8.5
    /// (still binding under A.4 #4): a row labelled "Tools" inside a
    /// container an operator calls "the Tools dock" is a real collision they
    /// trip on.
    BatchTools,
    /// The redaction review surface (Pass 8.1, `docs/ui_specs/pass-8-
    /// redaction.md` §3): the live list of `/Redact` marks on the OPEN
    /// document, the authoring entry points, and the way to the Apply
    /// report.
    ///
    /// **A dock panel, not the ui-spec's own `SidePanel::right`.** §3.1 was
    /// written before decision 017 existed and reasoned its way to a
    /// dedicated side panel because the Tools dock's intro sentence ("These
    /// tools work with files outside the one you have open") would have been
    /// falsified by a surface that acts on the open document. That reasoning
    /// still holds — and the dock it was reasoning about no longer exists in
    /// that form. Today the dock is a general panel host with per-panel tabs
    /// and per-panel tooltips, `tools_dock_intro` belongs to
    /// [`Self::BatchTools`] alone, and R80 states the opposite requirement:
    /// **no panel is reachable only outside the dock.** So the spec's
    /// conclusion (Redact is not a Batch-Tools row) is honoured exactly,
    /// while its mechanism (a second right-hand `SidePanel` competing with
    /// the dock for width) is not — it would be the float-OR-dock dual mode
    /// decision 017 A.4 #2 deliberately retired.
    Redact,
}

impl DockPanel {
    /// Every panel, in the order the default layout introduces them.
    ///
    /// The enumeration the module's tests sweep, so that a variant added
    /// without a mount point in [`default_tree`] — or without a label, or
    /// with a tooltip that only restates its label — fails a test instead of
    /// becoming a silently unreachable or silently unexplained surface.
    /// R80's concern is a panel reachable ONLY as a floating window; a panel
    /// reachable *nowhere* is the worse version of the same defect, and this
    /// is what catches it.
    ///
    /// Deliberately not `#[cfg(test)]`: it is the honest public statement of
    /// "these are the panels", and the next surface that needs to enumerate
    /// them (a "show panel ▾" menu, a fail-soft remount that re-adds a
    /// missing mandatory panel after a layout restore) should reuse it
    /// rather than write a second list that can drift.
    #[allow(
        dead_code,
        reason = "the panel enumeration; swept by this module's tests today, and the list any future panel-picker or fail-soft remount must read rather than re-derive" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub const ALL: [Self; 4] = [
        Self::Objects,
        Self::Properties,
        Self::BatchTools,
        Self::Redact,
    ];

    /// The panel's tab label (decision 002 R1: through the catalog).
    pub fn label(self) -> &'static str {
        match self {
            Self::Objects => ui_text::dock_panel_objects_label(),
            Self::Properties => ui_text::dock_panel_properties_label(),
            Self::BatchTools => ui_text::dock_panel_batch_tools_label(),
            Self::Redact => ui_text::dock_panel_redact_label(),
        }
    }

    /// The panel's purpose tooltip — says **when to reach for it**, never a
    /// restatement of the label (decision 017 §8.6, still binding under
    /// A.4 #5). Doubles as the tab's AccessKit name (module docs).
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Objects => ui_text::dock_panel_objects_tooltip(),
            Self::Properties => ui_text::dock_panel_properties_tooltip(),
            Self::BatchTools => ui_text::dock_panel_batch_tools_tooltip(),
            Self::Redact => ui_text::dock_panel_redact_tooltip(),
        }
    }
}

/// Build the DEFAULT dock layout (decision 017 A.3, A.8 §10 Q3).
///
/// ```text
/// vertical
/// ├── tabs [ Objects | Redact ]           ← the layer tree, on top
/// └── tabs [ Properties | Batch Tools ]   ← visible AT THE SAME TIME
/// ```
///
/// ## Where Redact sits, and why not in the lower group (Pass 8.1)
///
/// A.3's narrow-column mitigation is an invariant, not a preference: **no
/// default tab group holds more than two labels**, because `egui_tiles`
/// 0.16.0 answers an overflowing tab bar by *hiding* tabs behind scroll
/// arrows. Adding [`DockPanel::Redact`] to the lower group would have made
/// it three and put a security surface behind a scroll arrow at ordinary
/// dock widths.
///
/// It joins the upper group instead, **second**, so `Objects` remains the
/// front tab and the A.3 requirement (tree above, properties below,
/// simultaneously) is untouched. The cost is that opening Redact covers the
/// object tree — which is the right thing to trade: reviewing redaction
/// marks is a page-and-canvas activity, and the pane it displaces is the one
/// an operator is not reading while they do it. Properties, which they might
/// be, stays visible throughout.
///
/// **The vertical split is the requirement, not a stylistic default.** A.3:
/// select an object in the tree above, edit its properties below, without
/// losing sight of the tree. Collapsing this to a single tab group would
/// re-break exactly what §3 fixed.
///
/// Objects gets the whole upper half to itself because it is the pane that
/// grows without bound (a page can decompose to thousands of rows), while
/// Properties is a fixed-height form; pairing Properties with Batch Tools in
/// the lower group keeps both tab bars to two labels or fewer, which is A.3's
/// narrow-column mitigation stated as an invariant rather than a hope.
///
/// Selection-scoped vs document-scoped (§3's compartment split, §10 Q3) is
/// preserved in spirit: the document-wide surfaces sit together at the
/// bottom, the page-scoped tree at the top. Under `egui_tiles` this is a
/// starting point the operator may drag apart, which is precisely why A.8
/// lowered the stakes on Q3 and said to ship the proposal.
#[must_use]
pub fn default_tree() -> DockTree {
    let mut tiles = egui_tiles::Tiles::default();
    let objects = tiles.insert_pane(DockPanel::Objects);
    let properties = tiles.insert_pane(DockPanel::Properties);
    let batch = tiles.insert_pane(DockPanel::BatchTools);
    let redact = tiles.insert_pane(DockPanel::Redact);

    let upper = tiles.insert_tab_tile(vec![objects, redact]);
    let lower = tiles.insert_tab_tile(vec![properties, batch]);
    let root = tiles.insert_vertical_tile(vec![upper, lower]);

    Tree::new(DOCK_TREE_ID, root, tiles)
}

/// The throwaway tree used for the borrow dance (module docs, gotcha 1).
#[must_use]
pub fn swap_tree() -> DockTree {
    Tree::empty(SWAP_TREE_ID)
}

/// Whether `panel` is currently the ACTIVE tab of its container.
///
/// Drives the toolbar Properties toggle's selected state, so that control
/// reports the truth ("Properties is the pane you are looking at") rather
/// than a stale boolean of its own — the float-era `properties_open` flag
/// that this replaced could disagree with what was on screen.
///
/// `active_tiles` walks from the root through each container's current tab,
/// so a pane sitting BEHIND another tab correctly reports `false`.
#[must_use]
pub fn panel_is_active(tree: &DockTree, panel: DockPanel) -> bool {
    tree.active_tiles()
        .into_iter()
        .any(|id| matches!(tree.tiles.get(id), Some(Tile::Pane(p)) if *p == panel))
}

/// Make `panel` the active tab of every container on the path to it, so a
/// command like "open Properties" lands the operator ON the panel rather
/// than merely somewhere in the dock.
///
/// A no-op (returning `false`) if the panel is not mounted at all — which
/// can happen once the operator can close panes, and must NOT be an error:
/// the caller's fallback is to remount the default layout, not to refuse.
pub fn activate(tree: &mut DockTree, panel: DockPanel) -> bool {
    tree.make_active(|_id, tile| matches!(tile, Tile::Pane(p) if *p == panel))
}

/// The `egui_tiles` [`egui_tiles::Behavior`] for pdfce's dock.
///
/// Holds `&mut PdfceApp` for the span of ONE `Tree::ui` call. This is only
/// sound because the caller moved the tree OUT of the app first (module
/// docs, gotcha 1) — while this exists, `app.dock` is the empty swap tree,
/// so nothing reachable from a panel body may touch it. Panel bodies that
/// need to change the layout push an [`Action`] instead, which is applied
/// after the tree is put back.
pub struct DockBehavior<'a> {
    /// The application state every panel body draws from.
    pub app: &'a mut PdfceApp,
    /// The frame's action queue — the one channel a panel body mutates the
    /// document through (there is no path from a widget to a `Document`;
    /// ARCHITECTURE.md §11.4).
    pub actions: &'a mut Vec<Action>,
    /// The tiles that are the current tab of their container, snapshotted
    /// **before** the tree drew.
    ///
    /// Needed because `Behavior` has no other way to learn active state at
    /// title time, and R84 requires the active tab to carry a weight cue and
    /// not only `egui_tiles`' background colour.
    pub active: Vec<TileId>,
}

impl egui_tiles::Behavior<DockPanel> for DockBehavior<'_> {
    /// Draw one panel — the single dispatcher R80 mandates.
    ///
    /// Always [`UiResponse::None`]: pdfce deliberately exposes **no
    /// drag-the-body-to-undock handle**. Tabs are draggable (that is the
    /// capability `egui_tiles` was adopted for and it is visibly afforded by
    /// the tab bar); a body-drag would be a second, invisible way to do the
    /// same thing, and R83 forbids an affordance that is not real just as
    /// firmly as it forbids a capability with no affordance.
    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: TileId, pane: &mut DockPanel) -> UiResponse {
        self.app.panel_body(*pane, ui, self.actions);
        UiResponse::None
    }

    /// A pane's tab label.
    fn tab_title_for_pane(&mut self, pane: &DockPanel) -> egui::WidgetText {
        pane.label().into()
    }

    /// A tab's title, **bold when that tab is the active one** (R84).
    ///
    /// `egui_tiles` signals the active tab with a background fill and a text
    /// colour, both of which are colour-only cues; the GUI-polish audit
    /// already flagged colour-fill-only selected state as a recurring blind
    /// spot on this project, and decision 017 §8.6 asked specifically that a
    /// new selection surface not repeat it in its inaugural instance. Bold
    /// survives greyscale and colour-vision deficiency; the fill does not.
    ///
    /// Container tabs (a tab group nested inside another) fall through to
    /// the upstream default, which names them after their container kind —
    /// a state pdfce's default layout never produces but the operator can
    /// build by dragging.
    fn tab_title_for_tile(
        &mut self,
        tiles: &egui_tiles::Tiles<DockPanel>,
        tile_id: TileId,
    ) -> egui::WidgetText {
        let Some(Tile::Pane(pane)) = tiles.get(tile_id) else {
            return match tiles.get(tile_id) {
                Some(Tile::Container(c)) => ui_text::dock_container_tab_label(c.kind()).into(),
                _ => ui_text::dock_missing_tile_label().into(),
            };
        };
        let text = egui::RichText::new(pane.label());
        if self.active.contains(&tile_id) {
            text.strong().into()
        } else {
            text.into()
        }
    }

    /// Attach the accessible name, the selected state and the purpose
    /// tooltip to each tab (module docs, A.4 #5).
    ///
    /// This hook rather than a wholesale `tab_ui` override on purpose: it is
    /// the ONE thing 0.16.0's tab bar is missing, and re-implementing the
    /// whole ~80-line default `tab_ui` to add it would silently fork
    /// upstream's painting, close-button and drag handling at the next
    /// upgrade. The `WidgetInfo` pattern is
    /// [`PdfceApp::labeled_icon_button`]'s, reused rather than re-derived.
    ///
    /// The role is `SelectableLabel`, not a tab role, because egui 0.35 has
    /// none — see the module docs' honest statement of that gap.
    fn on_tab_button(
        &mut self,
        tiles: &mut egui_tiles::Tiles<DockPanel>,
        tile_id: TileId,
        button_response: egui::Response,
    ) -> egui::Response {
        let Some(Tile::Pane(pane)) = tiles.get(tile_id) else {
            return button_response;
        };
        let pane = *pane;
        let selected = self.active.contains(&tile_id);
        let response = button_response.on_hover_text(pane.tooltip());
        let name = pane.tooltip().to_owned();
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                selected,
                name.clone(),
            )
        });
        response
    }

    /// Keep a tab bar even when a container holds ONE pane (module docs,
    /// gotcha 2).
    ///
    /// The upstream default would prune it, and a pane with no tab bar has
    /// no visible name, no drag handle and no way back — which in pdfce's
    /// default layout would strip the Objects pane's tab the moment the
    /// operator moved Properties or Batch Tools next to it.
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default layout must mount EVERY panel — a variant with no mount
    /// point would be unreachable, which is worse than R80's
    /// floating-window-only case it exists to prevent.
    #[test]
    fn the_default_layout_mounts_every_panel() {
        let tree = default_tree();
        for panel in DockPanel::ALL {
            assert!(
                tree.tiles.find_pane(&panel).is_some(),
                "{panel:?} is not mounted in the default layout"
            );
        }
    }

    /// Decision 017 A.3's surviving requirement, asserted rather than
    /// trusted: Objects and Properties must be visible **at the same time**
    /// in the DEFAULT layout, not after the operator drags a pane out.
    ///
    /// `active_tiles` is exactly "what is on screen right now", so a
    /// regression that folded the vertical split into one tab group would
    /// leave only one of the two active and fail here.
    #[test]
    fn objects_and_properties_are_simultaneously_visible_by_default() {
        let tree = default_tree();
        assert!(
            panel_is_active(&tree, DockPanel::Objects),
            "the object/layer tree is not visible in the default layout"
        );
        assert!(
            panel_is_active(&tree, DockPanel::Properties),
            "document properties is not visible in the default layout — \
             decision 017 A.3 requires it beside the tree, not behind a tab"
        );
    }

    /// Batch Tools shares the lower tab group with Properties, so it starts
    /// BEHIND it. That is deliberate (it is the least-used surface), and it
    /// must be reachable by activation rather than only by dragging.
    #[test]
    fn a_backgrounded_panel_can_be_brought_forward() {
        let mut tree = default_tree();
        assert!(!panel_is_active(&tree, DockPanel::BatchTools));
        assert!(activate(&mut tree, DockPanel::BatchTools));
        assert!(panel_is_active(&tree, DockPanel::BatchTools));
        // Bringing Batch Tools forward must not cost the operator the tree
        // above it — that is the whole point of the vertical split.
        assert!(panel_is_active(&tree, DockPanel::Objects));
        assert!(!panel_is_active(&tree, DockPanel::Properties));
    }

    /// The redaction panel starts behind the object tree and must come
    /// forward on request WITHOUT costing the operator the properties form
    /// below it — the vertical split's whole purpose, asserted for the
    /// panel that was added to the upper group rather than the lower one
    /// (Pass 8.1, see [`default_tree`]'s placement note).
    #[test]
    fn the_redaction_panel_comes_forward_without_collapsing_the_split() {
        let mut tree = default_tree();
        assert!(!panel_is_active(&tree, DockPanel::Redact));
        assert!(activate(&mut tree, DockPanel::Redact));
        assert!(panel_is_active(&tree, DockPanel::Redact));
        assert!(
            !panel_is_active(&tree, DockPanel::Objects),
            "Redact shares the upper group with Objects, so it displaces it"
        );
        assert!(
            panel_is_active(&tree, DockPanel::Properties),
            "opening Redact must not cost the operator the properties form"
        );
    }

    /// A.3's narrow-column mitigation, asserted rather than remembered: no
    /// DEFAULT tab group may hold more than two panes.
    ///
    /// `egui_tiles` 0.16.0 answers an overflowing tab bar by hiding tabs
    /// behind scroll arrows, so a third label in a group is a panel an
    /// operator can lose at ordinary dock widths. This is the test that
    /// stops the next panel from being dropped into whichever group looked
    /// convenient.
    #[test]
    fn no_default_tab_group_holds_more_than_two_panes() {
        let tree = default_tree();
        for (id, tile) in tree.tiles.iter() {
            if let Tile::Container(egui_tiles::Container::Tabs(tabs)) = tile {
                assert!(
                    tabs.children.len() <= 2,
                    "tab group {id:?} holds {} panes; A.3 caps a default group at 2",
                    tabs.children.len()
                );
            }
        }
    }

    /// Activating a panel that is not mounted is a `false` answer, never a
    /// panic — the caller's fallback is to remount the default layout.
    #[test]
    fn activating_an_unmounted_panel_reports_failure_instead_of_panicking() {
        let mut tree = swap_tree();
        assert!(!activate(&mut tree, DockPanel::Properties));
        assert!(!panel_is_active(&tree, DockPanel::Properties));
    }

    /// Every panel's tooltip must say something the label does not — the
    /// decision 017 §8.6 rule that a tooltip states WHEN to use a surface,
    /// not what it is called.
    #[test]
    fn every_panel_tooltip_adds_information_beyond_its_label() {
        for panel in DockPanel::ALL {
            let (label, tooltip) = (panel.label(), panel.tooltip());
            assert!(!tooltip.is_empty(), "{panel:?} has no tooltip");
            assert_ne!(label, tooltip, "{panel:?}'s tooltip restates its label");
            assert!(
                tooltip.len() > label.len() + 20,
                "{panel:?}'s tooltip is too thin to be saying WHEN to use it: {tooltip:?}"
            );
        }
    }

    /// Labels must be distinct, or two tabs in one bar are indistinguishable.
    #[test]
    fn panel_labels_are_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for panel in DockPanel::ALL {
            assert!(seen.insert(panel.label()), "duplicate label on {panel:?}");
        }
    }
}
