//! What pdfce looks like — the single place that decides.
//!
//! # Why this module exists
//!
//! Until now nothing set a style at all. The whole application ran on
//! `egui`'s stock appearance, and every colour it drew beyond that was a
//! `Color32::from_rgb(…)` literal at its use site — 26 of them, four
//! named, the rest inline in a 27,000-line file. There was no answer to
//! "what colour is pdfce's accent?" other than reading the source.
//!
//! That is not a cosmetic problem, it is a *change-cost* problem. A
//! restyle under those conditions is a sweep through every call site
//! where the failure mode is not a crash but INCONSISTENCY — the sites
//! you miss leave two-thirds of a theme, which looks worse than none and
//! cannot be caught by a test that only knows about compilation.
//!
//! So the look is data, in one place, and [`check-theme-colors.sh`]
//! forbids raw colours outside it. That is the same shape as
//! [`crate::ui_text`] (every operator-visible string, gated) and
//! [`crate::icons`] (every glyph, gated by parse and raster tests). Both
//! of those already make their kind of change safe; this is the third.
//!
//! [`check-theme-colors.sh`]: ../../../tools/check-theme-colors.sh
//!
//! # ★ CHROME IS THEMED. DOCUMENT COLOUR IS NOT. THEY ARE NOT THE SAME KIND.
//!
//! This is the distinction that makes a colour sweep dangerous, and it
//! is the reason the gate has an escape hatch rather than being absolute.
//!
//! Two colours in this application are written **into the PDF**:
//!
//! - `PdfceApp::markup_color` — the colour of an annotation the operator
//!   authors. It reaches `/C` on the annotation and the appearance
//!   stream's colour operators.
//! - `PdfceApp::prop_color` — the same, for the properties panel.
//!
//! Those are the *operator's* choice about *document content*. They are
//! not chrome, they are not pdfce's, and a theme must never touch them:
//! restyling the application would silently change the colour of markup
//! a user is about to commit to a file, and the change would only be
//! visible after saving. A dark theme that quietly authored pale-grey
//! annotations onto a white page would be a data defect wearing a
//! cosmetic disguise.
//!
//! Everything else — panel backgrounds, selection highlights, snap
//! guides, node marks, measurement previews, the canvas backdrop — is
//! chrome, belongs here, and changes with the theme.
//!
//! The rule for anyone adding a colour: **if it can end up in a saved
//! file, it is not a theme colour.** Mark such a site with the literal
//! comment `// DOCUMENT COLOUR:` and the gate will allow it, because the
//! gate's job is to catch the colour someone forgot to name, not to
//! forbid the two that must stay where they are.
//!
//! # Overlay colours are semantics, not decoration
//!
//! The canvas overlay palette is not free choice. Several of its entries
//! carry meaning the operator is expected to learn:
//!
//! - the node mark and the subpath outline are different colours because
//!   they answer different questions ("a point is here" vs "this run is
//!   one subpath");
//! - the measurement preview and the committed dimension differ because
//!   one is a proposal and one is document state (rule 4 — pdfce's own
//!   inferences are visibly distinct from what the operator committed);
//! - the form-field chrome has a hue of its own, distinct from the
//!   object-selection accent, because it means "a control lives here"
//!   rather than "this is selected".
//!
//! A theme may re-tune those hues. It may **not** collapse two of them
//! into one, and `distinct_overlay_roles` in this module's tests enforces
//! that for every preset: any theme in which two semantically distinct
//! roles resolve to the same colour fails the build. Under R84 colour is
//! never the only cue for any of these — each also carries a shape, a
//! dash pattern or a label — but a theme that merges two roles removes a
//! cue that was doing work, and it would do so silently.
//!
//! # Why presets, and why the operator can switch at runtime
//!
//! Three presets ship. That is not indecision — the right look for this
//! application is a question about the operator's environment, not about
//! the code, and it cannot be settled by reading the source. A CAD user
//! on a dark toolchain, a document reviewer on a bright monitor and a
//! long-session editor genuinely want different things.
//!
//! Switching is live, from Settings, because the alternative is choosing
//! a look from screenshots — and a screenshot cannot show what an hour in
//! the application feels like.
//!
//! # Metrics travel with the palette
//!
//! [`Metrics`] carries the spacing and sizing decisions that make a look
//! coherent — control height, gutter, panel padding, corner radius. They
//! belong with the palette rather than in a second module because they
//! are not independent: a generous-padding theme with a dense theme's
//! control height reads as a mistake, and keeping them in one struct
//! makes that combination unrepresentable rather than merely discouraged.

use eframe::egui;
use eframe::egui::Color32;

/// The named colour roles pdfce draws with.
///
/// Every field is a **role**, never a colour name: `accent`, not `blue`.
/// A theme that wants a green accent should not have to be read as
/// "blue, except it's green now" — which is exactly what a field called
/// `blue` forces on the next reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The window and panel background.
    pub surface: Color32,
    /// A panel or dock sitting on top of [`Self::surface`], one step
    /// separated from it.
    pub panel: Color32,
    /// The area behind the page itself — deliberately its own role
    /// rather than reusing `surface`, because the page must read as an
    /// object ON something, and a backdrop equal to the panel makes the
    /// page edge disappear.
    pub canvas_backdrop: Color32,
    /// Ordinary text.
    pub text: Color32,
    /// Secondary text: captions, hints, counts.
    pub text_muted: Color32,
    /// The single accent — selection, focus, the active tab.
    pub accent: Color32,
    /// A separator or control border.
    pub outline: Color32,
    /// Something is wrong and the operator must act.
    pub danger: Color32,
    /// Something is worth knowing and nothing is broken. Distinct from
    /// [`Self::danger`] because pdfce reports a great deal that is
    /// *disclosure* rather than fault — a hidden layer, a short signature
    /// range, a substituted font — and colouring those as errors is how
    /// an operator learns to ignore the colour that means error.
    pub notice: Color32,
    /// A selected object's fill on the canvas (translucent).
    pub selection_fill: Color32,
    /// A vector node mark: "a point is here".
    pub node_mark: Color32,
    /// The node mark's interior, so a mark reads against dark artwork.
    pub node_mark_fill: Color32,
    /// One subpath's outline — distinct from [`Self::node_mark`] because
    /// it answers a different question.
    pub subpath_outline: Color32,
    /// A committed ce dimension, selected.
    pub dimension_selected: Color32,
    /// A ce dimension being dragged, and every other *uncommitted
    /// proposal*. Distinct from [`Self::dimension_selected`] under rule
    /// 4: what pdfce is proposing must not look like what the operator
    /// has committed.
    pub preview: Color32,
    /// A snap or alignment guide — a weaker relative of
    /// [`Self::preview`], because a guide is a hint about a proposal
    /// rather than the proposal itself.
    pub guide: Color32,
    /// A measurement or annotation label's backdrop, so text stays
    /// readable over arbitrary artwork (translucent).
    pub label_backdrop: Color32,
    /// Text drawn on [`Self::label_backdrop`]. Not [`Self::text`]: the
    /// backdrop is near-opaque and light in every preset, including the
    /// dark one, because it sits over the PAGE rather than over chrome —
    /// and the page is whatever colour the document says.
    pub label_text: Color32,
    /// Form-field chrome: "a control lives here", not "this is
    /// selected".
    pub field_chrome: Color32,
}

/// Spacing and sizing, travelling with the palette (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Height of an ordinary control.
    pub control_height: f32,
    /// Gap between adjacent controls in a row.
    pub gutter: f32,
    /// Padding inside a panel.
    pub panel_padding: f32,
    /// Corner radius on buttons and panels.
    pub corner_radius: u8,
    /// Icon size in points. Read by [`crate::icons`].
    pub icon_pts: f32,
}

/// A complete look: a palette and its metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Which preset this is, for persistence and for the picker.
    pub preset: Preset,
    /// The colours.
    pub palette: Palette,
    /// The spacing and sizing.
    pub metrics: Metrics,
}

/// The shipped looks.
///
/// `#[non_exhaustive]` is deliberate: a preset added later must not be a
/// breaking change to anything matching on this, and every consumer
/// should be routing through [`Theme`] rather than branching on the
/// name anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum Preset {
    /// Muted greys, one restrained accent, tight spacing. The page
    /// dominates and the chrome recedes — the convention a document tool
    /// is measured against.
    #[default]
    Quiet,
    /// Lighter, more generous padding, softer edges, clearer grouping.
    /// Easier to scan; costs screen area.
    Airy,
    /// Dark chrome against a light page, as CAD tools do it. High
    /// contrast at the page edge and easier on a long session.
    Dark,
}

impl Preset {
    /// Every preset, for the picker and for the tests that check all of
    /// them. A preset missing here ships unverified.
    pub const ALL: &'static [Preset] = &[Preset::Quiet, Preset::Airy, Preset::Dark];

    /// The settings-file token for this preset.
    ///
    /// Stable identifiers, never the display name: a display string is
    /// operator-visible text and belongs in [`crate::ui_text`], where it
    /// can be reworded without invalidating everyone's saved settings.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Preset::Quiet => "quiet",
            Preset::Airy => "airy",
            Preset::Dark => "dark",
        }
    }

    /// Parse a settings-file token, `None` if it names no preset.
    ///
    /// `None` rather than a default so the caller can say *"the settings
    /// file asked for a theme pdfce does not have"* instead of silently
    /// resetting the operator's choice — the difference between a note
    /// and a mystery.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.key() == key)
    }
}

impl Theme {
    /// The theme for a preset.
    #[must_use]
    pub fn new(preset: Preset) -> Self {
        match preset {
            Preset::Quiet => Self::quiet(),
            Preset::Airy => Self::airy(),
            Preset::Dark => Self::dark(),
        }
    }

    /// The default look, and the one that most closely reproduces what
    /// pdfce looked like before this module existed — so an operator who
    /// never opens Settings sees a tidied version of what they had, not
    /// a different application.
    fn quiet() -> Self {
        Self {
            preset: Preset::Quiet,
            palette: Palette {
                surface: Color32::from_rgb(0xF2, 0xF2, 0xF3),
                panel: Color32::from_rgb(0xE8, 0xE8, 0xEA),
                canvas_backdrop: Color32::from_rgb(0x6E, 0x70, 0x74),
                text: Color32::from_rgb(0x1C, 0x1C, 0x1E),
                text_muted: Color32::from_rgb(0x5E, 0x60, 0x66),
                // Deliberately NOT (30, 110, 220): that is `node_mark`'s
                // historical value, and the first run of
                // `distinct_overlay_roles_stay_distinct_in_every_preset`
                // caught the collision — an accent chosen for chrome
                // happened to land exactly on an overlay role that means
                // something else. Deeper, so selection and "a point is
                // here" stay tellable apart on the same canvas.
                accent: Color32::from_rgb(0x17, 0x5C, 0xC4),
                outline: Color32::from_rgb(0xC4, 0xC6, 0xCA),
                danger: Color32::from_rgb(0xC0, 0x2A, 0x2A),
                notice: Color32::from_rgb(0xB0, 0x6A, 0x1A),
                selection_fill: Color32::from_rgba_unmultiplied(90, 140, 220, 70),
                node_mark: Color32::from_rgb(30, 110, 220),
                node_mark_fill: Color32::from_rgb(250, 250, 252),
                subpath_outline: Color32::from_rgb(210, 140, 40),
                dimension_selected: Color32::from_rgb(40, 150, 160),
                preview: Color32::from_rgb(210, 90, 40),
                guide: Color32::from_rgb(160, 90, 40),
                label_backdrop: Color32::from_rgba_unmultiplied(250, 250, 250, 220),
                label_text: Color32::from_rgb(20, 20, 20),
                field_chrome: Color32::from_rgb(150, 90, 200),
            },
            metrics: Metrics {
                control_height: 24.0,
                gutter: 4.0,
                panel_padding: 6.0,
                corner_radius: 3,
                icon_pts: 16.0,
            },
        }
    }

    /// Lighter and roomier. Same hues, more air.
    fn airy() -> Self {
        let quiet = Self::quiet();
        Self {
            preset: Preset::Airy,
            palette: Palette {
                surface: Color32::from_rgb(0xFA, 0xFA, 0xFB),
                panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                canvas_backdrop: Color32::from_rgb(0x8A, 0x8D, 0x93),
                text: Color32::from_rgb(0x24, 0x26, 0x2B),
                text_muted: Color32::from_rgb(0x6C, 0x70, 0x78),
                outline: Color32::from_rgb(0xDC, 0xDE, 0xE3),
                ..quiet.palette
            },
            metrics: Metrics {
                control_height: 28.0,
                gutter: 8.0,
                panel_padding: 12.0,
                corner_radius: 6,
                icon_pts: 17.0,
            },
        }
    }

    /// Dark chrome, light page.
    ///
    /// The overlay roles are RE-TUNED rather than inherited: the quiet
    /// preset's node mark and accent are legible on light chrome and
    /// muddy on dark, and an overlay that cannot be seen is a cue that
    /// has been removed. `label_backdrop` and `label_text` deliberately
    /// stay light-on-dark-text, because they sit over the PAGE, whose
    /// colour the document decides and the theme does not.
    fn dark() -> Self {
        let quiet = Self::quiet();
        Self {
            preset: Preset::Dark,
            palette: Palette {
                surface: Color32::from_rgb(0x24, 0x26, 0x2A),
                panel: Color32::from_rgb(0x2E, 0x31, 0x36),
                canvas_backdrop: Color32::from_rgb(0x16, 0x17, 0x1A),
                text: Color32::from_rgb(0xE6, 0xE8, 0xEC),
                text_muted: Color32::from_rgb(0x9A, 0x9E, 0xA6),
                accent: Color32::from_rgb(0x4C, 0x9A, 0xFF),
                outline: Color32::from_rgb(0x44, 0x48, 0x4F),
                danger: Color32::from_rgb(0xFF, 0x6B, 0x6B),
                notice: Color32::from_rgb(0xE0, 0xA0, 0x40),
                node_mark: Color32::from_rgb(90, 160, 255),
                subpath_outline: Color32::from_rgb(240, 175, 70),
                dimension_selected: Color32::from_rgb(70, 200, 210),
                preview: Color32::from_rgb(255, 130, 70),
                guide: Color32::from_rgb(210, 130, 70),
                field_chrome: Color32::from_rgb(190, 130, 240),
                ..quiet.palette
            },
            metrics: quiet.metrics,
        }
    }

    /// Push this theme into `egui`'s own style.
    ///
    /// # This is the hook that did not exist
    ///
    /// Before this module, the only `style_mut` call in the entire crate
    /// was a local text-wrap fix inside the ribbon. Nothing set a
    /// background, a text colour, a rounding or a spacing — so pdfce's
    /// appearance was `egui`'s defaults plus whatever each call site drew
    /// on top. Calling this once per frame is what makes the palette
    /// actually govern the widgets rather than only the overlays.
    ///
    /// Applied every frame rather than once at startup so a theme change
    /// takes effect immediately, with no restart and no cache to
    /// invalidate. It is a handful of field writes against a struct egui
    /// already owns.
    /// # `all_styles_mut`, not `set_style`
    ///
    /// egui 0.35 keeps a SEPARATE `Style` per light/dark theme and picks
    /// between them from the system setting. Writing only one of them
    /// would make pdfce's appearance depend on the operator's OS theme —
    /// so a machine set to dark would show a half-styled application
    /// while the developer's light machine looked correct, which is the
    /// worst kind of appearance bug because it is invisible where it is
    /// being written. Both styles get the same palette, and the preset
    /// alone decides whether pdfce is dark.
    pub fn apply(&self, ctx: &egui::Context) {
        let p = self.palette;
        let m = self.metrics;
        let preset = self.preset;
        ctx.all_styles_mut(move |style| Self::write_style(style, &p, &m, preset));
        // Stash the whole theme where any drawing code can reach it.
        //
        // egui's `Style` has nowhere to put the OVERLAY roles — node
        // marks, snap guides, dimension previews — because they are
        // pdfce's vocabulary, not egui's. Without this the canvas
        // painters would have to be handed a palette through every
        // signature between here and them, and the ones that were missed
        // would silently keep the default: a dark theme with light-theme
        // node marks, which is the two-thirds-of-a-theme failure this
        // module exists to prevent, and no test would see it.
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("pdfce-theme"), *self));
    }

    /// The theme in force for this frame, read back from the context.
    ///
    /// Falls back to the default if nothing has been stashed — which
    /// happens only before the first [`Theme::apply`], i.e. never during
    /// a painted frame.
    #[must_use]
    pub fn of(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp(egui::Id::new("pdfce-theme")))
            .unwrap_or_default()
    }

    /// The style write itself, shared by both of egui's per-theme styles.
    fn write_style(style: &mut egui::Style, p: &Palette, m: &Metrics, preset: Preset) {
        let v = &mut style.visuals;

        v.dark_mode = matches!(preset, Preset::Dark);
        v.override_text_color = Some(p.text);
        v.panel_fill = p.surface;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.panel;
        v.faint_bg_color = p.panel;
        v.window_stroke = egui::Stroke::new(1.0, p.outline);
        v.selection.bg_fill = p.selection_fill;
        v.selection.stroke = egui::Stroke::new(1.0, p.accent);
        v.hyperlink_color = p.accent;
        v.error_fg_color = p.danger;
        v.warn_fg_color = p.notice;

        let radius = egui::CornerRadius::same(m.corner_radius);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = radius;
            w.bg_stroke = egui::Stroke::new(1.0, p.outline);
            w.fg_stroke = egui::Stroke::new(1.0, p.text);
        }
        // Hover and active lift toward the accent rather than toward an
        // arbitrary grey, so the one accent is what the eye tracks.
        v.widgets.inactive.weak_bg_fill = p.panel;
        v.widgets.hovered.weak_bg_fill = p.surface;
        v.widgets.active.weak_bg_fill = p.accent;
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.label_backdrop);

        style.spacing.item_spacing = egui::vec2(m.gutter, m.gutter);
        style.spacing.button_padding = egui::vec2(m.gutter, m.gutter * 0.5);
        style.spacing.interact_size.y = m.control_height;
        style.spacing.window_margin = egui::Margin::same(m.panel_padding as i8);
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(Preset::default())
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "test code; the slice bound is a loop invariant and a panic names the failure"
)]
mod tests {
    use super::*;

    /// **Every preset keeps its semantically distinct overlay roles
    /// distinct.**
    ///
    /// The module docs argue that a theme may re-tune the overlay hues
    /// but must not collapse two of them: a node mark and a subpath
    /// outline answer different questions, and a preview must not look
    /// like a committed dimension (rule 4). Colour is never the only cue
    /// — R84 requires a shape or dash as well — but a merged pair removes
    /// a cue that was doing work, and nothing else would notice.
    ///
    /// This is the test that makes "the dark preset re-tunes its
    /// overlays" a checked claim rather than an intention. The dark
    /// preset inherits most of the palette with `..quiet.palette`, and
    /// that idiom is exactly how two roles quietly become one.
    #[test]
    fn distinct_overlay_roles_stay_distinct_in_every_preset() {
        for preset in Preset::ALL {
            let p = Theme::new(*preset).palette;
            let roles: [(&str, Color32); 7] = [
                ("node_mark", p.node_mark),
                ("subpath_outline", p.subpath_outline),
                ("dimension_selected", p.dimension_selected),
                ("preview", p.preview),
                ("guide", p.guide),
                ("field_chrome", p.field_chrome),
                ("accent", p.accent),
            ];
            for (i, (an, a)) in roles.iter().enumerate() {
                for (bn, b) in &roles[i + 1..] {
                    assert_ne!(
                        a, b,
                        "{preset:?}: `{an}` and `{bn}` resolve to the same colour, \
                         so a cue that distinguishes them is gone"
                    );
                }
            }
        }
    }

    /// **Text is legible on the surface it is drawn on, in every
    /// preset.**
    ///
    /// A crude relative-luminance gap rather than a full WCAG contrast
    /// ratio: the point is to catch a preset where someone set a light
    /// text colour against a light panel — which is what a `..quiet` spread
    /// does the moment a surface is darkened and the text is not — and a
    /// coarse check that always fires beats a precise one nobody runs.
    #[test]
    fn text_contrasts_with_its_background_in_every_preset() {
        fn luma(c: Color32) -> f32 {
            0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
        }
        for preset in Preset::ALL {
            let p = Theme::new(*preset).palette;
            for (name, bg) in [("surface", p.surface), ("panel", p.panel)] {
                let gap = (luma(p.text) - luma(bg)).abs();
                assert!(
                    gap > 90.0,
                    "{preset:?}: `text` on `{name}` has a luminance gap of {gap:.0}, \
                     which is not readable"
                );
            }
            let muted = (luma(p.text_muted) - luma(p.surface)).abs();
            assert!(
                muted > 45.0,
                "{preset:?}: `text_muted` on `surface` is too faint (gap {muted:.0})"
            );
        }
    }

    /// **The label backdrop stays light in every preset, including the
    /// dark one.**
    ///
    /// Labels sit over the PAGE, not over chrome, and the page is
    /// whatever colour the document says — overwhelmingly white. A dark
    /// theme that darkened the label backdrop would put dark text on a
    /// dark plate on a white page, which is unreadable in the one place
    /// it matters most.
    ///
    /// Worth a test because it is precisely the field a careless "make
    /// everything dark" edit would flip.
    #[test]
    fn label_plates_stay_page_facing_not_chrome_facing() {
        for preset in Preset::ALL {
            let p = Theme::new(*preset).palette;
            assert!(
                p.label_backdrop.r() > 200 && p.label_backdrop.b() > 200,
                "{preset:?}: the label backdrop follows the page, not the chrome"
            );
            assert!(
                p.label_text.r() < 80,
                "{preset:?}: label text must be dark, to sit on that backdrop"
            );
        }
    }

    /// **The token `pdfce-core` defaults to is one this shell knows.**
    ///
    /// `Settings::theme` is a `String` because core must never gain a
    /// GUI dependency, so nothing in the type system connects core's
    /// default `"quiet"` to [`Preset::Quiet`]. If either side is renamed
    /// the two drift, and the symptom is a fresh install showing the
    /// "this settings file asks for a theme this version does not have"
    /// note on its very first run — a message about the operator's file
    /// that is really about ours.
    ///
    /// This is the seam that check exists for: one crate's literal
    /// against another's enum, verified in the crate that owns the enum.
    #[test]
    fn the_core_default_theme_token_is_one_the_shell_knows() {
        let core_default = pdfce_core::settings::Settings::default().theme;
        assert_eq!(
            Preset::from_key(&core_default),
            Some(Preset::default()),
            "pdfce-core defaults to theme {core_default:?}, which this shell does not resolve to its own default preset"
        );
    }

    /// Settings keys round-trip, and an unknown key is `None` rather
    /// than a silent default.
    #[test]
    fn preset_keys_round_trip_and_unknown_keys_are_refused() {
        for preset in Preset::ALL {
            assert_eq!(Preset::from_key(preset.key()), Some(*preset));
        }
        assert_eq!(Preset::from_key("solarized"), None);
        assert_eq!(Preset::from_key(""), None);
    }
}
