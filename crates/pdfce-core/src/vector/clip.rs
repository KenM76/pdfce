//! # The object clipboard — copy and paste page content (`Pass 120.0`)
//!
//! The model half of `EditSession::copy_objects` / `paste_objects`: what a
//! copied selection **is**, and the planning that turns one back into content
//! on another page. The session owns the object allocation, the staging buffer
//! and the undo command; everything else is here.
//!
//! ## ★ What the requesting shell got right, and the half it did not see
//!
//! `pdfceGUI` asked for this on the reading that `EditSession::import_object`
//! already does the hard part — a recursive cross-document object-graph copy
//! with reference remapping, cycle handling and stream re-staging — so the ask
//! was *"expose the one you have at object granularity"*.
//!
//! **That reading is correct, and it is the smaller half.** `import_object`
//! copies *indirect objects*. A page's content objects are not indirect
//! objects: a path, a text run and an image invocation are **byte ranges
//! inside a content stream**, and the operators in those bytes refer to
//! resources **by NAME** — `/F1 12 Tf`, `/Im1 Do`. Those names are page-local.
//!
//! So the genuinely hard part is the one nobody asked about: **on the
//! destination page, `/F1` is a different font.** Pasting the bytes verbatim
//! draws the right shapes in the wrong typeface, or draws nothing, and neither
//! failure errors. Copy therefore records which names each item consumes and
//! carries the objects behind them; paste re-binds every one to a fresh
//! non-colliding name on the destination page and **rewrites the names inside
//! the copied bytes**. See [`plan_paste`].
//!
//! ## The clip owns its resources; it does not point at them
//!
//! [`ObjectClip`] carries the transitive closure of every object its items
//! reference, **by value**, with stream payloads owned as bytes rather than as
//! spans into a document that may already be closed. Three things fall out of
//! that, and the third is why it is worth the copy:
//!
//! 1. **Copy, close the source, paste** works.
//! 2. **Cross-document paste** is the same code path as same-document paste —
//!    there is no source view to consult at paste time, so there is no case
//!    to special-case.
//! 3. **`ObjectClip::to_bytes` (`Pass 120.1`) becomes a serialisation problem
//!    rather than a design problem.** A clip that referenced its source could
//!    not be serialised at all without inventing this structure later.
//!
//! ## Placement
//!
//! An item's bytes were written under the CTM in force where they were copied
//! from. Appended at the end of a destination page's content — where the CTM
//! is the identity by the same convention `add_image` and `add_text` already
//! rely on — they are wrapped in `q <M> … Q` with
//!
//! ```text
//!     M = source_ctm × at
//! ```
//!
//! so the marks land exactly where they were, transformed by the caller's
//! page-space matrix. `at = Matrix::IDENTITY` is paste-in-place;
//! `Matrix::translate(dx, dy)` is paste-with-offset; `Matrix::about` gives
//! paste-scaled and paste-rotated. One verb, four gestures — the requester's
//! own argument, and the same one that made `transform_objects` take a matrix.

use std::collections::{BTreeMap, BTreeSet};

use crate::content::{ContentStream, ContentTokenKind};
use crate::object::{Dict, Name, Object};
use crate::span::ByteSpan;

use super::decompose::VectorObject;
use super::geometry::{Bounds, Matrix};

/// The clip format version pdfce writes and the highest it reads.
///
/// A clip from a **newer** build is refused rather than guessed at — the same
/// posture the ce-dimension sidecar takes, and for the same reason: the
/// operator runs two builds side by side out of two folders, by design, and
/// *will* copy in one and paste in the other. `Pass 120.1` (`to_bytes`) is
/// what makes that reachable; the version is carried from the start so the
/// refusal exists before the payload can travel.
pub const CLIP_VERSION: u32 = 1;

/// The seven resource categories a content stream can name (§7.8.3 Table 33).
///
/// `/ProcSet` is deliberately absent: it is deprecated in PDF 2.0 and names no
/// object, so nothing in a copied byte range can refer to it.
pub const RESOURCE_CATEGORIES: [&[u8]; 7] = [
    b"Font",
    b"XObject",
    b"ExtGState",
    b"ColorSpace",
    b"Pattern",
    b"Shading",
    b"Properties",
];

/// One object on the clipboard: its content-stream bytes, verbatim, plus
/// everything needed to re-place and re-bind it elsewhere.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClipItem {
    /// The object's operator run, copied byte-for-byte from the source
    /// content stream.
    ///
    /// **Verbatim on purpose.** Re-emitting from a parsed model would
    /// normalise operand spelling, expand an `re`, and lose whatever the
    /// producer wrote — a paste that changes the drawing's bytes for no
    /// reason the operator asked for. Only the resource *names* are rewritten,
    /// and only where an operator consumes one.
    pub bytes: Vec<u8>,
    /// The CTM in force where the bytes were copied from.
    pub ctm: Matrix,
    /// A short kind label (`"path"` / `"text"` / `"image"`), for diagnostics
    /// and for the shell's own paste summary.
    pub kind: &'static str,
    /// The object's page-space bounds at copy time.
    pub bbox: Bounds,
    /// Every `(category, name)` the bytes consume, paired with the clip-local
    /// object number carrying it. Sorted, so a clip is deterministic.
    pub bindings: Vec<ClipBinding>,
    /// ★ **The graphics state this item DEPENDS ON but does not itself
    /// establish**, as content-stream operators, emitted inside the paste
    /// wrapper immediately before [`Self::bytes`] (`Pass 120.2`).
    ///
    /// # The defect this exists to fix, found on a real file and not on a
    /// fixture
    ///
    /// Text state is **graphics state** (§8.4.1 Table 52), so a producer may
    /// set `/F8 12 Tf` **once** and then emit many `BT`…`ET` blocks that
    /// inherit it. That is exactly what a CAD exporter does — and a text
    /// object's byte span is its `BT`…`ET`, so **the `Tf` is not in it.**
    ///
    /// Copying such an object recorded no font binding (there was no name in
    /// the bytes to bind) and pasted content that showed text **with no font
    /// selected at all**. pdfce's own extractor said so about the export:
    /// *"a show operator appeared with no font selected (§9.4.1 requires Tf
    /// first)"* — `chars=0 codes=4 failed=4`. Nothing errored at copy or at
    /// paste.
    ///
    /// The same argument applies to every other inherited state a copied
    /// object relies on: a path stroked after `0.5 g 2 w` carries neither
    /// operator in its own span, so it would paste black and hairline.
    ///
    /// # Why a prelude rather than rewriting the bytes
    ///
    /// [`Self::bytes`] stays **verbatim** (see its own note). Prepending to it
    /// would normalise nothing today and everything eventually, and it would
    /// make the copied bytes no longer comparable with the source's. A
    /// separate field keeps "what the producer wrote" and "what pdfce had to
    /// re-establish" distinguishable — which also makes the second one
    /// **disclosable**, and it is.
    pub prelude: Vec<u8>,
}

/// One resource-name reference inside a [`ClipItem`]'s bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ClipBinding {
    /// The `/Resources` category (`Font`, `XObject`, …).
    pub category: Vec<u8>,
    /// The name as it appears in the copied bytes (`F1`, `Im1`, …).
    pub name: Vec<u8>,
    /// The clip-local object number the name resolved to at copy time.
    pub object: u32,
}

/// One object in a clip's owned resource closure.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClipObject {
    /// The object's value, with references renumbered into clip-local space.
    ///
    /// A [`Object::Stream`]'s `data_span` here is **meaningless** — the bytes
    /// live in [`Self::payload`]. That is the price of owning the graph rather
    /// than pointing at it, and it is why this type exists instead of a bare
    /// `Object`.
    pub value: Object,
    /// A stream object's decoded-or-raw payload, owned.
    pub payload: Option<Vec<u8>>,
}

/// One annotation on the clipboard (`Pass 120.4`).
///
/// # ★ Why annotations are a SEPARATE payload rather than more `items`
///
/// A `ClipItem` is a byte range in a content stream. An annotation is not
/// content at all — it is a dictionary in the page's `/Annots`, with its own
/// coordinate convention, its own appearance stream, and, for the two kinds
/// pdfce authors itself, its own registration outside the page: a ce dimension
/// has a `/PieceInfo` sidecar record and a group; a widget has an `/AcroForm`
/// field-tree entry and a field name that must not collide.
///
/// **The original acceptance criteria for this Pass said "refuse loudly", and
/// that was written before `120.0` shipped.** Once copy addressed content
/// objects by paint-order index, there was no index by which those verbs could
/// even *name* an annotation to refuse it — the refusal had nowhere to live.
/// This is the address space that gives it one, and having built it, most of
/// the kinds turned out to be paste-able rather than refuse-able.
///
/// # Copied through the MODEL, not through the object graph
///
/// A raw dictionary copy would be structurally right and semantically wrong: a
/// pasted ce dimension would carry a `/PieceInfo` record naming a group that
/// does not exist in the destination, and a pasted widget would carry a field
/// name that already means something there. So a markup annotation round-trips
/// through [`MarkupSpec`](crate::annot_author::MarkupSpec) and a ce dimension
/// through its [`DimensionKind`](crate::dimension::DimensionKind) — the same
/// models `add_markup` and `add_dimension` author from, so the destination
/// re-bakes the appearance and re-registers the sidecar itself.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ClipAnnotation {
    /// A markup annotation, as the spec `add_markup` authors from.
    Markup(Box<crate::annot_author::MarkupSpec>),
    /// A **ce dimension** — pdfce's own measured annotation (project rule 15;
    /// this is never a *pdf dimension*, which is page content and travels as a
    /// [`ClipItem`]).
    ///
    /// The group is carried **by name and unit**, not by id: a `GroupId` means
    /// nothing in another document, and the destination either has a group of
    /// that name already or gets one created. That is what makes a dimension
    /// pasted between documents keep its scale and its label format rather
    /// than arriving as bare geometry.
    Dimension {
        /// The source group's name — matched, or created, on paste.
        group_name: String,
        /// The source group's unit.
        unit: crate::dimension::Unit,
        /// The measured geometry.
        kind: Box<crate::dimension::DimensionKind>,
    },
    /// An annotation kind this cut does not model — carried so the count is
    /// honest, refused by name on paste.
    ///
    /// **A widget is here deliberately.** Pasting one means registering a
    /// field in the destination's `/AcroForm` under a name that does not
    /// collide, and a *renamed* field is a different field: any JavaScript,
    /// calculation order or parent-child relationship that named the old one
    /// is silently broken. That is a decision about the operator's form, not a
    /// copy, so it is refused rather than guessed at.
    Unsupported {
        /// The annotation's `/Subtype`, so the refusal can name it.
        subtype: String,
    },
}

impl ClipAnnotation {
    /// A short kind label for a paste summary.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Markup(_) => "markup".to_owned(),
            Self::Dimension { .. } => "ce dimension".to_owned(),
            Self::Unsupported { subtype } => format!("{subtype} (unsupported)"),
        }
    }
}

/// A copied selection of page objects — the clipboard payload (`Pass 120.0`).
///
/// Opaque by intent: the shell moves one of these around, hands it back to
/// [`paste_objects`](crate::edit::EditSession::paste_objects), and asks it only
/// the questions a paste UI needs ([`Self::len`], [`Self::bbox`],
/// [`Self::kinds`]). Its internals are public for the crate's own planner and
/// for `Pass 120.1`'s serialiser, not as a supported poke-at-it surface.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ObjectClip {
    /// The format version this clip was written at. See [`CLIP_VERSION`].
    pub version: u32,
    /// The copied objects, in the source page's paint order.
    ///
    /// **Paint order is preserved deliberately.** Pasting them back in a
    /// different order would restack them — a filled shape that was behind
    /// text arriving in front of it — which is a visible change nobody asked
    /// for and which no error would report.
    pub items: Vec<ClipItem>,
    /// The resource closure, keyed by clip-local object number.
    pub objects: BTreeMap<u32, ClipObject>,
    /// The union of the items' page-space bounds at copy time — what a shell
    /// needs to draw a paste preview outline before it commits.
    pub bbox: Bounds,
    /// The copied **annotations** (`Pass 120.4`) — a separate payload from
    /// [`Self::items`], for the reasons on [`ClipAnnotation`].
    ///
    /// **Not serialised by [`Self::to_bytes`] in this cut**, and that is a
    /// stated limit rather than an oversight: `MarkupSpec` and `DimensionKind`
    /// are rich enums whose byte encoding would be a second format to version
    /// alongside the content one, and getting it wrong means a clip that
    /// parses and pastes the wrong shape. An in-session or in-process
    /// annotation clipboard works today; a serialised one is its own decision.
    /// [`Self::to_bytes`] therefore drops them and
    /// [`Self::annotations_survive_serialisation`] says so, rather than
    /// letting a caller discover it from a count.
    pub annotations: Vec<ClipAnnotation>,
}

impl ObjectClip {
    /// How many objects the clip holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the clip holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The union of the copied objects' page-space bounds, as copied.
    #[must_use]
    pub const fn bbox(&self) -> Bounds {
        self.bbox
    }

    /// The distinct object kinds on the clip, in a stable order — for a paste
    /// summary that says *"3 paths and a text run"* rather than *"4 objects"*.
    #[must_use]
    pub fn kinds(&self) -> Vec<&'static str> {
        let set: BTreeSet<&'static str> = self.items.iter().map(|i| i.kind).collect();
        set.into_iter().collect()
    }

    /// How many resource objects travel with the clip.
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.objects.len()
    }

    /// How many annotations the clip holds (`Pass 120.4`).
    #[must_use]
    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }

    /// Whether [`Self::to_bytes`] preserves this clip completely.
    ///
    /// `false` when it holds annotations, which this cut does not serialise —
    /// see [`Self::annotations`]. Published as a **question a caller can ask**
    /// rather than left to be discovered from a count that silently drops:
    /// a shell writing a clip to disk can warn, or keep the in-process copy,
    /// instead of finding out on the paste.
    #[must_use]
    pub fn annotations_survive_serialisation(&self) -> bool {
        self.annotations.is_empty()
    }
}

/// Why a copy or paste was refused, by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ClipError {
    /// A resource name the copied bytes consume does not resolve in the source
    /// page's `/Resources`.
    ///
    /// Copied anyway, the paste would draw text in no font, or invoke an
    /// XObject that is not there — a silent nothing rather than an error.
    /// Refused at COPY time, which is the earliest point at which the operator
    /// can be told, and while their selection is still on screen.
    #[error(
        "the selection uses the resource /{name} ({category}), which does not resolve on this page -- copying it would paste content that draws nothing"
    )]
    UnresolvedResource {
        /// The resource category.
        category: String,
        /// The name as written in the content stream.
        name: String,
    },
    /// The clip was written by a newer build of pdfce.
    ///
    /// Refused rather than partially understood: a format this build does not
    /// know may carry items whose bytes depend on structure it would drop, and
    /// pasting the part it recognises is exactly the silent-subset behaviour
    /// `R168` exists to end.
    #[error(
        "this clipboard payload was written by a newer build of pdfce (format {found}; this build reads {supported}) -- refusing rather than pasting the part it understands"
    )]
    NewerFormat {
        /// The version found on the payload.
        found: u32,
        /// The highest version this build reads.
        supported: u32,
    },
    /// The payload does not begin with [`CLIP_MAGIC`].
    ///
    /// Checked **first**, before any length prefix is read, so an unrelated
    /// payload — the OS clipboard handing back something else, a truncated
    /// file, a text file — is refused with a sentence rather than with
    /// whatever a length prefix read out of the wrong bytes.
    #[error("this is not a pdfce clipboard payload (it does not carry the clip signature)")]
    NotAClip,
    /// The payload ended mid-structure.
    #[error("the clipboard payload is truncated -- it ends in the middle of a value")]
    Truncated,
    /// A clip item names a clip-local object the clip does not carry.
    ///
    /// Impossible from [`copy_objects`](crate::edit::EditSession::copy_objects)'s
    /// own output; reachable from a hand-built or truncated payload, which is
    /// exactly what `Pass 120.1` will make possible.
    #[error(
        "the clipboard payload is inconsistent: it references object {object}, which it does not carry"
    )]
    DanglingClipObject {
        /// The clip-local object number.
        object: u32,
    },
    /// The item's content bytes could not be parsed.
    #[error("a clipboard item's content could not be parsed: {0}")]
    Content(String),
}

// ===========================================================================
// Serialisation (`Pass 120.1`)
// ===========================================================================

/// The magic bytes every serialised clip starts with.
///
/// Present so a shell that registers this on the system clipboard, or writes
/// it to a file, can tell a pdfce clip from any other private format **before**
/// parsing it — and so a truncated or unrelated payload is refused with a
/// sentence rather than with whatever a length prefix read out of the wrong
/// bytes.
pub const CLIP_MAGIC: &[u8; 12] = b"PDFCECLIP\x00\x00\x01";

/// A little-endian `u32` appended to `out`.
fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A little-endian `f64` appended to `out`.
///
/// Bit-exact rather than decimal: a clip round-trips a CTM, and a matrix that
/// changes in the last place on every copy/paste cycle would drift a shape
/// visibly after enough of them, for no reason the operator caused.
fn put_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A length-prefixed byte string appended to `out`.
fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_u32(out, u32::try_from(bytes.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(bytes);
}

/// A cursor over a serialised clip, which refuses rather than panicking on a
/// short read.
struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ClipError> {
        let end = self.at.checked_add(n).ok_or(ClipError::Truncated)?;
        let slice = self.buf.get(self.at..end).ok_or(ClipError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, ClipError> {
        // `try_into` rather than four indexes: `take` already guaranteed the
        // length, and the conversion says so to the compiler instead of to a
        // reader. A short read is `Truncated` either way -- there is no path
        // here that panics.
        let b: [u8; 4] = self.take(4)?.try_into().map_err(|_| ClipError::Truncated)?;
        Ok(u32::from_le_bytes(b))
    }

    fn f64(&mut self) -> Result<f64, ClipError> {
        let b: [u8; 8] = self.take(8)?.try_into().map_err(|_| ClipError::Truncated)?;
        Ok(f64::from_le_bytes(b))
    }

    fn bytes(&mut self) -> Result<Vec<u8>, ClipError> {
        let len = self.u32()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    fn matrix(&mut self) -> Result<Matrix, ClipError> {
        Ok(Matrix {
            a: self.f64()?,
            b: self.f64()?,
            c: self.f64()?,
            d: self.f64()?,
            e: self.f64()?,
            f: self.f64()?,
        })
    }

    fn bounds(&mut self) -> Result<Bounds, ClipError> {
        let (min_x, min_y, max_x, max_y) = (self.f64()?, self.f64()?, self.f64()?, self.f64()?);
        Ok(Bounds {
            min: super::geometry::Point::new(min_x, min_y),
            max: super::geometry::Point::new(max_x, max_y),
        })
    }
}

fn put_matrix(out: &mut Vec<u8>, m: Matrix) {
    for v in [m.a, m.b, m.c, m.d, m.e, m.f] {
        put_f64(out, v);
    }
}

fn put_bounds(out: &mut Vec<u8>, b: Bounds) {
    for v in [b.min.x, b.min.y, b.max.x, b.max.y] {
        put_f64(out, v);
    }
}

/// The kind labels, as a closed set, so a round trip cannot invent one.
const KINDS: [&str; 3] = ["path", "text", "image"];

impl ObjectClip {
    /// Serialise the clip to a self-contained byte payload (`Pass 120.1`).
    ///
    /// # Why this exists, and why it is not "render the selection as a PDF"
    ///
    /// A clip that lives only inside one `EditSession` cannot be pasted into
    /// the other document tab, which is most of what the operator wants — and
    /// the requesting shell said so: *"this is what makes cross-document and
    /// cross-session paste free rather than a second feature."*
    ///
    /// It is deliberately **not** the same thing as `Pass 120.2`'s
    /// "render this selection as a standalone one-page PDF". That is an
    /// *interchange* format for other applications and is lossy in the way
    /// that matters here: a one-page PDF does not carry which byte range was
    /// which object, what each item's CTM was, or which name each item's
    /// operators consumed. Re-deriving those on the way back in is exactly the
    /// step that would make a pdfce→pdfce paste worse than a pdfce→Illustrator
    /// one.
    ///
    /// # The format, in one paragraph
    ///
    /// [`CLIP_MAGIC`], a version, then length-prefixed items and objects.
    /// Numbers are little-endian and **bit-exact** — a matrix that changed in
    /// the last place on every copy/paste cycle would drift a shape visibly
    /// after enough of them. Object *values* are written as PDF syntax by the
    /// crate's own [`write_object`](crate::writer::serialize::write_object) and
    /// read back by its own [`Parser`](crate::parser::Parser), so the COS
    /// grammar has exactly one implementation on each side rather than a
    /// second one living here.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        use crate::object::ObjId;
        use crate::writer::encoder::IdentityEncoder;
        use crate::writer::serialize::write_object;

        let mut out = Vec::new();
        out.extend_from_slice(CLIP_MAGIC);
        put_u32(&mut out, self.version);
        put_bounds(&mut out, self.bbox);

        put_u32(
            &mut out,
            u32::try_from(self.items.len()).unwrap_or(u32::MAX),
        );
        for item in &self.items {
            put_bytes(&mut out, &item.bytes);
            put_matrix(&mut out, item.ctm);
            put_bounds(&mut out, item.bbox);
            let tag = KINDS.iter().position(|k| *k == item.kind).unwrap_or(0);
            put_u32(&mut out, u32::try_from(tag).unwrap_or(0));
            put_u32(
                &mut out,
                u32::try_from(item.bindings.len()).unwrap_or(u32::MAX),
            );
            for binding in &item.bindings {
                put_bytes(&mut out, &binding.category);
                put_bytes(&mut out, &binding.name);
                put_u32(&mut out, binding.object);
            }
            put_bytes(&mut out, &item.prelude);
        }

        put_u32(
            &mut out,
            u32::try_from(self.objects.len()).unwrap_or(u32::MAX),
        );
        for (&id, object) in &self.objects {
            put_u32(&mut out, id);
            // A stream's DICTIONARY is what is written; its payload travels
            // beside it, because `Object::Stream` carries a span into a buffer
            // that does not exist here (see `ClipObject::value`).
            let (value, payload): (Object, Option<&Vec<u8>>) = match &object.value {
                Object::Stream(stream) => {
                    (Object::Dict(stream.dict.clone()), object.payload.as_ref())
                }
                other => (other.clone(), None),
            };
            let mut encoded = Vec::new();
            write_object(
                &mut encoded,
                &value,
                ObjId::new(id, 0),
                &[],
                &IdentityEncoder,
            );
            put_bytes(&mut out, &encoded);
            match payload {
                Some(bytes) => {
                    out.push(1);
                    put_bytes(&mut out, bytes);
                }
                None => out.push(0),
            }
        }
        out
    }

    /// Parse a payload written by [`Self::to_bytes`] (`Pass 120.1`).
    ///
    /// # Errors
    ///
    /// [`ClipError::NotAClip`] when the magic does not match — checked first,
    /// so an unrelated payload is refused with a sentence rather than with
    /// whatever a length prefix read out of the wrong bytes.
    /// [`ClipError::NewerFormat`] for a payload from a newer build, refused
    /// rather than partially understood. [`ClipError::Truncated`] for a short
    /// read, and [`ClipError::Content`] for an object value that is not COS
    /// syntax.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ClipError> {
        let mut r = Reader::new(bytes);
        if r.take(CLIP_MAGIC.len())? != CLIP_MAGIC.as_slice() {
            return Err(ClipError::NotAClip);
        }
        let version = r.u32()?;
        if version > CLIP_VERSION {
            return Err(ClipError::NewerFormat {
                found: version,
                supported: CLIP_VERSION,
            });
        }
        let bbox = r.bounds()?;

        let item_count = r.u32()? as usize;
        let mut items = Vec::with_capacity(item_count.min(4096));
        for _ in 0..item_count {
            let item_bytes = r.bytes()?;
            let ctm = r.matrix()?;
            let item_bbox = r.bounds()?;
            let tag = r.u32()? as usize;
            let kind = KINDS.get(tag).copied().unwrap_or("path");
            let binding_count = r.u32()? as usize;
            let mut bindings = Vec::with_capacity(binding_count.min(4096));
            for _ in 0..binding_count {
                bindings.push(ClipBinding {
                    category: r.bytes()?,
                    name: r.bytes()?,
                    object: r.u32()?,
                });
            }
            let prelude = r.bytes()?;
            items.push(ClipItem {
                bytes: item_bytes,
                ctm,
                kind,
                bbox: item_bbox,
                bindings,
                prelude,
            });
        }

        let object_count = r.u32()? as usize;
        let mut objects = BTreeMap::new();
        for _ in 0..object_count {
            let id = r.u32()?;
            let encoded = r.bytes()?;
            let value = crate::parser::Parser::at(&encoded, 0)
                .parse_object()
                .map_err(|e| ClipError::Content(e.to_string()))?;
            let payload = if r.take(1)?.first().copied().unwrap_or(0) == 1 {
                Some(r.bytes()?)
            } else {
                None
            };
            // A stream is reconstructed from its dictionary plus its payload;
            // the span is meaningless by construction, exactly as it was on
            // the way out.
            let value = match (&value, &payload) {
                (Object::Dict(dict), Some(bytes)) => Object::Stream(crate::object::Stream {
                    dict: dict.clone(),
                    data_span: ByteSpan::new(0, bytes.len()),
                }),
                _ => value,
            };
            objects.insert(id, ClipObject { value, payload });
        }

        Ok(Self {
            version,
            items,
            objects,
            bbox,
            // Not carried by this format -- see `ObjectClip::annotations`.
            annotations: Vec::new(),
        })
    }
}

// ===========================================================================
// Interchange: a standalone one-page PDF (`Pass 120.2`)
// ===========================================================================

/// What [`ObjectClip::to_pdf`] produced, beside the bytes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClipPdf {
    /// The complete, standalone one-page PDF.
    pub bytes: Vec<u8>,
    /// The page size in points — the clip's own bounds, so the sheet is the
    /// selection and nothing else.
    pub size: (f64, f64),
    /// How many objects were drawn onto it.
    pub objects: usize,
    /// Whether the clip's bounds were **empty or degenerate** and a minimum
    /// page size was substituted.
    ///
    /// A zero-area `/MediaBox` is not merely ugly: §7.7.3.3 requires a
    /// rectangle, and a reader given one of zero extent shows an empty window
    /// or refuses the file. An operator who copied a zero-height rule and got
    /// back a document that will not open would have no way to tell which of
    /// the two steps failed, so the substitution happens and is **disclosed**
    /// rather than being left to look like corruption.
    pub size_substituted: bool,
}

/// The smallest page pdfce will emit for a degenerate selection, in points.
///
/// One point, not zero: large enough to be a legal rectangle, small enough
/// that nobody mistakes it for a deliberate page size.
const MIN_CLIP_PAGE: f64 = 1.0;

impl ObjectClip {
    /// Render the clip as a **standalone one-page PDF** (`Pass 120.2`).
    ///
    /// The page is exactly the selection's bounding box, with the content
    /// translated so it sits at the origin — so a consumer that places the
    /// file gets the objects and no surrounding whitespace.
    ///
    /// # ★ Why this is NOT [`Self::to_bytes`], and must not be merged with it
    ///
    /// They serve different consumers and the difference is lossy in one
    /// direction only.
    ///
    /// [`Self::to_bytes`] is **pdfce's own format**: it carries which byte
    /// range was which object, each item's per-object CTM, and which resource
    /// names each item's operators consumed. A PDF carries **none** of those.
    /// Reading this file back in would mean re-decomposing a page and guessing
    /// at the structure the clip already knew exactly — which would make a
    /// pdfce→pdfce round trip *worse* than a pdfce→Illustrator one, and for no
    /// gain, since the private format already exists.
    ///
    /// So: **this is an export, not a serialisation.** Put both on the system
    /// clipboard — the private format for a pdfce→pdfce paste, this for
    /// everyone else — which is exactly the split the requesting shell asked
    /// for: *"I am not asking you to touch the OS clipboard. That is mine."*
    ///
    /// # What it does not carry, stated rather than discovered
    ///
    /// The clip's resource closure travels, so fonts and images arrive. What
    /// does **not** is anything that was never in the clip: annotations,
    /// optional-content membership, structure tags. A selection is content,
    /// and this is that content on a page of its own.
    #[must_use]
    pub fn to_pdf(&self) -> ClipPdf {
        use crate::object::{ObjId, Stream};
        use crate::settings::{TrailingEol, XrefEntryEol};
        use crate::writer::encoder::IdentityEncoder;
        use crate::writer::fileid;
        use crate::writer::serialize;
        use crate::writer::xref_out;
        use crate::xref::XrefEntry;

        // ---- geometry -------------------------------------------------
        let degenerate = self.bbox.min.x > self.bbox.max.x
            || !self.bbox.min.x.is_finite()
            || !self.bbox.max.y.is_finite();
        let (width, height) = if degenerate {
            (MIN_CLIP_PAGE, MIN_CLIP_PAGE)
        } else {
            (
                (self.bbox.max.x - self.bbox.min.x).max(MIN_CLIP_PAGE),
                (self.bbox.max.y - self.bbox.min.y).max(MIN_CLIP_PAGE),
            )
        };
        let size_substituted = degenerate
            || (self.bbox.max.x - self.bbox.min.x) < MIN_CLIP_PAGE
            || (self.bbox.max.y - self.bbox.min.y) < MIN_CLIP_PAGE;
        let origin = if degenerate {
            Matrix::IDENTITY
        } else {
            Matrix::translate(-self.bbox.min.x, -self.bbox.min.y)
        };

        // ---- objects ---------------------------------------------------
        //
        // Numbering: 1 catalog, 2 pages, 3 page, 4 content, then the clip's
        // own objects at 5.., in clip-local order. A clip-local reference `n`
        // therefore maps to `n + 4`, which is why the remap below is
        // arithmetic rather than a map — clip ids are already dense and
        // 1-based by construction (`clip_import` allocates them that way).
        const FIRST: u32 = 5;
        let shift = |id: u32| ObjId::new(id.saturating_add(FIRST - 1), 0);

        let mut names: Vec<(Vec<u8>, Vec<u8>, u32)> = Vec::new();
        let mut chosen: BTreeMap<(Vec<u8>, u32), Vec<u8>> = BTreeMap::new();
        let mut taken: BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>> = BTreeMap::new();
        let empty = Dict::new();
        for item in &self.items {
            for binding in &item.bindings {
                let key = (binding.category.clone(), binding.object);
                if chosen.contains_key(&key) {
                    continue;
                }
                let used = taken.entry(binding.category.clone()).or_default();
                let name = free_name(&empty, &binding.category, used);
                used.insert(name.clone());
                names.push((binding.category.clone(), name.clone(), binding.object));
                chosen.insert(key, name);
            }
        }

        let mut content = Vec::new();
        for item in &self.items {
            let mut mapping: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>> = BTreeMap::new();
            for binding in &item.bindings {
                if let Some(name) = chosen.get(&(binding.category.clone(), binding.object)) {
                    mapping.insert(
                        (binding.category.clone(), binding.name.clone()),
                        name.clone(),
                    );
                }
            }
            // A rewrite failure here means the item's own bytes do not parse,
            // which `copy_objects` could not have produced. Emitting them
            // unrewritten would bind to nothing; emitting nothing at all loses
            // the object silently. The bytes are emitted verbatim, which at
            // worst draws in a default state -- the same degradation
            // `serialize` documents for an unresolvable span.
            let rewritten =
                rewrite_names(&item.bytes, &mapping).unwrap_or_else(|_| item.bytes.clone());
            let placement = item.ctm.post_concat(origin);
            content.extend_from_slice(b"\nq ");
            for v in [
                placement.a,
                placement.b,
                placement.c,
                placement.d,
                placement.e,
                placement.f,
            ] {
                crate::writer::content::emit_number(&mut content, v);
                content.push(b' ');
            }
            content.extend_from_slice(b"cm\n");
            if !item.prelude.is_empty() {
                content.extend_from_slice(
                    &rewrite_names(&item.prelude, &mapping)
                        .unwrap_or_else(|_| item.prelude.clone()),
                );
                content.push(b'\n');
            }
            content.extend_from_slice(&rewritten);
            content.extend_from_slice(b"\nQ");
        }

        let mut resources = Dict::new();
        for (category, name, clip_object) in &names {
            let mut sub = resources
                .get(category)
                .and_then(Object::as_dict)
                .cloned()
                .unwrap_or_default();
            sub.insert(Name(name.clone()), Object::Reference(shift(*clip_object)));
            resources.insert(Name(category.clone()), Object::Dict(sub));
        }

        let mut catalog = Dict::new();
        catalog.insert(Name::from(b"Type"), Object::Name(Name::from(b"Catalog")));
        catalog.insert(Name::from(b"Pages"), Object::Reference(ObjId::new(2, 0)));

        let mut pages = Dict::new();
        pages.insert(Name::from(b"Type"), Object::Name(Name::from(b"Pages")));
        pages.insert(
            Name::from(b"Kids"),
            Object::Array(vec![Object::Reference(ObjId::new(3, 0))]),
        );
        pages.insert(Name::from(b"Count"), Object::Integer(1));

        let mut page = Dict::new();
        page.insert(Name::from(b"Type"), Object::Name(Name::from(b"Page")));
        page.insert(Name::from(b"Parent"), Object::Reference(ObjId::new(2, 0)));
        page.insert(
            Name::from(b"MediaBox"),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );
        page.insert(Name::from(b"Resources"), Object::Dict(resources));
        page.insert(Name::from(b"Contents"), Object::Reference(ObjId::new(4, 0)));

        // ---- staging + emission ---------------------------------------
        //
        // One staging buffer for every stream payload, exactly as
        // `pageops::assemble` does it, so `write_indirect`'s span model has a
        // single buffer to index.
        let mut staging: Vec<u8> = Vec::new();
        let stage = |bytes: &[u8], staging: &mut Vec<u8>| -> ByteSpan {
            let start = staging.len();
            staging.extend_from_slice(bytes);
            ByteSpan::new(start, bytes.len())
        };

        let mut objects: BTreeMap<u32, Object> = BTreeMap::new();
        objects.insert(1, Object::Dict(catalog));
        objects.insert(2, Object::Dict(pages));
        objects.insert(3, Object::Dict(page));

        let content_span = stage(&content, &mut staging);
        let mut content_dict = Dict::new();
        content_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(content.len()).unwrap_or(i64::MAX)),
        );
        objects.insert(
            4,
            Object::Stream(Stream {
                dict: content_dict,
                data_span: content_span,
            }),
        );

        for (&clip_id, object) in &self.objects {
            let value = match (&object.value, &object.payload) {
                (Object::Stream(stream), Some(bytes)) => {
                    let span = stage(bytes, &mut staging);
                    Object::Stream(Stream {
                        dict: shift_refs(&Object::Dict(stream.dict.clone()), FIRST)
                            .as_dict()
                            .cloned()
                            .unwrap_or_default(),
                        data_span: span,
                    })
                }
                (other, _) => shift_refs(other, FIRST),
            };
            objects.insert(clip_id.saturating_add(FIRST - 1), value);
        }

        let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let body_start = out.len();
        let mut entries: BTreeMap<u32, XrefEntry> = BTreeMap::new();
        entries.insert(
            0,
            XrefEntry::Free {
                next_free: 0,
                generation: 65_535,
            },
        );
        for (number, value) in &objects {
            entries.insert(
                *number,
                XrefEntry::InUse {
                    offset: out.len() as u64,
                    generation: 0,
                },
            );
            serialize::write_indirect(
                &mut out,
                ObjId::new(*number, 0),
                value,
                &staging,
                &IdentityEncoder,
            );
        }
        let body_end = out.len();
        let highest = objects.keys().copied().max().unwrap_or(0);
        for number in 0..=highest {
            entries.entry(number).or_insert(XrefEntry::Free {
                next_free: 0,
                generation: 65_535,
            });
        }

        let mut trailer = Dict::new();
        trailer.insert(
            Name::from(b"Size"),
            Object::Integer(i64::from(highest).saturating_add(1)),
        );
        trailer.insert(Name::from(b"Root"), Object::Reference(ObjId::new(1, 0)));
        // §14.4, and derived from the body bytes with no clock involved — so
        // two exports of the same clip are byte-identical, which is what makes
        // a shell able to cache one.
        let body = out.get(body_start..body_end).unwrap_or(&[]);
        trailer.insert(
            Name::from(b"ID"),
            Object::Array(vec![
                Object::String(
                    fileid::changing_identifier(b"pdfce/clip-export/permanent", 0, body).to_vec(),
                ),
                Object::String(
                    fileid::changing_identifier(b"pdfce/clip-export/changing", 0, body).to_vec(),
                ),
            ]),
        );

        let section_offset = out.len() as u64;
        let _ = xref_out::write_classic_table(&mut out, &entries, XrefEntryEol::default());
        xref_out::write_classic_tail(&mut out, &trailer, section_offset, TrailingEol::default());

        ClipPdf {
            bytes: out,
            size: (width, height),
            objects: self.items.len(),
            size_substituted,
        }
    }
}

/// Renumber every reference in a value from clip-local space into the export's
/// numbering (clip `n` becomes `n + first - 1`).
///
/// Saturating rather than wrapping: a clip id near `u32::MAX` cannot arise
/// from [`crate::edit::EditSession::copy_objects`], which allocates densely
/// from 1, but a hand-built payload could carry one, and a wrapped reference
/// would point at the catalog.
fn shift_refs(value: &Object, first: u32) -> Object {
    match value {
        Object::Reference(id) => Object::Reference(crate::object::ObjId::new(
            id.num.saturating_add(first - 1),
            0,
        )),
        Object::Array(items) => Object::Array(items.iter().map(|v| shift_refs(v, first)).collect()),
        Object::Dict(dict) => {
            let mut out = Dict::new();
            for (key, v) in dict.iter() {
                out.insert(key.clone(), shift_refs(v, first));
            }
            Object::Dict(out)
        }
        other => other.clone(),
    }
}

/// The operators that consume a resource **name**, and the category the name
/// resolves in (§7.8.3 Table 33, and each operator's own clause).
///
/// # Why this is a table rather than a match arm per operator
///
/// The table is the whole correctness argument for name rewriting, and it has
/// to be readable as one piece: **a name rewritten in the wrong place corrupts
/// the stream, and a name NOT rewritten silently binds to whatever the
/// destination page happens to call `/F1`.** Both failures are invisible in a
/// diff. The `usize` is which operand carries the name — `0` for the first,
/// `usize::MAX` meaning "the last", which is `scn`/`SCN`'s shape.
const NAME_OPERATORS: [(&[u8], &[u8], usize); 10] = [
    (b"Tf", b"Font", 0),
    (b"Do", b"XObject", 0),
    (b"gs", b"ExtGState", 0),
    (b"cs", b"ColorSpace", 0),
    (b"CS", b"ColorSpace", 0),
    (b"scn", b"Pattern", usize::MAX),
    (b"SCN", b"Pattern", usize::MAX),
    (b"sh", b"Shading", 0),
    (b"BDC", b"Properties", 1),
    (b"DP", b"Properties", 1),
];

/// Colour-space names that are **built in** and name no resource (§8.6.3).
///
/// A `cs`/`CS` operand may legally be one of these instead of a resource key,
/// and treating one as a missing resource would refuse a perfectly ordinary
/// fill. `/Pattern` is included: as a bare `cs` operand it selects the pattern
/// colour space itself, not an entry in `/Pattern`.
const BUILTIN_COLOUR_SPACES: [&[u8]; 4] = [b"DeviceGray", b"DeviceRGB", b"DeviceCMYK", b"Pattern"];

/// One resource name found in an item's bytes, with the span to rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NameSite {
    /// The `/Resources` category the name resolves in.
    pub(crate) category: Vec<u8>,
    /// The name, without its leading solidus.
    pub(crate) name: Vec<u8>,
    /// The name token's span in the item's own bytes, including the solidus.
    pub(crate) span: ByteSpan,
}

/// Find every resource-name reference in a content-byte range, with the span
/// each one occupies.
///
/// This is the read half of name rebinding; [`rewrite_names`] is the write
/// half, and they share this function's output so a name can never be
/// *detected* in one place and *rewritten* in another.
///
/// # Errors
///
/// [`ClipError::Content`] when the bytes do not parse as a content stream.
pub(crate) fn name_sites(bytes: &[u8]) -> Result<Vec<NameSite>, ClipError> {
    let stream =
        ContentStream::parse(bytes.to_vec()).map_err(|e| ClipError::Content(e.to_string()))?;
    let mut sites = Vec::new();
    for op in stream.operations() {
        let Some(keyword) = op.operator_name(&stream.buf) else {
            continue; // an inline image is its own operation and names nothing
        };
        let Some(&(_, category, which)) = NAME_OPERATORS.iter().find(|(k, _, _)| *k == keyword)
        else {
            continue;
        };
        let token = if which == usize::MAX {
            op.operands.last()
        } else {
            op.operands.get(which)
        };
        let Some(token) = token else { continue };
        let ContentTokenKind::Operand(Object::Name(name)) = &token.kind else {
            // `scn` most often ends in a number, and `BDC` most often carries
            // an inline dictionary. Both are ordinary, not errors.
            continue;
        };
        if category == b"ColorSpace" && BUILTIN_COLOUR_SPACES.contains(&name.as_bytes()) {
            continue;
        }
        sites.push(NameSite {
            category: category.to_vec(),
            name: name.as_bytes().to_vec(),
            span: token.span,
        });
    }
    Ok(sites)
}

/// Rewrite resource names in an item's bytes, given a `(category, old) → new`
/// mapping.
///
/// Every byte outside a rewritten name token is copied verbatim — the item's
/// operands, its operator spelling, its whitespace. See [`ClipItem::bytes`] for
/// why that matters.
///
/// # Errors
///
/// [`ClipError::Content`] when the bytes do not parse.
pub(crate) fn rewrite_names(
    bytes: &[u8],
    mapping: &BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>,
) -> Result<Vec<u8>, ClipError> {
    let sites = name_sites(bytes)?;
    let mut out = Vec::with_capacity(bytes.len());
    let mut cursor = 0usize;
    for site in sites {
        let Some(new) = mapping.get(&(site.category.clone(), site.name.clone())) else {
            continue;
        };
        if site.span.start < cursor {
            continue; // defensive: overlapping sites cannot occur
        }
        if let Some(gap) = bytes.get(cursor..site.span.start) {
            out.extend_from_slice(gap);
        }
        out.push(b'/');
        out.extend_from_slice(new);
        cursor = site.span.end();
    }
    if let Some(tail) = bytes.get(cursor..) {
        out.extend_from_slice(tail);
    }
    Ok(out)
}

/// The content bytes one paste appends, with the resource bindings the
/// destination page must gain — everything a paste needs that is not object
/// allocation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PastePlan {
    /// The content to append to the destination page, already wrapped and
    /// re-bound.
    pub content: Vec<u8>,
    /// `(category, new page-resource name, clip-local object)` for every
    /// binding the destination page must gain.
    pub resources: Vec<(Vec<u8>, Vec<u8>, u32)>,
    /// How many objects the paste places.
    pub items: usize,
    /// The page-space bounds the pasted content will occupy.
    pub bbox: Bounds,
}

/// Plan a paste: re-bind every resource name to a fresh destination name,
/// rewrite the copied bytes accordingly, and wrap each item so it lands where
/// the caller asked (`Pass 120.0`).
///
/// `existing` is the destination page's resolved `/Resources`, consulted only
/// to avoid colliding with names already there. `at` is a **page-space**
/// matrix, for the same reason `transform_objects` takes one.
///
/// # ★ Why every binding gets a fresh name, even when the old one is free
///
/// A destination page that happens not to use `/F1` is not a page where `/F1`
/// means what the clip means by it — and a *later* paste, or an unrelated
/// edit, could give `/F1` a meaning while the pasted content is still on the
/// page. Reusing the source's spelling makes correctness depend on a
/// coincidence that nothing preserves. Fresh names are cheap; a font that
/// silently changes months later is not.
///
/// # Errors
///
/// [`ClipError::NewerFormat`], [`ClipError::DanglingClipObject`],
/// [`ClipError::Content`].
pub fn plan_paste(clip: &ObjectClip, existing: &Dict, at: Matrix) -> Result<PastePlan, ClipError> {
    if clip.version > CLIP_VERSION {
        return Err(ClipError::NewerFormat {
            found: clip.version,
            supported: CLIP_VERSION,
        });
    }

    // One destination name per (category, clip object) — NOT per (category,
    // source name). Two items that both said `/F1` and meant the same font
    // share one binding; two that said `/F1` and meant DIFFERENT fonts (which
    // is what happens when a clip spans two pages) get two, and neither is
    // silently given the other's.
    let mut chosen: BTreeMap<(Vec<u8>, u32), Vec<u8>> = BTreeMap::new();
    let mut used: BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>> = BTreeMap::new();
    let mut resources: Vec<(Vec<u8>, Vec<u8>, u32)> = Vec::new();

    for item in &clip.items {
        for binding in &item.bindings {
            if !clip.objects.contains_key(&binding.object) {
                return Err(ClipError::DanglingClipObject {
                    object: binding.object,
                });
            }
            let key = (binding.category.clone(), binding.object);
            if chosen.contains_key(&key) {
                continue;
            }
            let taken = used.entry(binding.category.clone()).or_default();
            let name = free_name(existing, &binding.category, taken);
            taken.insert(name.clone());
            resources.push((binding.category.clone(), name.clone(), binding.object));
            chosen.insert(key, name);
        }
    }

    let mut content = Vec::new();
    let mut bbox = Bounds::EMPTY;
    for item in &clip.items {
        let mut mapping: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>> = BTreeMap::new();
        for binding in &item.bindings {
            if let Some(name) = chosen.get(&(binding.category.clone(), binding.object)) {
                mapping.insert(
                    (binding.category.clone(), binding.name.clone()),
                    name.clone(),
                );
            }
        }
        let rewritten = rewrite_names(&item.bytes, &mapping)?;
        // `source_ctm × at`: the bytes were written under `source_ctm`, and the
        // append point's CTM is the identity, so this reproduces the original
        // placement and then applies the caller's page-space matrix.
        let placement = item.ctm.post_concat(at);
        content.extend_from_slice(b"\nq ");
        for v in [
            placement.a,
            placement.b,
            placement.c,
            placement.d,
            placement.e,
            placement.f,
        ] {
            crate::writer::content::emit_number(&mut content, v);
            content.push(b' ');
        }
        content.extend_from_slice(b"cm\n");
        // The inherited state first, INSIDE the wrapper -- so it applies to
        // this item and is discarded by the closing `Q` rather than leaking
        // onto whatever the destination page draws next.
        if !item.prelude.is_empty() {
            content.extend_from_slice(&rewrite_names(&item.prelude, &mapping)?);
            content.push(b'\n');
        }
        content.extend_from_slice(&rewritten);
        content.extend_from_slice(b"\nQ");
        bbox = bbox.union(transformed_bounds(item.bbox, at));
    }

    Ok(PastePlan {
        content,
        resources,
        items: clip.items.len(),
        bbox,
    })
}

/// A free resource name in `category`, avoiding both the destination page's
/// existing names and the ones this paste has already claimed.
///
/// The `pdfce` prefix matches [`free_xobject_name`](crate::edit::EditSession)'s
/// convention: a name an operator sees in an object dump is immediately
/// attributable, and a producer is vanishingly unlikely to have used it.
fn free_name(existing: &Dict, category: &[u8], taken: &BTreeSet<Vec<u8>>) -> Vec<u8> {
    let present = existing
        .get(category)
        .and_then(Object::as_dict)
        .cloned()
        .unwrap_or_default();
    let tag = match category {
        b"Font" => "F",
        b"XObject" => "X",
        b"ExtGState" => "G",
        b"ColorSpace" => "C",
        b"Pattern" => "P",
        b"Shading" => "S",
        _ => "R",
    };
    for n in 0u32.. {
        let candidate = format!("pdfceP{tag}{n}").into_bytes();
        if present.get(candidate.as_slice()).is_none() && !taken.contains(&candidate) {
            return candidate;
        }
    }
    // `0u32..` is not empty, so this is unreachable; a deterministic name is
    // still better than a panic if it ever were.
    format!("pdfceP{tag}").into_bytes()
}

/// A bounding box under a matrix — all four corners mapped, then re-enclosed,
/// because a rotation makes the naive two-corner version wrong.
fn transformed_bounds(b: Bounds, m: Matrix) -> Bounds {
    if b.min.x > b.max.x {
        return b;
    }
    let mut out = Bounds::EMPTY;
    for (x, y) in [
        (b.min.x, b.min.y),
        (b.max.x, b.min.y),
        (b.max.x, b.max.y),
        (b.min.x, b.max.y),
    ] {
        out = out.union_point(m.map_point(super::geometry::Point::new(x, y)));
    }
    out
}

/// Build the **prelude** for a decomposed object: the graphics state it
/// depends on but does not establish in its own bytes (`Pass 120.2`).
///
/// See [`ClipItem::prelude`] for the defect this exists to fix and why it is a
/// separate field rather than a rewrite of the item's bytes.
///
/// # What is emitted, and what deliberately is not
///
/// Only state the decomposition already **measured**, and only when the item's
/// own bytes do not set it:
///
/// - **`Tf`** for a text object, from [`TextObject::font`] — the case the real
///   file found.
/// - **`w`** (line width) and the fill/stroke colours for a path, from
///   [`PathObject`]'s recorded values.
///
/// Not emitted: dash patterns, `gs` parameters, clipping, blend modes,
/// rendering intent. Those are **not in the object model**, so emitting
/// anything for them would be fabrication rather than transcription — and a
/// fabricated dash is worse than an absent one, because it looks deliberate.
/// A caller who needs them has a copy that renders solid where the source
/// rendered dashed, which is visible; the alternative is a copy that renders
/// wrong in a way that looks right.
pub(crate) fn item_prelude(o: &VectorObject, own_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let sets = |keyword: &[u8]| -> bool {
        // (borrow-free by construction: `own_bytes` outlives the closure)
        // Cheap and deliberately conservative: if the item's own bytes mention
        // the operator ANYWHERE, the prelude stays out of its way. A false
        // positive costs an inherited value the object was going to set for
        // itself; a false negative would double-set it, which is harmless but
        // noisy. Parsing here would be more precise and would also make the
        // prelude depend on the item parsing, which `to_pdf` deliberately
        // survives without.
        own_bytes.windows(keyword.len()).any(|w| w == keyword)
    };
    match o {
        VectorObject::Text(t) => {
            if let Some(font) = t.font.as_ref()
                && !sets(b"Tf")
            {
                out.push(b'/');
                out.extend_from_slice(font.resource.as_bytes());
                out.push(b' ');
                crate::writer::content::emit_number(&mut out, font.size);
                out.extend_from_slice(b" Tf");
            }
        }
        VectorObject::Path(p) => {
            if !sets(b" w") && !out_starts_with_w(own_bytes) && p.line_width > 0.0 {
                crate::writer::content::emit_number(&mut out, p.line_width);
                out.extend_from_slice(b" w ");
            }
            // Colours are emitted as DeviceRGB, which is what the model
            // records. A source in DeviceCMYK or a `Separation` arrives as its
            // RGB equivalent -- a real limitation, stated here rather than
            // discovered: the decomposition holds an `Rgb`, and inventing a
            // colourant name pdfce never measured would be worse.
            if !sets(b" rg") {
                emit_rgb(&mut out, p.fill_color, false);
            }
            if !sets(b" RG") {
                emit_rgb(&mut out, p.stroke_color, true);
            }
        }
        VectorObject::Image(_) => {}
    }
    out
}

/// Whether the bytes BEGIN with a line-width operator, which `sets(b" w")`
/// cannot see because there is no leading space.
fn out_starts_with_w(bytes: &[u8]) -> bool {
    bytes.starts_with(b"w ") || bytes.starts_with(b"w\n") || bytes.starts_with(b"w\r")
}

/// `r g b rg` / `r g b RG` appended to `out`.
fn emit_rgb(out: &mut Vec<u8>, colour: super::geometry::Rgb, stroking: bool) {
    for v in [colour.r, colour.g, colour.b] {
        crate::writer::content::emit_number(out, f64::from(v));
        out.push(b' ');
    }
    out.extend_from_slice(if stroking { b"RG " } else { b"rg " });
}

/// The category label and CTM of a decomposed object — the two things a clip
/// item needs that are not its bytes.
pub(crate) const fn item_kind(o: &VectorObject) -> &'static str {
    match o {
        VectorObject::Path(_) => "path",
        VectorObject::Text(_) => "text",
        VectorObject::Image(_) => "image",
    }
}

/// The object's captured CTM, whatever its kind.
pub(crate) const fn item_ctm(o: &VectorObject) -> Matrix {
    match o {
        VectorObject::Path(p) => p.ctm,
        VectorObject::Text(t) => t.ctm,
        VectorObject::Image(i) => i.ctm,
    }
}

/// Build the `/Resources` sub-dictionary additions a [`PastePlan`] asks for,
/// given the destination object id each clip-local object was imported to.
///
/// Split out so the session's command builder does not also own the shape of a
/// resource dictionary — one place decides what a `/Font` entry looks like.
#[must_use]
pub fn paste_resource_dict(
    existing: &Dict,
    plan: &PastePlan,
    imported: &BTreeMap<u32, crate::object::ObjId>,
    resolve: &dyn Fn(&Object) -> Object,
) -> Dict {
    let mut out = existing.clone();
    for (category, name, clip_object) in &plan.resources {
        let Some(id) = imported.get(clip_object) else {
            continue;
        };
        let mut sub = out
            .get(category)
            .map(resolve)
            .and_then(|o| o.as_dict().cloned())
            .unwrap_or_default();
        sub.insert(Name(name.clone()), Object::Reference(*id));
        out.insert(Name(category.clone()), Object::Dict(sub));
    }
    out
}
