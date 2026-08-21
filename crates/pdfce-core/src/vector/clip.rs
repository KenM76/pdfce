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
            items.push(ClipItem {
                bytes: item_bytes,
                ctm,
                kind,
                bbox: item_bbox,
                bindings,
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
        })
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
