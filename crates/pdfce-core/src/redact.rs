//! # Redaction — true content removal (ISO 32000-1 §12.5.6.23)
//!
//! The **one deliberately destructive** subsystem in pdfce, and the one
//! where *correctness IS security*. Every other operation honours the
//! §5 minimal-diff / round-trip invariant; redaction is the single,
//! explicitly-named exception (R35, `ARCHITECTURE.md` §5 corollary):
//! applying a redaction must **truly remove** the covered content, not
//! visually mask it.
//!
//! ## The cardinal rule, above everything
//!
//! **pdfce must NEVER claim content is redacted when it is not.**
//! Under-redaction that is *disclosed* or *refused* is acceptable;
//! silent under-redaction is a catastrophic failure. Every carrier this
//! module cannot fully scrub in a given build is reported to the
//! operator as an un-redacted residual ([`RedactionReport`]), never
//! silently left. This is "fuzzy, never sneaky" (rule 4) at maximum
//! force.
//!
//! ## The spec frame — outcome-bound, method-deferred
//!
//! §12.5.6.23 is explicit that content removal "is application-specific"
//! and specifies **no removal algorithm**. What it *does* impose are four
//! `shall`-strength OUTCOME constraints, which are pdfce's acceptance
//! test rather than an algorithm to copy:
//!
//! 1. remove **all traces** of the specified content, plus the /Redact
//!    annotation itself;
//! 2. image data **shall be destroyed** in-region — "clipping or image
//!    masks shall not be used to hide that data";
//! 3. remove the /Redact annotations after applying;
//! 4. be **diligent** about all content that can exist — XFA and XMP
//!    named explicitly.
//!
//! The removal MECHANICS are assembled in the spec RAG's derived
//! consolidator `iso32000__ref__redaction_removal.md`; this module is
//! their enactment.
//!
//! ## What this cut does, and what it discloses instead of doing
//!
//! | Concern | This build |
//! |---|---|
//! | Text glyphs in-region | **removed** — advance-preserving content-stream surgery (§3 below) |
//! | Surviving text on the same line | **kept in place** — the removed run is replaced by an equivalent `TJ` advance |
//! | Object streams holding a removed/edited object | **decomposed** (§7.5.7 Strategy B — promote survivors, drop container) so no removed byte survives compressed |
//! | Overlapping annotations (& their /AP/Contents/RC) | **removed** (the stricter Acrobat-parity reading) |
//! | `/Info` and XMP strings duplicating redacted text | **scrubbed** (the redacted characters are known — the interpreter decodes them while removing them) |
//! | Prior incremental revisions | **dropped** — apply forces a FULL REWRITE (R35), never incremental |
//! | Images intersecting a region | **REFUSED, by name** — pdfce does not yet destroy image pixels, and a clip/overlay would be a false redaction (§12.5.6.23) |
//! | Form-XObject content in-region | **disclosed** — not surgically redacted this cut (verify manually) |
//! | XFA / file attachments / structure-tree ActualText / thumbnails | **detected + disclosed** (not asserted-absent) |
//!
//! ## §3 — the advance-preservation hazard, stated once
//!
//! Deleting a show operator mid-line shifts every subsequent
//! advance-relative glyph LEFT by the removed run's advance (§9.4.4:
//! painting a glyph advances `Tm` by `tx = ((w0)·Tfs + Tc + Tw)·Th`).
//! A naive deletion therefore *moves the survivors* — a correctness
//! failure that "looks almost right". The fix (approach 1 of the RAG's
//! three): replace the removed run with a `TJ` numeric adjustment that
//! consumes the **exact same** `tx`, so `Tm` advances identically and
//! the survivors stay put. `TJ` numbers are thousandths of text space,
//! subtracted (§9.4.3), so the adjustment for a removed run of total
//! advance `Σtx` (text-line units) is `N = −Σtx · 1000 / (Tfs·Th)`.
//!
//! **The security guarantee is independent of width accuracy.** Whether
//! a survivor ends up one point off has no bearing on whether the
//! redacted glyph's bytes are gone — they are removed from the show
//! string regardless. Width precision affects only the *cosmetic*
//! quality of advance preservation, so an estimated width (disclosed) is
//! never a security regression.
//!
//! ## Spec sources (PDF-spec RAG, ISO 32000-1:2008)
//!
//! - `iso32000__s__12.5.6.23.md` — the /Redact mark, Table 192, the four
//!   `shall` outcome constraints, the overlay precedence ladder.
//! - `iso32000__ref__redaction_removal.md` — the derived removal
//!   mechanics: content-stream text surgery (§9.4/§8.2), object-stream
//!   container decomposition (§7.5.7/§7.5.8), image re-encode/refuse, the
//!   carrier sweep, the forced-full-rewrite rule.
//! - `iso32000__s__9.4.md` — the §9.4.4 advance formula this module's
//!   surgery is built on.

use std::collections::BTreeSet;

use crate::content::{ContentStream, ContentTokenKind};
use crate::document::Document;
use crate::graph::ObjectGraph;
use crate::object::{Dict, Name, ObjId, Object, Stream};
use crate::page_tree::{self, PageTreeError, Rect};
use crate::span::ByteSpan;
use crate::text_extract::font::ExtractFont;
use crate::writer::content::{ContentBuilder, Paint, emit_literal_string, emit_number};
use crate::writer::{SaveOptions, WriteError, save_full};

/// The vertical over-coverage of a glyph box, as a fraction of the font
/// size, below the baseline and above the em top.
///
/// A glyph's advance box is `x ∈ [0, w0]`, but its ink extends below the
/// baseline (descenders) and to the cap/ascent above. Redaction
/// **over-covers** deliberately (fuzzy-never-sneaky: a partial glyph at a
/// region edge is removed, not kept — a leak is worse than an
/// over-redaction), so the region-intersection test uses a slightly
/// enlarged box.
const GLYPH_BOX_DESCENT: f64 = 0.25;
const GLYPH_BOX_ASCENT: f64 = 1.0;

/// A 2-D affine transform in PDF's row-vector convention
/// `[a b 0 / c d 0 / e f 1]` (§8.3.3), in `f64` for interpreter
/// precision.
#[derive(Debug, Clone, Copy)]
struct Mat {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Mat {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// `self × other`, PDF order — `self` applies first.
    fn mul(self, o: Self) -> Self {
        Self {
            a: self.a * o.a + self.b * o.c,
            b: self.a * o.b + self.b * o.d,
            c: self.c * o.a + self.d * o.c,
            d: self.c * o.b + self.d * o.d,
            e: self.e * o.a + self.f * o.c + o.e,
            f: self.e * o.b + self.f * o.d + o.f,
        }
    }

    const fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Transform a point (row-vector convention).
    fn apply(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

/// A rectangular redaction region in default user space (the AABB of one
/// /Redact quad or /Rect — orientation is irrelevant for a removal mask,
/// so quads are reduced to bounds per the RAG's guidance).
#[derive(Debug, Clone, Copy)]
struct RegionBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl RegionBox {
    fn from_rect(r: Rect) -> Self {
        Self {
            min_x: r.llx,
            min_y: r.lly,
            max_x: r.urx,
            max_y: r.ury,
        }
    }

    /// AABB overlap (touch counts — the over-redaction bias).
    fn intersects(self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
        self.min_x <= max_x && min_x <= self.max_x && self.min_y <= max_y && min_y <= self.max_y
    }
}

/// Why a redaction apply could not be performed. Every variant names a
/// condition an operator can act on — there is no catch-all.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedactError {
    /// The page tree could not be walked.
    #[error("page tree: {0}")]
    PageTree(#[from] PageTreeError),
    /// The document has no /Redact annotations to apply.
    #[error("the document has no redaction marks to apply")]
    NothingToApply,
    /// A redaction region intersects a raster image (XObject or inline).
    /// Refused **by name** rather than masked: pdfce cannot yet destroy
    /// image pixels, and a clip/overlay would leave the pixels
    /// recoverable — the exact false-redaction failure §12.5.6.23 names.
    #[error(
        "redaction region on page {page} intersects an image; pdfce cannot yet destroy image \
         pixels (clipping or masking would leave them recoverable, ISO 32000-1 §12.5.6.23) — \
         apply refused rather than producing a false redaction"
    )]
    ImageRegion {
        /// 1-based page number carrying the intersecting image.
        page: usize,
    },
    /// A content stream could not be tokenized, so its glyphs could not
    /// be located for removal. Refused rather than risk leaving covered
    /// text behind on an unreadable page.
    #[error(
        "page {page} content could not be parsed, so redaction cannot verify removal: {source}"
    )]
    Content {
        /// 1-based page number.
        page: usize,
        /// The underlying tokenization error.
        source: crate::content::ContentError,
    },
    /// The document is encrypted; per-object string decryption is Pass 5,
    /// so redaction is refused rather than operating on ciphertext.
    #[error(
        "this document is encrypted (/Encrypt); redaction of encrypted documents is not yet supported"
    )]
    Encrypted,
    /// The full-rewrite save failed.
    #[error("writing the redacted document failed: {0}")]
    Write(#[from] WriteError),
}

/// What a carrier sweep found and did for one class of duplicated
/// content. This is the executable form of §12.5.6.23's "diligent about
/// all content that can exist" obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierStatus {
    /// A short stable identifier for the carrier (`info`, `xmp`, `xfa`,
    /// `struct_tree`, `attachments`, `ocg`, `thumbnails`,
    /// `object_streams`, `prior_revisions`, `overlapping_annotations`).
    pub carrier: &'static str,
    /// Whether this carrier was present in the document at all.
    pub present: bool,
    /// What pdfce did about it.
    pub action: CarrierAction,
}

/// What redaction did about one carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CarrierAction {
    /// Not present — nothing to do.
    Absent,
    /// pdfce removed the redacted content from this carrier; it is part
    /// of the absence guarantee.
    Scrubbed,
    /// The carrier's superseded content is dropped as a side effect of
    /// the forced full rewrite (prior revisions, object-stream survivors).
    DroppedByRewrite,
    /// **Present and NOT scrubbed** — disclosed to the operator as an
    /// un-redacted residual to verify manually. The cardinal-rule-honest
    /// outcome for a carrier this build cannot fully redact.
    DisclosedNotScrubbed,
}

impl CarrierAction {
    /// A short stable identifier for machine-readable output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Scrubbed => "scrubbed",
            Self::DroppedByRewrite => "dropped_by_rewrite",
            Self::DisclosedNotScrubbed => "DISCLOSED_NOT_SCRUBBED",
        }
    }
}

/// The redaction report: exactly what was removed and which carriers
/// were checked, scrubbed, or left. This report existing — and being
/// printed — is the mechanism that makes silent under-redaction
/// impossible: every residual pdfce cannot remove is named here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedactionReport {
    /// Pages that carried at least one /Redact mark.
    pub pages_redacted: usize,
    /// /Redact annotations applied and then removed.
    pub marks_applied: u64,
    /// Text character codes removed from content streams.
    pub glyphs_removed: u64,
    /// Show operators (`Tj`/`TJ`/`'`/`"`) rewritten by the surgery.
    pub show_operators_edited: u64,
    /// Content stream objects replaced with a redacted rewrite.
    pub content_streams_rewritten: u64,
    /// Annotations (redaction marks + overlapping annotations) removed.
    pub annotations_removed: u64,
    /// Object-stream containers decomposed so no removed object survived
    /// compressed (§7.5.7 Strategy B).
    pub containers_decomposed: u64,
    /// Objects promoted out of an object stream by the decomposition.
    pub objects_promoted: u64,
    /// `/Info` string entries scrubbed of redacted text.
    pub info_strings_scrubbed: u64,
    /// Distinct fonts whose advance widths were estimated (no `/Widths`,
    /// not standard-14) — affects only advance-preservation cosmetics,
    /// never the removal itself. Disclosed.
    pub estimated_width_fonts: u64,
    /// Per-carrier diligence status (§12.5.6.23's "all content" sweep).
    pub carriers: Vec<CarrierStatus>,
    /// The distinct redacted text strings, for the operator's review and
    /// for the absence-proof gate to grep. Kept because the interpreter
    /// decodes the removed codes while removing them.
    pub redacted_text: Vec<String>,
    /// Named diagnostics and disclosures (human-readable).
    pub notes: Vec<String>,
}

impl RedactionReport {
    fn note(&mut self, text: String) {
        if !self.notes.contains(&text) {
            self.notes.push(text);
        }
    }

    fn add_carrier(&mut self, carrier: &'static str, present: bool, action: CarrierAction) {
        self.carriers.push(CarrierStatus {
            carrier,
            present,
            action,
        });
    }

    /// Whether any carrier was present but disclosed-not-scrubbed — i.e.
    /// the operator must verify a residual manually. A caller (CLI/GUI)
    /// surfaces this loudly.
    #[must_use]
    pub fn has_disclosed_residuals(&self) -> bool {
        self.carriers
            .iter()
            .any(|c| c.action == CarrierAction::DisclosedNotScrubbed)
    }
}

// ===================================================================
// §3 — content-stream text surgery interpreter
// ===================================================================

/// The graphics + text state the surgery interpreter maintains. A focused
/// mirror of the Pass-4 extraction walker's state, kept **self-contained**
/// in this module so the security-critical byte surgery is auditable in
/// one place (the RAG's §3 recommendation: "edit surgically and re-emit
/// the rest byte-faithfully, so the diff is auditable").
struct Surgeon<'a> {
    doc: &'a Document,
    resources: &'a Dict,
    regions: &'a [RegionBox],
    // graphics state
    ctm: Mat,
    ctm_stack: Vec<Mat>,
    // text state (§9.3)
    tm: Mat,
    tlm: Mat,
    tf_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    trise: f64,
    tl: f64,
    ts_stack: Vec<TextSnapshot>,
    font: Option<ExtractFont>,
    // outputs
    edits: Vec<Edit>,
    removed_text: Vec<String>,
    glyphs_removed: u64,
    ops_edited: u64,
    image_intersect: bool,
    form_intersect: bool,
    estimated_fonts: BTreeSet<String>,
}

/// The text-state fields saved/restored by `q`/`Q`. (`Tf`/font is part of
/// text state too, but cloning an `ExtractFont` per `q` is wasteful; the
/// interpreter re-resolves the font on the next `Tf`, and `q`/`Q` in real
/// content rarely straddles a `Tf` boundary that matters to geometry.)
#[derive(Clone, Copy)]
struct TextSnapshot {
    tf_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    trise: f64,
    tl: f64,
}

/// One byte-range replacement in the decoded content buffer.
struct Edit {
    start: usize,
    end: usize,
    bytes: Vec<u8>,
}

/// The result of redacting one page's content.
struct SurgeryResult {
    /// The rewritten (redacted + overlay-baked) content bytes.
    content: Vec<u8>,
    removed_text: Vec<String>,
    glyphs_removed: u64,
    ops_edited: u64,
    /// A raster image intersects a region — the caller must refuse.
    image_intersect: bool,
    /// A form XObject intersects a region — disclosed, not refused.
    form_intersect: bool,
    estimated_fonts: BTreeSet<String>,
}

impl<'a> Surgeon<'a> {
    fn new(doc: &'a Document, resources: &'a Dict, regions: &'a [RegionBox]) -> Self {
        Self {
            doc,
            resources,
            regions,
            ctm: Mat::IDENTITY,
            ctm_stack: Vec::new(),
            tm: Mat::IDENTITY,
            tlm: Mat::IDENTITY,
            tf_size: 0.0,
            tc: 0.0,
            tw: 0.0,
            th: 1.0,
            trise: 0.0,
            tl: 0.0,
            ts_stack: Vec::new(),
            font: None,
            edits: Vec::new(),
            removed_text: Vec::new(),
            glyphs_removed: 0,
            ops_edited: 0,
            image_intersect: false,
            form_intersect: false,
            estimated_fonts: BTreeSet::new(),
        }
    }

    /// Resolve the numeric operands of an operation, in order.
    fn nums(operands: &[crate::content::ContentToken]) -> Vec<f64> {
        operands
            .iter()
            .filter_map(|t| match &t.kind {
                ContentTokenKind::Operand(o) => o.as_number(),
                _ => None,
            })
            .collect()
    }

    /// Resolve a `/Font /<name>` resource to an [`ExtractFont`].
    fn resolve_font(&self, name: &[u8]) -> Option<ExtractFont> {
        let fonts = self
            .resources
            .get(b"Font")
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict)?;
        let font_dict = fonts
            .get(name)
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict)?;
        // `&self.doc.view()` (Pass 17.1): `ExtractFont::resolve` now takes a
        // read VIEW because it may need a `/ToUnicode` stream's bytes. This
        // census deliberately reads the loaded document, so the contiguous
        // base view is the right one and behaviour is unchanged.
        Some(ExtractFont::resolve(&self.doc.view(), font_dict))
    }

    /// Is a named XObject an image (or a form) whose unit-square placement
    /// intersects a region? Sets `image_intersect` / `form_intersect`.
    fn check_xobject(&mut self, name: &[u8]) {
        let Some(xobjects) = self
            .resources
            .get(b"XObject")
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict)
        else {
            return;
        };
        let Some(obj) = xobjects.get(name).map(|o| self.doc.resolve(o)) else {
            return;
        };
        let Object::Stream(stream) = obj else {
            return;
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .and_then(Object::as_name)
            .map(|n| n.as_bytes().to_vec())
            .unwrap_or_default();
        // A form/image placement is the unit square (0,0)-(1,1) × CTM.
        if !self.unit_square_intersects() {
            return;
        }
        match subtype.as_slice() {
            b"Image" => self.image_intersect = true,
            b"Form" => self.form_intersect = true,
            _ => {}
        }
    }

    /// Whether the current CTM's unit square intersects any region.
    fn unit_square_intersects(&self) -> bool {
        let corners = [
            self.ctm.apply(0.0, 0.0),
            self.ctm.apply(1.0, 0.0),
            self.ctm.apply(0.0, 1.0),
            self.ctm.apply(1.0, 1.0),
        ];
        let (min_x, min_y, max_x, max_y) = aabb(&corners);
        self.regions
            .iter()
            .any(|r| r.intersects(min_x, min_y, max_x, max_y))
    }

    /// Set the text matrix and line matrix (`Td`/`TD`/`T*`/`Tm`).
    fn set_line(&mut self, m: Mat) {
        self.tm = m;
        self.tlm = m;
    }

    /// Process one content operation, updating state and (for show
    /// operators) recording any surgery edit.
    fn operation(&mut self, op: &crate::content::Operation<'_>, buf: &[u8]) {
        let Some(name) = op.operator_name(buf) else {
            // An inline image: its unit-square placement may intersect.
            if let ContentTokenKind::InlineImage { .. } = op.operator.kind
                && self.unit_square_intersects()
            {
                self.image_intersect = true;
            }
            return;
        };
        let n = Self::nums(op.operands);
        match name {
            b"q" => {
                self.ctm_stack.push(self.ctm);
                self.ts_stack.push(self.snapshot());
            }
            b"Q" => {
                if let Some(m) = self.ctm_stack.pop() {
                    self.ctm = m;
                }
                if let Some(s) = self.ts_stack.pop() {
                    self.restore(s);
                }
            }
            b"cm" => {
                if let [a, b, c, d, e, f] = n.as_slice() {
                    self.ctm = Mat {
                        a: *a,
                        b: *b,
                        c: *c,
                        d: *d,
                        e: *e,
                        f: *f,
                    }
                    .mul(self.ctm);
                }
            }
            b"BT" => {
                self.tm = Mat::IDENTITY;
                self.tlm = Mat::IDENTITY;
            }
            b"Tf" => {
                if let Some(fname) = op.operands.iter().find_map(|t| match &t.kind {
                    ContentTokenKind::Operand(Object::Name(nm)) => Some(nm.as_bytes().to_vec()),
                    _ => None,
                }) {
                    self.font = self.resolve_font(&fname);
                }
                if let Some(size) = n.last() {
                    self.tf_size = *size;
                }
            }
            b"Td" => {
                if let [tx, ty] = n.as_slice() {
                    self.set_line(Mat::translate(*tx, *ty).mul(self.tlm));
                }
            }
            b"TD" => {
                if let [tx, ty] = n.as_slice() {
                    self.tl = -*ty;
                    self.set_line(Mat::translate(*tx, *ty).mul(self.tlm));
                }
            }
            b"Tm" => {
                if let [a, b, c, d, e, f] = n.as_slice() {
                    self.set_line(Mat {
                        a: *a,
                        b: *b,
                        c: *c,
                        d: *d,
                        e: *e,
                        f: *f,
                    });
                }
            }
            b"T*" => {
                self.set_line(Mat::translate(0.0, -self.tl).mul(self.tlm));
            }
            b"Tc" => {
                if let Some(v) = n.first() {
                    self.tc = *v;
                }
            }
            b"Tw" => {
                if let Some(v) = n.first() {
                    self.tw = *v;
                }
            }
            b"Tz" => {
                if let Some(v) = n.first() {
                    self.th = *v / 100.0;
                }
            }
            b"TL" => {
                if let Some(v) = n.first() {
                    self.tl = *v;
                }
            }
            b"Ts" => {
                if let Some(v) = n.first() {
                    self.trise = *v;
                }
            }
            b"Do" => {
                if let Some(xname) = op.operands.iter().find_map(|t| match &t.kind {
                    ContentTokenKind::Operand(Object::Name(nm)) => Some(nm.as_bytes().to_vec()),
                    _ => None,
                }) {
                    self.check_xobject(&xname);
                }
            }
            b"Tj" => self.show_simple(op, ShowKind::Tj),
            b"'" => {
                self.set_line(Mat::translate(0.0, -self.tl).mul(self.tlm));
                self.show_simple(op, ShowKind::Quote);
            }
            b"\"" => {
                if let [aw, ac] = n.as_slice() {
                    self.tw = *aw;
                    self.tc = *ac;
                }
                self.set_line(Mat::translate(0.0, -self.tl).mul(self.tlm));
                self.show_simple(op, ShowKind::DoubleQuote);
            }
            b"TJ" => self.show_array(op),
            _ => {}
        }
    }

    fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            tf_size: self.tf_size,
            tc: self.tc,
            tw: self.tw,
            th: self.th,
            trise: self.trise,
            tl: self.tl,
        }
    }

    fn restore(&mut self, s: TextSnapshot) {
        self.tf_size = s.tf_size;
        self.tc = s.tc;
        self.tw = s.tw;
        self.th = s.th;
        self.trise = s.trise;
        self.tl = s.tl;
    }

    /// The horizontal advance `tx` for one code (text-line units, §9.4.4),
    /// and whether the glyph's box intersects a region.
    fn glyph(&self, code: u32, word_spacing: bool) -> (f64, bool) {
        let Some(font) = &self.font else {
            return (0.0, false);
        };
        let w0 = f64::from(font.width(code));
        let tw = if word_spacing { self.tw } else { 0.0 };
        let tx = (w0 * self.tf_size + self.tc + tw) * self.th;

        // Trm = [Tfs·Th 0 0 Tfs 0 Ts] × Tm × CTM (§9.4.4). The glyph box
        // in text space is x ∈ [0, w0], y ∈ [-descent, ascent], deliberately
        // over-covered (module docs).
        let params = Mat {
            a: self.tf_size * self.th,
            b: 0.0,
            c: 0.0,
            d: self.tf_size,
            e: 0.0,
            f: self.trise,
        };
        let trm = params.mul(self.tm.mul(self.ctm));
        let corners = [
            trm.apply(0.0, -GLYPH_BOX_DESCENT),
            trm.apply(w0, -GLYPH_BOX_DESCENT),
            trm.apply(0.0, GLYPH_BOX_ASCENT),
            trm.apply(w0, GLYPH_BOX_ASCENT),
        ];
        let (min_x, min_y, max_x, max_y) = aabb(&corners);
        let hit = self
            .regions
            .iter()
            .any(|r| r.intersects(min_x, min_y, max_x, max_y));
        (tx, hit)
    }

    /// `Tj`/`'`/`"`: a single show string. Build an advance-preserving
    /// replacement if any code is in-region.
    fn show_simple(&mut self, op: &crate::content::Operation<'_>, kind: ShowKind) {
        let Some(string) = op.operands.iter().rev().find_map(|t| match &t.kind {
            ContentTokenKind::Operand(Object::String(s)) => Some(s.clone()),
            _ => None,
        }) else {
            return;
        };
        let Some(elem) = self.redact_string(&string) else {
            // Nothing in-region: advance Tm for the whole string and keep
            // the operator verbatim.
            self.advance_string(&string);
            return;
        };
        // Something was removed. Emit the replacement TJ (the survivors +
        // compensating advances) with any leading positioning kind needs.
        let mut out = Vec::new();
        match kind {
            ShowKind::Tj => {}
            ShowKind::Quote => out.extend_from_slice(b"T* "),
            ShowKind::DoubleQuote => {
                // Re-establish Tw/Tc (the " operator sets them and they
                // persist for following operators) then the line move.
                let nums = Self::nums(op.operands);
                if let [aw, ac] = nums.as_slice() {
                    emit_num(&mut out, *aw);
                    out.extend_from_slice(b" Tw ");
                    emit_num(&mut out, *ac);
                    out.extend_from_slice(b" Tc ");
                }
                out.extend_from_slice(b"T* ");
            }
        }
        emit_tj_array(&mut out, &elem);
        let (start, end) = op_span(op);
        self.record_edit(start, end, out);
    }

    /// `TJ`: an array of strings and kerning numbers. Rebuild it,
    /// replacing in-region code runs with compensating advances.
    fn show_array(&mut self, op: &crate::content::Operation<'_>) {
        let Some(items) = op.operands.iter().rev().find_map(|t| match &t.kind {
            ContentTokenKind::Operand(Object::Array(a)) => Some(a.clone()),
            _ => None,
        }) else {
            return;
        };
        let mut new_elems: Vec<TjElem> = Vec::new();
        let mut any_removed = false;
        for item in &items {
            match item {
                Object::String(s) => match self.redact_string(s) {
                    Some(elems) => {
                        any_removed = true;
                        new_elems.extend(elems);
                    }
                    None => {
                        self.advance_string(s);
                        new_elems.push(TjElem::Str(s.clone()));
                    }
                },
                other => {
                    // A kerning number: it adjusts Tm too, but does not
                    // show a glyph, so it is never "in region" — keep it.
                    if let Some(v) = other.as_number() {
                        self.tm =
                            Mat::translate(-v / 1000.0 * self.tf_size * self.th, 0.0).mul(self.tm);
                        new_elems.push(TjElem::Num(v));
                    }
                }
            }
        }
        if !any_removed {
            return;
        }
        let mut out = Vec::new();
        emit_tj_array(&mut out, &new_elems);
        let (start, end) = op_span(op);
        self.record_edit(start, end, out);
    }

    /// Advance `Tm` across a whole non-redacted string (so following
    /// operators stay correctly positioned) without recording an edit.
    fn advance_string(&mut self, string: &[u8]) {
        let Some(font) = self.font.clone() else {
            return;
        };
        for code in font.codes(string) {
            let (tx, _) = self.glyph(code.value, code.word_spacing_applies);
            self.tm = Mat::translate(tx, 0.0).mul(self.tm);
        }
    }

    /// Walk one show string's codes, computing which are in-region and
    /// advancing `Tm` per code. Returns `None` if none are in-region
    /// (caller keeps the operator verbatim), or the rebuilt element list
    /// (surviving byte segments + compensating advances) otherwise.
    ///
    /// Codes are segmented on **code** boundaries (1 byte for a simple
    /// font, 2 for a composite CID) so a multi-byte CID is never split
    /// (`iso32000__ref__redaction_removal.md` §3).
    fn redact_string(&mut self, string: &[u8]) -> Option<Vec<TjElem>> {
        let font = self.font.clone()?;
        let bpc = font.bytes_per_code();
        let codes = font.codes(string);
        // First pass: per-code hit + advance, and whether anything hits.
        let mut hits = Vec::with_capacity(codes.len());
        let mut any = false;
        for code in &codes {
            let (tx, hit) = self.glyph(code.value, code.word_spacing_applies);
            self.tm = Mat::translate(tx, 0.0).mul(self.tm);
            if hit {
                any = true;
            }
            hits.push((tx, hit, code.value));
        }
        if !any {
            return None;
        }
        if font.width_estimated() {
            self.estimated_fonts.insert(font.base_font_name());
        }
        // Second pass: build the replacement elements, coalescing runs.
        let mut elems: Vec<TjElem> = Vec::new();
        let mut seg_bytes: Vec<u8> = Vec::new();
        let mut removed_tx = 0.0f64;
        let mut removed_text = String::new();
        for (i, (tx, hit, code_val)) in hits.iter().enumerate() {
            let byte_start = i * bpc;
            let seg = string.get(byte_start..byte_start + bpc).unwrap_or(&[]);
            if *hit {
                // flush any pending surviving segment
                if !seg_bytes.is_empty() {
                    elems.push(TjElem::Str(std::mem::take(&mut seg_bytes)));
                }
                removed_tx += *tx;
                let (chars, _) = font.to_unicode(*code_val);
                removed_text.push_str(&chars);
                self.glyphs_removed += 1;
            } else {
                // flush any pending removed run as a compensating advance
                if removed_tx != 0.0 {
                    elems.push(TjElem::Num(advance_to_tj(
                        removed_tx,
                        self.tf_size,
                        self.th,
                    )));
                    removed_tx = 0.0;
                }
                seg_bytes.extend_from_slice(seg);
            }
        }
        if !seg_bytes.is_empty() {
            elems.push(TjElem::Str(seg_bytes));
        }
        if removed_tx != 0.0 {
            elems.push(TjElem::Num(advance_to_tj(
                removed_tx,
                self.tf_size,
                self.th,
            )));
        }
        if !removed_text.is_empty() {
            self.removed_text.push(removed_text);
        }
        self.ops_edited += 1;
        Some(elems)
    }

    fn record_edit(&mut self, start: usize, end: usize, bytes: Vec<u8>) {
        self.edits.push(Edit { start, end, bytes });
    }
}

/// Which single-string show operator is being rewritten.
enum ShowKind {
    Tj,
    Quote,
    DoubleQuote,
}

/// One element of a rebuilt `TJ` array.
enum TjElem {
    Str(Vec<u8>),
    Num(f64),
}

/// The `TJ` number that consumes a removed run's total advance `Σtx`
/// (text-line units): `N = −Σtx · 1000 / (Tfs·Th)` (§9.4.3). Guards a
/// zero scale (invisible text advances nothing).
fn advance_to_tj(sum_tx: f64, tfs: f64, th: f64) -> f64 {
    let scale = tfs * th;
    if scale.abs() < f64::EPSILON {
        0.0
    } else {
        -sum_tx * 1000.0 / scale
    }
}

/// The AABB of a set of transformed corner points.
fn aabb(pts: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (min_x, min_y, max_x, max_y)
}

/// The byte span of a whole operation (operands + operator) in the
/// decoded buffer, for replacement.
fn op_span(op: &crate::content::Operation<'_>) -> (usize, usize) {
    let start = op
        .operands
        .first()
        .map_or(op.operator.span.start, |t| t.span.start);
    (start, op.operator.span.end())
}

/// Emit a number into a content stream (integer form when integral).
fn emit_num(out: &mut Vec<u8>, v: f64) {
    emit_number(out, v);
}

/// Emit a rebuilt `TJ` array: `[ (str) num (str) … ] TJ`.
fn emit_tj_array(out: &mut Vec<u8>, elems: &[TjElem]) {
    out.push(b'[');
    for (i, e) in elems.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        match e {
            TjElem::Str(s) => emit_literal_string(out, s),
            TjElem::Num(v) => emit_number(out, *v),
        }
    }
    out.extend_from_slice(b"] TJ");
}

/// Redact one page's concatenated content: run the interpreter, splice
/// the edits, and append the overlay marking.
fn redact_page_content(
    doc: &Document,
    resources: &Dict,
    regions: &[RegionBox],
    stream: &ContentStream,
    overlay: &[u8],
) -> SurgeryResult {
    let mut surgeon = Surgeon::new(doc, resources, regions);
    for op in stream.operations() {
        surgeon.operation(&op, &stream.buf);
    }
    // Splice edits (sorted, non-overlapping) into the buffer.
    surgeon.edits.sort_by_key(|e| e.start);
    let mut content = Vec::with_capacity(stream.buf.len() + overlay.len());
    let mut cursor = 0usize;
    for edit in &surgeon.edits {
        if edit.start < cursor {
            continue; // defensive: overlapping edit, skip
        }
        if let Some(gap) = stream.buf.get(cursor..edit.start) {
            content.extend_from_slice(gap);
        }
        content.extend_from_slice(&edit.bytes);
        cursor = edit.end;
    }
    if let Some(tail) = stream.buf.get(cursor..) {
        content.extend_from_slice(tail);
    }
    // The overlay marking is drawn AFTER the (now-redacted) content so it
    // sits on top. A leading EOL guards against fusing onto a final token.
    if !overlay.is_empty() {
        content.push(b'\n');
        content.extend_from_slice(overlay);
    }
    SurgeryResult {
        content,
        removed_text: surgeon.removed_text,
        glyphs_removed: surgeon.glyphs_removed,
        ops_edited: surgeon.ops_edited,
        image_intersect: surgeon.image_intersect,
        form_intersect: surgeon.form_intersect,
        estimated_fonts: surgeon.estimated_fonts,
    }
}

/// Build the overlay content bytes for a page's regions: a filled box per
/// region in the fill colour (default black), wrapped in `q … Q` so it
/// does not perturb following state. RO/OverlayText burn-in is a named
/// follow-up (module docs); the `/IC`-fill regime is Acrobat's default.
fn build_overlay(regions: &[(RegionBox, [f64; 3])]) -> Vec<u8> {
    let mut b = ContentBuilder::new();
    b.save_state();
    for (region, rgb) in regions {
        b.set_fill_rgb(rgb[0], rgb[1], rgb[2]);
        b.rect(
            region.min_x,
            region.min_y,
            region.max_x - region.min_x,
            region.max_y - region.min_y,
        );
        b.paint(Paint::Fill);
    }
    b.restore_state();
    b.into_bytes()
}

// ===================================================================
// Apply orchestration — the destructive path (R35 forced full rewrite)
// ===================================================================

/// The `/Redact` annotations found on one page, resolved into geometry.
struct PageRedaction {
    page_id: ObjId,
    /// Surgery regions (all quads across all marks on this page).
    boxes: Vec<RegionBox>,
    /// Overlay boxes with their fill colour (default black).
    overlay: Vec<(RegionBox, [f64; 3])>,
    /// The `/Redact` annotation object ids to remove.
    redact_ids: Vec<ObjId>,
    /// Non-redact annotations intersecting a region — removed (the
    /// stricter Acrobat-parity reading; security over convenience).
    overlap_ids: Vec<ObjId>,
}

/// Apply every `/Redact` mark in `doc`: remove the covered content, scrub
/// the diligence carriers, drop the marks, and return the **full-rewrite**
/// bytes plus the [`RedactionReport`].
///
/// This is the one deliberately destructive operation in pdfce (R35). It
/// **forces a full rewrite** — never an incremental save — so no prior
/// revision survives with the un-redacted content, and every carrier
/// scrub rides that same rewrite (an incremental scrub would leave the
/// "removed" carrier recoverable in the prior revision).
///
/// # Errors
///
/// [`RedactError::NothingToApply`] if there are no marks;
/// [`RedactError::ImageRegion`] if a region intersects a raster image
/// (refused rather than falsely masked); [`RedactError::Content`] if a
/// redacted page cannot be parsed; [`RedactError::Encrypted`];
/// [`RedactError::Write`].
pub fn apply_redactions(
    doc: &Document,
    options: &SaveOptions,
) -> Result<(Vec<u8>, RedactionReport), RedactError> {
    if doc.trailer().contains_key(b"Encrypt") {
        return Err(RedactError::Encrypted);
    }
    let pages = page_tree::pages(doc)?;

    // --- gather the marks per page ---
    let mut plan: Vec<(usize, PageRedaction, Vec<ObjId>)> = Vec::new(); // (page_index, plan, contents)
    for (index, page) in pages.iter().enumerate() {
        let Some(redaction) = gather_page(doc, page.id) else {
            continue;
        };
        plan.push((index, redaction, page.contents.clone()));
    }
    if plan.is_empty() {
        return Err(RedactError::NothingToApply);
    }

    let mut report = RedactionReport::default();
    let mut dirty = crate::writer::DirtySet::empty();
    let mut staging: Vec<u8> = Vec::new();
    let base_len = doc.bytes().len();
    let mut next_num = doc.next_object_number().unwrap_or(1);
    let mut form_intersect_any = false;
    let mut estimated_fonts: BTreeSet<String> = BTreeSet::new();

    for (index, red, contents) in &plan {
        let page = pages.get(*index).ok_or(RedactError::NothingToApply)?;
        // Parse the page's concatenated content. BASE READ (decision 018
        // caller audit): `apply_redactions` is a one-shot whole-document
        // operation over a loaded `&Document` — there is no session here,
        // and the spans it computes are consumed by the writer, which is
        // contractually a base-bytes consumer.
        let stream =
            ContentStream::from_page(&doc.view(), page).map_err(|e| RedactError::Content {
                page: index + 1,
                source: e,
            })?;
        let overlay = build_overlay(&red.overlay);
        let result = redact_page_content(doc, &page.resources, &red.boxes, &stream, &overlay);

        // Image intersection → refuse by name (never a false redaction).
        if result.image_intersect {
            return Err(RedactError::ImageRegion { page: index + 1 });
        }
        if result.form_intersect {
            form_intersect_any = true;
        }
        estimated_fonts.extend(result.estimated_fonts);
        report.glyphs_removed += result.glyphs_removed;
        report.show_operators_edited += result.ops_edited;
        for t in result.removed_text {
            if !report.redacted_text.contains(&t) {
                report.redacted_text.push(t);
            }
        }

        // Rewrite the FIRST content object with the redacted+overlay bytes;
        // empty the rest. (Content streams are File-provenance, never in an
        // object stream, so save_full re-serializes them and the old glyph
        // bytes never reach the output. Emptying-in-place avoids the delete/
        // sharing traps.)
        let content_id = match contents.first() {
            Some(id) => *id,
            None => {
                // No content: create a stream just for the overlay.
                let id = ObjId::new(alloc(&mut next_num), 0);
                let span = stage(&mut staging, base_len, &result.content);
                dirty.replace(id, make_raw_stream(span, result.content.len()));
                // Wire it into the page /Contents below via the page write.
                report.content_streams_rewritten += 1;
                id
            }
        };
        if !contents.is_empty() {
            let span = stage(&mut staging, base_len, &result.content);
            dirty.replace(content_id, make_raw_stream(span, result.content.len()));
            report.content_streams_rewritten += 1;
            for extra in contents.iter().skip(1) {
                let empty = stage(&mut staging, base_len, &[]);
                dirty.replace(*extra, make_raw_stream(empty, 0));
            }
        }

        // Delete the redaction marks + overlapping annotations (and their
        // appearance/popup streams — an /AP over the region renders the
        // redacted content).
        let mut remove_annots: Vec<ObjId> = Vec::new();
        remove_annots.extend(&red.redact_ids);
        remove_annots.extend(&red.overlap_ids);
        for aid in &remove_annots {
            for sub in appearance_children(doc, *aid) {
                dirty.delete(sub);
            }
            dirty.delete(*aid);
            report.annotations_removed += 1;
        }

        // Rewrite the page dict: /Contents -> [content_id], /Annots with the
        // removed marks/overlaps gone, /Thumb dropped.
        let page_write = rewrite_page_dict(doc, red.page_id, content_id, &remove_annots);
        if let Some((new_dict, thumb)) = page_write {
            dirty.replace(red.page_id, Object::Dict(new_dict));
            if let Some(thumb_id) = thumb {
                dirty.delete(thumb_id);
            }
        }
        report.pages_redacted += 1;
        report.marks_applied += red.redact_ids.len() as u64;
    }

    for f in &estimated_fonts {
        report.note(format!(
            "redaction: advance widths for font {f} were estimated (no /Widths) — survivor \
             positioning is approximate; the removal itself is unaffected"
        ));
    }
    report.estimated_width_fonts = estimated_fonts.len() as u64;

    // --- carrier sweep (the §12.5.6.23 diligence obligation) ---
    let redacted_text = report.redacted_text.clone();
    carrier_info(doc, &redacted_text, &mut dirty, &mut report);
    carrier_xmp(
        doc,
        &redacted_text,
        &mut staging,
        base_len,
        &mut dirty,
        &mut report,
    );
    carrier_detect_disclose(doc, form_intersect_any, &mut report);

    // --- container decomposition (§7.5.7 Strategy B) ---
    decompose_containers(doc, &mut dirty, &mut report);

    // Prior revisions are dropped by the full rewrite itself.
    report.add_carrier("prior_revisions", true, CarrierAction::DroppedByRewrite);

    // --- forced FULL REWRITE (R35) ---
    if !staging.is_empty() {
        dirty.set_staging(staging);
    }
    let (bytes, _save) = save_full(doc, &dirty, options)?;
    Ok((bytes, report))
}

/// Allocate the next object number, advancing the counter.
fn alloc(next: &mut u32) -> u32 {
    let n = *next;
    *next = next.saturating_add(1);
    n
}

/// Append `bytes` to the staging buffer and return their combined-space
/// span (base ++ staging), so a created stream keeps the span model.
fn stage(staging: &mut Vec<u8>, base_len: usize, bytes: &[u8]) -> ByteSpan {
    let start = base_len + staging.len();
    staging.extend_from_slice(bytes);
    ByteSpan::new(start, bytes.len())
}

/// A raw (unfiltered) content stream object with the given data span and
/// length. No `/Filter`: the redacted content is emitted verbatim.
fn make_raw_stream(span: ByteSpan, len: usize) -> Object {
    let mut dict = Dict::new();
    dict.insert(
        Name::from(b"Length"),
        Object::Integer(i64::try_from(len).unwrap_or(i64::MAX)),
    );
    Object::Stream(Stream {
        dict,
        data_span: span,
    })
}

/// Resolve one page's `/Redact` annotations into geometry, or `None` if
/// the page carries none.
fn gather_page(doc: &Document, page_id: ObjId) -> Option<PageRedaction> {
    let page = doc
        .get(page_id)
        .map(|io| &io.value)
        .and_then(Object::as_dict)?;
    let annots = page
        .get(b"Annots")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)?;

    let mut boxes = Vec::new();
    let mut overlay = Vec::new();
    let mut redact_ids = Vec::new();
    let mut other: Vec<(ObjId, RegionBox)> = Vec::new();

    for entry in annots {
        let Some(aid) = entry.as_reference() else {
            continue;
        };
        let Some(dict) = doc.get(aid).map(|io| &io.value).and_then(Object::as_dict) else {
            continue;
        };
        let subtype = dict
            .get(b"Subtype")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_name)
            .map(|n| n.as_bytes().to_vec())
            .unwrap_or_default();
        if subtype == b"Redact" {
            let fill = annot_fill(doc, dict);
            for rb in annot_regions(doc, dict) {
                boxes.push(rb);
                overlay.push((rb, fill));
            }
            redact_ids.push(aid);
        } else if let Some(rb) = annot_rect_box(doc, dict) {
            other.push((aid, rb));
        }
    }
    if redact_ids.is_empty() {
        return None;
    }
    // Overlapping non-redact annotations → removed (stricter reading).
    let mut overlap_ids = Vec::new();
    for (aid, rb) in other {
        if boxes
            .iter()
            .any(|r| r.intersects(rb.min_x, rb.min_y, rb.max_x, rb.max_y))
        {
            overlap_ids.push(aid);
        }
    }
    Some(PageRedaction {
        page_id,
        boxes,
        overlay,
        redact_ids,
        overlap_ids,
    })
}

/// The regions a `/Redact` annotation covers: its `/QuadPoints`
/// (8×n numbers → n quads) if present, else its `/Rect` (Table 192).
fn annot_regions(doc: &Document, dict: &Dict) -> Vec<RegionBox> {
    if let Some(qp) = dict
        .get(b"QuadPoints")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)
    {
        let nums: Vec<f64> = qp
            .iter()
            .filter_map(|o| doc.resolve(o).as_number())
            .collect();
        let mut boxes = Vec::new();
        for quad in nums.chunks_exact(8) {
            let [x0, y0, x1, y1, x2, y2, x3, y3] = quad else {
                continue;
            };
            let xs = [*x0, *x1, *x2, *x3];
            let ys = [*y0, *y1, *y2, *y3];
            boxes.push(RegionBox {
                min_x: xs.iter().copied().fold(f64::INFINITY, f64::min),
                min_y: ys.iter().copied().fold(f64::INFINITY, f64::min),
                max_x: xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                max_y: ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            });
        }
        if !boxes.is_empty() {
            return boxes;
        }
    }
    annot_rect_box(doc, dict).into_iter().collect()
}

/// An annotation's `/Rect` as a [`RegionBox`].
fn annot_rect_box(doc: &Document, dict: &Dict) -> Option<RegionBox> {
    let arr = dict
        .get(b"Rect")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)?;
    let n: Vec<f64> = arr
        .iter()
        .filter_map(|o| doc.resolve(o).as_number())
        .collect();
    if let [x0, y0, x1, y1] = n.as_slice() {
        Some(RegionBox::from_rect(Rect::from_corners(*x0, *y0, *x1, *y1)))
    } else {
        None
    }
}

/// A `/Redact` mark's fill colour: `/IC` (DeviceRGB, three numbers) or the
/// default black. `/IC` is ignored if `/RO` is present (Table 192); RO
/// burn-in is a named follow-up, so this build honours `/IC`/default.
fn annot_fill(doc: &Document, dict: &Dict) -> [f64; 3] {
    if let Some(ic) = dict
        .get(b"IC")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)
    {
        let n: Vec<f64> = ic
            .iter()
            .filter_map(|o| doc.resolve(o).as_number())
            .collect();
        if let [r, g, b] = n.as_slice() {
            return [*r, *g, *b];
        }
    }
    [0.0, 0.0, 0.0]
}

/// The appearance/popup child object ids of an annotation (its `/AP`
/// `/N`/`/D`/`/R` streams and `/Popup`), which must be deleted with it so
/// no rendered copy of the redacted content survives.
fn appearance_children(doc: &Document, annot_id: ObjId) -> Vec<ObjId> {
    let mut out = Vec::new();
    let Some(dict) = doc
        .get(annot_id)
        .map(|io| &io.value)
        .and_then(Object::as_dict)
    else {
        return out;
    };
    if let Some(ap) = dict
        .get(b"AP")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
    {
        for (_k, v) in ap.iter() {
            collect_refs(doc, v, &mut out);
        }
    }
    if let Some(Object::Reference(p)) = dict.get(b"Popup") {
        out.push(*p);
    }
    out
}

/// Collect the indirect-reference ids reachable one level down from `obj`
/// (a stream reference, or a sub-dictionary of appearance-state
/// references).
fn collect_refs(_doc: &Document, obj: &Object, out: &mut Vec<ObjId>) {
    match obj {
        Object::Reference(id) => out.push(*id),
        Object::Dict(d) => {
            for (_k, v) in d.iter() {
                if let Object::Reference(id) = v {
                    out.push(*id);
                }
            }
        }
        _ => {}
    }
}

/// Build the rewritten page dictionary: `/Contents -> [content_id]`,
/// `/Annots` with `remove` filtered out, `/Thumb` dropped. Returns the new
/// dict and the dropped `/Thumb` object id, or `None` if the page dict is
/// unreadable.
fn rewrite_page_dict(
    doc: &Document,
    page_id: ObjId,
    content_id: ObjId,
    remove: &[ObjId],
) -> Option<(Dict, Option<ObjId>)> {
    let page = doc
        .get(page_id)
        .map(|io| &io.value)
        .and_then(Object::as_dict)?;
    let mut updated = page.clone();
    updated.insert(
        Name::from(b"Contents"),
        Object::Array(vec![Object::Reference(content_id)]),
    );
    // /Annots: drop the removed refs. If it is an indirect array, inline a
    // fresh direct array (simplest correct rewrite for the destructive path).
    if let Some(annots) = page
        .get(b"Annots")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)
    {
        let kept: Vec<Object> = annots
            .iter()
            .filter(|o| o.as_reference().is_none_or(|id| !remove.contains(&id)))
            .cloned()
            .collect();
        updated.insert(Name::from(b"Annots"), Object::Array(kept));
    }
    let thumb = match page.get(b"Thumb") {
        Some(Object::Reference(id)) => Some(*id),
        _ => None,
    };
    if thumb.is_some() {
        updated.remove(b"Thumb");
    }
    Some((updated, thumb))
}

/// Carrier: `/Info` — remove any string entry whose bytes contain a
/// redacted string (over-scrub: drop the whole entry). The scrub rides the
/// forced full rewrite, so the old `/Info` object's bytes do not survive.
fn carrier_info(
    doc: &Document,
    redacted: &[String],
    dirty: &mut crate::writer::DirtySet,
    report: &mut RedactionReport,
) {
    let Some(info_id) = doc.trailer().get(b"Info").and_then(Object::as_reference) else {
        report.add_carrier("info", false, CarrierAction::Absent);
        return;
    };
    let Some(info) = doc
        .get(info_id)
        .map(|io| &io.value)
        .and_then(Object::as_dict)
    else {
        report.add_carrier("info", false, CarrierAction::Absent);
        return;
    };
    let mut updated = info.clone();
    let mut changed = 0u64;
    let keys: Vec<Name> = info.iter().map(|(k, _)| k.clone()).collect();
    for key in keys {
        if let Some(Object::String(bytes)) = info.get(key.as_bytes())
            && redacted.iter().any(|t| bytes_contain_text(bytes, t))
        {
            updated.remove(key.as_bytes());
            changed += 1;
        }
    }
    if changed > 0 {
        report.info_strings_scrubbed = changed;
        dirty.replace(info_id, Object::Dict(updated));
        report.add_carrier("info", true, CarrierAction::Scrubbed);
    } else {
        report.add_carrier("info", true, CarrierAction::Absent);
    }
}

/// Carrier: XMP `/Metadata` — decode the packet, blank every occurrence of
/// a redacted string, and re-emit it **raw** (dropping any filter) so the
/// scrubbed packet cannot survive compressed either.
fn carrier_xmp(
    doc: &Document,
    redacted: &[String],
    staging: &mut Vec<u8>,
    base_len: usize,
    dirty: &mut crate::writer::DirtySet,
    report: &mut RedactionReport,
) {
    let meta = doc
        .catalog()
        .ok()
        .and_then(|c| c.get(b"Metadata").map(|o| doc.resolve(o)).cloned());
    let Some(Object::Stream(stream)) = meta else {
        report.add_carrier("xmp", false, CarrierAction::Absent);
        return;
    };
    let meta_id = doc
        .catalog()
        .ok()
        .and_then(|c| c.get(b"Metadata").and_then(Object::as_reference));
    let Some(meta_id) = meta_id else {
        report.add_carrier("xmp", true, CarrierAction::Absent);
        return;
    };
    let Some(raw) = stream.data_span.slice(doc.bytes()) else {
        report.add_carrier("xmp", true, CarrierAction::Absent);
        return;
    };
    let decoded = crate::filters::decode_stream(&stream.dict, raw).unwrap_or_else(|_| raw.to_vec());
    let mut scrubbed = decoded.clone();
    let mut changed = false;
    for t in redacted {
        if replace_all_bytes(&mut scrubbed, t.as_bytes(), b'X') {
            changed = true;
        }
        // Also the UTF-16BE encoding, in case the packet is UTF-16.
        let u16be = utf16be(t);
        if replace_all_bytes(&mut scrubbed, &u16be, b'X') {
            changed = true;
        }
    }
    if !changed {
        report.add_carrier("xmp", true, CarrierAction::Absent);
        return;
    }
    let mut dict = stream.dict.clone();
    dict.remove(b"Filter");
    dict.remove(b"DecodeParms");
    dict.insert(
        Name::from(b"Length"),
        Object::Integer(i64::try_from(scrubbed.len()).unwrap_or(i64::MAX)),
    );
    let span = stage(staging, base_len, &scrubbed);
    dirty.replace(
        meta_id,
        Object::Stream(Stream {
            dict,
            data_span: span,
        }),
    );
    report.add_carrier("xmp", true, CarrierAction::Scrubbed);
}

/// Carriers pdfce **detects but does not scrub** this build — disclosed as
/// residuals for manual verification (never silently left).
fn carrier_detect_disclose(doc: &Document, form_intersect: bool, report: &mut RedactionReport) {
    let catalog = doc.catalog().ok();

    // XFA — a parallel XML copy of form/text content (§12.5.6.23 names it).
    let xfa = catalog
        .and_then(|c| {
            c.get(b"AcroForm")
                .map(|o| doc.resolve(o))
                .and_then(Object::as_dict)
        })
        .is_some_and(|acro| acro.contains_key(b"XFA"));
    if xfa {
        report.add_carrier("xfa", true, CarrierAction::DisclosedNotScrubbed);
        report.note(
            "redaction: /AcroForm /XFA present — the XFA XML may duplicate redacted content and \
             was NOT scrubbed (pdfce is XFA detect-only); verify or remove XFA manually"
                .to_string(),
        );
    } else {
        report.add_carrier("xfa", false, CarrierAction::Absent);
    }

    // Structure tree /ActualText/Alt/E — tagged replacement text that an
    // extractor reads even after glyph removal.
    let struct_tree = catalog.is_some_and(|c| c.contains_key(b"StructTreeRoot"));
    if struct_tree {
        report.add_carrier("struct_tree", true, CarrierAction::DisclosedNotScrubbed);
        report.note(
            "redaction: /StructTreeRoot present — tagged /ActualText//Alt//E may duplicate \
             redacted glyphs and was NOT scrubbed; verify the structure tree manually"
                .to_string(),
        );
    } else {
        report.add_carrier("struct_tree", false, CarrierAction::Absent);
    }

    // Embedded files / attachments — whole documents outside region scope.
    let attachments = catalog.is_some_and(|c| {
        c.get(b"Names")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .is_some_and(|n| n.contains_key(b"EmbeddedFiles"))
    });
    if attachments {
        report.add_carrier("attachments", true, CarrierAction::DisclosedNotScrubbed);
        report.note(
            "redaction: embedded files (/Names /EmbeddedFiles) present — out of region scope and \
             NOT scrubbed; review attachments manually"
                .to_string(),
        );
    } else {
        report.add_carrier("attachments", false, CarrierAction::Absent);
    }

    // OCG layers — redacted by GEOMETRY (the interpreter walks all content
    // regardless of optional-content visibility), so covered content in an
    // OFF layer is still removed. Reported as scrubbed-by-geometry.
    let ocg = catalog.is_some_and(|c| c.contains_key(b"OCProperties"));
    if ocg {
        report.add_carrier("ocg", true, CarrierAction::Scrubbed);
        report.note(
            "redaction: optional-content (/OCProperties) present — redacted by GEOMETRY (layer \
             visibility ignored), so content in hidden layers within a region was still removed"
                .to_string(),
        );
    } else {
        report.add_carrier("ocg", false, CarrierAction::Absent);
    }

    if form_intersect {
        report.note(
            "redaction: a form XObject overlaps a redaction region — its content was NOT \
             surgically redacted this build; verify manually or flatten the form first"
                .to_string(),
        );
    }
}

/// Container decomposition (§7.5.7 Strategy B): any object the redaction
/// removed or replaced that lives in an object stream would otherwise
/// survive verbatim inside its untouched container (pdfce's `save_full`
/// re-emits `/ObjStm` intact by design). Promote every survivor of such a
/// container out to file level and drop the container, so no removed byte
/// survives compressed.
fn decompose_containers(
    doc: &Document,
    dirty: &mut crate::writer::DirtySet,
    report: &mut RedactionReport,
) {
    // Snapshot the objects the redaction already touches.
    let touched: BTreeSet<ObjId> = dirty.iter().collect();
    // Which object-stream containers hold a touched object?
    let mut containers: BTreeSet<ObjId> = BTreeSet::new();
    for id in &touched {
        if let Some(io) = doc.get(*id)
            && let Some(c) = io.provenance.container()
        {
            containers.insert(c);
        }
    }
    if containers.is_empty() {
        report.add_carrier("object_streams", false, CarrierAction::Absent);
        return;
    }
    let mut promoted = 0u64;
    for container in &containers {
        for io in doc.objects() {
            if io.provenance.container() == Some(*container) && !touched.contains(&io.id) {
                // Promote the survivor: replacing it with its current value
                // makes save_full write it at file level (type-1),
                // superseding the type-2 entry.
                dirty.replace(io.id, io.value.clone());
                promoted += 1;
            }
        }
        // Drop the now-empty container so its verbatim bytes (holding the
        // removed object) are never emitted.
        dirty.delete(*container);
    }
    report.containers_decomposed = containers.len() as u64;
    report.objects_promoted = promoted;
    report.add_carrier("object_streams", true, CarrierAction::DroppedByRewrite);
    report.note(format!(
        "redaction: decomposed {} object stream(s), promoting {} survivor(s) out so no removed \
         object survives compressed (ISO 32000-1 §7.5.7)",
        containers.len(),
        promoted
    ));
}

/// Whether `value` (raw PDF string bytes) contains `needle` in either its
/// ASCII/PDFDocEncoding form or its UTF-16BE form (§7.9.2's two text-string
/// encodings). Case-insensitive on the ASCII form.
fn bytes_contain_text(value: &[u8], needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let hay_lower: Vec<u8> = value.iter().map(u8::to_ascii_lowercase).collect();
    let need_lower: Vec<u8> = needle.bytes().map(|b| b.to_ascii_lowercase()).collect();
    if contains_subslice(&hay_lower, &need_lower) {
        return true;
    }
    let u16be = utf16be(needle);
    contains_subslice(value, &u16be)
}

/// UTF-16BE encoding of a string (no BOM).
fn utf16be(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for u in s.encode_utf16() {
        out.push((u >> 8) as u8);
        out.push((u & 0xff) as u8);
    }
    out
}

/// Replace every occurrence of `needle` in `hay` with `fill`-repeated
/// bytes of the same length. Returns whether anything changed.
fn replace_all_bytes(hay: &mut [u8], needle: &[u8], fill: u8) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    let mut changed = false;
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay.get(i..i + needle.len()) == Some(needle) {
            for j in i..i + needle.len() {
                if let Some(slot) = hay.get_mut(j) {
                    *slot = fill;
                }
            }
            changed = true;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    changed
}

/// First index of `needle` in `hay`, else `None`.
fn contains_subslice(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Count the `/Redact` marks currently present in a document — the census
/// the GUI status bar uses to disclose UNAPPLIED redactions (computed from
/// the graph itself, never a session counter, so it survives save/reload
/// and cannot lie about a marked-but-not-applied file).
///
/// # Why this is generic over [`ObjectGraph`] (Pass 17.1)
///
/// It used to take `&Document`, and the GUI's only caller passed
/// `session.document()` — the BASE revision. The consequence was the
/// precise failure this disclosure exists to prevent, wearing the
/// disclosure's own face: **a `/Redact` mark the operator placed during
/// this session was not counted**, so the very banner whose job is to say
/// "this document has marks you have not applied yet" stayed silent about
/// the marks most likely to be forgotten — the ones just made. Decision
/// 018 §8 names this the confirmed bug of the Pass 17.1 audit.
///
/// Generic rather than `&DocumentView` because the census reads **only**
/// dictionaries (`/Annots` → `/Subtype`); it never touches stream bytes,
/// so it needs an object graph and nothing else. That keeps every existing
/// caller (`pdfce-cli`, the redaction tests) compiling unchanged — a
/// `&Document` *is* an `ObjectGraph` — while the GUI can now pass
/// `&session.graph()` and get the truth.
///
/// A mark applied and then undone is correctly *not* counted: the session
/// overlay holds the base value again, and this walks values, never a
/// history.
#[must_use]
pub fn count_redaction_marks<G: ObjectGraph + ?Sized>(graph: &G) -> usize {
    let mut n = 0;
    let Ok(pages) = page_tree::pages_in(graph) else {
        return 0;
    };
    for page in &pages {
        let Some(dict) = graph.value(page.id).and_then(Object::as_dict) else {
            continue;
        };
        let Some(annots) = dict
            .get(b"Annots")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_array)
        else {
            continue;
        };
        for entry in annots {
            if let Some(aid) = entry.as_reference()
                && let Some(ad) = graph.value(aid).and_then(Object::as_dict)
                && ad
                    .get(b"Subtype")
                    .and_then(Object::as_name)
                    .is_some_and(|n| n.as_bytes() == b"Redact")
            {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]
mod tests {
    use super::*;
    use crate::edit::EditSession;
    use crate::text_extract::{self, ExtractOptions};
    use crate::writer::SaveOptions;

    /// Assemble a classic single-page PDF from body strings (objects
    /// `1..=n`), computing a correct xref table. Object 1 must be the
    /// catalog.
    fn assemble(bodies: &[&str], extra_trailer: &str) -> Vec<u8> {
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
            format!(
                "trailer\n<< /Size {n} /Root 1 0 R {extra_trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
            )
            .as_bytes(),
        );
        buf
    }

    /// A page whose content shows "SECRET PUBLIC" in one `Tj`, in
    /// standard-14 Helvetica (accurate AFM widths, so advance preservation
    /// is exact). `SECRET ` is what we redact; ` PUBLIC` must survive in
    /// place.
    fn redactable_pdf() -> Vec<u8> {
        let content = b"BT /F1 24 Tf 20 100 Td (SECRET PUBLIC) Tj ET";
        let stream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        );
        assemble(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
                 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
                &stream,
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            ],
            "",
        )
    }

    /// Decode every content stream of a document into one blob (for the
    /// absence assertion over decoded bytes).
    fn all_decoded_content(doc: &Document) -> Vec<u8> {
        let mut out = Vec::new();
        let pages = page_tree::pages(doc).unwrap();
        for page in &pages {
            if let Ok(cs) = ContentStream::from_page(&doc.view(), page) {
                out.extend_from_slice(&cs.buf);
            }
        }
        out
    }

    fn contains(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }

    /// Mark "SECRET" by search, save, reload — the state apply operates on.
    fn mark_and_save(input: &[u8]) -> Vec<u8> {
        let doc = Document::from_bytes(input.to_vec()).unwrap();
        let mut session = EditSession::new(doc);
        let ids = session.mark_redactions_by_search("SECRET", false).unwrap();
        assert!(!ids.is_empty(), "search should have found SECRET");
        let (bytes, _) = session
            .to_incremental_bytes(&SaveOptions::identity())
            .unwrap();
        bytes
    }

    /// Pass 17.1 regression gate: a mark added THIS SESSION is counted.
    ///
    /// This is the confirmed bug of decision 018 §8's `session.document()`
    /// audit, and it is the nastiest shape a bug can take — the disclosure
    /// whose entire job is to say *"this document has redaction marks you
    /// have not applied yet"* was blind to exactly the marks most likely to
    /// be forgotten: the ones just made. `count_redaction_marks` took a
    /// `&Document`, and the GUI's only caller handed it
    /// `session.document()`, the base revision, which by construction cannot
    /// carry an unsaved mark.
    ///
    /// The test pins all three states, because only the contrast makes the
    /// fix meaningful:
    ///
    /// 1. the BASE still counts 0 (the file on disk really has no mark);
    /// 2. the SESSION graph counts 1 (what the operator must be told);
    /// 3. after undo the session counts 0 again — proving the census walks
    ///    object VALUES rather than a counter that edits increment, which is
    ///    the property that lets it survive save/reload and refuse to lie.
    #[test]
    fn a_mark_added_this_session_is_counted_over_the_session_graph() {
        use crate::annot_author::{Quad, RedactSpec};
        use crate::vartext::Quadding;

        let doc = Document::from_bytes(redactable_pdf()).unwrap();
        let mut session = EditSession::new(doc);
        assert_eq!(
            count_redaction_marks(session.document()),
            0,
            "the fixture starts with no marks"
        );

        session
            .add_redaction(
                0,
                &RedactSpec {
                    quads: vec![Quad::from_rect(Rect::from_corners(
                        20.0, 90.0, 120.0, 130.0,
                    ))],
                    fill: None,
                    overlay_text: None,
                    quadding: Quadding::Left,
                },
            )
            .unwrap();

        assert_eq!(
            count_redaction_marks(&session.graph()),
            1,
            "a /Redact mark added this session MUST be disclosed — this is the Pass 17.1 bug"
        );
        assert_eq!(
            count_redaction_marks(session.document()),
            0,
            "the base revision is unchanged until the document is saved"
        );

        session.undo().expect("the mark is one undoable command");
        assert_eq!(
            count_redaction_marks(&session.graph()),
            0,
            "an undone mark is not a pending mark — the census walks values, not a counter"
        );
    }

    // -- THE HEADLINE GATE: absence proof --------------------------------

    #[test]
    fn apply_removes_redacted_text_from_the_whole_saved_file() {
        let marked = mark_and_save(&redactable_pdf());
        let doc = Document::from_bytes(marked).unwrap();
        assert_eq!(count_redaction_marks(&doc), 1);

        let (out, report) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();

        // ABSENCE PROOF: "SECRET" appears nowhere in the raw saved bytes...
        assert!(
            !contains(&out, b"SECRET"),
            "redacted text 'SECRET' survived in the raw saved bytes"
        );
        // ...and nowhere in any decoded content stream.
        let back = Document::from_bytes(out.clone()).unwrap();
        let decoded = all_decoded_content(&back);
        assert!(
            !contains(&decoded, b"SECRET"),
            "redacted text 'SECRET' survived in a decoded content stream"
        );
        // The mark itself is gone, and the surviving text remains.
        assert_eq!(
            count_redaction_marks(&back),
            0,
            "the /Redact mark must be removed"
        );
        assert!(
            contains(&decoded, b"PUBLIC"),
            "un-redacted text 'PUBLIC' must survive"
        );
        assert!(report.glyphs_removed >= 6, "SECRET is 6 glyphs");
        assert!(report.marks_applied >= 1);
    }

    #[test]
    fn apply_forces_full_rewrite_dropping_prior_revisions() {
        // `marked` is an incremental save (base revision holds the
        // un-redacted content). Apply full-rewrites from it: the output
        // must carry NO /Prev (single revision) and NOT contain the text.
        let marked = mark_and_save(&redactable_pdf());
        let doc = Document::from_bytes(marked).unwrap();
        let (out, _) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();
        let back = Document::from_bytes(out.clone()).unwrap();
        assert!(
            back.trailer().get(b"Prev").is_none(),
            "a redaction full rewrite must have no /Prev (no prior revision)"
        );
        assert!(!contains(&out, b"SECRET"));
    }

    // -- advance preservation --------------------------------------------

    #[test]
    fn surviving_text_stays_positioned_after_a_mid_string_redaction() {
        // Extract the original 'P' of PUBLIC, then the redacted 'P', and
        // assert it did not shift (the advance-preserving TJ compensates
        // for the removed SECRET run).
        let original = Document::from_bytes(redactable_pdf()).unwrap();
        let orig_x = first_glyph_x(&original, 'P').expect("original P");

        let marked = mark_and_save(&redactable_pdf());
        let doc = Document::from_bytes(marked).unwrap();
        let (out, _) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();
        let back = Document::from_bytes(out).unwrap();
        let new_x = first_glyph_x(&back, 'P').expect("redacted P");

        assert!(
            (orig_x - new_x).abs() < 1.0,
            "PUBLIC's 'P' shifted from {orig_x} to {new_x} after redaction (advance not preserved)"
        );
    }

    /// The device-space x of the first glyph whose character is `ch`.
    fn first_glyph_x(doc: &Document, ch: char) -> Option<f32> {
        let text = text_extract::extract_document(doc, &ExtractOptions::default()).ok()?;
        for page in &text.pages {
            for run in &page.runs {
                for g in &run.glyphs {
                    let start = g.text_start as usize;
                    let seg = run.text.get(start..start + g.text_len as usize)?;
                    if seg.starts_with(ch) {
                        return Some(g.x);
                    }
                }
            }
        }
        None
    }

    // -- image refuse ----------------------------------------------------

    #[test]
    fn a_region_over_an_image_is_refused_not_masked() {
        let img = "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
                   /BitsPerComponent 8 /ColorSpace /DeviceGray /Length 1 >>\nstream\n\x00\nendstream";
        let content = b"q 100 0 0 50 50 100 cm /Im1 Do Q";
        let stream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        );
        let pdf = assemble(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
                 /Resources << /XObject << /Im1 5 0 R >> >> /Contents 4 0 R >>",
                &stream,
                img,
            ],
            "",
        );
        // Author a redaction over the image placement (50,100)-(150,150).
        let doc = Document::from_bytes(pdf).unwrap();
        let mut session = EditSession::new(doc);
        let spec = crate::annot_author::RedactSpec {
            quads: vec![crate::annot_author::Quad::from_rect(Rect::from_corners(
                60.0, 110.0, 120.0, 140.0,
            ))],
            fill: None,
            overlay_text: None,
            quadding: crate::vartext::Quadding::Left,
        };
        session.add_redaction(0, &spec).unwrap();
        let (marked, _) = session
            .to_incremental_bytes(&SaveOptions::identity())
            .unwrap();

        let doc = Document::from_bytes(marked).unwrap();
        let err = apply_redactions(&doc, &SaveOptions::identity()).unwrap_err();
        assert!(
            matches!(err, RedactError::ImageRegion { page: 1 }),
            "expected a named image refusal, got {err:?}"
        );
    }

    // -- carrier scrub (/Info) -------------------------------------------

    #[test]
    fn info_dictionary_strings_are_scrubbed_and_disclosed() {
        let content = b"BT /F1 24 Tf 20 100 Td (SECRET PUBLIC) Tj ET";
        let stream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        );
        let pdf = assemble(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
                 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
                &stream,
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
                "<< /Title (SECRET dossier) /Author (Nobody) >>",
            ],
            "/Info 6 0 R",
        );
        let marked = mark_and_save(&pdf);
        let doc = Document::from_bytes(marked).unwrap();
        let (out, report) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();
        assert!(!contains(&out, b"SECRET"), "SECRET survived in /Info");
        assert!(report.info_strings_scrubbed >= 1);
        // The non-matching /Author entry survives.
        assert!(contains(&out, b"Nobody"));
        // The carrier is reported as scrubbed.
        assert!(
            report
                .carriers
                .iter()
                .any(|c| c.carrier == "info" && c.action == CarrierAction::Scrubbed)
        );
    }

    #[test]
    fn a_structure_tree_is_disclosed_as_an_unscrubbed_residual() {
        // A tagged document's /ActualText//Alt//E can duplicate redacted
        // glyphs; this build detects and DISCLOSES it (never silently
        // leaves it), triggering the refusal-acknowledgement gate.
        let content = b"BT /F1 24 Tf 20 100 Td (SECRET PUBLIC) Tj ET";
        let stream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        );
        let pdf = assemble(
            &[
                "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
                 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
                &stream,
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
                "<< /Type /StructTreeRoot /K [] >>",
            ],
            "",
        );
        let marked = mark_and_save(&pdf);
        let doc = Document::from_bytes(marked).unwrap();
        let (out, report) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();
        // The visible content is still removed...
        assert!(!contains(&out, b"SECRET"));
        // ...but the structure tree is disclosed, not silently scrubbed.
        assert!(
            report.has_disclosed_residuals(),
            "a present structure tree must be disclosed as a residual"
        );
        assert!(report.carriers.iter().any(|c| {
            c.carrier == "struct_tree" && c.action == CarrierAction::DisclosedNotScrubbed
        }));
    }

    #[test]
    fn nothing_to_apply_is_a_named_error() {
        let doc = Document::from_bytes(redactable_pdf()).unwrap();
        let err = apply_redactions(&doc, &SaveOptions::identity()).unwrap_err();
        assert!(matches!(err, RedactError::NothingToApply));
    }

    // -- container decomposition (§7.5.7 Strategy B) ---------------------

    /// A big-endian byte slice of `v` in `width` bytes (xref-stream field).
    fn be(v: u64, width: usize) -> Vec<u8> {
        v.to_be_bytes().get(8 - width..).unwrap_or(&[]).to_vec()
    }

    /// The body of an object stream holding `objects` (§7.5.7 layout: the
    /// `N` pairs, then the values at `/First`).
    fn objstm_body_local(objects: &[(u32, &str)]) -> String {
        let mut header = String::new();
        let mut body = String::new();
        for (num, text) in objects {
            header.push_str(&format!("{num} {} ", body.len()));
            body.push_str(text);
            body.push(' ');
        }
        let first = header.len();
        let data = format!("{header}{body}");
        format!(
            "<< /Type /ObjStm /N {} /First {first} /Length {} >>\nstream\n{data}\nendstream",
            objects.len(),
            data.len(),
        )
    }

    /// A PDF whose page tree, page dict and `/Info` dict live **compressed
    /// inside an object stream** (obj 6), reached via a cross-reference
    /// stream (obj 7). The `/Info` carries "SECRET" — the vector that would
    /// survive verbatim inside the untouched container if decomposition
    /// were not performed.
    fn build_objstm_pdf() -> Vec<u8> {
        let content = b"BT /F1 24 Tf 20 100 Td (SECRET PUBLIC) Tj ET";
        let cstream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            std::str::from_utf8(content).unwrap()
        );
        let objstm = objstm_body_local(&[
            (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (
                3,
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] \
                 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            ),
            (8, "<< /Title (SECRET dossier) /Author (Nobody) >>"),
        ]);
        let file_objs: Vec<(u32, String)> = vec![
            (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
            (4, cstream),
            (
                5,
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
            ),
            (6, objstm),
        ];
        let mut buf = b"%PDF-1.5\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        for (num, body) in &file_objs {
            offsets.push((*num, buf.len()));
            buf.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref_num = 7u32;
        let xref_at = buf.len();
        offsets.push((xref_num, xref_at));
        let size = 9u32;
        let mut data = Vec::new();
        for num in 0..size {
            let (t, f2, f3): (u64, u64, u64) = if num == 0 {
                (0, 0, 65535)
            } else if let Some((_, off)) = offsets.iter().find(|(n, _)| *n == num) {
                (1, *off as u64, 0)
            } else {
                match num {
                    2 => (2, 6, 0),
                    3 => (2, 6, 1),
                    8 => (2, 6, 2),
                    _ => (0, 0, 0),
                }
            };
            data.extend(be(t, 1));
            data.extend(be(f2, 4));
            data.extend(be(f3, 2));
        }
        let dict = format!(
            "<< /Type /XRef /Size {size} /W [1 4 2] /Root 1 0 R /Info 8 0 R /Length {} >>",
            data.len()
        );
        buf.extend_from_slice(format!("{xref_num} 0 obj\n{dict}\nstream\n").as_bytes());
        buf.extend_from_slice(&data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        buf.extend_from_slice(format!("startxref\n{xref_at}\n%%EOF\n").as_bytes());
        buf
    }

    #[test]
    fn redacting_content_with_an_objstm_info_decomposes_the_container() {
        // Sanity: the fixture loads and /Info came from the object stream.
        let pdf = build_objstm_pdf();
        let d0 = Document::from_bytes(pdf.clone()).unwrap();
        let info_id = d0
            .trailer()
            .get(b"Info")
            .and_then(Object::as_reference)
            .unwrap();
        assert_eq!(
            d0.get(info_id).unwrap().provenance.container(),
            Some(ObjId::new(6, 0)),
            "the /Info dict must start life compressed in object stream 6"
        );

        let marked = mark_and_save(&pdf);
        let doc = Document::from_bytes(marked).unwrap();
        let (out, report) = apply_redactions(&doc, &SaveOptions::identity()).unwrap();

        // If the container were re-emitted verbatim, the compressed /Info's
        // "SECRET dossier" would survive. It must not.
        assert!(
            !contains(&out, b"SECRET"),
            "SECRET survived — the object stream was not decomposed"
        );
        let back = Document::from_bytes(out.clone()).unwrap();
        assert!(!contains(&all_decoded_content(&back), b"SECRET"));
        assert!(
            report.containers_decomposed >= 1,
            "the /Info's object stream must be decomposed"
        );
        assert!(report.info_strings_scrubbed >= 1);
        // The unrelated /Author survives the scrub + decomposition.
        assert!(contains(&out, b"Nobody"));
    }
}
