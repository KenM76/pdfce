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
//! ## What it can and cannot say (ui-spec §B.3/§B.4)
//!
//! §B.4's two binding core asks are now met, so the spec's illustrative rows
//! ARE buildable: `pdfce_core::vector::TextObject` carries a decoded
//! [`TextPreview`] and a [`TextFont`], and `ImageObject` carries
//! `pixel_size`. This module surfaces them — and surfaces their absence just
//! as loudly, because the interesting cases are the ones where a value is
//! missing:
//!
//! | Core says | This module reports | Why not something friendlier |
//! |---|---|---|
//! | `TextPreview::Decoded { lossy: false, .. }` | [`ObjectSummary::text`] = the string | — |
//! | `TextPreview::Decoded { lossy: true, .. }` | the string **plus** [`ObjectNote::TextPartlyUndecodable`] | The `�`s in the row are real; a note is what turns them from "pdfce is broken" into "this font's encoding is incomplete". |
//! | `TextPreview::Undecodable` | `text = None` **plus** [`ObjectNote::TextUndecodable`] | A row of replacement characters looks like a defect. The honest answer is *"this text cannot be read, here is why"*. |
//! | `TextPreview::Unavailable` | `text = None`, no note | Nothing was attempted (no font resolver — the headless/unit-test path). The GUI always resolves fonts, so an operator never sees this state; disclosing it would be noise about a code path they are not on. |
//! | `TextPreview::Empty` | `text = None`, no note | The object really does show nothing. |
//! | `font: None` | [`ObjectSummary::font`] = `None` | No `Tf` was in effect. Never invented. |
//! | `pixel_size: None` | [`ObjectSummary::pixels`] = `None` | A form XObject has no samples; a malformed image's `/Width`/`/Height` are unusable. Deriving a number from the bbox would state a resolution the file does not have. |
//!
//! The one thing still not said is a text object's **exact** extent. The
//! bbox is now laid out from the font's own metrics — per-code advances
//! from `/Widths`/`/W`/the standard-14 AFM tables, height from
//! `/FontDescriptor` — so it is where a conforming reader puts the run, but
//! it is not measured glyph ink, and for a font with no usable metrics it
//! falls back to the old origin hull. [`ObjectNote::ApproximateTextBounds`]
//! is on every text object and carries which of the four constructions
//! produced this one.
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
//! | [`ObjectNote::ApproximateTextBounds`] | `TextObject`'s bbox is never measured glyph ink, so it can enclose paper the operator can see is empty (a font's designed ascent sits above most lowercase letters) and, in the `EmBox` fallback, can miss visible glyphs entirely. `approximate` is always `true`, so this note is on every text object; its payload says which construction produced the box, and therefore which of four sentences explains it. |
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

use pdfce_core::vector::{
    Bounds, ImageSource, PaintStyle, PathPaint, Rgb, TextBoundsBasis, TextFont, TextPreview,
    VectorObject,
};

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
    /// The bounds of a text object are an approximation — of the kind the
    /// payload names.
    ///
    /// Parameterized rather than split into four sibling variants for the
    /// same reason [`ObjectNote::DegenerateBounds`] carries a
    /// [`Degeneracy`]: every consumer that asks *"is this box
    /// approximate?"* wants one answer for all four, and every consumer
    /// that EXPLAINS the box needs to know which one — so the question and
    /// the detail must not be two separate notes that could disagree.
    ///
    /// ui-spec §E.3 makes carrying the reason binding: shipping a better
    /// box with one sentence describing all four cases would leave some
    /// text objects disclosed by a sentence that no longer matches the box
    /// actually drawn, which is a regression in honesty rather than the
    /// improvement the geometry work was for.
    ApproximateTextBounds(TextBoundsBasis),
    /// The path paints no pixels — an `n`-op clip or discarded construction.
    PaintsNothing,
    /// The bounds have zero extent on one or both axes.
    DegenerateBounds(Degeneracy),
    /// The object has no finite geometry at all, so no outline can be drawn.
    NoBounds,
    /// A form XObject: one opaque object covering a whole nested drawing.
    FormNotDecomposed,
    /// Not one character of the object's text could be recovered: every
    /// character code reached ISO 32000-1 §9.10.2's failure clause.
    ///
    /// A *document* fact, not a pdfce limitation — the clause itself
    /// concedes that for such a font "there is no way to determine what the
    /// character code represents". Disclosed rather than shown as `���`,
    /// which would read as a defect in the reader.
    TextUndecodable,
    /// Some — not all — of the object's characters could not be recovered,
    /// so the shown string contains U+FFFD replacements.
    ///
    /// Distinct from [`ObjectNote::TextUndecodable`] because the operator's
    /// question is different: here there IS a readable string and the
    /// question is why part of it is `�`.
    TextPartlyUndecodable,
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
    pub const ALL: [Self; 12] = [
        Self::ApproximateTextBounds(TextBoundsBasis::FontMetrics),
        Self::ApproximateTextBounds(TextBoundsBasis::MetricAdvancesNominalHeight),
        Self::ApproximateTextBounds(TextBoundsBasis::EstimatedAdvances),
        Self::ApproximateTextBounds(TextBoundsBasis::EmBox),
        Self::PaintsNothing,
        Self::DegenerateBounds(Degeneracy::VerticalRule),
        Self::DegenerateBounds(Degeneracy::HorizontalRule),
        Self::DegenerateBounds(Degeneracy::Point),
        Self::NoBounds,
        Self::FormNotDecomposed,
        Self::TextUndecodable,
        Self::TextPartlyUndecodable,
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
    /// The object's decoded text preview — text objects only, and only when
    /// something was actually recovered (module docs' table).
    ///
    /// Already capped at `pdfce_core::vector::MAX_TEXT_PREVIEW_CHARS` by the
    /// decomposition, which is the MEMORY bound; a display applies its own,
    /// shorter, LINE-LENGTH bound on top (see `ui_text::object_detail`).
    /// Two caps because they answer two different questions, and collapsing
    /// them would tie a row's width to the object model's storage budget.
    pub text: Option<String>,
    /// Whether [`Self::text`] is a prefix of a longer string. A display
    /// marks the elision rather than presenting a prefix as the whole.
    pub text_truncated: bool,
    /// The font in effect at the object's first show operator — text
    /// objects only, `None` when no `Tf` was in effect.
    pub font: Option<TextFont>,
    /// The image's `(width, height)` in **samples** — image objects with a
    /// usable `/Width`+`/Height` only (ISO 32000-1 §8.9.5, Table 89).
    ///
    /// A sample count, not a size on the page: an image occupies the unit
    /// square under the CTM, so [`Self::bounds`] is where it is and this is
    /// what it is made of. Both are shown, and the pair is what lets an
    /// operator judge effective resolution.
    pub pixels: Option<(u32, u32)>,
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
    /// the kind, so an exact text bbox turns the dashes off by itself with
    /// no second place to update.
    ///
    /// **All four [`TextBoundsBasis`] cases stay dashed**, including
    /// [`TextBoundsBasis::FontMetrics`] (ui-spec §E.2): a metrics-derived
    /// box is where a conforming reader lays the run out, but it is still
    /// not measured ink — accented capitals exceed `/Ascent` by that
    /// entry's own definition and italic overhang leans past the advance.
    /// The dashes' claim narrowed; it did not become false.
    #[must_use]
    pub fn bounds_are_approximate(&self) -> bool {
        self.notes
            .iter()
            .any(|n| matches!(n, ObjectNote::ApproximateTextBounds(_)))
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
                colour: visible_colour(p.style, &p.fill_paint, &p.stroke_paint),
                nodes: Some(nodes),
                line_width: p.style.stroke.then_some(p.line_width),
                text: None,
                text_truncated: false,
                font: None,
                pixels: None,
                bounds,
                notes,
            }
        }
        VectorObject::Text(t) => {
            // The decode disclosures come BEFORE the approximation one is
            // inserted at the head, so the final order is: approximation
            // first (it explains the box, which is what the operator is
            // looking at), then why the string reads as it does.
            if let Some(note) = decode_note(&t.preview) {
                notes.push(note);
            }
            if t.approximate {
                // Insert FIRST: for text this is the whole explanation, and a
                // degenerate text bbox (possible for an empty `BT`/`ET`) is
                // the lesser fact. The basis travels with the note so the
                // sentence shown always describes the box actually drawn
                // (ui-spec §E.3).
                notes.insert(0, ObjectNote::ApproximateTextBounds(t.bounds_basis));
            }
            let (text, text_truncated) = match &t.preview {
                // An all-U+FFFD string is withheld: `ObjectNote::
                // TextUndecodable` says the same thing in words, and a row
                // of replacement characters reads as a pdfce defect rather
                // than as a property of the file (module docs' table).
                TextPreview::Decoded {
                    text, truncated, ..
                } => (Some(text.clone()), *truncated),
                TextPreview::Undecodable | TextPreview::Unavailable | TextPreview::Empty => {
                    (None, false)
                }
            };
            ObjectSummary {
                kind: ObjectKind::Text,
                paint: None,
                colour: None,
                nodes: None,
                line_width: None,
                text,
                text_truncated,
                font: t.font.clone(),
                pixels: None,
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
                text: None,
                text_truncated: false,
                font: None,
                pixels: i.pixel_size,
                bounds,
                notes,
            }
        }
    }
}

/// The disclosure, if any, a text preview's decoding outcome earns.
///
/// `Unavailable` earns none on purpose: it means no font resolver was
/// supplied, which only happens on the headless/unit-test path
/// (`decompose` rather than `decompose_page`). The GUI always resolves
/// fonts, so a note about it would be a sentence describing a code path the
/// operator is never on — noise, and noise in a disclosure surface teaches
/// people to stop reading it. `Empty` earns none because "this text object
/// shows nothing" is not a failure to explain.
fn decode_note(preview: &TextPreview) -> Option<ObjectNote> {
    match preview {
        TextPreview::Undecodable => Some(ObjectNote::TextUndecodable),
        TextPreview::Decoded { lossy: true, .. } => Some(ObjectNote::TextPartlyUndecodable),
        TextPreview::Decoded { lossy: false, .. }
        | TextPreview::Unavailable
        | TextPreview::Empty => None,
    }
}

/// The colour a viewer actually sees for a path, per its paint disposition.
///
/// A stroke-only path never shows its fill colour, and an `n`-op path shows
/// neither — so reporting `fill_color` unconditionally would print a colour
/// that appears nowhere on the page. This is the same resolution the Objects
/// panel's row label already used; centralising it here is what stops the row
/// and the readout from drifting apart.
fn visible_colour(style: PaintStyle, fill: &PathPaint, stroke: &PathPaint) -> Option<Rgb> {
    // ★ TAKES `PathPaint`, NOT `Rgb`, AND THAT IS THE WHOLE FIX HERE.
    //
    // This function's LOGIC was always right — it correctly declines to report
    // a fill colour for a stroke-only path. Its INPUT was wrong: the
    // decomposer had no arm for `cs`/`scn`, so a path painted in a
    // `/Separation`, `/DeviceN`, `/ICCBased`, `/Indexed` or `/Lab` space
    // carried a stale colour from an unrelated earlier object, and this panel
    // printed it as a confident hex — `stroked #1A73E8` — about an ink that is
    // not that colour.
    //
    // `PathPaint::rgb()` returns `None` for a space pdfce does not decode, so
    // the row now shows no swatch rather than a wrong one. On a print job that
    // difference is a plate.
    if style.fill.is_some() {
        fill.rgb()
    } else if style.stroke {
        stroke.rgb()
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

    /// Describe every object on a FIXTURE's first page, through
    /// `decompose_page` — i.e. with real font and XObject resolvers, which
    /// is the path the GUI is actually on.
    ///
    /// [`describe_all`] above uses the resolver-free `decompose`, which is
    /// the right seam for the geometry cases (no file needed) but reports
    /// `TextPreview::Unavailable` for every text object by construction. The
    /// text-preview and pixel-size cases can only be honest against a real
    /// document, so they use this.
    fn describe_fixture(rel: &str) -> Vec<ObjectSummary> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic")
            .join(rel);
        let doc = pdfce_core::document::Document::load(&path).expect("the fixture loads");
        let pages = pdfce_core::page_tree::pages(&doc).expect("a page tree");
        let model = pdfce_core::vector::decompose_page(&doc.view(), &pages[0], Matrix::IDENTITY)
            .expect("the page decomposes");
        model.objects.iter().map(describe_object).collect()
    }

    /// §B.4 #1, end to end: a text object now carries the string it shows
    /// and the typeface that shows it, decoded through `text_extract`'s
    /// §9.10.2 ladder rather than a second decoder written here.
    #[test]
    fn a_text_object_reports_its_string_and_its_font() {
        let objects = describe_fixture("text/simple-winansi.pdf");
        let text = objects
            .iter()
            .find(|s| s.kind == ObjectKind::Text)
            .expect("the fixture has a text object");
        // SOURCED characters only, verbatim: the fixture's `TJ` opens the
        // gap between "Hello" and "world" with a -2000 kerning offset and NO
        // space glyph, and its second line is a `Td` with no line marker —
        // §14.8.2.5 S3/S5, neither of which the file states. `text_extract`
        // DERIVES both for `plain_text` and omits both for `sourced_text`;
        // a preview is the latter, because a row label is not the place to
        // present a reader's guess as the document's content.
        assert_eq!(text.text.as_deref(), Some("HelloworldSecond line"));
        assert!(!text.text_truncated);
        let font = text.font.as_ref().expect("a font was in effect");
        assert_eq!(font.base_font.as_deref(), Some("Helvetica"));
        assert_eq!(font.size, 24.0);
        // A decodable string earns no decode disclosure — only the
        // ever-present approximate-bounds one. The fixture's Helvetica is a
        // standard-14 face, so its widths and its ascent/descent are both
        // real metrics and the basis is the good one.
        assert_eq!(
            text.notes,
            vec![ObjectNote::ApproximateTextBounds(
                TextBoundsBasis::FontMetrics
            )]
        );
    }

    /// The honest-failure case: a font whose encoding defeats decoding
    /// yields NO string and a note saying why — never a row of `�`, which
    /// would read as a pdfce defect rather than as a property of the file.
    #[test]
    fn text_that_cannot_be_decoded_is_disclosed_not_mojibake() {
        let objects = describe_fixture("text/identity-h-no-tounicode.pdf");
        let text = objects
            .iter()
            .find(|s| s.kind == ObjectKind::Text)
            .expect("the fixture has a text object");
        assert_eq!(text.text, None, "no string may be fabricated or mangled");
        assert!(
            text.notes.contains(&ObjectNote::TextUndecodable),
            "{:?}", // ui-text-exempt: test assertion payload
            text.notes
        );
        // The FONT is still named: knowing which font cannot be read is
        // most of the value of the disclosure.
        assert!(text.font.is_some());
    }

    /// §B.4 #2: an image reports its sample count from `/Width`/`/Height`
    /// (§8.9.5 Table 89) — and a form XObject reports none, because a form
    /// has no samples.
    #[test]
    fn an_image_reports_its_pixel_dimensions() {
        let objects = describe_fixture("vector/mixed.pdf");
        let image = objects
            .iter()
            .find(|s| s.kind == ObjectKind::ImageXObject)
            .expect("the fixture has an image XObject");
        // The fixture's image is 2×2 DeviceGray (its PROVENANCE entry).
        assert_eq!(image.pixels, Some((2, 2)));
        // The sample count is NOT the size on the page: the image is placed
        // by the CTM, so the two numbers differ and both are reported.
        assert_ne!(
            image.size().map(|(w, h)| (w as u32, h as u32)),
            image.pixels,
            "a 2x2 image placed at 2x2 pt would make this test prove nothing"
        );
    }

    /// Nothing is invented for the kinds that carry no such fact: a path has
    /// no string or pixel size, and a text object with no `Tf` has no font.
    #[test]
    fn no_kind_gains_a_detail_it_does_not_have() {
        let path = only(b"0 0 1 rg 10 10 80 80 re f");
        assert_eq!(path.text, None);
        assert_eq!(path.font, None);
        assert_eq!(path.pixels, None);

        // A show operator with no preceding `Tf`: an object, but no font to
        // name and therefore none named.
        let text = only(b"BT 40 40 Td (Hi) Tj ET");
        assert_eq!(text.kind, ObjectKind::Text);
        assert_eq!(text.font, None);
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
    /// a text object is ALWAYS approximate, so its outline covers
    /// whitespace around and above the glyphs.
    ///
    /// `only` decomposes with no document behind it, so no font resolves and
    /// the basis is the em-box fallback — which is exactly the state whose
    /// disclosure has to be the blunt one.
    #[test]
    fn a_text_object_always_discloses_its_approximate_bounds() {
        let s = only(b"BT /F1 12 Tf 40 40 Td (Hi) Tj ET");
        assert_eq!(s.kind, ObjectKind::Text);
        assert_eq!(
            s.notes.first(),
            Some(&ObjectNote::ApproximateTextBounds(TextBoundsBasis::EmBox))
        );
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
