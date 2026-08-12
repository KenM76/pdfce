//! The document-structure panels: Signatures, Layers and Bookmarks.
//!
//! # What these three have in common
//!
//! Each answers a question about the document's STRUCTURE rather than
//! its page content — what a signature covers, which layers a reader
//! would draw, where the outline points. All three are read-mostly:
//! Bookmarks navigates, Layers changes what is drawn for this session
//! only, and Signatures cannot change anything at all. None of them
//! edits a document.
//!
//! They also share the posture that took the longest to get right in
//! this project: **each says what it cannot tell you, first.** The
//! Signatures panel opens with the sentence that pdfce performs no
//! cryptographic verification, because a panel headed "Signatures"
//! listing byte counts is the single likeliest place in this
//! application for an operator to take away more than was said. The
//! Layers panel opens by saying a toggle changes what you see and not
//! the document.
//!
//! # Why they are methods and not free functions
//!
//! They take `&mut self` and reach a dozen fields of `PdfceApp` —
//! selection state, session overrides, draft maps, the diagnostics
//! cache. Converting them to free functions would mean threading those
//! through parameter lists, which is a design change wearing a
//! refactor's clothes.
//!
//! Rust allows an `impl` block for a type in any module of the same
//! crate, and a child module can reach its ancestors' private items —
//! so these move as methods, touching the same private fields, with the
//! signatures unchanged. That is what makes this stage a pure move: the
//! compiler required no visibility markers at all.
//!
//! # ★ These three panels once had no way in
//!
//! Worth recording here, because this is now the file someone reads
//! when they touch them. All three shipped with a `PaneSubject`, a
//! panel body, a rail entry and a diagnostic step — and no control an
//! operator could click. Their only callers were the harness step
//! handlers, so every verification passed while the panels were
//! unreachable in a real build (R184).
//!
//! `every_pane_subject_is_reachable_without_the_harness` in `main.rs`
//! is the gate that now prevents it. If a fourth panel is added here,
//! it needs a ribbon control, not just a `PaneSubject`.

use crate::{Action, PdfceApp, Status, diag, ui_text};
use eframe::egui;

impl PdfceApp {
    /// The Signatures panel — what each signature covers.
    ///
    /// # It is not a validity check, and the panel says so first
    ///
    /// pdfce performs no cryptographic verification. A panel headed
    /// "Signatures", listing byte counts, is the single most likely place
    /// in this application for an operator to take away more than was
    /// said — so the caveat is the first line, above the list, not a
    /// tooltip and not a footnote.
    ///
    /// # The file length is read from DISK each time, deliberately
    ///
    /// `/ByteRange` is a claim about bytes, so it can only be checked
    /// against bytes. The length used is the file **on disk right now**,
    /// not a length captured when the document was opened, and that
    /// choice changes what the number means: it answers *"does the
    /// signature cover the file as it currently exists"*, which is the
    /// question worth asking. A captured length would answer "did it
    /// cover the file when you opened it" and would go stale the moment
    /// anything appended to it — including pdfce's own incremental save.
    ///
    /// Unsaved edits are not counted, and cannot be: they are not in the
    /// file yet. The panel says which state it is describing rather than
    /// leaving an operator to assume.
    pub(crate) fn signatures_panel(&mut self, ui: &mut egui::Ui, _actions: &mut [Action]) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        // A stat, not a read. Cheap enough per frame, and the alternative
        // is a cached number that silently describes a file that no
        // longer exists in that form.
        let Ok(meta) = std::fs::metadata(&doc.path) else {
            ui.label(ui_text::signatures_file_unreadable());
            return;
        };
        let graph = doc.session.graph();
        let coverage = pdfce_core::signature::byte_range_coverage(&graph, meta.len());

        if coverage.is_empty() {
            ui.label(ui_text::signatures_none());
            return;
        }

        // The caveat FIRST. Everything below it is a measurement, and a
        // measurement read as a verdict is the failure this prevents.
        ui.label(
            egui::RichText::new(ui_text::signatures_not_a_validity_check())
                .small()
                .weak(),
        );
        ui.separator();

        for c in &coverage {
            let name = c
                .field_name
                .clone()
                .unwrap_or_else(|| ui_text::signature_unnamed().to_owned());
            ui.label(egui::RichText::new(name).strong());

            // Malformed is reported BEFORE coverage, because it changes
            // what the coverage numbers mean: a reader that rejects the
            // array computes something else, or nothing.
            if !c.ranges_well_formed {
                ui.label(ui_text::signature_range_malformed());
            }
            if c.pair_count == 1 {
                ui.label(ui_text::signature_single_range());
            }
            ui.label(if c.covers_to_eof() {
                ui_text::signature_covers_whole_file(c.covered)
            } else {
                ui_text::signature_leaves_tail(c.covered, c.uncovered_tail)
            });
            diag::trace(|| {
                format!(
                    "signature-row field={:?} covered={} tail={} pairs={} well_formed={}",
                    c.field_name, c.covered, c.uncovered_tail, c.pair_count, c.ranges_well_formed
                )
            });
            ui.separator();
        }
    }

    /// The Layers panel — the document's optional-content groups.
    ///
    /// # Session state, with no file-format footprint
    ///
    /// Toggling a layer in a viewer is **session state that does not
    /// touch the document** unless the operator explicitly saves
    /// (`Acrobat_Features/layers__ocg_visibility_and_defaults.md`).
    /// pdfce does the first half and not the second: the checkbox
    /// changes what is drawn, `/OCProperties /D` is untouched, nothing
    /// marks the document dirty, and nothing survives reopening. The
    /// panel says so in its first line rather than leaving it inferred.
    ///
    /// # ★ This doc said the opposite for three commits
    ///
    /// It read: *"the renderer takes no visibility override, and there
    /// is no save path for one. So the panel lists and does not offer a
    /// checkbox."* That was true when written and false from `6ab72ec`,
    /// which added [`pdfce_render::LayerVisibility`], the override the
    /// renderer consumes, and the checkbox this comment denies exists.
    ///
    /// It was found by a consultant reading the file, not by any check —
    /// nothing compiles a doc comment against the behaviour it
    /// describes. Same shape as the `git remote` error rule 8 records: a
    /// document asserting a capability fact nobody re-measured. Noted
    /// here rather than silently overwritten, because the correction is
    /// cheap and the class of error is not.
    ///
    /// R83 still governs what the panel offers, and it is why the
    /// checkbox could not have shipped before `71592d3`: until the
    /// renderer honoured content-stream `/OC`, a tick against a CAD
    /// drawing's layers would have changed nothing on the page.
    ///
    /// # What it shows that a name cannot
    ///
    /// Whether a reader opening this document with no interaction would
    /// DRAW each layer. A "Confidential" watermark that is off by default
    /// is a different document from one where it is on, and the two are
    /// indistinguishable by name.
    ///
    /// That value comes from `annot.rs`'s `optional_content_default_off`
    /// — the same resolver the renderer consults for annotations — so the
    /// panel cannot say "on" about content the page hides.
    /// The complete hidden set for this render, or `None` to obey the
    /// document.
    ///
    /// # Why this recomputes the WHOLE set rather than sending the deltas
    ///
    /// [`pdfce_render::LayerVisibility`] REPLACES the document's default
    /// configuration; it is not merged with it (see that type's module
    /// docs — every merge rule would be a rendering decision invisible at
    /// the call site). So the answer has to start from what the document
    /// asks and apply the operator's changes on top, which is exactly
    /// what this does.
    ///
    /// Sending only the toggled groups would show every layer the
    /// document had turned off, which is the failure that contract
    /// exists to make impossible to reach by accident.
    ///
    /// Returns `None` when nothing has been toggled — not an empty set.
    /// An empty set means "hide nothing", which would reveal a
    /// document's own hidden layers the moment the panel was opened.
    pub(crate) fn layer_visibility(&self) -> Option<pdfce_render::LayerVisibility> {
        if self.layer_overrides.is_empty() {
            return None;
        }
        let Status::Open(doc) = &self.status else {
            return None;
        };
        let mut hidden = pdfce_core::annot::optional_content_default_off(&doc.session.graph());
        for (id, visible) in &self.layer_overrides {
            if *visible {
                hidden.remove(id);
            } else {
                hidden.insert(*id);
            }
        }
        Some(pdfce_render::LayerVisibility::hiding(hidden))
    }

    /// Set one layer's visibility for this session, honouring
    /// `/RBGroups` (Table 101).
    ///
    /// # Radio behaviour, because a radio group is not a hint
    ///
    /// Table 101's `/RBGroups` are "radio button" groups: at most one
    /// member visible at a time. Turning one on therefore turns its
    /// siblings off. Leaving that to the operator would let pdfce show a
    /// combination the document declares impossible — two mutually
    /// exclusive alternates painted over each other, which on a CAD
    /// drawing means two different title blocks in the same place.
    ///
    /// Turning one OFF does not turn a sibling on: "at most one" permits
    /// none, and picking a replacement would be pdfce choosing which
    /// alternate the operator meant.
    pub(crate) fn set_layer_visible(
        &mut self,
        id: pdfce_core::object::ObjId,
        visible: bool,
        siblings: &[pdfce_core::object::ObjId],
    ) {
        self.layer_overrides.insert(id, visible);
        if visible {
            for sib in siblings {
                if *sib != id {
                    self.layer_overrides.insert(*sib, false);
                }
            }
        }
        self.layers_generation = self.layers_generation.wrapping_add(1);
    }

    pub(crate) fn layers_panel(&mut self, ui: &mut egui::Ui, _actions: &mut [Action]) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        let read = pdfce_core::layers::read_layers(&doc.session.graph());

        if read.diagnostics.no_optional_content {
            ui.label(ui_text::layers_none());
            return;
        }
        ui.label(ui_text::layers_count(read.layers.len()));
        ui.label(
            egui::RichText::new(ui_text::layers_session_only_note())
                .small()
                .weak(),
        );
        // Only offered once there is something to undo, so the control
        // never sits there implying a change that has not happened.
        let mut reset = false;
        if !self.layer_overrides.is_empty() {
            ui.horizontal(|ui| {
                ui.label(ui_text::layers_overridden(self.layer_overrides.len()));
                reset = ui
                    .button(ui_text::layers_reset_label())
                    .on_hover_text(ui_text::layers_reset_tooltip())
                    .clicked();
            });
        }
        // §8.11.4.4: some of these states are not the document's to
        // state — a viewer recomputes them from the magnification. The
        // rows below show what the document OPENS in, so a zoom-banded
        // layer can read "shown" while its content is off the page.
        // Said here rather than left to be discovered as a defect.
        if read.diagnostics.auto_managed_groups > 0 {
            ui.label(
                egui::RichText::new(ui_text::layers_auto_managed(
                    read.diagnostics.auto_managed_groups,
                ))
                .small(),
            );
        }
        ui.separator();

        // Collected while the read is borrowed, applied after — the
        // toggle needs `&mut self` and the loop holds `&self.status`.
        let mut toggled: Option<(
            pdfce_core::object::ObjId,
            bool,
            Vec<pdfce_core::object::ObjId>,
        )> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for l in &read.layers {
                // An undeclared name shows as a placeholder, never as an
                // invented one. `/Name` is Required (Table 98), so its
                // absence is a real malformation and a synthesised
                // "Layer 3" would disguise it as data from the file.
                let name = if l.name_declared {
                    l.name.clone()
                } else {
                    ui_text::layer_unnamed().to_owned()
                };
                // The EFFECTIVE state: the operator's choice if there is
                // one, otherwise the document's. Never the document's
                // alone — a checkbox that ignored the override would tick
                // itself back the moment the panel repainted.
                let effective = self
                    .layer_overrides
                    .get(&l.id)
                    .copied()
                    .unwrap_or(l.visible_by_default);
                ui.horizontal(|ui| {
                    // Table 101 `/Locked`: "the UI shall not allow the
                    // visibility state to be changed". Disabled and
                    // explained, never hidden and never silently
                    // ignored — R83.
                    let mut want = effective;
                    let cb = ui
                        .add_enabled(!l.locked, egui::Checkbox::new(&mut want, ""))
                        .on_hover_text(if l.locked {
                            ui_text::layer_locked_tooltip()
                        } else {
                            ui_text::layer_toggle_tooltip()
                        });
                    if cb.changed() {
                        let siblings = l
                            .radio_group
                            .and_then(|g| read.radio_groups.get(g))
                            .cloned()
                            .unwrap_or_default();
                        toggled = Some((l.id, want, siblings));
                    }
                    // The state as TEXT as well as a checkbox — rule 6's
                    // no-colour-only-cue, and it still says which way the
                    // document itself asks when the two disagree.
                    ui.label(if effective {
                        ui_text::layer_visible_marker()
                    } else {
                        ui_text::layer_hidden_marker()
                    });
                    let mut label = ui.label(name);
                    if effective != l.visible_by_default {
                        label = label.on_hover_text(ui_text::layer_overridden_tooltip(
                            l.visible_by_default,
                        ));
                    }
                    // §8.11.2.3: a group whose `/Intent` excludes `View`
                    // does not participate in visibility, so its state in
                    // `/OFF` has no effect on what a reader draws.
                    //
                    // Said out loud because the alternative is an
                    // operator seeing a layer marked visible that the
                    // file's own `/OFF` array names, with no way to tell
                    // whether that is intent filtering or a pdfce bug.
                    // pdfce inferred something (this group does not
                    // count) and the inference changed the page — rule
                    // 4 says the inference is visible, not merely
                    // correct.
                    if !l.intent_view {
                        label = label.on_hover_text(ui_text::layer_design_intent_tooltip());
                    }
                    if l.locked {
                        label = label.on_hover_text(ui_text::layer_locked_tooltip());
                    }
                    if !l.in_default_config {
                        label = label.on_hover_text(ui_text::layer_unregistered_tooltip());
                    }
                    if l.radio_group.is_some() {
                        label.on_hover_text(ui_text::layer_radio_tooltip());
                    }
                });
                diag::trace(|| {
                    format!(
                        "layer-row name={:?} visible={effective} default={} locked={} registered={} intent_view={}",
                        l.name, l.visible_by_default, l.locked, l.in_default_config, l.intent_view
                    )
                });
            }
        });

        if let Some((id, visible, siblings)) = toggled {
            self.set_layer_visible(id, visible, &siblings);
        }
        if reset {
            // Back to the document's own configuration, in one gesture
            // and one undo-equivalent (R49) — clearing the map is exactly
            // "no override", which is not the same as "show everything".
            self.layer_overrides.clear();
            self.layers_generation = self.layers_generation.wrapping_add(1);
        }
    }

    /// The Bookmarks panel — the document's outline, as navigation.
    ///
    /// # Why the tree is read fresh each frame rather than cached
    ///
    /// `read_outline` takes an `&ObjectGraph`, not `&mut self`, so unlike
    /// Find it CAN run inside the render closure — and the outline is a
    /// property of the document that page edits can change (deleting a
    /// page can leave a bookmark pointing nowhere). A cache would need
    /// invalidating on every edit and undo, which is a correctness
    /// problem traded for a parse of a structure that is a few hundred
    /// items at most. Measure before trading back.
    ///
    /// # A bookmark with no destination is NOT an error
    ///
    /// Three distinct states, and collapsing them would mislead:
    /// a bookmark that points at a page (clickable), a HEADING with no
    /// destination at all (legal, common, groups its children), and one
    /// whose destination pdfce could not resolve (the document meant
    /// something and pdfce could not follow it). Only the third is a
    /// problem, and only the first is worth a click — so the second and
    /// third are shown disabled with tooltips that say which they are.
    pub(crate) fn bookmarks_panel(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        let outline = pdfce_core::outline::read_outline(&doc.session.graph());

        let total = outline.diagnostics.items;
        // The current page, so a driven click has an observable to check
        // against — the only oracle available when the operator is using
        // the machine and a screenshot harness would seize their screen.
        diag::trace(|| {
            format!(
                "bookmarks-panel page={} items={total}",
                doc.view.page_index + 1
            )
        });
        ui.label(ui_text::bookmarks_count(total));
        // The truncation disclosure sits ABOVE the list, not below it: an
        // operator who scrolls a short list and stops has already drawn a
        // conclusion by the time a footnote would reach them.
        if outline.diagnostics.cycles_broken > 0
            || outline.diagnostics.depth_truncations > 0
            || outline.diagnostics.item_budget_exhausted
        {
            ui.label(
                egui::RichText::new(ui_text::bookmarks_truncated())
                    .small()
                    .weak(),
            );
        }
        if outline.items.is_empty() {
            ui.label(ui_text::bookmarks_empty());
            return;
        }
        ui.separator();

        // Collected first, applied after — the same defer-then-apply the
        // rest of this shell uses, because the click handler needs
        // `&mut self` and the walk holds `&self.status`.
        let mut go: Option<usize> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            Self::bookmark_rows(ui, &outline.items, &mut go);
        });
        if let Some(page) = go {
            actions.push(Action::GoToPage(page));
        }
    }

    /// Draw one level of the outline, recursing into children.
    ///
    /// Indentation carries the structure. `ui.indent` is keyed by the
    /// item's object id rather than its index: two siblings at the same
    /// index in different subtrees would otherwise collide in egui's id
    /// space, which shows up as the wrong row responding to a hover.
    pub(crate) fn bookmark_rows(
        ui: &mut egui::Ui,
        items: &[pdfce_core::outline::OutlineItem],
        go: &mut Option<usize>,
    ) {
        use pdfce_core::outline::Destination;
        for it in items {
            // The page a click would reach, if any. Only a resolved page
            // destination is navigable — a named destination pdfce could
            // not look up, or a remote file, is shown and not offered
            // (R83: never an affordance for something that cannot work).
            let target = match &it.destination {
                Some(Destination::Page { page_index, .. }) => Some(*page_index),
                _ => None,
            };
            let (enabled, tip) = match (&it.destination, target) {
                (_, Some(p)) => (true, ui_text::bookmark_row_tooltip(p + 1)),
                (None, _) => (false, ui_text::bookmark_row_heading_tooltip().to_owned()),
                (Some(_), None) => (false, ui_text::bookmark_row_unresolved_tooltip().to_owned()),
            };

            let label = if it.title.trim().is_empty() {
                // An untitled bookmark is legal and unclickable-looking.
                // Its own row still has to exist, or its children lose
                // their parent and appear at the wrong depth.
                ui_text::bookmark_untitled().to_owned()
            } else {
                it.title.clone()
            };

            let resp = ui
                .add_enabled(enabled, egui::Button::new(label).frame(false))
                .on_hover_text(tip.clone());
            let resp = if enabled {
                resp
            } else {
                resp.on_disabled_hover_text(tip)
            };
            diag::trace(|| {
                format!(
                    "bookmark-row level={} title={:?} page={:?} enabled={enabled} rect={:?}",
                    it.level,
                    it.title,
                    target.map(|p| p + 1),
                    resp.rect
                )
            });
            if resp.clicked()
                && let Some(p) = target
            {
                *go = Some(p);
            }

            if !it.children.is_empty() {
                ui.indent(("bookmark", it.id.num, it.id.generation), |ui| {
                    Self::bookmark_rows(ui, &it.children, go);
                });
            }
        }
    }

    /// The Fonts panel — what fonts the document declares, what their
    /// embedded programs cost, and which of those could be removed.
    ///
    /// # Read-only, and there is deliberately nothing to click
    ///
    /// Phase A ships the report and not the removal. No control here
    /// changes a byte, and none is stubbed: a greyed-out "Remove" button, or
    /// a "coming soon" note, would be an affordance for something that
    /// cannot work (R83), and a `Safe` verdict rendered as an accent colour
    /// or a checkmark would read as an invitation to press it.
    ///
    /// Every verdict is therefore drawn at the **same visual weight**, as a
    /// plain label. That is not restraint for its own sake — a blocked
    /// verdict is a fact about the *file*, and error styling would make it
    /// read as a pdfce failure.
    ///
    /// # ★ Why the panel says *why*, when the parity reference does not
    ///
    /// Acrobat refuses to unembed a font whose character codes are glyph
    /// indices into its own embedded program, and it refuses **silently** —
    /// the font simply is not in its unembed list, with no reason shown
    /// anywhere (`Acrobat_Features/optimize__font_unembedding.md`, sourced
    /// to a former Adobe Principal Scientist; independently corroborated by
    /// a user whose largest, most size-costly font was absent from the list
    /// with no explanation offered).
    ///
    /// A shorter list is not actionable. "This font's character codes are
    /// positions inside this specific embedded program" is. That is project
    /// rule 4 applied to a refusal rather than to a suggestion, and it is
    /// this panel's main reason to exist.
    ///
    /// The measured stakes, from a 64-file survey of the PDFBox corpus: of
    /// the 30 files that embed fonts, 87 % embed subsets, 40 % use
    /// `Identity-H`, and only 50 % carry `/ToUnicode`. So the common case
    /// for "just remove the embedded fonts" is a case where removal
    /// destroys the document, and the operator has no way to know that from
    /// a font list alone.
    ///
    /// # ★ The coverage note is above the list, not beneath it
    ///
    /// A font inventory that quietly misses a surface and prints a
    /// confident list is this project's most-repeated defect shape (R186 —
    /// a check that confirms the marker rather than the thing). So the
    /// panel states which font-bearing surfaces were searched **and the one
    /// that was not**, unconditionally, before the list. Acrobat's own
    /// coverage here is recorded as an unconfirmed GAP, so pdfce states its
    /// own scope rather than assuming parity with a behaviour nobody has
    /// measured.
    ///
    /// # Why the inventory is cached on the document
    ///
    /// The sweep decodes every embedded font program — it has to, to read
    /// the `OS/2` table — and on a document carrying a megabyte of CJK
    /// outlines that is not a per-frame cost. It is computed once and
    /// dropped by `OpenDoc::refresh_pages`, which already runs after every
    /// edit, undo and redo, so an `add-text` that embeds a new subset shows
    /// up without a second invalidation path to keep in step.
    pub(crate) fn fonts_panel(&mut self, ui: &mut egui::Ui, _actions: &mut [Action]) {
        use pdfce_core::fontinfo::{Program, Removability, RemovabilityUnknown, Surface};

        let Status::Open(doc) = &mut self.status else {
            return;
        };
        if doc.fonts.is_none() {
            let inv = pdfce_core::fontinfo::inventory(&doc.session.view());
            doc.fonts = Some(inv);
        }
        let Some(inv) = doc.fonts.as_ref() else {
            return;
        };

        // The page scan failing FIRST, above everything, because it changes
        // what an empty list beneath it means. Without this an operator
        // reads "0 fonts" as an answer about the document.
        if inv.diagnostics.page_scan_failed {
            ui.label(ui_text::fonts_page_scan_failed());
            ui.separator();
        }
        if inv.diagnostics.resource_scan_truncated {
            ui.label(ui_text::fonts_scan_truncated());
            ui.separator();
        }

        if inv.fonts.is_empty() {
            ui.label(ui_text::fonts_none());
            ui.label(
                egui::RichText::new(ui_text::fonts_coverage_note())
                    .small()
                    .weak(),
            );
            return;
        }

        ui.label(ui_text::fonts_count(inv.fonts.len()));
        // The document total, from the same per-font numbers the rows below
        // show, so the two cannot disagree. Acrobat's nearest equivalent
        // (Audit Space Usage's aggregate Fonts bucket) is a paid-tier
        // feature and gives no per-font breakdown at all.
        let total = usize::try_from(inv.embedded_bytes()).unwrap_or(usize::MAX);
        ui.label(ui_text::fonts_total_size(&ui_text::byte_size(total)));
        ui.label(
            egui::RichText::new(ui_text::fonts_coverage_note())
                .small()
                .weak(),
        );
        ui.separator();

        // Largest first. The operator opening this panel is usually asking
        // "which font is costing me the most", and that ordering answers it
        // with no control to find. Ties keep discovery order, which
        // `sort_by_key` preserves.
        let mut rows: Vec<&pdfce_core::fontinfo::FontRecord> = inv.fonts.iter().collect();
        rows.sort_by_key(|f| std::cmp::Reverse(f.stored_bytes()));

        for (row_index, f) in rows.iter().enumerate() {
            // Keyed by object identity, not by row index: two independent
            // subsets of one face de-prefix to the SAME display name, and an
            // index-keyed header would swap its expanded state under the
            // operator when the sort order moved.
            let key =
                f.id.map_or_else(|| format!("direct-{row_index}"), |id| format!("{}", id.num));
            let verdict = match &f.removability {
                Removability::Removable => ui_text::font_verdict_removable(),
                Removability::BlockedIdentityEncoded { .. } => {
                    ui_text::font_verdict_blocked_identity()
                }
                Removability::BlockedType3 => ui_text::font_verdict_blocked_type3(),
                Removability::NotEmbedded => ui_text::font_verdict_not_embedded(),
                _ => ui_text::font_verdict_unknown(),
            };
            let display = f.family_name().unwrap_or_else(|| ui_text::font_unnamed());
            let size = ui_text::byte_size(f.stored_bytes());
            let header = ui_text::font_row_header(display, &size, verdict);

            let response = egui::CollapsingHeader::new(header)
                .id_salt(format!("font-{key}"))
                .default_open(false)
                .show(ui, |ui| {
                    // The verdict's REASON first, because it is the reason
                    // the row was opened.
                    let reason = match &f.removability {
                        Removability::Removable => ui_text::font_reason_removable().to_owned(),
                        Removability::BlockedIdentityEncoded { to_unicode, .. } => {
                            ui_text::font_reason_blocked_identity(*to_unicode)
                        }
                        Removability::BlockedType3 => {
                            ui_text::font_reason_blocked_type3().to_owned()
                        }
                        Removability::NotEmbedded => ui_text::font_reason_not_embedded().to_owned(),
                        Removability::Unknown(why) => match why {
                            RemovabilityUnknown::SymbolicBuiltinEncoding => {
                                ui_text::font_reason_unknown_symbolic().to_owned()
                            }
                            RemovabilityUnknown::PredefinedCMap => {
                                ui_text::font_reason_unknown_predefined_cmap().to_owned()
                            }
                            RemovabilityUnknown::EmbeddedCMap => {
                                ui_text::font_reason_unknown_embedded_cmap().to_owned()
                            }
                            RemovabilityUnknown::ProgramUnreadable => {
                                ui_text::font_reason_unknown_program_unreadable().to_owned()
                            }
                            RemovabilityUnknown::NoDescendant => {
                                ui_text::font_reason_unknown_no_descendant().to_owned()
                            }
                            _ => ui_text::font_reason_unknown_subtype().to_owned(),
                        },
                        _ => ui_text::font_reason_unknown_subtype().to_owned(),
                    };
                    ui.label(reason);
                    ui.separator();

                    let kind = match &f.descendant_subtype {
                        Some(d) => ui_text::font_composite_type(f.subtype.label(), d.label()),
                        None => f.subtype.label().to_owned(),
                    };
                    ui.label(ui_text::font_type_line(&kind));
                    ui.label(ui_text::font_encoding_line(&f.encoding.label()));

                    match &f.program {
                        Program::Embedded(p) => {
                            let key_label = match &p.subtype {
                                Some(s) => ui_text::font_program_key_with_subtype(p.key.label(), s),
                                None => p.key.label().to_owned(),
                            };
                            ui.label(ui_text::font_embedded_line(&key_label));
                            ui.label(ui_text::font_size_line(
                                &ui_text::byte_size(p.stored_bytes),
                                p.stored_bytes,
                            ));
                            // Only when it differs — a line repeating the
                            // number above is noise, and noise is how the
                            // lines that matter get skimmed past.
                            if let Some(decoded) = p.decoded_bytes
                                && decoded != p.stored_bytes
                            {
                                ui.label(ui_text::font_decoded_size_line(&ui_text::byte_size(
                                    decoded,
                                )));
                            }
                            fonts_panel_fs_type(ui, &p.fs_type);
                        }
                        // "Declared but unreadable" is damage; the reason
                        // sentence above already said so, and repeating a
                        // size of zero here would suggest a measurement was
                        // taken.
                        Program::Unreadable { .. } | Program::NotEmbedded => {
                            ui.label(ui_text::font_fstype_not_embedded());
                        }
                        _ => {}
                    }

                    ui.label(if f.has_to_unicode {
                        ui_text::font_to_unicode_present()
                    } else {
                        ui_text::font_to_unicode_absent()
                    });

                    ui.separator();
                    if !f.pages.is_empty() {
                        ui.label(ui_text::font_pages_line(
                            &pdfce_core::fontinfo::format_page_ranges(&f.pages),
                            f.pages.len(),
                        ));
                    }
                    for (surface, text) in [
                        (
                            Surface::AcroFormDefaultResources,
                            ui_text::font_found_in_form_resources(),
                        ),
                        (
                            Surface::AnnotationAppearance,
                            ui_text::font_found_in_annotation(),
                        ),
                        (Surface::Type3CharProcs, ui_text::font_found_in_type3()),
                    ] {
                        if f.surfaces.contains(&surface) {
                            ui.label(text);
                        }
                    }
                });

            // The subset tag lives here rather than in the row, because the
            // row shows the DE-PREFIXED name and two independent subsets of
            // one face therefore render identically. Without somewhere for
            // the tag to resurface, two adjacent identical rows read as a
            // rendering fault instead of as the real fact that the document
            // subsetted the face twice.
            if let Some(full) = f.base_font.as_deref() {
                response
                    .header_response
                    .on_hover_text(ui_text::font_full_name_tooltip(full));
            }
        }

        diag::trace(|| {
            format!(
                "fonts-panel rows={} embedded={} bytes={} verdicts={:?}",
                inv.fonts.len(),
                inv.embedded_count(),
                inv.embedded_bytes(),
                inv.verdict_counts()
            )
        });
    }
}

/// Render one font's `fsType` state.
///
/// ★ Four states, and **none of them may look like `0`.** `fsType == 0`
/// genuinely *means* Installable — the most permissive value the field can
/// express — so a blank, a dash, or an empty line for "we could not read it"
/// would assert the broadest embedding right there is on the strength of
/// bytes nobody read. The OpenType specification defines no default for the
/// absent case (`font__opentype_os2_fstype.md` N1), so pdfce defines none
/// either: unknown says the word "Unknown" in its own sentence, and
/// "this format has no such field" says that instead.
///
/// A free function rather than a method because it needs nothing from
/// `PdfceApp` — only the bits and a `Ui`.
fn fonts_panel_fs_type(ui: &mut egui::Ui, fs: &pdfce_core::fontinfo::FsType) {
    use pdfce_core::fontinfo::{EmbeddingPermission, FsType};
    match fs {
        FsType::NotApplicable => {
            ui.label(ui_text::font_fstype_no_field());
        }
        // Both failure states say "unknown" in words. They differ in cause
        // and not in what an operator can conclude, which is nothing.
        FsType::ProgramNotDecoded | FsType::Unreadable(_) => {
            ui.label(ui_text::font_fstype_unknown());
        }
        FsType::Known(bits) => {
            ui.label(match bits.permission {
                EmbeddingPermission::Installable => ui_text::font_fstype_installable(bits.raw),
                EmbeddingPermission::Restricted => ui_text::font_fstype_restricted(bits.raw),
                EmbeddingPermission::PreviewPrint => ui_text::font_fstype_preview_print(bits.raw),
                EmbeddingPermission::Editable => ui_text::font_fstype_editable(bits.raw),
                EmbeddingPermission::Ambiguous => ui_text::font_fstype_ambiguous(bits.raw),
                _ => ui_text::font_fstype_unspecified(bits.raw),
            });
            if bits.no_subsetting {
                ui.label(ui_text::font_fstype_no_subsetting());
            }
            if bits.bitmap_only {
                ui.label(ui_text::font_fstype_bitmap_only());
            }
            if bits.version_gated_bits_ignored {
                ui.label(ui_text::font_fstype_version_gated());
            }
        }
        // `FsType` is `#[non_exhaustive]`. A state this build does not know
        // must render as unknown, never as a permission.
        _ => {
            ui.label(ui_text::font_fstype_unknown());
        }
    }
}
