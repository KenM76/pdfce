//! # Vector-object content-stream SURGERY (Pass 9c-min, decision 011 §2.5)
//!
//! The **write half** of the vector object model: the three, and only
//! three, basic-editing operations decision 011 §2.5 scopes for the first
//! beta —
//!
//! 1. **move object** — translate every path-construction operand of a
//!    selected object by a page-space `(dx, dy)`, CTM-aware
//!    ([`plan_move`]);
//! 2. **delete object** — remove an object's construction **and** painting
//!    operators from the content stream ([`plan_delete`]);
//! 3. **drag node** — rewrite ONE anchor's coordinate pair in an `m`/`l`/
//!    `c`/`v`/`y` operand list ([`plan_move_node`]).
//!
//! All three are **content-stream surgery** through the same
//! advance-preserving interpreter Pass 8.0 (redaction) and Pass 14.x
//! (text edit) are built on — the **R46 named exception, ISO 32000-1 §5.7**
//! (`docs/ARCHITECTURE.md` §5.7): the object's operator byte range is
//! located from the read-only Pass 9a decomposition
//! ([`super::decompose`]), the numeric operands are rewritten (or the whole
//! run removed), and ONLY the edited content stream is re-emitted; every
//! other object in the file stays **byte-verbatim**. This module is the
//! geometric mirror of [`crate::redact`]'s operator removal.
//!
//! ## What this module does and does not do (crate placement)
//!
//! It is a set of **pure planners**: each takes a tokenized
//! [`ContentStream`] plus the target object (from the SAME decomposition)
//! and returns the **new decoded content buffer** ([`PlannedEdit::content`])
//! — it never touches a [`Document`](crate::document::Document), a writer,
//! or the undo stack. The session-integrated, one-undoable-command wrappers
//! that stage the new bytes and re-emit exactly the edited stream live in
//! [`crate::edit::EditSession`] (`move_object`/`delete_object`/`move_node`),
//! mirroring how [`crate::text_edit::edit::plan_edit`] feeds
//! [`crate::edit::EditSession::edit_text`]. The whole module is GUI-free
//! (`pdfce-core`, no egui/eframe/winit/wgpu — the load-bearing invariant),
//! so the eventual WASM fork inherits the surgery unchanged; the GUI owns
//! only the drag gesture that produces the `(dx, dy)` / node index.
//!
//! ## Coordinate spaces (the CTM round-trip, §8.3.4)
//!
//! An object's construction operands live in the **user space** its
//! captured CTM ([`PathObject::ctm`]) maps *from*; the operator's drag and
//! the snap target are in **page space** (default user space, §8.3.2.3).
//! So [`plan_move`] converts the page-space displacement to a user-space
//! displacement with the CTM's **linear inverse** ([`Matrix::map_vector`]
//! ∘ [`Matrix::inverse`] — the delta transform, translation excluded), and
//! [`plan_move_node`] converts the page-space target *point* with the full
//! affine inverse ([`Matrix::map_point`] ∘ [`Matrix::inverse`]). A singular
//! CTM (an object flattened to a line) has no unambiguous pre-image and is
//! refused by name ([`VectorEditError::DegenerateCtm`]) — never fabricated
//! (rule 4, fuzzy-never-sneaky).
//!
//! ## Agreement with Pass 9a (node ordering, decision 011 Z2)
//!
//! [`plan_move_node`]'s `node_index` is the index into the object's anchors
//! in **decomposition order** — the flattening of
//! `obj.subpaths.flat_map(Subpath::anchors)` the snap engine and GUI node
//! hit-test already present. This module reproduces the EXACT subpath /
//! empty-subpath / `h`-reopen bookkeeping [`super::decompose`] uses, so the
//! nth anchor a caller sees and the nth anchor this surgery rewrites are the
//! same anchor by construction (the geometry analogue of the R49/R60 "one
//! pipeline" discipline), not by two hand-derived orderings kept in sync.
//!
//! ## Anchors whose coordinates are written NOWHERE (Pass 30.0)
//!
//! Two anchor kinds have no operand of their own to overwrite, and both were
//! refused for that reason until Pass 30.0:
//!
//! - an **`re` rectangle corner**. `re` carries an origin and a *size*, so
//!   only corner 0 appears literally, and even it cannot move alone — editing
//!   `x y` slides all four. Worse, the shape a dragged corner produces is in
//!   general NOT a box, and `re` has no spelling for that shape at all.
//! - the **implicit reused start** of a subpath reopened after `h`: the
//!   segment inherits the closed subpath's start point (§8.5.2.1) rather than
//!   naming it.
//!
//! Both are now edited by *materializing the missing operand* rather than by
//! refusing. The rectangle is expanded to the spec's own stated equivalent —
//! `x y m`, `x+w y l`, `x+w y+h l`, `x y+h l`, `h` (§8.5.2.1, Table 59) —
//! whose trailing `h` is load-bearing: a stroked subpath left open takes two
//! line caps where the closed one takes a corner join, so dropping it would
//! change the picture. The implicit start gets the `m` the file omitted,
//! inserted immediately before the segment that inherited it, which no earlier
//! geometry can observe because `h` has already terminated the subpath before
//! it.
//!
//! Both rewrites leave the anchor COUNT and ORDER unchanged, which is what
//! lets a front end hold a node index across the drag it just performed.
//! Both are [disclosed](PlannedEdit::disclosures): the drawing is identical,
//! the bytes are not, and dragging back does not restore the original form.
//!
//! ## Panic-free / adversarial input (ARCHITECTURE.md §10)
//!
//! Every operand access is checked; a construction operator whose operand
//! arity does not match the spec (§8.5.2.1, Table 59) is left byte-verbatim
//! for a node-drag (only the one edited operator is re-emitted) and is a
//! by-name refusal ([`VectorEditError::MalformedOperand`]) for a whole-object
//! move (a partially-moved shape would be torn — refused, never silently
//! half-applied). Degenerate coordinates (`NaN`, `±∞`, huge magnitudes) are
//! re-emitted through [`emit_number`], which is total. The fuzz target
//! `fuzz/fuzz_targets/vector_edit.rs` drives exactly these shapes.

use crate::content::{ContentStream, ContentToken, ContentTokenKind};
use crate::text_edit::edit::splice;
use crate::writer::content::emit_number;

use super::decompose::{PathObject, VectorObject};
use super::geometry::{Point, rect_corners};

/// Why a vector-edit surgery could not be planned.
///
/// Every variant names a condition the operator (or the calling front end)
/// can act on; there is deliberately no catch-all "edit failed" (mirrors
/// [`crate::edit::EditError`]'s discipline). Surgery that cannot be
/// performed cleanly is refused **before** any byte is produced — the
/// caller's session is never left half-edited (rule 4).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum VectorEditError {
    /// The object index is past the end of the page's decomposition.
    #[error("object index {index} is out of range (the page decomposes to {count} object(s))")]
    ObjectOutOfRange {
        /// The 0-based index that was asked for.
        index: usize,
        /// How many selectable objects the page actually has.
        count: usize,
    },
    /// A move or node-drag was requested on an object that is not a
    /// **path** (a text or image/form object). Text and image objects are
    /// selectable-for-move/delete in the model, but node editing and
    /// operand-translation are path-only in the beta (decision 011 §2.1:
    /// text/image are "not node-editable"). Move of a text/image object is
    /// a named fast-follow (it needs `Tm`/`cm`-operand surgery, a different
    /// operator family); 9c-min moves **paths**.
    #[error(
        "object {index} is not a path object (it is {kind}), which 9c-min move/node editing does not cover"
    )]
    NotAPath {
        /// The object's index.
        index: usize,
        /// A short kind label (`"text"` / `"image"`), for the diagnostic.
        kind: &'static str,
    },
    /// The object's captured CTM is singular (non-invertible), so a
    /// page-space drag has no unambiguous user-space pre-image. Refused
    /// rather than fabricated (rule 4).
    #[error(
        "the object's transform is singular (non-invertible), so a page-space drag cannot be mapped to its user space"
    )]
    DegenerateCtm,
    /// A whole-object move hit a construction operator whose operand arity
    /// does not match the spec (Table 59), so the object cannot be moved
    /// **as a whole** without tearing it. Refused by name; the object is
    /// left untouched.
    #[error(
        "the object contains a malformed construction operator (unexpected operand count), so it cannot be moved without tearing it"
    )]
    MalformedOperand,
    /// Deleting this subpath would silently MOVE the next one.
    ///
    /// The following subpath was started implicitly — by a segment operator
    /// after `h`, which reopens at the closed subpath's start point
    /// (§8.5.2.1). Its start is INHERITED and carried by no operand, so
    /// excising the operators before it changes where it begins: a
    /// byte-minimal edit that passes `--verify-undo` and every content-identity
    /// check, and is still wrong.
    #[error(
        "deleting part {index} would move the part after it, which starts where this one ends rather than at coordinates of its own"
    )]
    DeleteWouldMoveNextSubpath {
        /// The subpath whose deletion was refused.
        index: usize,
    },
    /// A subpath delete named an index past the object's subpath count.
    #[error("subpath index {index} is out of range (the object has {count} subpath(s))")]
    SubpathOutOfRange {
        /// The 0-based subpath index that was asked for.
        index: usize,
        /// How many subpaths the object has, in decomposition order.
        count: usize,
    },
    /// A subpath delete was asked for on a path that establishes a **clipping
    /// region** (`W` / `W*`, §8.5.4) rather than painting marks.
    ///
    /// Refused because the visible effect would be somewhere the operator was
    /// not looking: removing one subpath of a clip changes which OTHER content
    /// shows through. Rule 4 (fuzzy, never sneaky) forbids exactly that.
    #[error(
        "this path defines a clipping region, so deleting part of it would change what other content is visible rather than removing a mark"
    )]
    ClippingPath,
    /// A subpath's recorded token range names no operator in this stream — an
    /// internal inconsistency between the decomposition and the content it was
    /// derived from.
    ///
    /// **This used to mean something else.** Before Pass 28.0, subpaths carried
    /// no token range, so an edit re-walked the operators and raised this
    /// whenever its walk disagreed in COUNT with the geometry — which happened
    /// for any object containing an implicit `h`-reopen, and refused every
    /// subpath in it. That case now has its own precise variant
    /// ([`VectorEditError::DeleteWouldMoveNextSubpath`]) and the count can no
    /// longer disagree, because one walk records both.
    ///
    /// What remains is a should-never-happen path kept as an error rather than
    /// a panic: the crate's policy is that adversarial or corrupt input is
    /// refused by name, never unwrapped (ARCHITECTURE.md §10).
    #[error(
        "this path's structure cannot be edited by subpath index ({from_operators} subpath(s) found in its operators, {from_decomposition} in the geometry), so the wrong one might be removed"
    )]
    SubpathStructureMismatch {
        /// Subpaths found by walking the object's construction operators.
        from_operators: usize,
        /// Subpaths the geometric decomposition reports.
        from_decomposition: usize,
    },
    /// A handle drag named a node with no Bézier control point on that side.
    ///
    /// Either the neighbouring segment is straight (`l`, or a rectangle edge),
    /// or there is no segment there at all (the end of an open subpath, or the
    /// far side of a subpath boundary). Refused rather than converted: turning
    /// a straight segment into a curve is a different operation with a
    /// different name, and inferring it from a drag on a handle that was never
    /// drawn is the silent reinterpretation rule 4 forbids.
    #[error(
        "node {index} has no {handle:?} curve handle — the segment on that side is straight or absent, and pdfce will not turn a straight line into a curve without being asked"
    )]
    NoHandleHere {
        /// The node whose handle was asked for.
        index: usize,
        /// Which side was asked for.
        handle: Handle,
    },
    /// A node-drag named an anchor index past the object's anchor count.
    #[error("node index {index} is out of range (the object has {count} anchor(s))")]
    NodeOutOfRange {
        /// The 0-based node index that was asked for.
        index: usize,
        /// How many anchors the object has, in decomposition order.
        count: usize,
    },
}

/// The result of a successful surgery plan: the **new decoded content
/// buffer** plus how many operators it rewrote/removed (a magnitude a front
/// end or a report can quote; the writer counts objects, this counts
/// operators).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEdit {
    /// The rewritten, decoded content-stream bytes — the buffer the session
    /// wrapper stages and re-emits as ONE raw (unfiltered) stream (R46
    /// named exception; every other object byte-verbatim).
    pub content: Vec<u8>,
    /// How many construction/painting operators the surgery rewrote (move:
    /// every construction operator; node-drag: exactly one; delete: the
    /// object's whole operator run counts as one removal).
    pub operators_touched: usize,
    /// What the operator must be told about HOW the edit was expressed, in
    /// operator-facing prose — empty for the common case.
    ///
    /// Populated when the surgery had to change the *form* of an operator to
    /// express the requested change, because some shapes in PDF cannot say
    /// what the operator just asked for. Dragging one corner of an `re`
    /// rectangle is the canonical case: `re` carries an origin and a size, so
    /// a four-sided shape that is not a box has no `re` spelling at all and
    /// the operator must be expanded to four lines (§8.5.2.1). The drawing is
    /// unchanged — but the bytes are not recoverable by dragging back, and an
    /// operator who cares about minimal diffs (R46) is owed that fact rather
    /// than left to find it in a diff.
    ///
    /// This is rule 4 (fuzzy, never sneaky) applied to *representation*: pdfce
    /// may reshape how a thing is written in order to do what was asked, and
    /// says so when it does.
    pub disclosures: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public planners
// ---------------------------------------------------------------------------

/// Plan a **move**: translate every path-construction operand of `obj` by
/// the page-space displacement `(dx_page, dy_page)`, CTM-aware.
///
/// The displacement is converted to user space with the CTM's linear
/// inverse (module docs), then added to each operand point: `m`/`l` shift
/// their single point, `c` all three points, `v`/`y` both explicit points,
/// and `re` its **origin only** (`x, y` — the width/height `w, h` are a
/// size, not a point, and must not move). `h` and the painting operator are
/// re-emitted byte-verbatim. The whole object's operator run is re-emitted
/// at the new coordinates; every other stream object is untouched.
///
/// # Errors
///
/// [`VectorEditError::DegenerateCtm`] (singular CTM) or
/// [`VectorEditError::MalformedOperand`] (a construction operator with a
/// spec-violating operand count — refused rather than tear the shape). Both
/// are raised before any content byte is produced.
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{decompose, NoXObjects, Matrix, VectorObject};
/// use pdfce_core::vector::edit::plan_move;
///
/// let cs = ContentStream::parse(b"10 20 m 100 200 l S".to_vec()).unwrap();
/// let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
/// let VectorObject::Path(path) = &model.objects[0] else { unreachable!() };
/// // Move +5 in x, −3 in y (identity CTM ⇒ page delta == user delta).
/// let plan = plan_move(&cs, path, 5.0, -3.0).unwrap();
/// assert_eq!(plan.content, b"15 17 m 105 197 l S");
/// ```
pub fn plan_move(
    content: &ContentStream,
    obj: &PathObject,
    dx_page: f64,
    dy_page: f64,
) -> Result<PlannedEdit, VectorEditError> {
    // Page-space drag → user-space delta via the CTM's linear inverse
    // (translation excluded — a displacement, not a point).
    let inv = obj.ctm.inverse().ok_or(VectorEditError::DegenerateCtm)?;
    let d = inv.map_vector(Point::new(dx_page, dy_page));
    let (du, dv) = (d.x, d.y);

    let mut edits: Vec<(usize, usize, Vec<u8>)> = Vec::new();
    let mut touched = 0usize;
    for item in ops_in_range(content, obj.tokens.start, obj.tokens.end) {
        let Some(keyword) = item.keyword(&content.buf) else {
            continue; // an inline image inside a path run: not a construction op
        };
        let nums = item.nums();
        // Rewrite only path-construction operators; everything else
        // (painting, `h`, state ops that slipped into the range) is left
        // byte-verbatim.
        let rewritten = match keyword {
            b"m" | b"l" => translate_points(&nums, &[true], du, dv),
            b"c" => translate_points(&nums, &[true, true, true], du, dv),
            b"v" | b"y" => translate_points(&nums, &[true, true], du, dv),
            // `re x y w h`: only the origin (x, y) moves; (w, h) is a size.
            b"re" => translate_rect(&nums, du, dv),
            // `h` and any painting operator: unchanged.
            _ => continue,
        };
        let Some(new_nums) = rewritten else {
            // A construction operator with the wrong operand arity: moving
            // the object as a whole would tear it. Refuse by name.
            return Err(VectorEditError::MalformedOperand);
        };
        edits.push((
            item.byte_start(),
            item.byte_end(),
            emit_op(&new_nums, keyword),
        ));
        touched += 1;
    }

    Ok(PlannedEdit {
        content: splice(&content.buf, &mut edits),
        operators_touched: touched,
        disclosures: clip_disclosure(content, obj),
    })
}

/// Plan a **delete**: remove `obj`'s construction **and** painting
/// operators from the content stream.
///
/// The object's byte span ([`VectorObject::bytes`]) covers exactly its
/// first construction operator through its painting operator (or the
/// `BT`→`ET` / `Do` run for a text/image object), captured by Pass 9a; the
/// span is spliced out and every other byte stays verbatim. Preceding
/// graphics-state operators (colour, line width, `q`/`cm`) are **not**
/// removed — they set state the object happened to use but that the
/// operator did not select for deletion (decision 011 §2.5: "remove the
/// object's construction + painting operators"), and a leftover `q…Q`
/// around nothing is inert. This deletes **any** object kind (path, text,
/// image), since it is a pure byte-span removal.
///
/// # Errors
///
/// Never — a delete is a total operation over a valid Pass 9a byte span.
/// (Returns `Result` for signature symmetry with the other two planners and
/// to stay forward-compatible if a future kind grows a refusal.)
#[allow(clippy::unnecessary_wraps)]
pub fn plan_delete(
    content: &ContentStream,
    obj: &VectorObject,
) -> Result<PlannedEdit, VectorEditError> {
    let span = obj.bytes();
    // Remove [start, end): splice with an empty replacement. `splice`
    // copies the gap before, inserts nothing, and resumes after the span,
    // leaving the surrounding whitespace/operators intact and separated.
    let mut edits = vec![(span.start, span.end(), Vec::new())];
    Ok(PlannedEdit {
        content: splice(&content.buf, &mut edits),
        operators_touched: 1,
        disclosures: Vec::new(),
    })
}

/// Plan a **subpath delete**: remove ONE subpath's construction operators
/// from a path object, leaving the object's other subpaths byte-verbatim.
///
/// # Why this operation exists at all
///
/// A CAD producer routinely emits an entire drawing view as one path object.
/// Measured on a real SolidWorks export: one stroked path with **1194
/// subpaths** covering a 550×500 pt isometric view. [`plan_delete`] can only
/// remove the whole view. This removes one line of it — which is what an
/// operator asking to "delete this line" almost always means on such a file.
///
/// # The index, and the guard that makes it safe
///
/// `subpath_index` is into `obj.subpaths` — the SAME ordering
/// [`super::hit_test_subpaths`] returns and the GUI selects with. That
/// agreement is not assumed: this re-derives the subpaths from the operator
/// bytes and **refuses if the two counts disagree**
/// ([`VectorEditError::SubpathStructureMismatch`]). A silent disagreement
/// would delete a different line from the one the operator picked, which is
/// the single worst outcome available here and is not detectable afterwards
/// by looking at the file.
///
/// Only subpaths begun by an explicit `m` or `re` are counted. A subpath that
/// PDF starts implicitly — a segment operator after `h`, which reopens at the
/// closed subpath's start point (§8.5.2.1) — has no operator of its own to
/// remove cleanly, so its presence trips the count guard and the whole edit is
/// refused rather than approximated. Note the asymmetry with MOVING such a
/// subpath (and with node-dragging its start), both of which now succeed by
/// materializing the `m` the file omitted: an insertion can supply a missing
/// coordinate, but a DELETION has nowhere to put one — removing the operators
/// before an implicit start changes where it begins, and there is no operand
/// to pin it with because the subpath being deleted is the one that would
/// have carried it.
///
/// # Clipping paths are refused
///
/// If the object's operators include `W` or `W*`, its subpaths define a
/// **clipping region** (§8.5.4), not marks on the page. Deleting one would
/// change which OTHER content is visible — an edit whose visible effect is
/// somewhere the operator was not looking, and the definition of sneaky (rule
/// 4). Refused by name.
///
/// # Deleting the last subpath deletes the object
///
/// A path object with no construction operators left is not a smaller object;
/// it is a painting operator with no path, which is meaningless. So when
/// `obj` has exactly one subpath this removes the object's whole byte span —
/// identical to [`plan_delete`]. Callers that need to distinguish the two
/// outcomes should check `obj.subpaths.len() == 1` before calling.
///
/// # Errors
///
/// [`VectorEditError::SubpathOutOfRange`], [`VectorEditError::ClippingPath`],
/// or [`VectorEditError::SubpathStructureMismatch`]. Each is raised before any
/// content byte is produced, so a refusal leaves the document untouched.
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{decompose, NoXObjects, Matrix, VectorObject};
/// use pdfce_core::vector::edit::plan_delete_subpath;
///
/// // Three separate lines painted by ONE `S` — one object, three subpaths.
/// let cs = ContentStream::parse(b"0 0 m 10 0 l 0 5 m 10 5 l 0 9 m 10 9 l S".to_vec()).unwrap();
/// let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
/// let VectorObject::Path(path) = &model.objects[0] else { unreachable!() };
/// assert_eq!(path.subpaths.len(), 3);
///
/// // Remove the middle one; the other two keep their exact bytes.
/// let plan = plan_delete_subpath(&cs, path, 1).unwrap();
/// assert_eq!(plan.content, b"0 0 m 10 0 l 0 9 m 10 9 l S");
/// ```
pub fn plan_delete_subpath(
    content: &ContentStream,
    obj: &PathObject,
    subpath_index: usize,
) -> Result<PlannedEdit, VectorEditError> {
    if is_clipping_path(content, obj.tokens.start, obj.tokens.end) {
        return Err(VectorEditError::ClippingPath);
    }

    let declared = obj.subpaths.len();
    if subpath_index >= declared {
        return Err(VectorEditError::SubpathOutOfRange {
            index: subpath_index,
            count: declared,
        });
    }

    // The last one standing: an empty path is not an object.
    if declared == 1 {
        let span = obj.bytes;
        let mut edits = vec![(span.start, span.end(), Vec::new())];
        return Ok(PlannedEdit {
            content: splice(&content.buf, &mut edits),
            operators_touched: 1,
            disclosures: Vec::new(),
        });
    }

    // The subpath's OWN recorded token range (Pass 28.0), converted to bytes.
    //
    // This replaces a second walk over the operators plus a count guard that
    // refused the whole object whenever the two walks disagreed. The range is
    // now recorded by the decomposition that produced the subpath, so the
    // index and the bytes cannot describe different things — the agreement is
    // structural rather than checked.
    let subpath = obj
        .subpaths
        .get(subpath_index)
        .ok_or(VectorEditError::SubpathOutOfRange {
            index: subpath_index,
            count: declared,
        })?;

    // The precise form of decision 026's `DeleteWouldMoveNextSubpath`. A
    // subpath started implicitly by a segment after `h` inherits its start
    // point from whatever precedes it, carried by no operand — so excising the
    // subpath BEFORE it silently moves a line the operator never touched, in a
    // byte-minimal edit that passes every round-trip check. Only that one
    // deletion is refused now; previously any `h`-reopen anywhere in the object
    // made every subpath in it undeletable.
    if obj
        .subpaths
        .get(subpath_index + 1)
        .is_some_and(|next| next.starts_implicitly)
    {
        return Err(VectorEditError::DeleteWouldMoveNextSubpath {
            index: subpath_index,
        });
    }

    let site = span_of_tokens(content, subpath.tokens).ok_or(
        VectorEditError::SubpathStructureMismatch {
            from_operators: 0,
            from_decomposition: declared,
        },
    )?;

    // Swallow the whitespace that FOLLOWED the removed operators, so the
    // separator before them becomes the separator between their neighbours.
    // Without this every delete leaves a widening gap behind — cosmetically
    // untidy on one edit, and on a 1194-subpath drawing an operator could
    // remove hundreds of lines and leave hundreds of orphaned runs of spaces.
    //
    // Trailing rather than leading, and bounded by the object's own span, so
    // this can never reach across into a neighbouring object's bytes or run
    // two tokens together.
    let end = extend_over_whitespace(&content.buf, site.1, obj.bytes.end());
    let mut edits = vec![(site.0, end, Vec::new())];
    Ok(PlannedEdit {
        content: splice(&content.buf, &mut edits),
        operators_touched: 1,
        disclosures: Vec::new(),
    })
}

/// Advance `end` past any PDF white-space characters (§7.2.2 Table 1), never
/// beyond `limit` and never beyond the buffer.
fn extend_over_whitespace(buf: &[u8], mut end: usize, limit: usize) -> usize {
    let limit = limit.min(buf.len());
    while end < limit
        && matches!(
            buf.get(end),
            Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x00' | b'\x0c')
        )
    {
        end += 1;
    }
    end
}

/// The byte range covered by a token range, as `(start, end)`.
///
/// `None` when the range names no operator — a subpath the decomposition
/// recorded but whose tokens fall outside this stream, which should be
/// impossible and is refused rather than assumed.
fn span_of_tokens(
    content: &ContentStream,
    tokens: super::decompose::TokenRange,
) -> Option<(usize, usize)> {
    let items = ops_in_range(content, tokens.start, tokens.end.saturating_add(1));
    let first = items.first()?;
    let last = items.last()?;
    Some((first.byte_start(), last.byte_end()))
}

/// Whether the object's operators establish a clipping region (`W` / `W*`,
/// §8.5.4).
///
/// Checked from the OPERATORS rather than from [`PaintStyle`]: `is_invisible`
/// is true for a bare `n` as well as for `W n`, and a bare `n` path clips
/// nothing. Refusing both would be over-broad, and over-broad refusals teach
/// an operator that the tool says no for no reason.
fn is_clipping_path(content: &ContentStream, start: usize, end: usize) -> bool {
    ops_in_range(content, start, end).iter().any(|item| {
        matches!(
            item.keyword(&content.buf),
            Some(b"W") | Some(b"W*") // ui-text-exempt: PDF operator keywords, §8.5.4
        )
    })
}

/// The disclosure a *move* of a clipping path owes the operator, if the object
/// is one — otherwise nothing.
///
/// # Why this discloses where subpath-DELETE refuses
///
/// Both edits change what OTHER content is visible rather than changing a mark
/// the operator can see, which is the condition rule 4 exists for. They differ
/// in whether a legitimate intent exists. Deleting one subpath of a clip has
/// none worth guessing at — it changes the region's topology, and the operator
/// asking to "delete this line" cannot have meant "reveal whatever is under
/// that part of the page." Moving one does: resizing a crop region is a real
/// task, and refusing it would leave clip geometry permanently uneditable.
///
/// So: refuse the one with no good reading, disclose the one that has one.
///
/// # Why this was easy to miss
///
/// Until Pass 30.0 a clip rectangle's corners were unreachable — clips are
/// overwhelmingly `re` rectangles (§8.5.4's canonical `re W n` idiom), and
/// `re` corners were refused as un-draggable. Making them draggable removed
/// that accidental cover, so the gap had to be closed in the same change.
/// Found by running the new node drag against a real file rather than a
/// fixture: the first closed 4-anchor object on its first page was a
/// full-page clip.
fn clip_disclosure(content: &ContentStream, obj: &PathObject) -> Vec<String> {
    if is_clipping_path(content, obj.tokens.start, obj.tokens.end) {
        vec![
            "This shape is a clipping region: it draws nothing itself, it controls \
             which OTHER content on the page is visible. Moving it changes what shows \
             through elsewhere on the page, not here."
                .to_owned(),
        ]
    } else {
        Vec::new()
    }
}

/// Plan a **subpath move**: translate ONE subpath's construction operands by a
/// page-space `(dx, dy)`, leaving the object's other subpaths byte-verbatim
/// (Pass 28.0).
///
/// # Why this could not be written before
///
/// `Subpath` carried no byte range. `plan_delete_subpath` worked around that by
/// re-walking the operators and refusing whenever its walk disagreed with the
/// geometry about how many subpaths there were — enough to EXCISE a span, but
/// not enough to rewrite operands inside one, because a move has to know which
/// operator each coordinate pair belongs to. Recording the token range on the
/// decomposition walk that already knew it is what makes this expressible.
///
/// # What is refused, and why each
///
/// - A subpath that **starts implicitly** (a segment after `h`, §8.5.2.1): its
///   start point is inherited and carried by no operand, so translating the
///   operands that ARE written would move the rest of the subpath away from a
///   start that stayed put — tearing it. Since Pass 30.0 this is HANDLED, not
///   refused: an explicit `m` at the moved start is inserted ahead of the
///   segment that inherited it, and the translation is then uniform. Disclosed
///   via [`PlannedEdit::disclosures`].
/// - A **malformed operand run**, for the same reason `plan_move` refuses one:
///   a partially-moved subpath is worse than an unmoved one.
/// - A **singular CTM**, which has no unambiguous user-space pre-image.
///
/// # Errors
///
/// [`VectorEditError::SubpathOutOfRange`],
/// [`VectorEditError::MalformedOperand`], [`VectorEditError::DegenerateCtm`].
/// Every refusal happens before any byte is produced.
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{decompose, NoXObjects, Matrix, VectorObject};
/// use pdfce_core::vector::edit::plan_move_subpath;
///
/// let cs = ContentStream::parse(b"0 0 m 10 0 l 0 5 m 10 5 l S".to_vec()).unwrap();
/// let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
/// let VectorObject::Path(path) = &model.objects[0] else { unreachable!() };
/// // Move only the SECOND line up by 20.
/// let plan = plan_move_subpath(&cs, path, 1, 0.0, 20.0).unwrap();
/// assert_eq!(plan.content, b"0 0 m 10 0 l 0 25 m 10 25 l S");
/// ```
pub fn plan_move_subpath(
    content: &ContentStream,
    obj: &PathObject,
    subpath_index: usize,
    dx: f64,
    dy: f64,
) -> Result<PlannedEdit, VectorEditError> {
    let count = obj.subpaths.len();
    let subpath = obj
        .subpaths
        .get(subpath_index)
        .ok_or(VectorEditError::SubpathOutOfRange {
            index: subpath_index,
            count,
        })?;
    // Page-space delta to user-space delta: the LINEAR inverse, translation
    // excluded — the same conversion `plan_move` makes, for the same reason.
    let inv = obj.ctm.inverse().ok_or(VectorEditError::DegenerateCtm)?;
    let d = inv.map_vector(Point::new(dx, dy));

    let mut edits: Vec<(usize, usize, Vec<u8>)> = Vec::new();
    let mut disclosures: Vec<String> = clip_disclosure(content, obj);
    let mut touched = 0usize;

    // An implicitly-started subpath (§8.5.2.1: a segment after `h` with no `m`
    // of its own) INHERITS its start from the closed subpath before it. Its
    // segment operands can be translated like any other, but its start point
    // is written nowhere, so translating the operands alone would move every
    // point of the subpath EXCEPT the first — shearing the shape rather than
    // moving it.
    //
    // This used to refuse for that reason. Materializing the `m` the file
    // omitted, at the inherited start plus the delta, removes the cause: after
    // it the subpath's start is its own, and the translation is uniform. The
    // insertion touches nothing before it, because `h` has already terminated
    // the previous subpath.
    // Prepended to the FIRST rewritten operator's bytes rather than pushed as
    // its own zero-width edit at the same offset: `splice` silently skips an
    // edit that starts before its cursor, so two edits sharing a start offset
    // would drop one of the pair, chosen by sort order. Prepending has no such
    // race and produces the identical bytes.
    let mut lead_insert: Option<Vec<u8>> = None;
    if subpath.starts_implicitly {
        // `subpath.start` is in USER space (the decomposer records operands
        // before the CTM), so the user-space delta applies to it directly.
        let moved = Point::new(subpath.start.x + d.x, subpath.start.y + d.y);
        let mut lead = emit_op(&[moved.x, moved.y], b"m");
        lead.push(b' ');
        lead_insert = Some(lead);
        disclosures.push(
            "This shape had no starting point of its own — it re-used the start of the \
             shape before it. A move instruction naming its start has been added so it \
             can be moved independently."
                .to_owned(),
        );
    }
    for item in ops_in_range(content, subpath.tokens.start, subpath.tokens.end + 1) {
        let Some(keyword) = item.keyword(&content.buf) else {
            continue;
        };
        let nums = item.nums();
        // Coordinate arity per Table 59. `h` carries none and needs no edit.
        let pairs = match keyword {
            b"m" | b"l" => 1,
            b"v" | b"y" => 2,
            b"c" => 3,
            b"re" => {
                // Only the ORIGIN moves; width and height are a size, not a
                // position, so translating all four would resize the rectangle.
                let &[x, y, w, h] = nums.as_slice() else {
                    return Err(VectorEditError::MalformedOperand);
                };
                let mut out = Vec::new();
                emit_number(&mut out, x + d.x);
                out.push(b' ');
                emit_number(&mut out, y + d.y);
                out.push(b' ');
                emit_number(&mut out, w);
                out.push(b' ');
                emit_number(&mut out, h);
                out.extend_from_slice(b" re");
                push_edit(&mut edits, &mut lead_insert, &item, out);
                touched += 1;
                continue;
            }
            b"h" => continue,
            _ => continue,
        };
        if nums.len() != pairs * 2 {
            return Err(VectorEditError::MalformedOperand);
        }
        let mut out = Vec::new();
        for (i, chunk) in nums.chunks_exact(2).enumerate() {
            // `chunks_exact(2)` guarantees the pair, but destructuring proves
            // it to the compiler rather than to a reader — the crate forbids
            // indexing that could panic even where the invariant holds.
            let &[cx, cy] = chunk else {
                return Err(VectorEditError::MalformedOperand);
            };
            if i > 0 {
                out.push(b' ');
            }
            emit_number(&mut out, cx + d.x);
            out.push(b' ');
            emit_number(&mut out, cy + d.y);
        }
        out.push(b' ');
        out.extend_from_slice(keyword);
        push_edit(&mut edits, &mut lead_insert, &item, out);
        touched += 1;
    }

    Ok(PlannedEdit {
        content: splice(&content.buf, &mut edits),
        operators_touched: touched,
        disclosures,
    })
}

/// Record one operator rewrite, prepending any pending lead-in bytes (an
/// explicit `m` materialized for an implicitly-started subpath) to the FIRST
/// rewrite only.
///
/// Exists so the insertion never becomes a second edit sharing a start offset
/// with the rewrite: [`splice`] skips an edit starting before its cursor, so
/// such a pair silently loses one member depending on sort order.
fn push_edit(
    edits: &mut Vec<(usize, usize, Vec<u8>)>,
    lead_insert: &mut Option<Vec<u8>>,
    item: &OpItem<'_>,
    body: Vec<u8>,
) {
    let bytes = match lead_insert.take() {
        Some(mut lead) => {
            lead.extend_from_slice(&body);
            lead
        }
        None => body,
    };
    edits.push((item.byte_start(), item.byte_end(), bytes));
}

/// Plan a **node drag**: rewrite the single anchor `node_index` of `obj` to
/// the page-space point `to_page` (anchor/corner move only — adjacent
/// Bézier control-point "handle" editing is a named fast-follow, decision
/// 011 §2.5).
///
/// `node_index` is into the object's anchors in **decomposition order**
/// (module docs). The target point is mapped from page space to the
/// object's user space with the full affine inverse, and only the ONE
/// operator that defines that anchor is re-emitted with its anchor pair
/// replaced — every other operator of the object, and every other object,
/// stays byte-verbatim (so exactly one operator's bytes change).
///
/// # Errors
///
/// [`VectorEditError::NodeOutOfRange`], [`VectorEditError::DegenerateCtm`]
/// (singular CTM), or [`VectorEditError::MalformedOperand`] (an operator whose
/// operand count contradicts Table 59). Each is raised before any content byte
/// is produced.
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{decompose, NoXObjects, Matrix, Point, VectorObject};
/// use pdfce_core::vector::edit::plan_move_node;
///
/// let cs = ContentStream::parse(b"10 20 m 100 200 l S".to_vec()).unwrap();
/// let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
/// let VectorObject::Path(path) = &model.objects[0] else { unreachable!() };
/// // Drag node 1 (the `l` endpoint) to (120, 250).
/// let plan = plan_move_node(&cs, path, 1, Point::new(120.0, 250.0)).unwrap();
/// assert_eq!(plan.content, b"10 20 m 120 250 l S");
/// ```
pub fn plan_move_node(
    content: &ContentStream,
    obj: &PathObject,
    node_index: usize,
    to_page: Point,
) -> Result<PlannedEdit, VectorEditError> {
    let inv = obj.ctm.inverse().ok_or(VectorEditError::DegenerateCtm)?;
    let to_user = inv.map_point(to_page);
    // Owed regardless of which of the three rewrites runs below, so it is
    // computed once here rather than in each arm — where the next arm added
    // would forget it.
    let clip = clip_disclosure(content, obj);

    let anchors = enumerate_anchors(content, obj.tokens.start, obj.tokens.end);
    let count = anchors.len();
    let site = anchors
        .get(node_index)
        .ok_or(VectorEditError::NodeOutOfRange {
            index: node_index,
            count,
        })?;

    match site.kind {
        // ---- (1) The operand exists: overwrite it in place. -------------
        AnchorKind::Editable => {
            // Replace the anchor's coordinate pair (operand indices 2k, 2k+1)
            // with the user-space target, then re-emit that one operator.
            // Everything else is byte-verbatim.
            let mut new_nums = site.operands.clone();
            let x_slot = site.pair_index * 2;
            let y_slot = x_slot + 1;
            // The anchor bookkeeping only marks an operator Editable when its
            // arity matched, so `y_slot` is in range; the guard degrades an
            // impossible out-of-range to a by-name refusal rather than an
            // index-panic (crate panic-free policy).
            if x_slot >= new_nums.len() || y_slot >= new_nums.len() {
                return Err(VectorEditError::MalformedOperand);
            }
            if let Some(x) = new_nums.get_mut(x_slot) {
                *x = to_user.x;
            }
            if let Some(y) = new_nums.get_mut(y_slot) {
                *y = to_user.y;
            }
            let mut edits = vec![(
                site.byte_start,
                site.byte_end,
                emit_op(&new_nums, &site.keyword),
            )];
            Ok(PlannedEdit {
                content: splice(&content.buf, &mut edits),
                operators_touched: 1,
                disclosures: clip,
            })
        }

        // ---- (2) `re` names a size, not four corners: expand it. --------
        AnchorKind::Rectangle { corner } => {
            let [x, y, w, h] = site.operands[..] else {
                return Err(VectorEditError::MalformedOperand);
            };
            // The spec's own equivalence (§8.5.2.1, Table 59): `x y w h re`
            // IS `x y m / x+w y l / x+w y+h l / x y+h l / h`. Emitting that
            // form changes no pixel — the trailing `h` is load-bearing and
            // must not be dropped, because a stroked subpath left OPEN gets
            // two line caps where the closed one gets a corner join.
            let mut pts = rect_corners(x, y, w, h);
            let Some(dragged) = pts.get_mut(corner) else {
                return Err(VectorEditError::NodeOutOfRange {
                    index: node_index,
                    count,
                });
            };
            *dragged = to_user;

            let mut out = Vec::new();
            for (i, p) in pts.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                out.extend_from_slice(&emit_op(&[p.x, p.y], if i == 0 { b"m" } else { b"l" }));
            }
            out.extend_from_slice(b" h");

            let mut edits = vec![(site.byte_start, site.byte_end, out)];
            Ok(PlannedEdit {
                content: splice(&content.buf, &mut edits),
                operators_touched: 1,
                disclosures: [
                    clip,
                    vec![
                        "This shape was stored as a rectangle, which can only describe a \
                     box with square corners. Moving one corner on its own makes it a \
                     four-sided shape that is no longer a box, so it has been rewritten \
                     as four lines. It draws identically; dragging the corner back will \
                     not restore the original rectangle form."
                            .to_owned(),
                    ],
                ]
                .concat(),
            })
        }

        // ---- (3) Nothing names this point: write the `m` that was left
        //          implicit, immediately before the segment that inherits it.
        AnchorKind::Implicit => {
            // After `h` (or `re`) the current point is the closed subpath's
            // start, and the next segment operator reopens there with no `m`
            // of its own (§8.5.2.1). An explicit `m` at the target overrides
            // exactly that inheritance and nothing else: the closed subpath
            // is already terminated, so no earlier geometry can see it.
            let mut m = emit_op(&[to_user.x, to_user.y], b"m");
            m.push(b' ');
            m.extend_from_slice(content.buf.get(site.byte_start..site.byte_end).ok_or(
                VectorEditError::NodeOutOfRange {
                    index: node_index,
                    count,
                },
            )?);
            let mut edits = vec![(site.byte_start, site.byte_end, m)];
            Ok(PlannedEdit {
                content: splice(&content.buf, &mut edits),
                operators_touched: 1,
                disclosures: [
                    clip,
                    vec![
                        "This point had no coordinates of its own — the file re-used the \
                     start of the shape before it. A move instruction naming the point \
                     has been added so it can be placed independently."
                            .to_owned(),
                    ],
                ]
                .concat(),
            })
        }
    }
}

/// Which of a node's two Bézier control points ("handles") to move.
///
/// An on-curve anchor sits between at most two segments, and each contributes
/// one control point to it: the segment arriving contributes its SECOND
/// control point, the segment leaving its FIRST. They are the two levers that
/// shape the curve on either side of the point without moving the point.
///
/// Named for direction of travel along the path rather than "first/second",
/// because first-and-second are properties of an *operator* while an operator
/// says nothing about which node a front end has selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handle {
    /// The control point governing the curve as it ARRIVES at the node — the
    /// second control point of the segment that ends here.
    Incoming,
    /// The control point governing the curve as it LEAVES the node — the
    /// first control point of the segment that starts here.
    Outgoing,
}

/// Plan a **handle drag**: move one Bézier control point of `node_index`,
/// leaving the on-curve node itself exactly where it is.
///
/// This is the operation that changes a curve's SHAPE. Without it
/// [`plan_move_node`] can only move the points a curve passes through, so a
/// curve's curvature was not editable at all.
///
/// # Implicit control points, and why a `v`/`y` drag rewrites the operator
///
/// Table 59 (§8.5.2.1) gives cubic segments three spellings, and two of them
/// omit a control point by making it equal to a point they already have:
///
/// | operator | operands | first control | second control |
/// |---|---|---|---|
/// | `c` | `x1 y1 x2 y2 x3 y3` | `(x1,y1)` | `(x2,y2)` |
/// | `v` | `x2 y2 x3 y3` | **the current point** | `(x2,y2)` |
/// | `y` | `x1 y1 x3 y3` | `(x1,y1)` | **the endpoint** |
///
/// So dragging `v`'s incoming-side handle or `y`'s outgoing-side handle is
/// an in-place operand rewrite, while dragging the OTHER one asks to move a
/// point whose whole definition is "equal to that other point". It cannot
/// stay implicit and also move, so the segment is re-spelled as the `c` that
/// states both control points — the same materialize-rather-than-refuse move
/// Pass 30.0 makes for `re` corners, and disclosed for the same reason.
///
/// # What is refused, and why not silently converted
///
/// A straight segment (`l`, or a rectangle edge) has no handles. Dragging one
/// could only mean "turn this line into a curve", which is a different
/// operation with a different name: it changes the object's shape vocabulary
/// rather than adjusting a curve that already exists. Guessing it from a drag
/// on a control point that was never drawn would be exactly the silent
/// reinterpretation rule 4 forbids, so it is refused by name and a caller that
/// wants the conversion asks for it.
///
/// # Errors
///
/// [`VectorEditError::NodeOutOfRange`], [`VectorEditError::NoHandleHere`]
/// (the neighbouring segment is straight, absent, or across a subpath
/// boundary), [`VectorEditError::DegenerateCtm`], or
/// [`VectorEditError::MalformedOperand`].
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{decompose, NoXObjects, Matrix, Point, VectorObject};
/// use pdfce_core::vector::edit::{plan_move_handle, Handle};
///
/// let cs = ContentStream::parse(b"0 0 m 10 40 60 40 70 0 c S".to_vec()).unwrap();
/// let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
/// let VectorObject::Path(path) = &model.objects[0] else { unreachable!() };
/// // Node 0 is the `m`; its OUTGOING handle is the `c`'s first control point.
/// let plan = plan_move_handle(&cs, path, 0, Handle::Outgoing, Point::new(5.0, 90.0)).unwrap();
/// assert_eq!(plan.content, b"0 0 m 5 90 60 40 70 0 c S");
/// ```
pub fn plan_move_handle(
    content: &ContentStream,
    obj: &PathObject,
    node_index: usize,
    handle: Handle,
    to_page: Point,
) -> Result<PlannedEdit, VectorEditError> {
    let inv = obj.ctm.inverse().ok_or(VectorEditError::DegenerateCtm)?;
    let to_user = inv.map_point(to_page);
    let clip = clip_disclosure(content, obj);

    let anchors = enumerate_anchors(content, obj.tokens.start, obj.tokens.end);
    let count = anchors.len();
    if node_index >= count {
        return Err(VectorEditError::NodeOutOfRange {
            index: node_index,
            count,
        });
    }

    // WHICH operator carries the handle:
    //
    // - Incoming: the operator that ENDS at this node — the node's own site.
    // - Outgoing: the operator that ends at the NEXT node, since that is the
    //   segment leaving here.
    //
    // Anchor indices are object-scoped and run straight across subpath
    // boundaries, so "the next anchor" after a subpath's last node is the NEXT
    // SUBPATH's first node — and reshaping its segment would edit geometry the
    // operator never selected.
    //
    // What actually prevents that is the KEYWORD match below: every anchor
    // that opens a subpath carries `m`, `re`, or (for an `h`-reopen) no
    // keyword at all, none of which is a curve, so all three fall through to
    // `NoHandleHere`. The `is_start` filter here is a second line of defence
    // that cannot fire today. It is kept because it states the intent
    // structurally rather than as a consequence of which keywords happen to
    // exist — but it is NOT the thing to rely on when touching the keyword
    // match, and the test that covers this boundary passes with this filter
    // deleted. That was verified, not assumed.
    let site = match handle {
        Handle::Incoming => anchors.get(node_index),
        Handle::Outgoing => anchors.get(node_index + 1).filter(|next| !next.is_start),
    }
    .ok_or(VectorEditError::NoHandleHere {
        index: node_index,
        handle,
    })?;

    // Which operand pair holds the requested control point, per Table 59 —
    // `None` where the spelling leaves it implicit and the operator has to be
    // promoted to `c`.
    let pair: Option<usize> = match (site.keyword.as_slice(), handle) {
        (b"c", Handle::Outgoing) => Some(0),
        (b"c", Handle::Incoming) => Some(1),
        // `v`'s second control point is explicit; its first IS the current
        // point, so it can only move by becoming a `c`.
        (b"v", Handle::Incoming) => Some(0),
        (b"v", Handle::Outgoing) => None,
        // `y` mirrors it: first explicit, second equals the endpoint.
        (b"y", Handle::Outgoing) => Some(0),
        (b"y", Handle::Incoming) => None,
        // `m`, `l`, `re`, or an implicit start: no curve on that side.
        _ => {
            return Err(VectorEditError::NoHandleHere {
                index: node_index,
                handle,
            });
        }
    };

    let (bytes, disclosures) = match pair {
        // In-place operand rewrite: the control point is already written down.
        Some(p) => {
            let mut nums = site.operands.clone();
            let (xs, ys) = (p * 2, p * 2 + 1);
            if ys >= nums.len() {
                return Err(VectorEditError::MalformedOperand);
            }
            if let Some(x) = nums.get_mut(xs) {
                *x = to_user.x;
            }
            if let Some(y) = nums.get_mut(ys) {
                *y = to_user.y;
            }
            (emit_op(&nums, &site.keyword), clip)
        }
        // Promotion: re-spell the segment as the `c` that states BOTH control
        // points, so the one that was implicit can hold its own value.
        None => {
            let promoted = promote_to_cubic(site, handle, to_user)?;
            (
                promoted,
                [
                    clip,
                    vec![
                        "This curve was written in a short form that left one of its \
                         shaping handles implied by another point, so it could not be \
                         moved on its own. The curve has been rewritten in the long \
                         form that states both handles. It draws identically."
                            .to_owned(),
                    ],
                ]
                .concat(),
            )
        }
    };

    let mut edits = vec![(site.byte_start, site.byte_end, bytes)];
    Ok(PlannedEdit {
        content: splice(&content.buf, &mut edits),
        operators_touched: 1,
        disclosures,
    })
}

/// Re-spell a `v` or `y` segment as the equivalent `c`, with the previously
/// implicit control point set to `to_user`.
///
/// The two spellings are shorthands for a cubic whose omitted control point
/// duplicates a point the operator already carries (Table 59), so the `c` form
/// is exactly equivalent when it repeats that point — which is what makes this
/// a re-spelling rather than a change of shape.
///
/// `v`'s implicit FIRST control point is the current point, which is not in
/// the operator's own operands; it is the previous node, which the caller
/// resolves. Here the value being written is the operator's new position, so
/// the old one is not needed at all — the point is being replaced, not copied.
fn promote_to_cubic(
    site: &AnchorSite,
    handle: Handle,
    to_user: Point,
) -> Result<Vec<u8>, VectorEditError> {
    let &[a, b, c, d] = site.operands.as_slice() else {
        return Err(VectorEditError::MalformedOperand);
    };
    let nums = match (site.keyword.as_slice(), handle) {
        // `x2 y2 x3 y3 v` → `NEW x2 y2 x3 y3 c`: the dragged first control
        // point takes the lead, the explicit second and the endpoint follow.
        (b"v", Handle::Outgoing) => [to_user.x, to_user.y, a, b, c, d],
        // `x1 y1 x3 y3 y` → `x1 y1 NEW x3 y3 c`: the explicit first control
        // point stays, the dragged second takes the middle, endpoint last.
        (b"y", Handle::Incoming) => [a, b, to_user.x, to_user.y, c, d],
        _ => return Err(VectorEditError::MalformedOperand),
    };
    Ok(emit_op(&nums, b"c"))
}

/// The number of node-draggable and non-draggable anchors an object
/// exposes, in decomposition order — the count a front end validates a node
/// index against, and the value [`VectorEditError::NodeOutOfRange`] reports.
///
/// Equal to `obj.subpaths.iter().map(|s| s.anchors().count()).sum()` by
/// construction (this walk mirrors [`super::decompose`]); provided so the
/// CLI/GUI need not re-derive the flattening.
#[must_use]
pub fn anchor_count(content: &ContentStream, obj: &PathObject) -> usize {
    enumerate_anchors(content, obj.tokens.start, obj.tokens.end).len()
}

// ---------------------------------------------------------------------------
// Operand arithmetic
// ---------------------------------------------------------------------------

/// Translate the point operands of a construction operator by `(du, dv)`.
/// `translate` has one flag per **point pair** (`m`/`l` = 1, `v`/`y` = 2,
/// `c` = 3); a `true` pair is shifted. Returns `None` if `nums` does not
/// hold exactly `2 × translate.len()` operands (a malformed operator).
fn translate_points(nums: &[f64], translate: &[bool], du: f64, dv: f64) -> Option<Vec<f64>> {
    if nums.len() != translate.len() * 2 {
        return None;
    }
    let mut out = nums.to_vec();
    for (pair, &shift) in translate.iter().enumerate() {
        if shift {
            // Indices are in range by the length check above; checked access
            // keeps the crate's panic-free policy (clippy::indexing_slicing).
            if let Some(x) = out.get_mut(pair * 2) {
                *x += du;
            }
            if let Some(y) = out.get_mut(pair * 2 + 1) {
                *y += dv;
            }
        }
    }
    Some(out)
}

/// Translate an `re x y w h` operator: shift the origin `(x, y)` by
/// `(du, dv)`, leave the size `(w, h)` unchanged. `None` if the arity is
/// not 4.
fn translate_rect(nums: &[f64], du: f64, dv: f64) -> Option<Vec<f64>> {
    match *nums {
        [x, y, w, h] => Some(vec![x + du, y + dv, w, h]),
        _ => None,
    }
}

/// Emit an operation as `operand operand … keyword` bytes (the `emit_tm`
/// pattern), the numbers formatted by the writer's total
/// [`emit_number`] and the keyword copied verbatim (so `f*`, `B*` survive).
fn emit_op(operands: &[f64], keyword: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, v) in operands.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        emit_number(&mut out, *v);
    }
    out.push(b' ');
    out.extend_from_slice(keyword);
    out
}

// ---------------------------------------------------------------------------
// Operation iteration over a token range
// ---------------------------------------------------------------------------

/// One operation (operand run + operator) inside a token range — the same
/// segmentation [`ContentStream::operations`] performs, but bounded to a
/// half-open token range and exposing the byte bounds the splice needs.
struct OpItem<'a> {
    operands: &'a [ContentToken],
    operator: &'a ContentToken,
}

impl OpItem<'_> {
    /// The operator keyword bytes, or `None` for an inline image (which is
    /// its own indivisible "operator" token but has no keyword).
    fn keyword<'b>(&self, buf: &'b [u8]) -> Option<&'b [u8]> {
        match self.operator.kind {
            ContentTokenKind::Operator => self.operator.span.slice(buf),
            _ => None,
        }
    }

    /// The numeric operands, in order (non-numeric operands skipped, matching
    /// the decomposer's tolerance).
    fn nums(&self) -> Vec<f64> {
        self.operands
            .iter()
            .filter_map(|t| match &t.kind {
                ContentTokenKind::Operand(o) => o.as_number(),
                _ => None,
            })
            .collect()
    }

    /// Byte offset of the operation's first operand (or the operator, when
    /// there are none) — the splice start.
    fn byte_start(&self) -> usize {
        self.operands
            .first()
            .map_or(self.operator.span.start, |t| t.span.start)
    }

    /// Byte offset one past the operator — the splice end.
    fn byte_end(&self) -> usize {
        self.operator.span.end()
    }
}

/// The operations whose operator token index lies in `[start, end)`.
///
/// `end` is the exclusive one-past-the-painting-operator bound Pass 9a
/// captures ([`super::decompose::TokenRange`]), so the painting operator at
/// `end - 1` is included. Operand runs are grouped exactly as
/// [`ContentStream::operations`] groups them.
fn ops_in_range(content: &ContentStream, start: usize, end: usize) -> Vec<OpItem<'_>> {
    let mut out = Vec::new();
    let end = end.min(content.tokens.len());
    let start = start.min(end);
    let mut run_start = start;
    for i in start..end {
        let Some(tok) = content.tokens.get(i) else {
            break;
        };
        if matches!(tok.kind, ContentTokenKind::Operand(_)) {
            continue;
        }
        let operands = content.tokens.get(run_start..i).unwrap_or(&[]);
        out.push(OpItem {
            operands,
            operator: tok,
        });
        run_start = i + 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Anchor enumeration (mirrors decompose's subpath bookkeeping)
// ---------------------------------------------------------------------------

/// How an anchor's coordinates are carried in the content stream — which
/// decides HOW a node drag rewrites it, not WHETHER it can.
///
/// All three are draggable as of Pass 30.0. The distinction survives because
/// the three need three different rewrites: one replaces an operand pair, one
/// expands an operator, one inserts a new operator. See [`plan_move_node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorKind {
    /// A real operand pair (`m`/`l`/`c`/`v`/`y`) — rewritable in place.
    Editable,
    /// A corner of an `re` rectangle. `re` carries an origin and a *size*, so
    /// no operand names this corner (only corner 0 appears literally, and
    /// even it cannot move alone without dragging the other three with it).
    /// Dragged by expanding the operator to its §8.5.2.1 equivalent.
    Rectangle {
        /// Which corner, in [`super::geometry::rect_corners`] order —
        /// `(x, y)`, `(x+w, y)`, `(x+w, y+h)`, `(x, y+h)` — which is also the
        /// order of the spec's equivalent `m`/`l`/`l`/`l` sequence, so the
        /// index means the same thing before and after the expansion.
        corner: usize,
    },
    /// The reused start of an `h`-reopened subpath (§8.5.2.1): its
    /// coordinates are *inherited* from the closed subpath's start rather
    /// than written anywhere. Dragged by inserting the `m` the file omitted.
    Implicit,
}

/// One anchor, with the operator that defines it (for the editable case).
struct AnchorSite {
    kind: AnchorKind,
    byte_start: usize,
    byte_end: usize,
    operands: Vec<f64>,
    keyword: Vec<u8>,
    /// Which operand **pair** carries the anchor's coordinates (`m`/`l` → 0,
    /// `v`/`y` → 1, `c` → 2). Ignored for non-editable anchors.
    pair_index: usize,
    /// Whether this anchor OPENS its subpath (an `m`, an `re` corner, or an
    /// implicit `h`-reopen) rather than ending a segment.
    ///
    /// Needed by handle editing to tell "the next anchor is the far end of my
    /// outgoing segment" from "the next anchor belongs to the next subpath
    /// entirely" — indices are object-scoped and run straight across the
    /// boundary, so without this a handle drag on a subpath's last node would
    /// silently reshape the following subpath's first segment.
    is_start: bool,
}

/// Enumerate the object's anchors in the SAME order
/// `obj.subpaths.flat_map(Subpath::anchors)` produces (module docs), by
/// replaying [`super::decompose`]'s subpath / empty-subpath / `h`-reopen
/// state machine over the token range.
fn enumerate_anchors(content: &ContentStream, start: usize, end: usize) -> Vec<AnchorSite> {
    let mut w = AnchorWalk::default();
    for item in ops_in_range(content, start, end) {
        let Some(keyword) = item.keyword(&content.buf) else {
            continue;
        };
        let nums = item.nums();
        let bs = item.byte_start();
        let be = item.byte_end();
        match keyword {
            b"m" => {
                if nums.len() == 2 {
                    // A new subpath: finalize the previous open one, then open
                    // at the `m` anchor.
                    w.finalize_open();
                    w.open_start = Some(AnchorSite {
                        kind: AnchorKind::Editable,
                        byte_start: bs,
                        byte_end: be,
                        operands: nums,
                        keyword: keyword.to_vec(),
                        pair_index: 0,
                        is_start: true,
                    });
                    w.open_ends.clear();
                    w.current = true;
                    w.needs_move = false;
                }
            }
            b"l" => w.segment(&nums, 1, 0, bs, be, keyword),
            b"c" => w.segment(&nums, 3, 2, bs, be, keyword),
            b"v" | b"y" => w.segment(&nums, 2, 1, bs, be, keyword),
            b"re" => {
                if nums.len() == 4 {
                    // A complete closed subpath of four corners, one operator.
                    w.finalize_open();
                    for corner in 0..4 {
                        w.committed.push(AnchorSite {
                            kind: AnchorKind::Rectangle { corner },
                            byte_start: bs,
                            byte_end: be,
                            operands: nums.clone(),
                            keyword: keyword.to_vec(),
                            pair_index: 0,
                            is_start: true,
                        });
                    }
                    w.current = true;
                    w.needs_move = true;
                }
            }
            b"h" => {
                // Close: finalize the open subpath; the current point becomes
                // the subpath start and the next segment reopens there.
                w.finalize_open();
                w.needs_move = true;
            }
            b"S" | b"s" | b"f" | b"F" | b"f*" | b"B" | b"B*" | b"b" | b"b*" | b"n" => {
                w.finalize_open();
            }
            _ => {}
        }
    }
    // A trailing open path with no painting operator is dropped (matches the
    // decomposer discarding an unpainted path); its anchors are not counted.
    w.committed
}

/// The anchor-walk state (mirrors `decompose::PathAccum`'s `open`/`current`/
/// `needs_move`, minus geometry).
#[derive(Default)]
struct AnchorWalk {
    committed: Vec<AnchorSite>,
    open_start: Option<AnchorSite>,
    open_ends: Vec<AnchorSite>,
    current: bool,
    needs_move: bool,
}

impl AnchorWalk {
    /// Commit the open subpath's anchors iff it has at least one segment end
    /// (a lone `m` produces no contour — the decomposer's `finalize_open_pa`
    /// drop of an empty open subpath).
    fn finalize_open(&mut self) {
        if self.open_ends.is_empty() {
            self.open_start = None;
            self.open_ends.clear();
        } else if let Some(startsite) = self.open_start.take() {
            self.committed.push(startsite);
            self.committed.append(&mut self.open_ends);
        } else {
            // A segment without a start (reopened implicit) — the implicit
            // start was already pushed as open_start in `segment`; if it is
            // None here the ends stand alone (defensive).
            self.committed.append(&mut self.open_ends);
        }
    }

    /// Handle a segment operator (`l`/`c`/`v`/`y`): `pairs` = point-pair
    /// count, `anchor_pair` = which pair is the segment's on-curve endpoint.
    fn segment(
        &mut self,
        nums: &[f64],
        pairs: usize,
        anchor_pair: usize,
        bs: usize,
        be: usize,
        keyword: &[u8],
    ) {
        if nums.len() != pairs * 2 {
            return; // malformed arity: no anchor (decomposer skips it)
        }
        if !self.current {
            return; // §8.5.2.1 segment with no current point: skipped
        }
        if self.needs_move {
            // Reopen a subpath at the current point: an implicit start anchor
            // with no operand of its own.
            self.open_start = Some(AnchorSite {
                kind: AnchorKind::Implicit,
                byte_start: bs,
                byte_end: be,
                operands: Vec::new(),
                keyword: Vec::new(),
                pair_index: 0,
                is_start: true,
            });
            self.open_ends.clear();
            self.needs_move = false;
        }
        self.open_ends.push(AnchorSite {
            kind: AnchorKind::Editable,
            byte_start: bs,
            byte_end: be,
            operands: nums.to_vec(),
            keyword: keyword.to_vec(),
            pair_index: anchor_pair,
            is_start: false,
        });
    }
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
    use crate::vector::{Matrix, NoXObjects, decompose};

    /// Decompose a source content stream and return `(stream, first path)`.
    fn path_of(src: &[u8]) -> (ContentStream, PathObject) {
        let cs = ContentStream::parse(src.to_vec()).unwrap();
        let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let path = model
            .objects
            .iter()
            .find_map(|o| match o {
                VectorObject::Path(p) => Some(p.clone()),
                _ => None,
            })
            .expect("a path object");
        (cs, path)
    }

    #[test]
    fn move_translates_every_construction_operand() {
        let (cs, path) = path_of(b"10 20 m 100 200 l 40 50 60 70 80 90 c S");
        let plan = plan_move(&cs, &path, 5.0, -3.0).unwrap();
        assert_eq!(plan.content, b"15 17 m 105 197 l 45 47 65 67 85 87 c S");
        assert_eq!(plan.operators_touched, 3); // m, l, c
    }

    #[test]
    fn move_shifts_re_origin_but_not_size() {
        let (cs, path) = path_of(b"10 10 80 40 re f");
        let plan = plan_move(&cs, &path, 100.0, 0.0).unwrap();
        // x,y move by 100; w,h unchanged.
        assert_eq!(plan.content, b"110 10 80 40 re f");
    }

    #[test]
    fn move_is_ctm_aware() {
        // Object drawn under a 2× scale: a page-space drag of (10,0) is a
        // user-space drag of (5,0), so the user-space operands shift by 5.
        let (cs, path) = path_of(b"2 0 0 2 0 0 cm 0 0 m 10 0 l S");
        let plan = plan_move(&cs, &path, 10.0, 0.0).unwrap();
        // Only the object's operators are rewritten; the `cm` stays verbatim.
        assert_eq!(plan.content, b"2 0 0 2 0 0 cm 5 0 m 15 0 l S");
    }

    #[test]
    fn move_refuses_a_singular_ctm() {
        // A CTM scaled flat to a line (determinant 0).
        let (cs, path) = path_of(b"1 0 0 0 0 0 cm 0 0 m 10 0 l S");
        assert_eq!(
            plan_move(&cs, &path, 1.0, 1.0),
            Err(VectorEditError::DegenerateCtm)
        );
    }

    #[test]
    fn delete_removes_exactly_the_object_span() {
        let cs =
            ContentStream::parse(b"1 0 0 RG 10 20 m 100 200 l S 0 0 m 5 5 l S".to_vec()).unwrap();
        let model = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        // Delete the FIRST path (indices are paint order).
        let plan = plan_delete(&cs, &model.objects[0]).unwrap();
        // The `1 0 0 RG` state op and the second path survive; only the first
        // path's operators are gone.
        let text = String::from_utf8(plan.content).unwrap();
        assert!(text.contains("1 0 0 RG"), "preceding state op kept: {text}");
        assert!(!text.contains("100 200 l"), "first path removed: {text}");
        assert!(text.contains("5 5 l"), "second path kept: {text}");
    }

    #[test]
    fn node_drag_rewrites_one_anchor_only() {
        let (cs, path) = path_of(b"10 20 m 100 200 l 300 400 l S");
        // Node 0 = m start, 1 = first l, 2 = second l.
        let plan = plan_move_node(&cs, &path, 1, Point::new(120.0, 250.0)).unwrap();
        assert_eq!(plan.content, b"10 20 m 120 250 l 300 400 l S");
        // Node 0 moves the start.
        let (cs2, path2) = path_of(b"10 20 m 100 200 l S");
        let plan0 = plan_move_node(&cs2, &path2, 0, Point::new(0.0, 0.0)).unwrap();
        assert_eq!(plan0.content, b"0 0 m 100 200 l S");
    }

    #[test]
    fn node_drag_targets_the_curve_endpoint_not_a_handle() {
        // `c x1 y1 x2 y2 x3 y3`: the anchor is (x3,y3); the handles stay put.
        let (cs, path) = path_of(b"10 10 m 20 30 40 50 60 70 c S");
        // Node 1 is the `c` endpoint (60,70).
        let plan = plan_move_node(&cs, &path, 1, Point::new(99.0, 88.0)).unwrap();
        assert_eq!(plan.content, b"10 10 m 20 30 40 50 99 88 c S");
    }

    #[test]
    fn node_count_matches_the_decomposition() {
        let (cs, path) = path_of(b"10 20 m 100 200 l 300 400 l S");
        assert_eq!(anchor_count(&cs, &path), 3);
        // Equal to the flattened subpath anchors.
        let flat: usize = path.subpaths.iter().map(|s| s.anchors().count()).sum();
        assert_eq!(anchor_count(&cs, &path), flat);
    }

    #[test]
    fn node_drag_expands_a_rectangle_corner() {
        let (cs, path) = path_of(b"10 10 80 40 re f");
        // A rectangle has four anchors, all `re` corners.
        assert_eq!(anchor_count(&cs, &path), 4);
        let plan = plan_move_node(&cs, &path, 0, Point::new(0.0, 0.0)).unwrap();
        // The spec's own equivalence (Table 59) with corner 0 relocated. The
        // trailing `h` must survive: without it a stroked box gets caps, not
        // a corner join.
        assert_eq!(plan.content, b"0 0 m 90 10 l 90 50 l 10 50 l h f");
        assert_eq!(plan.disclosures.len(), 1);
    }

    #[test]
    fn node_drag_out_of_range_is_named() {
        let (cs, path) = path_of(b"10 20 m 100 200 l S");
        assert_eq!(
            plan_move_node(&cs, &path, 9, Point::new(0.0, 0.0)),
            Err(VectorEditError::NodeOutOfRange { index: 9, count: 2 })
        );
    }

    #[test]
    fn a_lone_move_contributes_no_anchor() {
        // `10 10 m` opens an empty subpath that the paint drops; the real
        // subpath starts at the second `m`. Anchor order must match.
        let (cs, path) = path_of(b"10 10 m 20 20 m 30 30 l S");
        // One committed subpath: start (20,20) + end (30,30) = 2 anchors.
        assert_eq!(anchor_count(&cs, &path), 2);
        assert_eq!(
            path.subpaths
                .iter()
                .map(|s| s.anchors().count())
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn degenerate_coordinates_do_not_panic() {
        let (cs, path) = path_of(b"0 0 m 10 10 l S");
        // A huge, non-finite-ish drag: the surgery must produce bytes, not panic.
        let _ = plan_move(&cs, &path, 1e308, -1e308);
        let _ = plan_move_node(&cs, &path, 1, Point::new(f64::MAX, f64::MIN));
    }
}
