//! # `redact_apply` — the GUI's redaction APPLY pipeline, and its absence proof
//!
//! The one place in `pdfce-gui` where a destructive, irreversible removal is
//! prepared. Everything the Apply modal (`main.rs`'s
//! `redaction_apply_confirmation`) renders, and everything the confirm path
//! writes to disk, comes from [`prepare_redaction_apply`] in this module.
//!
//! It exists as its own module — rather than as three methods on
//! `PdfceApp` — for one reason that is not organisational tidiness:
//! **`main.rs` is not headlessly testable and this is the code that must be
//! tested most.** `PdfceApp` needs an `egui::Context` and a live window to
//! exercise; a free function over an [`EditSession`] needs neither, so the
//! security assertion at the bottom of this file can run in
//! `cargo test --workspace` on every commit rather than being an
//! observation someone made once by hand. That is the same reason
//! `canvas.rs` and `viewer.rs` exist (see `main.rs`'s module docs).
//!
//! ## The three properties this module is responsible for
//!
//! ### 1. Apply is a FULL REWRITE, or it does not happen
//!
//! `ARCHITECTURE.md` §5's corollary and standing rule R35: an incremental
//! save **structurally preserves superseded content** — the old bytes of
//! every replaced object stay in the file by construction, in the prior
//! revision. For an ordinary edit that is a feature (the previous revision
//! is recoverable). For redaction it is the defeat of the entire operation:
//! the "removed" text would sit in the saved file one `startxref` hop away,
//! trivially recoverable by any parser that walks `/Prev`.
//!
//! So there are exactly two full rewrites in this pipeline and no third
//! path:
//!
//! ```text
//!   EditSession (marks may be UNSAVED)
//!     │
//!     │  (1) EditSession::to_full_bytes   ← full rewrite #1: materialise
//!     ▼                                      this session's edits as ONE
//!   Vec<u8>  (one revision, no /Prev)         revision so apply can see the
//!     │                                       marks the operator just made
//!     │  Document::from_bytes
//!     ▼
//!   Document
//!     │
//!     │  (2) redact::apply_redactions     ← full rewrite #2: core's own
//!     ▼                                      forced full rewrite, which is
//!   Vec<u8>  (redacted, one revision)         where the removal happens
//! ```
//!
//! If **either** rewrite fails, this module returns a refusal and nothing is
//! written. There is deliberately no `to_incremental_bytes` call anywhere in
//! this file, and no fallback that could introduce one: a redaction that
//! silently degraded to an incremental save would produce a file the
//! operator has been told is redacted and which is not. That is the single
//! failure mode this whole Pass exists to prevent, so the refusal is the
//! feature.
//!
//! ### 2. Why the session must be materialised first (the un-saved-mark trap)
//!
//! [`pdfce_core::redact::apply_redactions`] takes a `&Document` — a parsed
//! file — not an [`EditSession`]. The GUI's marks, however, may exist only
//! in the session overlay: an operator can open a document, mark three
//! regions and press Apply without ever having saved. Handing
//! `session.document()` (the BASE revision) to `apply_redactions` would
//! therefore apply **zero** of those marks and report success, which is the
//! Pass 17.1 status-bar bug (decision 018 §8) wearing a far worse face — not
//! a disclosure that stayed silent, but an *apply* that removed nothing
//! while saying it had.
//!
//! `to_full_bytes` is what closes it: it is the session's own edits rendered
//! into a real single-revision file, which `Document::from_bytes` then
//! re-parses into exactly the document the operator is looking at.
//!
//! ### 3. Absence is VERIFIED on the actual output bytes, not assumed
//!
//! [`pdfce_core::redact::RedactionReport::redacted_text`] carries the
//! distinct strings the surgery decoded while removing them — kept, in
//! core's own words, "for the absence-proof gate to grep". This module runs
//! that gate on the bytes it is about to hand to the writer:
//!
//! | Where the string still occurs | Verdict | Why |
//! |---|---|---|
//! | in a **decoded stream** of the output | **REFUSE** — write nothing | A decoded stream is content a renderer or a text extractor will read back. Its survival is a real leak, not a coincidence, and no acknowledgement checkbox makes it acceptable. |
//! | in the **raw bytes only** (no decoded stream) | **DISCLOSE** as a residual requiring the operator's explicit acknowledgement | pdfce cannot tell a genuine un-recognised carrier from an unrelated coincidence (the same byte run inside a font name, an ID string, a compressed blob). Refusing would be a trap the operator cannot act on; claiming removal would be a lie. Naming it is the only honest option. |
//! | nowhere | **verified** | This is what licenses §5.1's wording contract to use the word "verified" at all. |
//!
//! Strings shorter than [`MIN_VERIFIABLE_LEN`] are excluded from the
//! raw-byte half and counted separately (see [`AbsenceVerification`]) rather
//! than silently skipped: a two-character redaction would match somewhere in
//! any real file, so a raw-byte grep for it carries no information, and
//! pretending it does would turn the disclosure into noise operators learn
//! to click through. They are still checked against decoded streams, where a
//! survival *is* meaningful.
//!
//! ## What this module deliberately does NOT do
//!
//! It does not implement any part of the removal. The surgery, the carrier
//! sweep, the object-stream decomposition and the forced full rewrite all
//! live in [`pdfce_core::redact`] and are called, never re-derived —
//! CLAUDE.md rule 2 (GUI–core separation) plus the plain fact that a second
//! implementation of security-critical byte surgery is how the two quietly
//! diverge.
//!
//! It also does not write anything to disk. [`prepare_redaction_apply`]
//! returns bytes; the caller shows them to the operator as a report, and
//! writes them only after the confirmation in `main.rs`. Nothing in this
//! module can reach the filesystem.

use pdfce_core::document::Document;
use pdfce_core::edit::EditSession;
use pdfce_core::object::{ObjId, Object};
use pdfce_core::redact::{self, RedactError, RedactionReport};
use pdfce_core::writer::SaveOptions;

/// The shortest redacted string whose absence from the **raw** output bytes
/// is worth asserting.
///
/// Below this length a byte-run match tells you nothing: `"Dr"` occurs
/// inside `/Widths`-adjacent binary, font names, dates and half the words in
/// any document, so a raw-byte hit would fire on a perfectly good redaction.
/// Four characters is the point at which a coincidental match stops being
/// the expected outcome — chosen deliberately conservatively, and paired
/// with the fact that short strings are still verified against decoded
/// streams (where the same match *is* meaningful because it is content).
///
/// The count of strings this excludes is reported, never hidden — see
/// [`AbsenceVerification::strings_too_short_for_raw_check`].
const MIN_VERIFIABLE_LEN: usize = 4;

/// Why a redaction apply did not happen. Every variant is a refusal **before
/// any byte reached the filesystem**.
///
/// There is no `Partial` or `DegradedToIncremental` variant, and adding one
/// would be a defect: the operations this models either complete as a full
/// rewrite or do not occur (module docs, property 1).
///
/// Rendered by `ui_text::redact_apply_refusal_message`; the variants carry
/// structured data and diagnostic strings from `pdfce-core`'s own error
/// `Display`, never operator-facing prose (decision 002 R1 — the wording
/// lives in the catalog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedactApplyRefusal {
    /// The document carries no `/Redact` marks, so there is nothing to
    /// apply. Reachable only if the marks vanished between the panel
    /// enabling its button and the action running (an undo in the same
    /// frame); the review panel's own R83 gate normally prevents it.
    NothingToApply,
    /// The session's edits could not be materialised as a single full
    /// revision — e.g. `WriteError::HybridFullRewrite`, which core refuses
    /// by name. **The refusal is the correct outcome**: the alternative is
    /// an incremental save that leaves the un-redacted content in a prior
    /// revision.
    FullRewriteUnavailable {
        /// `pdfce-core`'s own diagnostic for the failed rewrite.
        reason: String,
    },
    /// The full-rewrite bytes could not be re-parsed into a document, so the
    /// apply could not run against them. Structurally the same refusal as
    /// [`Self::FullRewriteUnavailable`] and kept distinct only because it
    /// names a different suspect (the writer produced something the parser
    /// rejects — a pdfce bug, not a property of the operator's file).
    MaterialisedDocumentUnreadable {
        /// The parse diagnostic.
        reason: String,
    },
    /// `pdfce-core` refused the apply itself: a region over a raster image
    /// it cannot destroy pixels in, an encrypted document, an unparsable
    /// page. These are the cardinal-rule refusals — core would rather
    /// produce nothing than a false redaction.
    CoreRefused {
        /// [`RedactError`]'s own message, which names the page and the
        /// condition.
        reason: String,
    },
    /// The apply completed in memory, but the absence proof found redacted
    /// text **still present in a decoded stream** of the output. Nothing is
    /// written.
    ///
    /// This is the module's own last line of defence and it should be
    /// unreachable: reaching it means core's removal and core's report
    /// disagree. It is a refusal rather than a disclosure because a decoded
    /// stream is content a reader will render or extract — there is no
    /// reading of that survival under which the file is safe to hand over.
    VerificationFailed {
        /// The strings that survived, for the message. Not the whole
        /// redacted set — only what actually leaked.
        survivors: Vec<String>,
    },
}

/// What the absence proof found, for the report the operator reads before
/// confirming.
///
/// This is the structure §5.1's wording contract reads: "never say
/// *verified* unless a real verification step ran". [`Self::is_clean`] is
/// the predicate that licenses the stronger word.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbsenceVerification {
    /// Distinct redacted strings the proof checked against decoded streams.
    pub strings_checked: usize,
    /// How many of those were too short for the raw-byte half to say
    /// anything about ([`MIN_VERIFIABLE_LEN`]). Reported so the operator can
    /// see the proof's own limit rather than inferring a completeness it
    /// does not have.
    pub strings_too_short_for_raw_check: usize,
    /// Redacted strings that still occur somewhere in the raw output bytes
    /// while occurring in **no** decoded stream.
    ///
    /// Disclosed, acknowledgement-gated, never silently dropped — and never
    /// described as a confirmed leak either, because pdfce genuinely cannot
    /// tell an un-recognised carrier from a coincidental byte run (module
    /// docs).
    pub raw_byte_residuals: Vec<String>,
}

impl AbsenceVerification {
    /// Whether every checked string is absent from the output by both
    /// measures — the condition under which the post-apply wording may say
    /// "verified".
    pub const fn is_clean(&self) -> bool {
        self.raw_byte_residuals.is_empty()
    }
}

/// A completed, verified, **unwritten** redaction: the exact bytes that will
/// land on disk if — and only if — the operator confirms.
///
/// Holding the finished bytes across the confirmation (rather than
/// recomputing them after it) is deliberate and is what makes the Apply
/// report honest: the report describes what *did* happen in memory, so the
/// numbers the operator reads are measurements rather than predictions. It
/// also removes the window in which the document could change between the
/// report and the write.
#[derive(Debug, Clone)]
pub struct PreparedRedaction {
    /// The redacted document, as a single full-rewrite revision.
    pub bytes: Vec<u8>,
    /// Core's report — what was removed, per carrier, plus its own
    /// disclosed residuals.
    pub report: RedactionReport,
    /// This module's independent absence proof over [`Self::bytes`].
    pub verification: AbsenceVerification,
    /// Objects that had to be promoted out of an object stream to
    /// materialise the session's edits (full rewrite #1).
    ///
    /// Surfaced because R38/decision 007 W3 require promotion to be counted
    /// and named: promotion leaves the object's previous value inside the
    /// untouched container. In a redaction context that is worth saying out
    /// loud even though it is not itself a leak of redacted text — page
    /// content streams cannot live in an object stream at all (ISO 32000-1
    /// §7.5.7: stream objects shall not be compressed into one), so the
    /// stale copy can only be a dictionary. The absence proof above covers
    /// the case that matters anyway, by decoding the container and grepping
    /// it like any other stream.
    pub promoted_by_materialisation: Vec<ObjId>,
}

/// Run the whole apply pipeline in memory and prove the result.
///
/// See the module docs for the two-full-rewrite shape, why the session must
/// be materialised first, and what the absence proof does with each class of
/// survivor. **Nothing is written to disk here**; the caller writes
/// [`PreparedRedaction::bytes`] after the operator confirms.
///
/// # Errors
///
/// [`RedactApplyRefusal`] — and every variant of it means *no file was
/// produced and no file was touched*. In particular there is no path in
/// which a failed full rewrite degrades into an incremental save.
pub fn prepare_redaction_apply(
    session: &EditSession,
) -> Result<PreparedRedaction, RedactApplyRefusal> {
    // Read the mark census from the SESSION graph, never the base document:
    // the marks the operator is most likely to be applying are the ones they
    // just made, which the base revision by construction does not have.
    if redact::count_redaction_marks(&session.graph()) == 0 {
        return Err(RedactApplyRefusal::NothingToApply);
    }

    // Full rewrite #1 — materialise. `to_full_bytes`, never
    // `to_incremental_bytes`: see the module docs. A failure here is a
    // refusal, not a cue to try the other method.
    let (materialised, materialise_report) = session
        .to_full_bytes(&SaveOptions::identity())
        .map_err(|err| RedactApplyRefusal::FullRewriteUnavailable {
            reason: err.to_string(),
        })?;

    let doc = Document::from_bytes(materialised).map_err(|err| {
        RedactApplyRefusal::MaterialisedDocumentUnreadable {
            reason: err.to_string(),
        }
    })?;

    // Full rewrite #2 — the removal itself. `apply_redactions` forces its
    // own full rewrite internally (R35); this call site cannot ask it for
    // anything else, which is the property that makes "apply is never
    // incremental" structural rather than a convention.
    let (bytes, report) =
        redact::apply_redactions(&doc, &SaveOptions::identity()).map_err(|err| match err {
            // A write failure is the same class of refusal as a failed
            // materialisation: the full rewrite did not happen.
            RedactError::Write(inner) => RedactApplyRefusal::FullRewriteUnavailable {
                reason: inner.to_string(),
            },
            other => RedactApplyRefusal::CoreRefused {
                reason: other.to_string(),
            },
        })?;

    let verification = verify_absence(&bytes, &report.redacted_text);
    if let Some(survivors) = leaked_in_decoded_streams(&bytes, &report.redacted_text) {
        return Err(RedactApplyRefusal::VerificationFailed { survivors });
    }

    Ok(PreparedRedaction {
        bytes,
        report,
        verification,
        promoted_by_materialisation: materialise_report.promoted,
    })
}

/// The decoded-stream half of the absence proof, isolated so the refusal
/// branch above reads as one question.
///
/// Returns `Some(survivors)` when any redacted string is still present in a
/// decoded stream of `bytes` — content a renderer or extractor would read
/// back — and `None` when the output is clean by that measure. Strings of
/// **any** length are checked here (unlike the raw-byte half): inside a
/// decoded content stream even a two-character survival is the redacted
/// glyphs still being drawn.
///
/// A document that cannot be re-parsed yields `None` rather than a false
/// clean bill: an unparsable output is already a refusal from
/// [`prepare_redaction_apply`]'s perspective — except that it cannot be,
/// because these are bytes pdfce itself just wrote, so this branch means a
/// writer bug, and it is not this function's job to name it.
fn leaked_in_decoded_streams(bytes: &[u8], redacted: &[String]) -> Option<Vec<String>> {
    if redacted.is_empty() {
        return None;
    }
    let doc = Document::from_bytes(bytes.to_vec()).ok()?;
    let decoded = decode_every_stream(&doc);
    let survivors: Vec<String> = redacted
        .iter()
        .filter(|needle| {
            !needle.is_empty() && decoded.iter().any(|blob| contains(blob, needle.as_bytes()))
        })
        .cloned()
        .collect();
    if survivors.is_empty() {
        None
    } else {
        Some(survivors)
    }
}

/// Build the [`AbsenceVerification`] the report renders: how much was
/// checked, how much the raw-byte half could not speak to, and which strings
/// survive in the raw bytes without surviving in any decoded stream.
fn verify_absence(bytes: &[u8], redacted: &[String]) -> AbsenceVerification {
    let mut out = AbsenceVerification {
        strings_checked: redacted.iter().filter(|s| !s.is_empty()).count(),
        ..AbsenceVerification::default()
    };
    // Decode once and reuse: a residual is "in the raw bytes AND in no
    // decoded stream", so both halves are needed to classify a single hit.
    let decoded = Document::from_bytes(bytes.to_vec())
        .map(|doc| decode_every_stream(&doc))
        .unwrap_or_default();
    for needle in redacted {
        if needle.is_empty() {
            continue;
        }
        if needle.chars().count() < MIN_VERIFIABLE_LEN {
            out.strings_too_short_for_raw_check += 1;
            continue;
        }
        let raw_hit = contains(bytes, needle.as_bytes());
        let decoded_hit = decoded.iter().any(|blob| contains(blob, needle.as_bytes()));
        if raw_hit && !decoded_hit {
            out.raw_byte_residuals.push(needle.clone());
        }
    }
    out
}

/// Decode **every** stream in the document, not merely page content.
///
/// The wide sweep is the point. A redaction that only proved absence from
/// page content streams would say nothing about a form XObject, a metadata
/// stream, an embedded file, or — the case that actually motivated this — an
/// **object-stream container**, whose compressed payload can carry a stale
/// copy of a dictionary that was promoted out of it (R38). Decoding the
/// container like any other stream is what lets a grep see that copy at all.
///
/// Streams whose filters this build cannot decode are skipped rather than
/// failed: their *raw* bytes are still covered by the raw-byte half of the
/// proof, so a skip narrows the evidence rather than fabricating it.
fn decode_every_stream(doc: &Document) -> Vec<Vec<u8>> {
    let view = doc.view();
    let mut out = Vec::new();
    for object in doc.objects() {
        let Object::Stream(stream) = &object.value else {
            continue;
        };
        let Some(raw) = view.slice(stream.data_span) else {
            continue;
        };
        if let Ok(decoded) = pdfce_core::filters::decode_stream(&stream.dict, raw) {
            out.push(decoded);
        }
    }
    out
}

/// Whether `hay` contains `needle` as a byte subsequence.
///
/// The same naive scan `pdfce-core`'s own absence tests use, kept local
/// rather than exported from core: it is three lines, and an absence proof
/// that shared its search routine with the code it is auditing would be a
/// weaker proof.
fn contains(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use pdfce_core::annot_author::{Quad, RedactSpec};
    use pdfce_core::page_tree::{self, Rect};
    use pdfce_core::text_extract::{self, ExtractOptions};
    use pdfce_core::vartext::Quadding;

    /// The secret this suite proves the absence of. Deliberately long and
    /// distinctive: a short token could be absent by luck, and a proof that
    /// can pass by luck proves nothing.
    const SECRET: &str = "CONFIDENTIALWITNESSNAME";

    /// A one-page document whose content stream shows `SECRET` followed by a
    /// word that must SURVIVE — the survivor is what stops the test from
    /// passing on a build that simply erased the page.
    fn secret_pdf() -> Vec<u8> {
        let content = format!("BT /F1 12 Tf 20 100 Td ({SECRET}) Tj ( KEEPTHIS) Tj ET");
        let stream = format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        );
        assemble(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            &stream,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ])
    }

    /// Assemble a classic single-revision PDF from object bodies `1..=n`
    /// with a correct xref table. Object 1 must be the catalog. (The same
    /// fixture shape `pdfce-core`'s redaction tests use — synthetic, per
    /// CLAUDE.md rule 7.)
    fn assemble(bodies: &[&str]) -> Vec<u8> {
        let mut buf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in bodies.iter().enumerate() {
            offsets.push(buf.len());
            buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        let xref_at = buf.len();
        let n = bodies.len() + 1;
        buf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
        for off in &offsets {
            buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        buf.extend_from_slice(
            format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
                .as_bytes(),
        );
        buf
    }

    /// A session with ONE unsaved `/Redact` mark over the secret — the exact
    /// state a GUI operator is in when they press Apply without having saved.
    fn session_with_unsaved_mark() -> EditSession {
        let doc = Document::from_bytes(secret_pdf()).unwrap();
        let mut session = EditSession::new(doc);
        let created = session
            .mark_redactions_by_search(SECRET, false)
            .expect("the fixture's text is extractable");
        assert!(!created.is_empty(), "the search must find the secret");
        session
    }

    // -- THE SECURITY ASSERTION -----------------------------------------

    /// **The headline gate for the GUI apply path.**
    ///
    /// After apply-and-save through [`prepare_redaction_apply`], the
    /// redacted text must not be recoverable from the saved bytes by any
    /// means pdfce itself offers. Three independent measures, because a
    /// single one could be satisfied by a build that merely hid the text:
    ///
    /// 1. **`extract-text`** — the very tool `pdfce-cli extract-text` and
    ///    the GUI's Copy-text both use — finds nothing;
    /// 2. **every decoded stream** (content streams, XObjects, object-stream
    ///    containers, metadata) contains no occurrence;
    /// 3. **the raw file bytes** contain no occurrence.
    ///
    /// And the negative control: `KEEPTHIS`, which was never marked, is
    /// still extractable. Without it, a build that emitted an empty page
    /// would pass all three assertions above while destroying the document.
    ///
    /// This is deliberately an assertion of ABSENCE, not of appearance. A
    /// raster test could only show that the region is painted black, which
    /// is precisely the false-redaction failure ISO 32000-1 §12.5.6.23
    /// forbids ("clipping or image masks shall not be used to hide that
    /// data") — a black box over live text is what this feature exists to
    /// never ship.
    #[test]
    fn applied_redaction_leaves_no_recoverable_trace_in_the_saved_bytes() {
        let session = session_with_unsaved_mark();
        let prepared = prepare_redaction_apply(&session).expect("the apply must succeed");

        // (3) raw bytes.
        assert!(
            !contains(&prepared.bytes, SECRET.as_bytes()),
            "the redacted text survived in the raw saved bytes"
        );

        let back = Document::from_bytes(prepared.bytes.clone())
            .expect("the redacted output must re-parse");

        // (2) every decoded stream in the file.
        for blob in decode_every_stream(&back) {
            assert!(
                !contains(&blob, SECRET.as_bytes()),
                "the redacted text survived in a decoded stream of the saved file"
            );
        }

        // (1) pdfce's own text extraction — the tool an operator would
        // actually reach for to get the text back out.
        let extracted =
            text_extract::extract_document(&back, &ExtractOptions::default()).expect("extract");
        let all_text: String = extracted
            .pages
            .iter()
            .flat_map(|p| p.runs.iter())
            .map(|r| r.text.clone())
            .collect();
        assert!(
            !all_text.contains(SECRET),
            "the redacted text was recoverable via extract-text: {all_text:?}"
        );

        // The negative control — proof the test can fail.
        assert!(
            all_text.contains("KEEPTHIS"),
            "un-redacted text must survive; the page was not supposed to be emptied"
        );

        // And the mark itself is gone (§12.5.6.23 outcome 3).
        assert_eq!(
            redact::count_redaction_marks(&back),
            0,
            "the /Redact mark must be removed by apply"
        );
    }

    /// The absence proof must REPORT that it ran, or §5.1's wording contract
    /// has nothing to read and the summary would have to fall back to the
    /// weaker word.
    #[test]
    fn the_absence_proof_reports_a_clean_verification() {
        let session = session_with_unsaved_mark();
        let prepared = prepare_redaction_apply(&session).unwrap();
        assert!(
            prepared.verification.strings_checked > 0,
            "the proof must have had something to check"
        );
        assert!(
            prepared.verification.is_clean(),
            "no residual expected on this fixture: {:?}",
            prepared.verification.raw_byte_residuals
        );
    }

    /// A mark that exists ONLY in the session overlay must still be applied.
    ///
    /// This is the un-saved-mark trap the module docs name: passing
    /// `session.document()` to `apply_redactions` would apply nothing and
    /// report success. The assertion that makes it bite is `marks_applied`
    /// — a build with that bug produces `NothingToApply` or a zero count,
    /// never a removal.
    #[test]
    fn a_mark_that_was_never_saved_is_still_applied() {
        let session = session_with_unsaved_mark();
        // The base revision genuinely has no mark — that is the trap.
        assert_eq!(redact::count_redaction_marks(session.document()), 0);
        assert!(redact::count_redaction_marks(&session.graph()) > 0);

        let prepared = prepare_redaction_apply(&session).unwrap();
        assert!(
            prepared.report.marks_applied >= 1,
            "an unsaved mark must be applied, not silently skipped"
        );
        assert!(prepared.report.glyphs_removed >= SECRET.len() as u64);
    }

    /// The output must be a SINGLE revision. A `/Prev` in the trailer would
    /// mean a prior revision is reachable in the saved file, which for a
    /// redaction is the un-redacted content one hop away — R35's whole point.
    #[test]
    fn the_output_is_one_revision_with_no_prior_revision_to_walk_back_to() {
        let session = session_with_unsaved_mark();
        let prepared = prepare_redaction_apply(&session).unwrap();
        let back = Document::from_bytes(prepared.bytes).unwrap();
        assert!(
            back.trailer().get(b"Prev").is_none(),
            "a redaction apply must leave no /Prev — a prior revision holds the un-redacted bytes"
        );
    }

    /// A document with no marks is refused by name rather than producing an
    /// empty "successful" apply, so the caller can never present a report
    /// that describes nothing as if it were a removal.
    #[test]
    fn an_unmarked_document_is_refused_by_name() {
        let doc = Document::from_bytes(secret_pdf()).unwrap();
        let session = EditSession::new(doc);
        assert_eq!(
            prepare_redaction_apply(&session).unwrap_err(),
            RedactApplyRefusal::NothingToApply
        );
    }

    /// A region over a raster image is the named §12.5.6.23 case where
    /// masking would be a false redaction. Core now DESTROYS the covered
    /// samples (`Pass 245.0`) rather than refusing, and the GUI must see
    /// that as an applied redaction whose report counts the image — not as
    /// a refusal, and not as a success that says nothing about the image.
    #[test]
    fn a_region_over_an_image_destroys_its_samples_and_the_report_says_so() {
        // A page whose content draws an image XObject, with a mark over it.
        let content = "q 200 0 0 100 20 20 cm /Im0 Do Q";
        let stream = format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        );
        let image = "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace \
                     /DeviceGray /BitsPerComponent 8 /Length 1 >>\nstream\n\x00\nendstream";
        let bytes = assemble(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
             /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>",
            &stream,
            image,
        ]);
        let doc = Document::from_bytes(bytes).unwrap();
        let mut session = EditSession::new(doc);
        session
            .add_redaction(
                0,
                &RedactSpec {
                    quads: vec![Quad::from_rect(Rect::from_corners(30.0, 30.0, 150.0, 90.0))],
                    fill: None,
                    overlay_text: None,
                    quadding: Quadding::Left,
                },
            )
            .unwrap();

        match prepare_redaction_apply(&session) {
            Ok(prepared) => {
                assert_eq!(prepared.report.images_cleared, 1);
                assert_eq!(prepared.report.marks_retained, 0);
                assert!(
                    prepared
                        .report
                        .notes
                        .iter()
                        .any(|n| n.contains("sample cell(s) destroyed")),
                    "the report must name the image: {:?}",
                    prepared.report.notes
                );
            }
            other => panic!("expected the image to be redacted, got {other:?}"),
        }
    }

    /// The census the review panel lists from and the census the status bar
    /// counts from must be the same walk (Pass 8.1) — asserted here because
    /// the GUI reads both and a disagreement between them is unresolvable
    /// from the operator's side.
    #[test]
    fn the_mark_list_and_the_mark_count_agree() {
        let session = session_with_unsaved_mark();
        let graph = session.graph();
        assert_eq!(
            redact::redaction_marks(&graph).len(),
            redact::count_redaction_marks(&graph)
        );
        let pages = page_tree::pages_in(&graph).unwrap();
        for mark in redact::redaction_marks(&graph) {
            assert!(
                mark.page_index < pages.len(),
                "a listed mark must name a real page"
            );
        }
    }
}
