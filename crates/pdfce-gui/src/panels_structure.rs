//! The document-structure panels: Signatures, Layers, Bookmarks and Fonts.
//!
//! # What these four have in common
//!
//! Each answers a question about the document's STRUCTURE rather than
//! its page content — what a signature covers, which layers a reader
//! would draw, where the outline points, which fonts the file carries
//! and what they cost.
//!
//! Three of them still change nothing: Bookmarks navigates, Layers
//! changes what is drawn for this session only, and Signatures cannot
//! change anything at all. **Fonts is no longer one of them.** `Pass
//! 67.0` phase B gave it the unembed controls, so this module now
//! contains a destructive operation, and it is the only panel here that
//! opens a confirmation and writes to the [`EditSession`] command log.
//!
//! ★ **That sentence was wrong for one commit, in the paragraph directly
//! above the one warning about exactly this.** Phase A's version read
//! "Signatures and Fonts cannot change anything at all"; phase B made it
//! false and did not come back for it. It was caught by `pdfce-librarian`
//! reading the two Passes side by side, not by any gate — and the
//! paragraph below already said, in as many words, that a module doc
//! enumerating its own contents has to be re-read whenever the file
//! changes and that **nothing enforces that**. The warning was accurate,
//! sitting inches away, and did not fire. R186's shape again: a note that
//! describes the hazard is not a check for it.
//!
//! Fonts joined this module in `Pass 67.0` phase A and is the newest,
//! which is why that sentence exists: the header said "these three" for
//! one commit after there were four. A module doc that enumerates its own
//! contents has to be re-read whenever something is added to the file,
//! and nothing enforces that — twice now.
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
    /// # Two controls, and why there were none until phase B
    ///
    /// Phase A shipped the report with **nothing to click**, and nothing
    /// stubbed either: a greyed-out "Remove" button, or a "coming soon"
    /// note, would have been an affordance for something that could not
    /// work (R83). Phase B added the removal, so the controls arrived with
    /// the capability rather than ahead of it.
    ///
    /// There are exactly two. A **batch** control at a fixed position under
    /// the document summary, computed from the whole inventory and never
    /// from a row's geometry — decision 024 §4.4 narrowed rule 4 precisely
    /// to reject a confirm control whose position is derived from the
    /// document. And a **per-font** control at the foot of an expanded row,
    /// shown only where the verdict is `Removable`.
    ///
    /// The per-font control is in the body rather than the header, and the
    /// usual cost of that — "you cannot see which rows are actionable
    /// without opening them" — is not paid here, because the **collapsed
    /// header already carries the verdict word**. "No blocker" against
    /// "Locked to program" answers the question at a glance.
    ///
    /// Every verdict is still drawn at the **same visual weight**, as a
    /// plain label, and the presence of the control is the only difference
    /// between a removable row and a refused one. That is not restraint for
    /// its own sake — a blocked verdict is a fact about the *file*, and
    /// error styling would make it read as a pdfce failure.
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
    pub(crate) fn fonts_panel(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        // The embed plan is refreshed BEFORE the list is drawn, outside the
        // document borrow, and only when something it depends on changed.
        self.refresh_embed_plan();
        // What the operator clicked, if anything. Collected inside the
        // document borrow and acted on after it ends: opening the unembed
        // question needs `&mut self` for `pending_unembed`, and the panel
        // body already holds `&mut self.status`. Rust makes the sequencing
        // explicit here, and the explicit sequencing is also the honest one
        // — nothing is decided while the list is still being drawn.
        let mut requested: Option<UnembedAsk> = None;
        self.fonts_panel_body(ui, actions, &mut requested);
        if let Some(ask) = requested {
            self.begin_unembed(&ask);
        }
    }

    /// Rebuild the cached embed plan when the inventory or the operator's
    /// font folders have changed, and not otherwise.
    ///
    /// # Why this is cached at all
    ///
    /// Building the plan resolves every non-embedded font against
    /// [`PdfceApp::font_env`] and **copies each donor's bytes** — on a
    /// document naming five faces out of a system font folder, several
    /// megabytes. egui redraws continuously, so the obvious immediate-mode
    /// shape (resolve inside each row's closure) would do that work sixty
    /// times a second. `page_texture` already keys off
    /// `font_env_generation` for exactly this reason; this follows it.
    ///
    /// # Why the plan and not just the resolutions
    ///
    /// The panel needs three things per row — which face was chosen, whether
    /// the match was exact, and, for a font that will NOT be embedded, the
    /// reason. All three are in the plan, which the core computes in one
    /// pass and which holds no donor bytes, so caching the plan is both
    /// smaller and closer to what the commit will actually do.
    fn refresh_embed_plan(&mut self) {
        let generation = self.font_env_generation;
        // Disjoint field borrows: the environment is read while the document
        // is borrowed mutably, which the borrow checker permits because they
        // are different fields of `self`.
        let env = &self.font_env;
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        if doc.fonts.is_none() {
            doc.fonts = Some(pdfce_core::fontinfo::inventory(&doc.session.view()));
        }
        if doc.embed_plan.is_some() && doc.embed_plan_generation == generation {
            return;
        }
        let Some(inventory) = doc.fonts.as_ref() else {
            return;
        };
        let request = build_embed_request(
            env,
            inventory,
            pdfce_core::font_embed_missing::EmbedSelection::AllMissing,
        );
        doc.embed_plan = Some(doc.session.embed_preview(&request));
        doc.embed_plan_generation = generation;
    }

    /// The Fonts panel's body — everything that reads the inventory.
    ///
    /// Split from [`Self::fonts_panel`] only to bound the `&mut self.status`
    /// borrow; there is no second concept here.
    fn fonts_panel_body(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut Vec<Action>,
        requested: &mut Option<UnembedAsk>,
    ) {
        use pdfce_core::fontinfo::{Program, Removability, RemovabilityUnknown, Surface};

        let Status::Open(doc) = &mut self.status else {
            return;
        };
        if doc.fonts.is_none() {
            let inv = pdfce_core::fontinfo::inventory(&doc.session.view());
            doc.fonts = Some(inv);
        }
        // Cloned rather than borrowed: `inv` borrows `doc.fonts` for the
        // rest of the function, and a set of object ids for the handful of
        // fonts one session unembeds is smaller than the borrow gymnastics
        // that would avoid the copy.
        let unembedded_here = doc.unembedded_this_session.clone();
        let embedded_here = doc.embedded_this_session.clone();
        let embed_plan = doc.embed_plan.clone();
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

        // ★ TWO batch blocks, STACKED, each with its own summary line and
        // its own button, embed above remove.
        //
        // Not one `ui.horizontal` carrying both. This panel already has an
        // incident for that shape: a summary-plus-button row overflowed the
        // dock's right edge and clipped the one field an operator opens the
        // panel for, with every headless assertion still green
        // (`D:\dev\rag\egui\headless_trace_asserts_reached_not_visible_a_clipped_widget_needs_a_pixel_oracle.md`).
        // Two such pairs on one line in a side dock reproduces it, and makes
        // two opposite-valence actions read as peers competing for a line.
        //
        // Embed goes FIRST for two independent reasons. It is the lower-stakes,
        // constructive action, and safe-first is the convention. And it is the
        // action an operator opens this panel for today — the driver for the
        // whole Pass was a book rejected by a print-on-demand service for a
        // missing font, not a file that was too large.
        //
        // Both blocks are offered only when there is something to offer: a
        // greyed-out control over a document it cannot act on is an
        // affordance for something that cannot work (R83).
        fonts_panel_embed_block(ui, actions, embed_plan.as_ref());

        let removable: Vec<pdfce_core::object::ObjId> = inv
            .fonts
            .iter()
            .filter(|f| f.removability.is_removable())
            .filter_map(|f| f.id)
            .collect();
        if !removable.is_empty() {
            let bytes: usize = inv
                .fonts
                .iter()
                .filter(|f| f.removability.is_removable())
                .map(pdfce_core::fontinfo::FontRecord::stored_bytes)
                .sum();
            ui.horizontal(|ui| {
                ui.label(ui_text::font_unembed_batch_summary(
                    removable.len(),
                    &ui_text::byte_size(bytes),
                ));
                let resp = ui
                    .button(ui_text::font_unembed_batch_button())
                    .on_hover_text(ui_text::font_unembed_batch_tooltip());
                // Traced with its RECT so the harness can drive the REAL
                // button rather than a step that calls the same function.
                // R184: three panels once shipped with no control an operator
                // could click, and every verification passed because the only
                // callers were harness step handlers.
                diag::trace(|| {
                    format!(
                        "font-unembed-batch rect={:?} fonts={} bytes={bytes}",
                        resp.rect,
                        removable.len()
                    )
                });
                if resp.clicked() {
                    *requested = Some(UnembedAsk::AllRemovable);
                }
            });
        }
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
                    // ★ Whose doing it was. After an unembed the row's
                    // verdict is `NotEmbedded` and its reason is the sentence
                    // a font that ARRIVED non-embedded carries — identical
                    // text, so the panel would erase the operator's own
                    // action from the one place they would look to confirm
                    // it. Said only when it is this session's doing; the
                    // status line already announced it once, and this is what
                    // survives the announcement scrolling away.
                    if f.id.is_some_and(|id| unembedded_here.contains(&id))
                        && matches!(f.removability, Removability::NotEmbedded)
                    {
                        ui.label(ui_text::font_unembed_done_this_session());
                    }
                    // The constructive twin, for the same reason: after an
                    // embed the row's verdict is one an already-embedded font
                    // would carry, so without this the panel erases the
                    // operator's own action from the place they would look.
                    if f.id.is_some_and(|id| embedded_here.contains(&id))
                        && !matches!(f.program, Program::NotEmbedded)
                    {
                        ui.label(ui_text::font_embed_done_this_session());
                    }
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

                    // The per-font control, LAST in the body — the same
                    // read-the-facts-then-act ordering the redaction Apply
                    // report uses. Its presence is the only difference
                    // between a removable row and a refused one, which is
                    // what keeps every verdict at the same visual weight
                    // (phase A's rule, and the reason a blocked verdict is
                    // not error-styled: it is a fact about the FILE).
                    //
                    // Discoverability does not depend on this control: the
                    // COLLAPSED header already carries the verdict word, so
                    // "which of these can go" is answerable without opening
                    // a single row.
                    //
                    // ★ That argument does NOT transfer to the embed control
                    // below it, and saying so is the point. The collapsed
                    // header reads "Not embedded" whether or not a face was
                    // found for it, so "which of these can be FIXED" is not
                    // answerable from the collapsed list at all. The batch
                    // block above answers it in aggregate instead — which is
                    // why that block states its counts unconditionally rather
                    // than only offering a button.
                    if let Some(id) = f.id
                        && f.removability.is_removable()
                    {
                        ui.separator();
                        let resp = ui
                            .button(ui_text::font_unembed_row_button())
                            .on_hover_text(ui_text::font_unembed_row_tooltip());
                        diag::trace(|| {
                            format!("font-unembed-row obj={} rect={:?}", id.num, resp.rect)
                        });
                        if resp.clicked() {
                            *requested = Some(UnembedAsk::One(id));
                        }
                    }

                    // The EMBED half of the row, in the same last position
                    // and the same read-the-facts-then-act ordering. The two
                    // are mutually exclusive per row: a font is either
                    // embedded (and possibly removable) or not embedded, so
                    // the two blocks can never stack in one body.
                    if matches!(f.program, Program::NotEmbedded) {
                        fonts_panel_embed_row(ui, actions, embed_plan.as_ref(), f);
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

// ---------------------------------------------------------------------------
// Font embedding — the constructive half of the Fonts panel (Pass 67.0 E)
// ---------------------------------------------------------------------------

/// What the operator asked the Fonts panel to embed into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmbedAsk {
    /// Every font the document does not carry a program for.
    AllMissing,
    /// One font, by its dictionary's object identity.
    One(pdfce_core::object::ObjId),
}

/// Build the core request for `selection`, resolving each missing font's
/// `/BaseFont` against the operator's registered faces.
///
/// # The crate boundary, in one function
///
/// `pdfce-render`'s [`FontEnvironment`](pdfce_render::FontEnvironment) knows
/// what faces exist; `pdfce-core` knows what may lawfully be written into a
/// font dictionary and must never depend on the renderer (project rule 2).
/// So the shell resolves a NAME to BYTES here and hands the bytes across. The
/// CLI's `embed-font` does the same thing with the same ladder — the ladder
/// itself lives in `FontEnvironment::resolve_for_embedding` so the two shells
/// cannot come to disagree about which face answers to which name (R171).
fn build_embed_request(
    env: &pdfce_render::FontEnvironment,
    inventory: &pdfce_core::fontinfo::FontInventory,
    selection: pdfce_core::font_embed_missing::EmbedSelection,
) -> pdfce_core::font_embed_missing::EmbedRequest {
    use pdfce_core::font_embed_missing::{EmbedRequest, EmbedSelection, FontMatch, SuppliedFont};
    use pdfce_core::fontinfo::Program;
    use pdfce_render::font::EmbedMatch;

    let mut request = match selection {
        EmbedSelection::Objects(ids) => EmbedRequest::objects(ids),
        EmbedSelection::Named(names) => EmbedRequest::named(names),
        _ => EmbedRequest::all_missing(),
    };
    for record in &inventory.fonts {
        if !matches!(record.program, Program::NotEmbedded) {
            continue;
        }
        let Some(base_font) = record.base_font.as_deref() else {
            continue;
        };
        // ★ The bundled faces are NOT offered from the GUI. They are
        // BSD-3-Clause (pdfium's Foxit-origin set), and embedding one puts it
        // inside a document the operator then distributes — a different act
        // from drawing with it on their own screen, and one that carries the
        // licence's attribution condition. That is the operator's decision,
        // and pdfce has no place to have taken it here; `pdfce-cli` exposes
        // it behind an explicit `--use-bundled-fonts` whose help text states
        // the obligation.
        let Some(donor) = env.resolve_for_embedding(base_font, false) else {
            continue;
        };
        let matched = match donor.quality {
            EmbedMatch::Exact => FontMatch::Exact,
            EmbedMatch::Alias => FontMatch::Alias,
            EmbedMatch::Bundled => FontMatch::Bundled,
        };
        request = request.with_font(
            base_font,
            SuppliedFont::new(
                donor.data.bytes().to_vec(),
                donor.face_name.clone(),
                donor.face_name.clone(),
                matched,
            ),
        );
    }
    request
}

/// One row's embed target in the cached plan, if it has one.
fn embed_target_for(
    plan: Option<&pdfce_core::font_embed_missing::EmbedPlan>,
    id: Option<pdfce_core::object::ObjId>,
) -> Option<&pdfce_core::font_embed_missing::EmbedTarget> {
    let (plan, id) = (plan?, id?);
    plan.targets.iter().find(|t| t.id == id)
}

/// The batch block: a summary line stating the exact/substitute split, then
/// the button — or, when nothing is missing, the end-state sentence.
///
/// A free function because it needs nothing from `PdfceApp` beyond the plan
/// the caller already cloned out of the document borrow.
fn fonts_panel_embed_block(
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
    plan: Option<&pdfce_core::font_embed_missing::EmbedPlan>,
) {
    let Some(plan) = plan else {
        return;
    };
    // ★ The end state, in the same fixed slot the offer occupies. The
    // operator's real question is whether a print service will accept the
    // file; this answers only what pdfce measured, and deliberately says
    // nothing about PDF/A or any vendor's check.
    if plan.missing_before == 0 {
        ui.label(ui_text::fonts_all_embedded());
        ui.separator();
        return;
    }
    if plan.targets.is_empty() {
        // Nothing resolves. Not silence: the per-row reasons say why, and
        // the route out of it is the Font Folders tool.
        ui.label(ui_text::font_embed_unresolved());
        let resp = ui.button(ui_text::font_folders_add_button());
        diag::trace(|| format!("font-embed-none rect={:?}", resp.rect));
        if resp.clicked() {
            actions.push(Action::OpenFontFolders);
        }
        ui.separator();
        return;
    }

    let substitute = plan
        .targets
        .iter()
        .filter(|t| t.matched.is_substitute())
        .count();
    let exact = plan.targets.len() - substitute;
    let bytes = usize::try_from(plan.bytes_added_uncompressed()).unwrap_or(usize::MAX);
    // ★ The summary is on its own line and the button BELOW it, not beside
    // it — and that is a measured decision, not a style preference.
    //
    // Written first as `ui.horizontal(summary, button)`, mirroring the
    // unembed block. Under `tools/gui-drive.ps1` the button traced a rect of
    // `[413.8 475.0] - [542.2 499.0]` in a dock whose content ends before
    // that, and **a scripted click at the centre of its own reported rect did
    // nothing**. The unembed button, whose summary is short enough to leave
    // room, took the same click and opened its dialog — which is how the
    // difference was isolated to the layout rather than to the harness.
    //
    // This is the incident
    // `D:\dev\rag\egui\headless_trace_asserts_reached_not_visible_a_clipped_widget_needs_a_pixel_oracle.md`
    // records, arriving through a new door: the widget was reached, it traced
    // its rect, every headless assertion about it passed, and it was not
    // clickable. Embed's summary carries three numbers and a size where
    // unembed's carries two, so a side-by-side layout that fits one does not
    // fit the other — and a *dock* is narrower than the centred windows this
    // panel's other controls live in.
    ui.label(ui_text::font_embed_batch_summary(
        exact,
        substitute,
        &ui_text::byte_size(bytes),
    ));
    let resp = ui
        .button(ui_text::font_embed_batch_button())
        .on_hover_text(ui_text::font_embed_batch_tooltip());
    // Traced with its RECT so the harness drives the REAL control rather
    // than a step that calls the same function (R184).
    diag::trace(|| {
        format!(
            "font-embed-batch rect={:?} fonts={} exact={exact} substitute={substitute} \
bytes={bytes}",
            resp.rect,
            plan.targets.len(),
        )
    });
    if resp.clicked() {
        actions.push(Action::EmbedAllFonts);
    }
    // Warn-coloured and only when true. A line shown every time is a line
    // operators stop reading, which is the same as not having it.
    if substitute > 0 {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            ui_text::font_embed_batch_substitute_note(substitute),
        );
    }
    ui.label(
        egui::RichText::new(ui_text::font_embed_layout_note())
            .small()
            .weak(),
    );
    ui.separator();
}

/// The per-row embed block: what was resolved, or why nothing was, then the
/// button.
///
/// Facts first, control last — the same ordering the unembed row and the
/// redaction report use. The exact-vs-substitute sentence sits DIRECTLY above
/// the button, which is what makes the click a deliberate act over a
/// disclosure already on screen, and therefore what makes the absence of a
/// confirmation window correct rather than a shortcut (decision 024 §4.4).
fn fonts_panel_embed_row(
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
    plan: Option<&pdfce_core::font_embed_missing::EmbedPlan>,
    record: &pdfce_core::fontinfo::FontRecord,
) {
    ui.separator();
    let Some(target) = embed_target_for(plan, record.id) else {
        // Not embeddable. Either nothing answers to the name, or the core
        // refused it — and a refusal is a fact about the FILE, so it is
        // rendered plainly, never error-styled (this panel's phase A rule).
        let refusal = plan.and_then(|p| {
            p.blocked
                .iter()
                .find(|b| b.id == record.id && b.blocker.token() != "no-source-font")
        });
        match refusal {
            Some(b) => {
                ui.label(ui_text::font_embed_refused_row(b.blocker.reason()));
            }
            None => {
                ui.label(ui_text::font_embed_unresolved());
                ui.label(
                    egui::RichText::new(ui_text::font_embed_no_folders_hint())
                        .small()
                        .weak(),
                );
                let resp = ui.button(ui_text::font_folders_add_button());
                diag::trace(|| {
                    format!(
                        "font-embed-row-unresolved obj={:?} rect={:?}",
                        record.id.map(|i| i.num),
                        resp.rect
                    )
                });
                if resp.clicked() {
                    actions.push(Action::OpenFontFolders);
                }
            }
        }
        return;
    };

    let requested = record
        .base_font
        .as_deref()
        .unwrap_or(ui_text::font_unnamed());
    if target.matched.is_substitute() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            ui_text::font_embed_resolved_substitute(requested, &target.face_name, &target.source),
        );
    } else {
        ui.label(ui_text::font_embed_resolved_exact(
            &target.face_name,
            &target.source,
        ));
    }
    if record.standard_14 {
        ui.label(
            egui::RichText::new(ui_text::font_embed_resolved_standard14())
                .small()
                .weak(),
        );
    }
    ui.label(
        egui::RichText::new(ui_text::font_embed_layout_note())
            .small()
            .weak(),
    );
    let resp = ui
        .button(ui_text::font_embed_row_button())
        .on_hover_text(ui_text::font_embed_row_tooltip());
    diag::trace(|| {
        format!(
            "font-embed-row obj={} rect={:?} match={} face={:?}",
            target.id.num,
            resp.rect,
            target.matched.token(),
            target.face_name,
        )
    });
    if resp.clicked() {
        actions.push(Action::EmbedOneFont(target.id));
    }
}

// ---------------------------------------------------------------------------
// Font unembedding — the destructive half of the Fonts panel (Pass 67.0 B)
// ---------------------------------------------------------------------------

/// What the operator asked the Fonts panel to unembed.
///
/// A tiny `Copy` enum rather than a request value, because it travels out of
/// the panel body across the end of a `&mut self.status` borrow. The real
/// [`UnembedRequest`](pdfce_core::font_unembed::UnembedRequest) is built from
/// it one line later, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnembedAsk {
    /// Every font the report says nothing blocks.
    AllRemovable,
    /// One font, by its dictionary's object identity.
    One(pdfce_core::object::ObjId),
}

/// A font-unembed question waiting for the operator.
///
/// The FIFTH independent pending state in `PdfceApp`, and it joins the same
/// `apply()` gate the other four use rather than getting one of its own.
/// The reason is the one already written down there: these are all
/// centre-anchored windows, so two on screen at once means one is
/// unclickable underneath the other — and this one is a **destructive**
/// question, which an operator must never be able to answer unseen.
pub(crate) struct PendingUnembed {
    /// What the operator asked for. Carried so the confirm path rebuilds the
    /// *same* request rather than re-deriving one from the plan.
    ask: UnembedAsk,
    /// The plan, computed once when the question opened.
    ///
    /// Not recomputed per frame: it walks the whole font inventory and
    /// decodes every embedded program. It cannot go stale while the dialog
    /// is up, because the `apply()` gate blocks every other action — and the
    /// commit recomputes it anyway, so the worst case is a report that
    /// matched what happened.
    plan: pdfce_core::font_unembed::UnembedPlan,
    /// The mandatory "the text will look different" acknowledgement.
    acknowledged: bool,
    /// The conditional "pdfce could not search everywhere" acknowledgement.
    acknowledged_scan_gap: bool,
    /// Whether the inventory sweep was cut short, captured WITH the plan so
    /// the gate and the checkbox cannot come to disagree about whether the
    /// tick is required.
    scan_gap: bool,
}

impl PendingUnembed {
    /// Whether every required acknowledgement has been given.
    fn ready_to_confirm(&self) -> bool {
        self.acknowledged && (!self.scan_gap || self.acknowledged_scan_gap)
    }
}

/// Build the core request for an ask.
///
/// One function, called by both the question and the commit, so the plan the
/// operator read and the operation that runs cannot describe different work.
fn request_for(ask: &UnembedAsk) -> pdfce_core::font_unembed::UnembedRequest {
    use pdfce_core::font_unembed::UnembedRequest;
    match ask {
        UnembedAsk::AllRemovable => UnembedRequest::all_removable(),
        UnembedAsk::One(id) => UnembedRequest::objects([*id]),
    }
}

impl PdfceApp {
    /// Open the unembed question for `ask`.
    ///
    /// Computes the plan and puts it on screen. **Nothing is changed here** —
    /// this is the disclosure half of rule 4, and the whole reason the core
    /// exposes a preview that runs the same function the commit does.
    pub(crate) fn begin_unembed(&mut self, ask: &UnembedAsk) {
        let Status::Open(doc) = &self.status else {
            return;
        };
        // R83: ask before offering. A document-level refusal (encrypted,
        // certification-locked) is reported through the status bar rather
        // than by opening a question the operator cannot answer.
        if let Some(refusal) = doc.session.unembed_refusal() {
            let note = ui_text::font_unembed_refused(&refusal.to_string());
            self.set_edit_note(note);
            return;
        }
        let plan = doc.session.unembed_preview(&request_for(ask));
        if plan.targets.is_empty() {
            // Nothing to ask about. The reasons are already on the rows, so
            // the status line points at them rather than repeating one.
            let note = plan.blocked.first().map_or_else(
                || ui_text::font_unembed_refused(ui_text::fonts_none()),
                |b| ui_text::font_unembed_refused(b.blocker.reason()),
            );
            self.set_edit_note(note);
            return;
        }
        let scan_gap = doc.fonts.as_ref().is_some_and(|inv| {
            inv.diagnostics.resource_scan_truncated || inv.diagnostics.page_scan_failed
        });
        diag::trace(|| {
            format!(
                "unembed-ask targets={} blocked={} pdfa={} scan_gap={scan_gap}",
                plan.targets.len(),
                plan.blocked.len(),
                plan.pdfa.token(),
            )
        });
        self.pending_unembed = Some(PendingUnembed {
            ask: *ask,
            plan,
            acknowledged: false,
            acknowledged_scan_gap: false,
            scan_gap,
        });
    }

    /// Draw the unembed confirmation.
    ///
    /// # Why a confirm step at all, when the click was deliberate
    ///
    /// Rule 4 as narrowed (decision 024 §4.4) drops the confirm step for a
    /// direct manipulation whose result is fully visible and reversible in
    /// one undo. This is not that. `Removability::Removable` is something
    /// pdfce **inferred** — the same family as the font-trust downgrade the
    /// rule names by example — and three of the four consequences are not
    /// visible on the canvas at all: the PDF/A break, the signature
    /// invalidation, and the name change. The fourth (the appearance shift)
    /// is only visible if the operator happens to be looking at an affected
    /// page, which they need not be to reach a side panel.
    ///
    /// The control's position is fixed: a centre-anchored window, not
    /// something anchored to a row or to page geometry — which is precisely
    /// the placement the narrowing was written to stop.
    pub(crate) fn unembed_confirmation(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        use pdfce_core::font_unembed::PdfaClaim;

        let Some(pending) = &mut self.pending_unembed else {
            return;
        };
        let plan = &pending.plan;
        // Read BEFORE the checkboxes are drawn, exactly as the redaction
        // dialog does and for the same reason: the one-frame lag makes it
        // impossible for the tick that enables the button and the click that
        // presses it to land in the same frame.
        let ready = pending.ready_to_confirm();
        let one = plan.targets.len() == 1;
        // Bound once, outside the closure: `ui_text::font_unnamed` is a
        // function and calling it inside would make the borrow checker read
        // the closure as capturing more than it does.
        let unnamed = ui_text::font_unnamed();
        let title = if one {
            let name = plan.targets.first().and_then(|t| t.base_font.as_deref());
            ui_text::font_unembed_confirm_title_one(name.unwrap_or(unnamed))
        } else {
            ui_text::font_unembed_confirm_title_many(plan.targets.len())
        };

        egui::Window::new(title)
            .collapsible(false)
            // Resizable, like the redaction report and unlike the two fixed
            // dialogs: the body carries a variable-length list.
            .resizable(true)
            .default_size([600.0, 480.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                // Warn-coloured and FIRST: the appearance change is not fine
                // print, and nothing below it makes sense without it.
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    ui_text::font_unembed_appearance_note(),
                );
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("unembed-report")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for t in &plan.targets {
                            let name = t.base_font.as_deref().unwrap_or(unnamed);
                            ui.label(ui_text::font_unembed_list_row(
                                name,
                                &ui_text::byte_size(t.stored_bytes),
                            ));
                            if let Some(new_name) = &t.rename {
                                ui.label(
                                    egui::RichText::new(ui_text::font_unembed_rename_note(
                                        name, new_name,
                                    ))
                                    .small()
                                    .weak(),
                                );
                            }
                            // Shown only when it is true. A saving that does
                            // not happen is the one number in this dialog an
                            // operator would act on and be wrong about.
                            if !t.program_freed {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    ui_text::font_unembed_shared_program_note(
                                        name,
                                        t.program_shared_with.len(),
                                    ),
                                );
                            }
                        }
                    });

                ui.separator();
                let reclaim = usize::try_from(plan.bytes_reclaimable()).unwrap_or(usize::MAX);
                ui.label(ui_text::font_unembed_reclaim_note(&ui_text::byte_size(
                    reclaim,
                )));
                match &plan.pdfa {
                    PdfaClaim::Identified { part, conformance } => {
                        let level = format!(
                            "PDF/A-{}{}",
                            part.as_deref().unwrap_or("?"),
                            conformance.as_deref().unwrap_or("")
                        );
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            ui_text::font_unembed_pdfa_note(&level),
                        );
                    }
                    // Said out loud rather than omitted: "we could not look"
                    // and "there is nothing there" are different facts, and
                    // only one of them is reassuring.
                    PdfaClaim::MetadataUnreadable => {
                        ui.label(ui_text::font_unembed_pdfa_unreadable_note());
                    }
                    _ => {}
                }
                ui.label(ui_text::font_unembed_signature_pointer());
                ui.label(ui_text::font_unembed_undo_note());

                ui.separator();
                let ack_rect = ui
                    .checkbox(
                        &mut pending.acknowledged,
                        ui_text::font_unembed_confirm_checkbox(),
                    )
                    .rect;
                // Exists ONLY when there is something to acknowledge. Always
                // showing it would make it a box operators tick without
                // reading, which is the same as not having it.
                if pending.scan_gap {
                    ui.checkbox(
                        &mut pending.acknowledged_scan_gap,
                        ui_text::font_unembed_scan_gap_checkbox(),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let cancel = ui.button(ui_text::font_unembed_cancel_button());
                    if cancel.clicked() {
                        actions.push(Action::CancelUnembed);
                    }
                    // R83: the affordance appears exactly when the capability
                    // does.
                    let confirm = ui
                        .add_enabled_ui(ready, |ui| {
                            let label = if one {
                                ui_text::font_unembed_confirm_button_one().to_owned()
                            } else {
                                ui_text::font_unembed_confirm_button_many(plan.targets.len())
                            };
                            ui.button(label)
                        })
                        .inner;
                    if confirm.clicked() {
                        actions.push(Action::ConfirmUnembed);
                    }
                    // Every clickable rect in this window, traced, so the
                    // harness drives the real controls (R184) and so the
                    // acknowledgement gate can be shown to actually gate.
                    diag::trace(|| {
                        format!(
                            "unembed-dialog targets={} ready={ready} confirm_rect={:?} cancel_rect={:?} ack_rect={:?}",
                            plan.targets.len(),
                            confirm.rect,
                            cancel.rect,
                            ack_rect,
                        )
                    });
                });
            });
    }

    /// **Add the font programs** for `ask`, directly.
    ///
    /// # Why there is no confirmation window, and why that is not a shortcut
    ///
    /// [`Self::unembed_confirmation`] exists because three of unembedding's
    /// four consequences are invisible on the canvas — it breaks a PDF/A
    /// claim, invalidates a signature, and renames the font — and its own
    /// doc comment ranks the fourth, the appearance shift, as the weak leg
    /// that would not justify a modal alone.
    ///
    /// Embedding has none of the first three: it moves a file **toward** ISO
    /// 19005 conformance, it renames only a subset tag that no longer
    /// describes anything, and it is non-destructive and reversible in one
    /// undo. What is left is exactly the leg phase B calls insufficient on
    /// its own — so decision 024 §4.4 applies, and what that clause asks for
    /// is a *disclosure*, not a gate: "the uncertainty is stated in the
    /// disclosure, not merely implied by the presence of a confirm button."
    ///
    /// The disclosure is therefore load-bearing rather than decorative. The
    /// exact-vs-substitute sentence sits directly above each row's button and
    /// the split is counted above the batch button, so every click happens
    /// over a statement of what pdfce chose.
    ///
    /// If a substitute turns out wrong, the recovery path already exists and
    /// needs no new machinery: the newly-embedded row now offers the
    /// unembed control, which removes exactly that one font's program
    /// without touching the rest of a batch. Ctrl+Z, by contrast, is
    /// all-or-nothing — `embed_fonts` commits one command however many fonts
    /// it embedded, which is §11.3's one-gesture-one-entry rule and is
    /// stated here as a deliberate property rather than left as an accident
    /// of what the core happens to do.
    pub(crate) fn embed_fonts(&mut self, ask: EmbedAsk) {
        use pdfce_core::font_embed_missing::EmbedSelection;

        let env = &self.font_env;
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        // R83: ask before offering. A document-level refusal (encrypted,
        // certification-locked) is reported through the status bar rather
        // than by drawing a control that cannot work.
        if let Some(refusal) = doc.session.embed_refusal() {
            let note = ui_text::font_embed_refused(&refusal.to_string());
            self.set_edit_note(note);
            return;
        }
        let Some(inventory) = doc.fonts.as_ref() else {
            return;
        };
        let selection = match ask {
            EmbedAsk::AllMissing => EmbedSelection::AllMissing,
            EmbedAsk::One(id) => EmbedSelection::Objects(vec![id]),
        };
        // Rebuilt rather than taken from the cached plan: the plan holds no
        // donor bytes (deliberately — it is redrawn every frame), and the
        // core recomputes its own plan inside `embed_fonts` anyway, so the
        // report the operator read and the operation that runs come from the
        // same function over the same request.
        let request = build_embed_request(env, inventory, selection);
        let outcome = doc.session_mut().embed_fonts(&request);
        match outcome {
            Ok(plan) => {
                for t in &plan.targets {
                    doc.embedded_this_session.insert(t.id);
                }
                let count = plan.targets.len();
                let substitute = plan
                    .targets
                    .iter()
                    .filter(|t| t.matched.is_substitute())
                    .count();
                let bytes = usize::try_from(plan.bytes_added_uncompressed()).unwrap_or(usize::MAX);
                let name = plan.targets.first().and_then(|t| t.base_font.clone());
                // Drops the cached inventory AND the cached embed plan, so
                // the rows redraw from the edited session.
                doc.refresh_pages();
                doc.ensure_object_provider();
                let note = match (count, name) {
                    (1, Some(name)) => {
                        ui_text::font_embed_done_one(&name, &ui_text::byte_size(bytes))
                    }
                    _ => {
                        ui_text::font_embed_done_many(count, substitute, &ui_text::byte_size(bytes))
                    }
                };
                self.set_edit_note(note);
                diag::trace(|| {
                    format!("embed-committed fonts={count} substitute={substitute} bytes={bytes}")
                });
            }
            Err(ref err) => {
                let note = ui_text::font_embed_refused(&err.to_string());
                self.set_edit_note(note);
                diag::trace(|| format!("embed-refused {err}"));
            }
        }
    }

    /// Perform the unembed the operator confirmed.
    ///
    /// The request is rebuilt from the same [`UnembedAsk`] the question was
    /// opened with, so the core recomputes its own plan and the two cannot
    /// have drifted. A failure is reported through the status bar rather
    /// than swallowed: the refusals reachable here — an encrypted document,
    /// a certification that forbids the change — are the reason the
    /// operation is safe, and an operator who is refused needs to know why.
    pub(crate) fn confirm_unembed(&mut self) {
        let Some(pending) = self.pending_unembed.take() else {
            return;
        };
        let request = request_for(&pending.ask);
        let Status::Open(doc) = &mut self.status else {
            return;
        };
        let outcome = doc.session_mut().unembed_fonts(&request);
        match outcome {
            Ok(plan) => {
                for t in &plan.targets {
                    doc.unembedded_this_session.insert(t.id);
                }
                let count = plan.targets.len();
                let bytes = usize::try_from(plan.bytes_reclaimable()).unwrap_or(usize::MAX);
                let name = plan
                    .targets
                    .first()
                    .and_then(|t| t.rename.clone().or_else(|| t.base_font.clone()));
                // Drops the cached inventory, so the rows redraw from the
                // edited session rather than from the pre-edit sweep.
                doc.refresh_pages();
                doc.ensure_object_provider();
                let note = match (count, name) {
                    (1, Some(name)) => {
                        ui_text::font_unembed_done_one(&name, &ui_text::byte_size(bytes))
                    }
                    _ => ui_text::font_unembed_done_many(count, &ui_text::byte_size(bytes)),
                };
                self.set_edit_note(note);
                diag::trace(|| format!("unembed-committed fonts={count} bytes={bytes}"));
            }
            Err(ref err) => {
                let note = ui_text::font_unembed_refused(&err.to_string());
                self.set_edit_note(note);
                diag::trace(|| format!("unembed-refused {err}"));
            }
        }
    }
}
