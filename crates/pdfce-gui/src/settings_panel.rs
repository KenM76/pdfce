//! The operator's settings window — where a spec ambiguity becomes a choice.
//!
//! # What this surface is for
//!
//! Standing rule **R169**, an operator directive of 2026-08-08: *"where
//! standards are ambiguous those should become settings that the user can
//! choose direction one, with the initial installed default as the best
//! guess of what is usually followed."*
//!
//! [`pdfce_core::settings`] is the store that makes such a choice survive
//! a restart. This module is the only place an operator can *make* one
//! without opening a text editor, and R83 — no affordance without
//! capability — cuts both ways here: a setting the program honours but
//! offers nowhere is just as much a gap as a control that does nothing.
//!
//! # Why a window and not a dock panel
//!
//! [`crate::dock`] deliberately has no Settings tab, and adding one would
//! contradict the reasoning that shaped it. The dock's own record states
//! the test: **"selection state is watched, workflows are entered."**
//! Watched things — the page rail, the object tree, the armed tool's
//! options — earn a permanent compartment because they are consulted
//! continuously *while doing something else*. Settings are the opposite:
//! consulted in bursts, deliberately, when there is something to change.
//! `Properties` was moved out of the dock on 2026-08-06 for exactly this
//! reason, after the operator used it and found the pairing wrong; putting
//! Settings in would be the same mistake with a different noun.
//!
//! So it is a window, opened from the **File** tab's Settings group, in
//! the same shape as the reset-layout chooser and the shortcuts reference
//! — and the shape an operator arriving from Office or PDF-XChange
//! already expects of *File → Options*.
//!
//! # Grouped, because eleven flat settings is a wall
//!
//! Settings are collapsed into five subject groups. This is not tidiness:
//! an operator opens this window with a *symptom* ("my black lines look
//! grey", "copied text has no spaces"), and the group headings are how a
//! symptom finds its setting. A flat list of eleven radio groups makes the
//! reader scan every one.
//!
//! The **Colour** group starts expanded because it holds the setting most
//! likely to have brought someone here — and the only one whose default
//! knowingly differs from other PDF viewers.
//!
//! # The three obligations this window carries beyond "show the value"
//!
//! A settings screen that lists keys and radio buttons would satisfy
//! nobody here. Three things must be visible that a conventional settings
//! dialog omits:
//!
//! 1. **What the default rests on.** The register behind these settings
//!    grades its recommended defaults (a) observed Acrobat behaviour,
//!    (b) corpus census, (c) other implementations, down to (d) reasoned
//!    inference — and **most are (d)**. "pdfce matched the dominant
//!    reader" and "pdfce guessed" are different claims and must not read
//!    alike, so a guess says it is a guess and the one well-sourced
//!    default (CMYK JPEG polarity) says that too.
//! 2. **That a choice was made at all.** These are settings *because the
//!    standard declines to have an opinion*. An operator who does not know
//!    that reads a difference between pdfce and Acrobat as a pdfce bug.
//!    Every group states what its clause leaves open.
//! 3. **Which way costs what.** A setting whose blast radius is the SAVED
//!    BYTES is a different kind of decision from one that only changes the
//!    preview, and every setting says which it is — rule 3's round-trip
//!    discipline is the operator's concern too, not only the engine's.
//!
//! # Divergence is labelled, not hidden
//!
//! One shipped default — [`CmykIntent::NeutralBlack`] — knowingly departs
//! from what Acrobat and pdfium do, on the operator's own 2026-08-08
//! ruling. The window says so *at that setting*, not in a footnote. A
//! future operator (or a future session) must be able to see that pdfce
//! chose differently **on purpose**, or the next render-parity difference
//! gets investigated as a defect.
//!
//! # Cancel is real
//!
//! The window edits a **working copy**. Nothing reaches the live
//! configuration or the disk until *Save*, and *Cancel* discards the lot.
//! This is not ceremony: several of these settings change saved bytes, so
//! an accidental radio click that took effect immediately would be an edit
//! the operator never intended and cannot see.

use eframe::egui;
use pdfce_core::pageops::SeparationPolicy;
use pdfce_core::settings::{
    ActualTextPrecedence, CmykIntent, CmykJpegPolarity, MaskResample, MinifyFilter,
    MissingAppearanceState, Settings, StoreLocation, TrailingEol, UnmappableCode, XrefEntryEol,
};

use crate::ui_text;

/// How far the working copy has drifted from what is on disk.
///
/// Drives whether *Save* is offered at all — a Save button that is always
/// live cannot tell the operator whether they have unsaved changes, and
/// this is a window someone may open just to read.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    /// The edits in progress.
    pub working: Settings,
    /// What the settings were when the window opened.
    pub original: Settings,
}

impl Draft {
    /// Start editing from the live configuration.
    #[must_use]
    pub fn new(current: &Settings) -> Self {
        Self {
            working: current.clone(),
            original: current.clone(),
        }
    }

    /// Whether anything has actually changed.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.working != self.original
    }

    /// Whether the working copy is entirely pdfce's shipped answer.
    ///
    /// Used to disable *Restore defaults* when it would do nothing — an
    /// enabled control that is a no-op is the R83 hazard.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        self.working == Settings::default()
    }
}

/// What the settings window is asking the shell to do.
///
/// Returned rather than performed, because this module renders and does
/// not own the application state — the split every other panel here uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Still open, nothing decided this frame.
    Idle,
    /// Write the working copy to disk and adopt it.
    Save,
    /// Discard the working copy and close.
    Cancel,
    /// Replace the working copy with pdfce's shipped answers. Does **not**
    /// save — the operator still has to confirm, and still has Cancel.
    RestoreDefaults,
}

/// Render the settings window.
///
/// `draft` is mutated in place as the operator clicks; nothing else is
/// touched. The caller decides what to do with the returned [`Outcome`].
pub fn show(
    ctx: &egui::Context,
    draft: &mut Draft,
    store: &StoreLocation,
    open: &mut bool,
) -> Outcome {
    let mut outcome = Outcome::Idle;

    egui::Window::new(ui_text::settings_window_title())
        .collapsible(false)
        .resizable(true)
        .default_width(600.0)
        .open(open)
        .show(ctx, |ui| {
            ui.label(ui_text::settings_intro());
            ui.add_space(4.0);
            // Where the file lives, always — an operator who does not know
            // which of the two homes is live cannot follow the update
            // instructions, and those instructions are the one place a
            // wrong guess costs them their configuration.
            ui.label(
                egui::RichText::new(ui_text::settings_store_location(store))
                    .small()
                    .weak(),
            );
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(460.0)
                .show(ui, |ui| {
                    group(ui, ui_text::settings_group_colour(), true, |ui| {
                        cmyk_intent_setting(ui, draft);
                        ui.add_space(10.0);
                        cmyk_jpeg_setting(ui, draft);
                    });
                    group(ui, ui_text::settings_group_images(), false, |ui| {
                        mask_setting(ui, draft);
                        ui.add_space(10.0);
                        minify_setting(ui, draft);
                    });
                    group(ui, ui_text::settings_group_text(), false, |ui| {
                        word_gap_setting(ui, draft);
                        ui.add_space(10.0);
                        unmappable_setting(ui, draft);
                        ui.add_space(10.0);
                        actual_text_setting(ui, draft);
                    });
                    group(ui, ui_text::settings_group_pages(), false, |ui| {
                        separations_setting(ui, draft);
                        ui.add_space(10.0);
                        missing_as_setting(ui, draft);
                    });
                    group(ui, ui_text::settings_group_saving(), false, |ui| {
                        xref_eol_setting(ui, draft);
                        ui.add_space(10.0);
                        trailing_eol_setting(ui, draft);
                    });
                });

            ui.separator();
            ui.horizontal(|ui| {
                // Disabled rather than hidden when there is nothing to
                // save, with the reason on hover — the treatment the
                // reset-layout chooser already gives its Apply button.
                let dirty = draft.is_dirty();
                let save = ui.add_enabled(dirty, egui::Button::new(ui_text::settings_save()));
                if save.clicked() {
                    outcome = Outcome::Save;
                }
                if !dirty {
                    save.on_hover_text(ui_text::settings_save_disabled_tooltip());
                }

                if ui
                    .button(ui_text::settings_cancel())
                    .on_hover_text(ui_text::settings_cancel_tooltip())
                    .clicked()
                {
                    outcome = Outcome::Cancel;
                }

                ui.add_space(12.0);
                let restorable = !draft.is_all_default();
                let restore = ui.add_enabled(
                    restorable,
                    egui::Button::new(ui_text::settings_restore_defaults()),
                );
                if restore.clicked() {
                    outcome = Outcome::RestoreDefaults;
                }
                if !restorable {
                    restore.on_hover_text(ui_text::settings_restore_disabled_tooltip());
                }
            });
        });

    outcome
}

/// One collapsible subject group.
fn group(
    ui: &mut egui::Ui,
    heading: &str,
    open_by_default: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::CollapsingHeader::new(egui::RichText::new(heading).strong())
        .default_open(open_by_default)
        .show(ui, body);
    ui.add_space(2.0);
}

/// One setting's heading plus the two sentences every setting owes: which
/// clause is silent, and what flipping it costs.
fn header(ui: &mut egui::Ui, title: &str, silence: &str, radius: &str) {
    ui.label(egui::RichText::new(title).strong());
    ui.label(egui::RichText::new(silence).small().weak());
    ui.label(egui::RichText::new(radius).small().weak());
    ui.add_space(2.0);
}

/// One radio option, with an optional explanatory line beneath it.
///
/// The note is optional because a few options are fully described by their
/// own label — "Carriage return then newline" needs no gloss — and padding
/// them with a restatement would be exactly the noise decision 017 §8.6
/// forbids in tooltips, one layer down.
fn option<T: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    label: &str,
    note: Option<&str>,
) {
    ui.radio_value(current, value, label);
    if let Some(note) = note {
        ui.label(egui::RichText::new(note).small().weak());
    }
}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

fn cmyk_intent_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_cmyk_title(),
        ui_text::setting_cmyk_silence(),
        ui_text::setting_cmyk_radius(),
    );
    let v = &mut draft.working.cmyk_intent;
    option(
        ui,
        v,
        CmykIntent::NeutralBlack,
        ui_text::setting_cmyk_neutral_black(),
        Some(ui_text::setting_cmyk_neutral_black_note()),
    );
    option(
        ui,
        v,
        CmykIntent::Calibrated,
        ui_text::setting_cmyk_calibrated(),
        Some(ui_text::setting_cmyk_calibrated_note()),
    );
    option(
        ui,
        v,
        CmykIntent::Naive,
        ui_text::setting_cmyk_naive(),
        Some(ui_text::setting_cmyk_naive_note()),
    );
    // The divergence disclosure, stated where the operator is choosing
    // rather than in a footnote: this reader is exactly the person who
    // needs to know pdfce departs from Acrobat here on purpose.
    ui.label(egui::RichText::new(ui_text::setting_cmyk_divergence()).small());
}

fn cmyk_jpeg_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_cmyk_jpeg_title(),
        ui_text::setting_cmyk_jpeg_silence(),
        ui_text::setting_cmyk_jpeg_radius(),
    );
    let v = &mut draft.working.cmyk_jpeg_polarity;
    option(
        ui,
        v,
        CmykJpegPolarity::NeverInvert,
        ui_text::setting_cmyk_jpeg_never(),
        Some(ui_text::setting_cmyk_jpeg_never_note()),
    );
    option(
        ui,
        v,
        CmykJpegPolarity::InvertOnApp14,
        ui_text::setting_cmyk_jpeg_invert(),
        Some(ui_text::setting_cmyk_jpeg_invert_note()),
    );
}

// ---------------------------------------------------------------------------
// Images and transparency
// ---------------------------------------------------------------------------

fn mask_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_mask_title(),
        ui_text::setting_mask_silence(),
        ui_text::setting_mask_radius(),
    );
    let v = &mut draft.working.mask_resample;
    option(
        ui,
        v,
        MaskResample::Nearest,
        ui_text::setting_mask_nearest(),
        Some(ui_text::setting_mask_nearest_note()),
    );
    option(
        ui,
        v,
        MaskResample::BoxAverage,
        ui_text::setting_mask_box(),
        Some(ui_text::setting_mask_box_note()),
    );
    option(
        ui,
        v,
        MaskResample::Bilinear,
        ui_text::setting_mask_bilinear(),
        Some(ui_text::setting_mask_bilinear_note()),
    );
}

fn minify_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_minify_title(),
        ui_text::setting_minify_silence(),
        ui_text::setting_minify_radius(),
    );
    let v = &mut draft.working.image_minify;
    option(
        ui,
        v,
        MinifyFilter::PointSample,
        ui_text::setting_minify_point(),
        Some(ui_text::setting_minify_point_note()),
    );
    option(
        ui,
        v,
        MinifyFilter::Smooth,
        ui_text::setting_minify_smooth(),
        Some(ui_text::setting_minify_smooth_note()),
    );
}

// ---------------------------------------------------------------------------
// Text extraction
// ---------------------------------------------------------------------------

fn word_gap_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_word_gap_title(),
        ui_text::setting_word_gap_silence(),
        ui_text::setting_word_gap_radius(),
    );
    // A slider rather than a text box: a free-text field invites a number
    // that then has to be silently clamped.
    //
    // The range is the STORE'S OWN accepted range, deliberately, and this
    // is not cosmetic. A narrower "usable band" — 0.05..=1.0 was the first
    // attempt — cannot represent a legal value the operator may already
    // have in their file, so merely OPENING this window would drag a
    // hand-edited 2.0 down to 1.0 and Save would write the changed value
    // back. That is precisely the silent, unrequested edit rule 4 exists
    // to prevent, and it would have been invisible: the operator never
    // touched the slider.
    //
    // Logarithmic because the useful resolution is at the low end, where
    // the difference between 0.15 and 0.25 decides whether words run
    // together; everything above 1.0 behaves much the same.
    ui.add(
        egui::Slider::new(
            &mut draft.working.word_gap_ratio,
            pdfce_core::settings::MIN_WORD_GAP_RATIO..=pdfce_core::settings::MAX_WORD_GAP_RATIO,
        )
        .logarithmic(true)
        .text(ui_text::setting_word_gap_slider_label()),
    );
    ui.label(
        egui::RichText::new(ui_text::setting_word_gap_note())
            .small()
            .weak(),
    );
}

fn unmappable_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_unmappable_title(),
        ui_text::setting_unmappable_silence(),
        ui_text::setting_unmappable_radius(),
    );
    let v = &mut draft.working.unmappable_code;
    option(
        ui,
        v,
        UnmappableCode::ReplacementChar,
        ui_text::setting_unmappable_replacement(),
        Some(ui_text::setting_unmappable_replacement_note()),
    );
    option(
        ui,
        v,
        UnmappableCode::QuestionMark,
        ui_text::setting_unmappable_question(),
        Some(ui_text::setting_unmappable_question_note()),
    );
    option(
        ui,
        v,
        UnmappableCode::Omit,
        ui_text::setting_unmappable_omit(),
        Some(ui_text::setting_unmappable_omit_note()),
    );
}

fn actual_text_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_actual_text_title(),
        ui_text::setting_actual_text_silence(),
        ui_text::setting_actual_text_radius(),
    );
    let v = &mut draft.working.actual_text;
    option(
        ui,
        v,
        ActualTextPrecedence::Always,
        ui_text::setting_actual_text_always(),
        Some(ui_text::setting_actual_text_always_note()),
    );
    option(
        ui,
        v,
        ActualTextPrecedence::TaggedOnly,
        ui_text::setting_actual_text_tagged(),
        Some(ui_text::setting_actual_text_tagged_note()),
    );
    option(
        ui,
        v,
        ActualTextPrecedence::Glyphs,
        ui_text::setting_actual_text_glyphs(),
        Some(ui_text::setting_actual_text_glyphs_note()),
    );
}

// ---------------------------------------------------------------------------
// Pages and printing
// ---------------------------------------------------------------------------

fn separations_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_separations_title(),
        ui_text::setting_separations_silence(),
        ui_text::setting_separations_radius(),
    );
    let v = &mut draft.working.separations;
    option(
        ui,
        v,
        SeparationPolicy::Repair,
        ui_text::setting_separations_repair(),
        Some(ui_text::setting_separations_repair_note()),
    );
    option(
        ui,
        v,
        SeparationPolicy::Discard,
        ui_text::setting_separations_discard(),
        Some(ui_text::setting_separations_discard_note()),
    );
    option(
        ui,
        v,
        SeparationPolicy::Refuse,
        ui_text::setting_separations_refuse(),
        Some(ui_text::setting_separations_refuse_note()),
    );
}

fn missing_as_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_missing_as_title(),
        ui_text::setting_missing_as_silence(),
        ui_text::setting_missing_as_radius(),
    );
    let v = &mut draft.working.missing_as;
    option(
        ui,
        v,
        MissingAppearanceState::PaintNothing,
        ui_text::setting_missing_as_nothing(),
        Some(ui_text::setting_missing_as_nothing_note()),
    );
    option(
        ui,
        v,
        MissingAppearanceState::FirstEntry,
        ui_text::setting_missing_as_first(),
        Some(ui_text::setting_missing_as_first_note()),
    );
    option(
        ui,
        v,
        MissingAppearanceState::OffElseNothing,
        ui_text::setting_missing_as_off(),
        Some(ui_text::setting_missing_as_off_note()),
    );
}

// ---------------------------------------------------------------------------
// Saving files
// ---------------------------------------------------------------------------

fn xref_eol_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_xref_eol_title(),
        ui_text::setting_xref_eol_silence(),
        ui_text::setting_xref_eol_radius(),
    );
    let v = &mut draft.working.xref_entry_eol;
    option(
        ui,
        v,
        XrefEntryEol::SpaceLf,
        ui_text::setting_xref_eol_space_lf(),
        Some(ui_text::setting_xref_eol_space_lf_note()),
    );
    // The remaining two are fully described by their labels; a gloss would
    // only restate them.
    option(
        ui,
        v,
        XrefEntryEol::SpaceCr,
        ui_text::setting_xref_eol_space_cr(),
        None,
    );
    option(
        ui,
        v,
        XrefEntryEol::CrLf,
        ui_text::setting_xref_eol_crlf(),
        None,
    );
}

fn trailing_eol_setting(ui: &mut egui::Ui, draft: &mut Draft) {
    header(
        ui,
        ui_text::setting_trailing_eol_title(),
        ui_text::setting_trailing_eol_silence(),
        ui_text::setting_trailing_eol_radius(),
    );
    let v = &mut draft.working.trailing_eol;
    option(
        ui,
        v,
        TrailingEol::Lf,
        ui_text::setting_trailing_eol_lf(),
        Some(ui_text::setting_trailing_eol_lf_note()),
    );
    option(
        ui,
        v,
        TrailingEol::None,
        ui_text::setting_trailing_eol_none(),
        Some(ui_text::setting_trailing_eol_none_note()),
    );
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use pdfce_core::settings::StoreKind;

    use super::*;

    #[test]
    fn a_fresh_draft_is_not_dirty() {
        let draft = Draft::new(&Settings::default());
        assert!(!draft.is_dirty(), "opening the window is not an edit");
        assert!(draft.is_all_default());
    }

    #[test]
    fn changing_a_value_makes_the_draft_dirty() {
        let mut draft = Draft::new(&Settings::default());
        draft.working.cmyk_intent = CmykIntent::Calibrated;
        assert!(draft.is_dirty());
        assert!(!draft.is_all_default());
    }

    #[test]
    fn changing_a_value_back_makes_it_clean_again() {
        // Save must not offer itself for a round trip that changed
        // nothing: an operator who clicks a radio and clicks back has made
        // no edit, and a dirty flag that latched would tell them they had.
        let mut draft = Draft::new(&Settings::default());
        let was = draft.working.cmyk_intent;
        draft.working.cmyk_intent = CmykIntent::Naive;
        draft.working.cmyk_intent = was;
        assert!(!draft.is_dirty());
    }

    #[test]
    fn a_draft_started_from_non_default_settings_is_clean_but_not_all_default() {
        // The two flags answer different questions and must not collapse:
        // "have I changed anything since opening" versus "is any of this
        // still pdfce's own answer".
        //
        // Built by mutating a draft rather than a struct expression:
        // `Settings` is `#[non_exhaustive]`, so an out-of-crate caller
        // cannot write one, and `let mut x = default(); x.f = ..` trips
        // `field_reassign_with_default`.
        let mut seed = Draft::new(&Settings::default());
        seed.working.separations = SeparationPolicy::Refuse;
        let draft = Draft::new(&seed.working);
        assert!(!draft.is_dirty(), "loading is not editing");
        assert!(
            !draft.is_all_default(),
            "Restore defaults must stay available"
        );
    }

    #[test]
    fn the_store_location_line_names_the_folder_it_is_using() {
        // The disclosure that matters: an operator following the update
        // instructions needs to know WHICH folder holds their settings.
        let portable = StoreLocation {
            path: Some(std::path::PathBuf::from("C:/pdfce/userdata/settings.txt")),
            kind: StoreKind::Portable,
        };
        assert!(ui_text::settings_store_location(&portable).contains("userdata"));

        let nowhere = StoreLocation {
            path: None,
            kind: StoreKind::None,
        };
        assert!(
            !ui_text::settings_store_location(&nowhere).is_empty(),
            "a session with nowhere to save must still say so"
        );
    }

    #[test]
    fn every_setting_the_store_carries_can_be_reached_from_this_window() {
        // R83, mechanised. A setting parsed and honoured but offered
        // nowhere in the UI is only settable by hand-editing a text file,
        // which is not a user interface — and the omission is invisible,
        // because nothing fails to compile.
        //
        // The check is deliberately crude: it reads THIS file's own source
        // and asserts every `pub` field of `Settings` is mentioned. A
        // field added to the store without a control here fails the test
        // with the field's name in the message.
        let store_src = include_str!("../../pdfce-core/src/settings/mod.rs");
        let panel_src = include_str!("settings_panel.rs");

        let fields: Vec<&str> = store_src
            .split("pub struct Settings {")
            .nth(1)
            .expect("the Settings struct must exist")
            .split("\n}")
            .next()
            .expect("the struct must be closed")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub "))
            .filter_map(|rest| rest.split(':').next())
            .collect();

        assert!(
            fields.len() >= 3,
            "parsed {} fields — the parser is stale, not the panel",
            fields.len()
        );
        for field in fields {
            assert!(
                panel_src.contains(&format!("working.{field}")),
                "`Settings::{field}` has no control in the settings window; \
                 an operator can only change it by hand-editing settings.txt"
            );
        }
    }
}
