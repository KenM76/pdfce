//! OCR text layers — the "sandwich" (ISO 32000-1 §9.3.6, Table 106 mode 3).
//!
//! # What this module is, and what it deliberately is not
//!
//! This is the half of OCR that is **engine-independent**: taking words with
//! page positions and writing them into a PDF as an invisible, selectable text
//! layer over content that is left completely untouched.
//!
//! It contains **no recogniser**. [`OcrEngine`] is a trait, and the engine
//! that implements it is a separate, feature-gated decision with real licence
//! consequences (see `docs/ocr-engine-survey.md`). Splitting it this way is
//! not tidiness — the text-layer authoring is identical whichever engine wins,
//! so building it first means the engine choice can be made on its merits
//! instead of under pressure from work already committed to one API.
//!
//! # The sandwich, concretely
//!
//! A scanned page is one big image and no text. OCR reads the image and
//! produces words with bounding boxes. Those words are then drawn ON TOP of
//! the image in text rendering mode **3** — Table 106's *"neither fill nor
//! stroke text (invisible)"*.
//!
//! The result: the page looks EXACTLY as it did, because nothing visible was
//! added and nothing existing was altered, while Find, copy, and text
//! extraction all work. That is the behaviour `PRIOR_ART.md` cites OCRmyPDF
//! for, and the spec corpus names mode 3 as the mechanism by name.
//!
//! # Why the original content is never touched
//!
//! An OCR layer is **additive**. It appends a second content stream; it does
//! not rewrite, re-encode or re-compress the scan. Two reasons, and the second
//! is the one that matters:
//!
//! 1. Round-trip/minimal-diff (project rule 3) — an object pdfce did not
//!    logically modify is re-emitted byte-identical or omitted entirely.
//! 2. **Re-encoding a scan loses evidence.** A scanned document is often the
//!    record of something — a signed contract, a survey, a drawing. Running
//!    its JPEG through a decode/re-encode cycle to "help" costs generation
//!    loss on an image the operator may need to defend the provenance of. OCR
//!    is supposed to make a document findable, not modify it.
//!
//! # Rule 4: OCR is an inference, and a large one
//!
//! Every word here is a GUESS. Project rule 4 requires that what pdfce
//! inferred is visible before it becomes document state, and that where an
//! inference is *inherently* uncertain the uncertainty is stated rather than
//! implied.
//!
//! So [`RecognizedWord::confidence`] is `Option<f32>`, and the `None` case is
//! load-bearing rather than a convenience: some engines expose no confidence
//! at all. A shell must say *"this engine reports no per-word confidence"*
//! rather than silently presenting unscored guesses as though they had been
//! checked. An absent score and a high score must never look the same.

/// Where OCR model files come from — operator-supplied or beside the binary,
/// never downloaded. See the module's own docs for why a downloader was
/// proposed and withdrawn.
pub mod models;

/// The sandwich writer — recognised words become an invisible, selectable
/// text layer over page content that is left byte-identical.
pub mod layer;

use crate::page_tree::Rect;

/// One recognised word, positioned in PDF default user space.
///
/// # Why a WORD and not a line or a character
///
/// The unit has to match what a reader searches and selects. Per-character
/// boxes make the extractor's job harder for no gain (it would have to
/// re-derive word boundaries pdfce already knew); per-line boxes make
/// selection coarse and make a search hit highlight a whole line.
///
/// Words also match what every candidate engine actually emits, so nothing is
/// re-derived from something else's output.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognizedWord {
    /// The recognised text.
    pub text: String,
    /// Where it sits on the page, PDF default user space, y-up.
    ///
    /// Engines almost universally report y-DOWN image pixels, so the
    /// conversion is the caller's job and is deliberately not hidden here —
    /// a silent flip is the single most common way an OCR layer ends up
    /// mirrored, and it is invisible until someone selects text and finds it
    /// lands on the wrong line.
    pub rect: Rect,
    /// The engine's confidence, `0.0..=1.0`, or `None` when the engine does
    /// not report one.
    ///
    /// `None` is NOT "assume it is fine". See the module docs: a shell must
    /// disclose the absence rather than let unscored output pass as checked.
    pub confidence: Option<f32>,
}

/// Everything recognised on one page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OcrPage {
    /// The words, in reading order as the engine reported it.
    ///
    /// Order matters: it becomes the order of the `Tj` operators, and
    /// therefore the order text extraction returns. An engine that reports
    /// nonsense order produces a searchable page whose copied text is
    /// scrambled — worth checking per engine rather than assuming.
    pub words: Vec<RecognizedWord>,
    /// Whether the engine reported ANY confidence values at all.
    ///
    /// Kept at page level as well as per word so a shell can make the rule-4
    /// disclosure ONCE ("this engine reports no confidence") instead of
    /// per word, and can tell "the engine has no confidence support" apart
    /// from "the engine had nothing to say about this particular word".
    pub confidence_available: bool,
}

impl OcrPage {
    /// The mean confidence over words that reported one, or `None`.
    ///
    /// Provided so a shell can lead with a single honest number. Deliberately
    /// skips unscored words rather than treating them as zero or as one —
    /// both would be inventing data, and in opposite directions.
    #[must_use]
    pub fn mean_confidence(&self) -> Option<f32> {
        let scored: Vec<f32> = self.words.iter().filter_map(|w| w.confidence).collect();
        if scored.is_empty() {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        Some(scored.iter().sum::<f32>() / scored.len() as f32)
    }

    /// Words whose confidence is below `threshold`, for review.
    ///
    /// Words with NO confidence are **included**, because an unscored word is
    /// exactly as unverified as a low-scored one — excluding them would let
    /// an engine that reports nothing produce an empty "needs review" list and
    /// look better than one that reports honestly.
    #[must_use]
    pub fn words_needing_review(&self, threshold: f32) -> Vec<&RecognizedWord> {
        self.words
            .iter()
            .filter(|w| w.confidence.is_none_or(|c| c < threshold))
            .collect()
    }
}

/// A text recogniser.
///
/// Implemented outside this module by whichever engine is chosen, behind its
/// own Cargo feature. The trait is deliberately tiny: everything about PDF —
/// the content stream, the font, the rendering mode, the resources — belongs
/// on this side of the boundary, and an engine that had to know any of it
/// would be doing pdfce's job.
pub trait OcrEngine {
    /// The error this engine reports.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Recognise text in an 8-bit greyscale image.
    ///
    /// `width`/`height` are pixels; `pixels` is row-major, top-down, one byte
    /// per pixel — the layout every candidate engine takes, so no conversion
    /// is imposed on the implementor.
    ///
    /// The returned rectangles are in **image pixel coordinates, y-down**.
    /// Converting them to PDF user space is [`words_to_page_space`]'s job,
    /// which keeps the flip in exactly one place instead of in every engine.
    ///
    /// # Errors
    ///
    /// Whatever the engine reports — a model that failed to load, an image it
    /// refuses, a recogniser failure.
    fn recognize(
        &self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Vec<RecognizedWord>, Self::Error>;

    /// Whether this engine reports per-word confidence.
    ///
    /// A required method with no default, deliberately. A default of `true`
    /// would make an engine that forgot to implement it claim scores it does
    /// not have; a default of `false` would let one that HAS them silently
    /// under-report. Making it explicit costs one line and removes both.
    fn reports_confidence(&self) -> bool;
}

/// Convert engine output from image pixels (y-down) to PDF user space (y-up).
///
/// # Why this is a free function and not done inside the engine
///
/// The y-flip is the most common OCR-layer defect there is: get it wrong and
/// every word lands mirrored vertically, the page still looks perfect, and
/// nobody notices until someone selects a line and gets a different one. Doing
/// it once, here, means an engine implementor cannot get it wrong and every
/// engine is wrong or right together.
///
/// `page_rect` is the region of the page the image covers, in user space —
/// normally the full crop box for a scanned page, but not necessarily, which
/// is why it is a parameter rather than assumed.
#[must_use]
pub fn words_to_page_space(
    words: &[RecognizedWord],
    image_width: u32,
    image_height: u32,
    page_rect: Rect,
) -> Vec<RecognizedWord> {
    if image_width == 0 || image_height == 0 {
        return Vec::new();
    }
    let sx = (page_rect.urx - page_rect.llx) / f64::from(image_width);
    let sy = (page_rect.ury - page_rect.lly) / f64::from(image_height);
    words
        .iter()
        .map(|w| {
            // The flip: an image row 0 is the TOP, a PDF y of ury is the top.
            let top = page_rect.ury - w.rect.lly * sy;
            let bottom = page_rect.ury - w.rect.ury * sy;
            RecognizedWord {
                text: w.text.clone(),
                rect: Rect::from_corners(
                    w.rect.llx.mul_add(sx, page_rect.llx),
                    bottom.min(top),
                    w.rect.urx.mul_add(sx, page_rect.llx),
                    bottom.max(top),
                ),
                confidence: w.confidence,
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn word(
        text: &str,
        llx: f64,
        lly: f64,
        urx: f64,
        ury: f64,
        conf: Option<f32>,
    ) -> RecognizedWord {
        RecognizedWord {
            text: text.to_string(),
            rect: Rect::from_corners(llx, lly, urx, ury),
            confidence: conf,
        }
    }

    /// ★ The y-flip, which is the defect this module exists to make impossible.
    ///
    /// A word at the TOP of the image (small y, because image rows count down)
    /// must land at the TOP of the page (large y, because PDF counts up). Get
    /// this backwards and the page still looks perfect while every selection
    /// lands on the wrong line — the failure nobody sees until they try to use
    /// it.
    #[test]
    fn a_word_at_the_top_of_the_image_lands_at_the_top_of_the_page() {
        let page = Rect::from_corners(0.0, 0.0, 612.0, 792.0);
        // Image 1224x1584 (i.e. 2x the page), word in the top 10% of rows.
        let top_word = word("HEADING", 0.0, 0.0, 100.0, 158.0, None);
        let out = words_to_page_space(&[top_word], 1224, 1584, page);
        assert_eq!(out.len(), 1);
        assert!(
            out[0].rect.ury > 700.0,
            "a word in the image's top rows must be near the page TOP (y~792), \
             got lly={} ury={}",
            out[0].rect.lly,
            out[0].rect.ury
        );
    }

    /// And the bottom of the image lands at the bottom of the page.
    ///
    /// Both directions, because a transform that negated without offsetting
    /// would pass the top test alone.
    #[test]
    fn a_word_at_the_bottom_of_the_image_lands_at_the_bottom_of_the_page() {
        let page = Rect::from_corners(0.0, 0.0, 612.0, 792.0);
        let bottom_word = word("FOOTER", 0.0, 1426.0, 100.0, 1584.0, None);
        let out = words_to_page_space(&[bottom_word], 1224, 1584, page);
        assert!(
            out[0].rect.lly < 90.0,
            "a word in the image's bottom rows must be near the page BOTTOM, \
             got lly={} ury={}",
            out[0].rect.lly,
            out[0].rect.ury
        );
    }

    /// The converted rect is always normalised, whichever way the flip runs.
    #[test]
    fn the_converted_rect_is_never_inverted() {
        let page = Rect::from_corners(0.0, 0.0, 612.0, 792.0);
        let out = words_to_page_space(&[word("x", 10.0, 20.0, 60.0, 90.0, None)], 612, 792, page);
        assert!(out[0].rect.lly < out[0].rect.ury, "lly must be below ury");
        assert!(out[0].rect.llx < out[0].rect.urx, "llx must be left of urx");
    }

    /// An unscored word counts as needing review, exactly like a low-scored one.
    ///
    /// The alternative — skipping unscored words — would let an engine that
    /// reports no confidence produce an EMPTY needs-review list and appear
    /// more trustworthy than one that reports honestly. That is precisely
    /// backwards, and it is the kind of thing that reads as a feature.
    #[test]
    fn an_unscored_word_still_needs_review() {
        let page = OcrPage {
            words: vec![
                word("certain", 0.0, 0.0, 1.0, 1.0, Some(0.99)),
                word("doubtful", 0.0, 0.0, 1.0, 1.0, Some(0.40)),
                word("unscored", 0.0, 0.0, 1.0, 1.0, None),
            ],
            confidence_available: true,
        };
        let review = page.words_needing_review(0.8);
        let texts: Vec<&str> = review.iter().map(|w| w.text.as_str()).collect();
        assert!(texts.contains(&"doubtful"), "a low score needs review");
        assert!(
            texts.contains(&"unscored"),
            "an UNSCORED word is exactly as unverified as a low-scored one"
        );
        assert!(!texts.contains(&"certain"));
    }

    /// The mean ignores unscored words rather than inventing a value for them.
    #[test]
    fn the_mean_confidence_skips_unscored_words() {
        let page = OcrPage {
            words: vec![
                word("a", 0.0, 0.0, 1.0, 1.0, Some(0.6)),
                word("b", 0.0, 0.0, 1.0, 1.0, Some(0.8)),
                word("c", 0.0, 0.0, 1.0, 1.0, None),
            ],
            confidence_available: true,
        };
        let mean = page.mean_confidence().expect("two words are scored");
        assert!(
            (mean - 0.7).abs() < 1e-6,
            "expected the mean of the SCORED words (0.7), got {mean}"
        );
    }

    /// An engine that reports nothing yields `None`, not zero.
    ///
    /// Zero would render as "0% confident" — a specific, alarming, and false
    /// claim about text that was never scored either way.
    #[test]
    fn no_confidence_anywhere_is_none_not_zero() {
        let page = OcrPage {
            words: vec![word("a", 0.0, 0.0, 1.0, 1.0, None)],
            confidence_available: false,
        };
        assert_eq!(page.mean_confidence(), None);
    }

    /// A degenerate image size yields nothing rather than dividing by zero.
    #[test]
    fn a_zero_sized_image_yields_no_words() {
        let page = Rect::from_corners(0.0, 0.0, 612.0, 792.0);
        assert!(
            words_to_page_space(&[word("x", 0.0, 0.0, 1.0, 1.0, None)], 0, 100, page).is_empty()
        );
        assert!(
            words_to_page_space(&[word("x", 0.0, 0.0, 1.0, 1.0, None)], 100, 0, page).is_empty()
        );
    }
}
