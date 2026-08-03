//! # object_summary — the ONE description of a selectable page object
//!
//! Turns a `pdfce_core::vector::VectorObject` into a small, GUI-shaped
//! **fact record** ([`ObjectSummary`]) that every surface which has to say
//! *"what is this thing?"* reads. Implements `docs/ui_specs/pass-17-dock-and-
//! layer-tree.md` §C.6's single-source-of-truth requirement:
//!
//! > a `describe_object(obj: &VectorObject) -> ObjectSummary` … computed once
//! > and consumed by both the tree row label and the Properties tab, so a
//! > Path's fill colour is never described one way in the tree and a
//! > different way in Properties.
//!
//! Three consumers exist today and they must never disagree:
//!
//! 1. the Objects panel's row label (`PdfceApp::objects_panel`),
//! 2. the status-bar selection readout (`PdfceApp::status_bar_body`) — the
//!    one that is visible with the dock CLOSED, which is the state the
//!    operator's confusion actually happens in,
//! 3. the canvas selection overlay's per-kind treatment and type badge
//!    (`draw_selection_outlines`).
//!
//! `object_provider.rs`'s own module docs cite decision 011 on exactly this
//! failure shape ("two decompositions quietly diverge"); two *descriptions*
//! of one decomposition is the same defect one layer up, and this module is
//! the structural answer to it.
//!
//! ## Why a fact record and not a `String`
//!
//! Decision 002 R1: every user-visible string lives in `ui_text.rs`. So this
//! module deliberately holds **no prose at all** — it classifies, measures
//! and counts, and `ui_text` alone renders. That split is also what makes it
//! unit-testable without an egui frame: the tests below assert on enum
//! variants and numbers, never on wording that a copy edit would break.
//!
//! ## What it can and cannot say (an honest ceiling, ui-spec §B.3/§B.4)
//!
//! `TextObject` carries a bbox, an `approximate` flag and token/byte spans —
//! **no string, no font name, no size**. `ImageObject` carries a source kind
//! and a bbox — **no pixel dimensions, no colourspace**. So the spec's
//! illustrative `Text · "Section A-A" · Helvetica 10pt` row is not buildable,
//! and this module does not pretend otherwise: it reports what the core model
//! knows and [`ObjectNote`] discloses the gaps in words. Fabricating the
//! missing detail would break rule 4 (fuzzy, never sneaky) in the one place
//! the operator is least able to catch it.
//!
//! ## [`ObjectNote`] — the point of the whole module
//!
//! The operator's report was *"sometimes I click and get a box highlighting
//! on the screen that doesn't seem to correspond to anything."* Three causes
//! of that were hit-testing bugs and are fixed. The residue is **legibility**:
//! a selection can be entirely correct and still enclose apparently-empty
//! paper. Every such case is a *known, already-computed fact* about the
//! object, and [`describe_object`] emits one note per applicable case:
//!
//! | Note | Real cause of a "box over nothing" |
//! |---|---|
//! | [`ObjectNote::ApproximateTextBounds`] | `TextObject`'s bbox is the hull of glyph ORIGINS inflated by the largest `Tf` size — routinely wider and taller than the ink, so clicking whitespace *near* text legitimately selects it. `approximate` is always `true`, so this note is on every text object. |
//! | [`ObjectNote::PaintsNothing`] | An `n`-op path (a clip, or a discarded construction) is a real, selectable object that paints no pixels at all (`PaintStyle::is_invisible`). |
//! | [`ObjectNote::DegenerateBounds`] | A horizontal or vertical rule has a bbox of zero height or width. It is selectable and correct — and a zero-extent outline rect strokes **nothing**, so before this Pass the operator saw a click that appeared to do nothing at all. |
//! | [`ObjectNote::NoBounds`] | The object has no finite geometry, so no outline can be drawn anywhere. Rare, and previously indistinguishable from a dead click. |
//! | [`ObjectNote::FormNotDecomposed`] | A form XObject is ONE opaque object: its outline covers the whole nested drawing, and its children are not individually listed or clickable. |
//!
//! What is deliberately **not** here: a same-colour ("white on white")
//! heuristic. Whether a fill matches its background cannot be decided from
//! `PathObject`'s own fields — the backdrop may be another filled shape, an
//! image, or blank paper — and ui-spec §C.2 names that as an honest limit
//! rather than a guess to make. The readout states the object's own colour
//! verbatim instead and lets the operator draw the conclusion.

use pdfce_core::vector::{Bounds, ImageSource, PaintStyle, Rgb, VectorObject};

/// Which kind of thing a selection is.
///
/// Finer-grained than [`VectorObject`]'s three variants on purpose: the model
/// folds inline images, image XObjects and form XObjects into one
/// `VectorObject::Image`, but those are three genuinely different answers to
/// "what did I select?" — a form XObject in particular is an entire nested
/// drawing treated as one opaque object, which is itself a common cause of
/// "why is the box so big?". Collapsing them would throw away a distinction
/// the operator needs precisely when they are confused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// A path object (`re`/`m`/`l`/`c` … then a painting operator).
    Path,
    /// A `BT`…`ET` text object.
    Text,
    /// A `BI`/`ID`/`EI` inline image.
    InlineImage,
    /// A `Do` on an image XObject.
    ImageXObject,
    /// A `Do` on a form XObject — one opaque object, not recursed into.
    FormXObject,
}

/// The SHAPE an object with a zero-extent bounding box actually is.
///
/// Split out of [`ObjectNote::DegenerateBounds`] so the readout can name the
/// real thing — "a horizontal rule" reads very differently from "a vertical
/// rule", and both read very differently from "a single point".
///
/// Named after the shape rather than after which axis is zero
/// (`ZeroWidth`/`ZeroHeight`/`ZeroBoth`, the first draft) because that is how
/// the operator will describe what they are looking at, and because a
/// same-prefix variant set is a clippy `enum_variant_names` error in this
/// workspace's `-D warnings` build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Degeneracy {
    /// Zero width, non-zero height.
    VerticalRule,
    /// Zero height, non-zero width.
    HorizontalRule,
    /// Zero on both axes.
    Point,
}

/// A disclosable fact that explains a selection the operator may not be able
/// to SEE (module docs' table).
///
/// Notes are facts already known to `pdfce-core`, never inferences: each one
/// is a field read or an exact comparison. That is what makes surfacing them
/// a disclosure rather than a guess (rule 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectNote {
    /// The bounds are an approximation inflated around glyph origins, so the
    /// outline is wider and taller than the visible text.
    ApproximateTextBounds,
    /// The path paints no pixels — an `n`-op clip or discarded construction.
    PaintsNothing,
    /// The bounds have zero extent on one or both axes.
    DegenerateBounds(Degeneracy),
    /// The object has no finite geometry at all, so no outline can be drawn.
    NoBounds,
    /// A form XObject: one opaque object covering a whole nested drawing.
    FormNotDecomposed,
}

impl ObjectNote {
    /// Every note, for the tests that sweep the catalog.
    ///
    /// The same discipline `dock::DockPanel::ALL` uses, and for the same
    /// reason: a note added without an explanation sentence — or with one
    /// copy-pasted from its neighbour — would ship as a disclosure that
    /// discloses nothing, which is worse than no note at all because it looks
    /// like the app answered the question. A test sweeping this list catches
    /// that; review does not, reliably.
    #[allow(
        dead_code,
        reason = "the note catalog; swept by the string tests today, and the list any future notes-legend or filter must read rather than re-derive" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub const ALL: [Self; 7] = [
        Self::ApproximateTextBounds,
        Self::PaintsNothing,
        Self::DegenerateBounds(Degeneracy::VerticalRule),
        Self::DegenerateBounds(Degeneracy::HorizontalRule),
        Self::DegenerateBounds(Degeneracy::Point),
        Self::NoBounds,
        Self::FormNotDecomposed,
    ];
}

impl ObjectKind {
    /// Every kind, for the tests that sweep the catalog (see
    /// [`ObjectNote::ALL`] for the rationale).
    #[allow(
        dead_code,
        reason = "the kind catalog; swept by the string tests today, and the list any future kind filter or legend must read rather than re-derive" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub const ALL: [Self; 5] = [
        Self::Path,
        Self::Text,
        Self::InlineImage,
        Self::ImageXObject,
        Self::FormXObject,
    ];
}

/// Everything the GUI can honestly say about one selectable object.
///
/// Cheap to build (field reads plus one anchor count) and built on demand
/// rather than cached: the Objects panel virtualizes, so only the rows
/// actually on screen are described, and the status readout describes at most
/// one object per frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSummary {
    /// What kind of object it is.
    pub kind: ObjectKind,
    /// Paint disposition — `Some` for paths only (nothing else has one).
    pub paint: Option<PaintStyle>,
    /// The colour a viewer actually SEES, resolved by paint disposition: the
    /// fill colour for a filled path, the stroke colour for a stroke-only
    /// path, `None` for a path that paints nothing (reporting its unused,
    /// default-black fill colour there would be a confidently wrong answer).
    pub colour: Option<Rgb>,
    /// Anchor count across every subpath — paths only.
    pub nodes: Option<usize>,
    /// Stroke width in user-space units at paint time — stroked paths only.
    pub line_width: Option<f64>,
    /// The object's page-space bounding box, verbatim from the model.
    pub bounds: Bounds,
    /// Every applicable disclosure, most-explanatory first (module docs).
    pub notes: Vec<ObjectNote>,
}

impl ObjectSummary {
    /// The bbox's width and height in PDF points, or `None` if it has no
    /// finite geometry. `(0.0, h)` and `(w, 0.0)` are legitimate answers —
    /// see [`Degeneracy`].
    #[must_use]
    pub fn size(&self) -> Option<(f64, f64)> {
        if self.bounds.is_empty() {
            return None;
        }
        Some((
            self.bounds.max.x - self.bounds.min.x,
            self.bounds.max.y - self.bounds.min.y,
        ))
    }

    /// Whether the outline drawn for this object needs a dashed treatment —
    /// i.e. whether the box is a deliberate APPROXIMATION of the object's
    /// extent rather than its measured extent.
    ///
    /// Pairing "dashed" with "approximate" is the canvas's shape cue under
    /// R84 (never colour alone): a solid box claims *this is where the object
    /// is*, a dashed box claims *the object is somewhere in here*. Today only
    /// text is approximate, but this asks the QUESTION rather than testing
    /// the kind, so an exact text bbox (ui-spec §B.4 #1) turns the dashes off
    /// by itself with no second place to update.
    #[must_use]
    pub fn bounds_are_approximate(&self) -> bool {
        self.notes.contains(&ObjectNote::ApproximateTextBounds)
    }
}

/// Describe one object — **the single description path** (module docs).
///
/// Note ordering is the order the operator should read them in: the note that
/// explains *why the box looks wrong* comes before the note that explains a
/// structural property of the object. For a text object that means the
/// approximation disclosure leads; for a degenerate path, the zero-extent
/// disclosure leads over "paints nothing", because an invisible hairline is
/// more surprising than an invisible clip path.
#[must_use]
pub fn describe_object(object: &VectorObject) -> ObjectSummary {
    let bounds = object.page_bbox();
    let mut notes = Vec::new();
    if let Some(note) = degeneracy_note(bounds) {
        notes.push(note);
    }
    match object {
        VectorObject::Path(p) => {
            let nodes = p.subpaths.iter().map(|sp| sp.anchors().count()).sum();
            if p.style.is_invisible() {
                notes.push(ObjectNote::PaintsNothing);
            }
            ObjectSummary {
                kind: ObjectKind::Path,
                paint: Some(p.style),
                colour: visible_colour(p.style, p.fill_color, p.stroke_color),
                nodes: Some(nodes),
                line_width: p.style.stroke.then_some(p.line_width),
                bounds,
                notes,
            }
        }
        VectorObject::Text(t) => {
            if t.approximate {
                // Insert FIRST: for text this is the whole explanation, and a
                // degenerate text bbox (possible for an empty `BT`/`ET`) is
                // the lesser fact.
                notes.insert(0, ObjectNote::ApproximateTextBounds);
            }
            ObjectSummary {
                kind: ObjectKind::Text,
                paint: None,
                colour: None,
                nodes: None,
                line_width: None,
                bounds,
                notes,
            }
        }
        VectorObject::Image(i) => {
            let kind = match i.source {
                ImageSource::Inline => ObjectKind::InlineImage,
                ImageSource::XObject => ObjectKind::ImageXObject,
                ImageSource::Form => ObjectKind::FormXObject,
            };
            if kind == ObjectKind::FormXObject {
                notes.push(ObjectNote::FormNotDecomposed);
            }
            ObjectSummary {
                kind,
                paint: None,
                colour: None,
                nodes: None,
                line_width: None,
                bounds,
                notes,
            }
        }
    }
}

/// The colour a viewer actually sees for a path, per its paint disposition.
///
/// A stroke-only path never shows its fill colour, and an `n`-op path shows
/// neither — so reporting `fill_color` unconditionally would print a colour
/// that appears nowhere on the page. This is the same resolution the Objects
/// panel's row label already used; centralising it here is what stops the row
/// and the readout from drifting apart.
fn visible_colour(style: PaintStyle, fill: Rgb, stroke: Rgb) -> Option<Rgb> {
    if style.fill.is_some() {
        Some(fill)
    } else if style.stroke {
        Some(stroke)
    } else {
        None
    }
}

/// Classify a bounding box's degeneracy, if any.
///
/// Exact comparison against zero rather than an epsilon, deliberately: the
/// case this exists for is a bbox whose two corners are *literally the same
/// number* (a `100 200 m 300 200 l S` rule, or a `re` with a zero operand),
/// which is what makes the outline rect strokable-but-invisible. A hairline
/// that is 0.01 pt tall does render an outline, so widening this to an
/// epsilon would start disclosing "zero height" about objects that are not.
fn degeneracy_note(bounds: Bounds) -> Option<ObjectNote> {
    if bounds.is_empty() {
        return Some(ObjectNote::NoBounds);
    }
    let zero_w = bounds.max.x - bounds.min.x == 0.0;
    let zero_h = bounds.max.y - bounds.min.y == 0.0;
    match (zero_w, zero_h) {
        (true, true) => Some(ObjectNote::DegenerateBounds(Degeneracy::Point)),
        (true, false) => Some(ObjectNote::DegenerateBounds(Degeneracy::VerticalRule)),
        (false, true) => Some(ObjectNote::DegenerateBounds(Degeneracy::HorizontalRule)),
        (false, false) => None,
    }
}

/// How many of each kind a multi-object selection contains.
///
/// The multi-select readout's whole job is orientation, not detail (ui-spec
/// §C.6): "3 objects selected (2 paths, 1 text)" tells the operator whether
/// their marquee caught what they meant, which a per-object dump would bury.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectionCensus {
    /// Total objects counted.
    pub total: usize,
    /// Path objects.
    pub paths: usize,
    /// Text objects.
    pub texts: usize,
    /// Inline images and image XObjects, together — the distinction matters
    /// when describing ONE object and is noise in a census.
    pub images: usize,
    /// Form XObjects.
    pub forms: usize,
}

/// Tally a selection's kinds.
///
/// Takes kinds rather than objects so the caller can feed it whatever it
/// already has (a `filter_map` over a selection set that may contain stale
/// targets, most usefully) without this function needing to know how a
/// `TargetId` resolves.
#[must_use]
pub fn census(kinds: impl IntoIterator<Item = ObjectKind>) -> SelectionCensus {
    let mut c = SelectionCensus::default();
    for kind in kinds {
        c.total += 1;
        match kind {
            ObjectKind::Path => c.paths += 1,
            ObjectKind::Text => c.texts += 1,
            ObjectKind::InlineImage | ObjectKind::ImageXObject => c.images += 1,
            ObjectKind::FormXObject => c.forms += 1,
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfce_core::content::ContentStream;
    use pdfce_core::vector::{Matrix, NoXObjects, decompose};

    /// Decompose a content stream and describe every object in paint order —
    /// the seam these tests share, so each case is a content-stream literal
    /// plus an assertion on the record.
    fn describe_all(src: &[u8]) -> Vec<ObjectSummary> {
        let cs = ContentStream::parse(src.to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        objects.objects.iter().map(describe_object).collect()
    }

    fn only(src: &[u8]) -> ObjectSummary {
        let mut all = describe_all(src);
        assert_eq!(all.len(), 1, "{all:?}"); // ui-text-exempt: test assertion payload, never displayed
        all.remove(0)
    }

    #[test]
    fn a_filled_path_reports_its_fill_colour_and_node_count() {
        let s = only(b"0 0 1 rg 10 10 80 80 re f");
        assert_eq!(s.kind, ObjectKind::Path);
        assert_eq!(s.nodes, Some(4));
        assert_eq!(s.colour.map(|c| c.b), Some(1.0));
        // Not stroked: no line width is reported, because none is used.
        assert_eq!(s.line_width, None);
        assert!(s.notes.is_empty(), "{:?}", s.notes); // ui-text-exempt: test assertion payload
        assert_eq!(s.size(), Some((80.0, 80.0)));
    }

    /// A stroke-only path must report the STROKE colour: its fill colour is
    /// never painted, so printing it would name a colour that is nowhere on
    /// the page.
    #[test]
    fn a_stroked_path_reports_its_stroke_colour_and_line_width() {
        let s = only(b"1 0 0 RG 2 w 10 10 m 90 90 l S");
        assert_eq!(s.kind, ObjectKind::Path);
        assert_eq!(s.colour.map(|c| c.r), Some(1.0));
        assert_eq!(s.line_width, Some(2.0));
    }

    /// The `n`-op case — a real, selectable object that paints no pixels.
    /// This is one of the two headline "box over nothing" explanations.
    #[test]
    fn a_no_paint_path_reports_that_it_paints_nothing_and_no_colour() {
        let s = only(b"10 10 80 80 re n");
        assert_eq!(s.kind, ObjectKind::Path);
        assert_eq!(s.colour, None);
        assert!(
            s.notes.contains(&ObjectNote::PaintsNothing),
            "{:?}",
            s.notes
        ); // ui-text-exempt: test assertion payload
    }

    /// The other headline case, and the one the operator most likely hit:
    /// a text object is ALWAYS approximate, so its outline routinely covers
    /// whitespace around and above the glyphs.
    #[test]
    fn a_text_object_always_discloses_its_approximate_bounds() {
        let s = only(b"BT /F1 12 Tf 40 40 Td (Hi) Tj ET");
        assert_eq!(s.kind, ObjectKind::Text);
        assert_eq!(s.notes.first(), Some(&ObjectNote::ApproximateTextBounds));
        assert!(s.bounds_are_approximate());
        // Nothing is fabricated for text: no string, no font, no colour.
        assert_eq!(s.colour, None);
        assert_eq!(s.nodes, None);
        assert_eq!(s.paint, None);
    }

    /// The bug found while observing: a horizontal rule selects correctly and
    /// its outline rect has zero height, so it strokes nothing at all. The
    /// note is what lets the overlay inflate it and the readout explain it.
    #[test]
    fn a_zero_height_path_is_disclosed_as_degenerate() {
        let s = only(b"100 200 m 300 200 l S");
        assert_eq!(s.size(), Some((200.0, 0.0)));
        assert!(
            s.notes
                .contains(&ObjectNote::DegenerateBounds(Degeneracy::HorizontalRule)),
            "{:?}", // ui-text-exempt: test assertion payload
            s.notes
        );
    }

    #[test]
    fn a_zero_width_path_is_disclosed_as_degenerate() {
        let s = only(b"200 100 m 200 300 l S");
        assert_eq!(s.size(), Some((0.0, 200.0)));
        assert!(
            s.notes
                .contains(&ObjectNote::DegenerateBounds(Degeneracy::VerticalRule)),
            "{:?}", // ui-text-exempt: test assertion payload
            s.notes
        );
    }

    /// A single-point path — degenerate on both axes at once.
    #[test]
    fn a_point_path_is_disclosed_as_degenerate_on_both_axes() {
        let s = only(b"150 150 m 150 150 l S");
        assert_eq!(s.size(), Some((0.0, 0.0)));
        assert!(
            s.notes
                .contains(&ObjectNote::DegenerateBounds(Degeneracy::Point)),
            "{:?}", // ui-text-exempt: test assertion payload
            s.notes
        );
    }

    /// An inline image is a distinct answer from an image XObject, and both
    /// are distinct from a form XObject — see [`ObjectKind`]'s own rationale.
    #[test]
    fn an_inline_image_is_reported_as_inline() {
        let s = only(b"q 100 0 0 50 10 10 cm BI /W 1 /H 1 /CS /G /BPC 8 ID \x00 EI Q");
        assert_eq!(s.kind, ObjectKind::InlineImage);
        assert_eq!(s.size(), Some((100.0, 50.0)));
        // Honest ceiling: no pixel dimensions exist in the model (§B.4 #2).
        assert!(s.notes.is_empty(), "{:?}", s.notes); // ui-text-exempt: test assertion payload
    }

    /// The census is what the multi-select readout is built from.
    #[test]
    fn the_census_tallies_each_kind() {
        let c = census([
            ObjectKind::Path,
            ObjectKind::Path,
            ObjectKind::Text,
            ObjectKind::InlineImage,
            ObjectKind::ImageXObject,
            ObjectKind::FormXObject,
        ]);
        assert_eq!(c.total, 6);
        assert_eq!(c.paths, 2);
        assert_eq!(c.texts, 1);
        // Inline and XObject images are one bucket in a census.
        assert_eq!(c.images, 2);
        assert_eq!(c.forms, 1);
        assert_eq!(census([]), SelectionCensus::default());
    }

    /// An empty bbox yields `NoBounds` and a `None` size — the case where no
    /// outline can be drawn anywhere, which must be disclosed rather than
    /// looking like a dead click.
    #[test]
    fn an_object_with_no_finite_geometry_reports_no_bounds() {
        assert_eq!(
            degeneracy_note(Bounds::EMPTY),
            Some(ObjectNote::NoBounds),
            "{:?}", // ui-text-exempt: test assertion payload
            Bounds::EMPTY
        );
    }
}
