//! # The editing session — command log, undo/redo, and the save-time diff
//!
//! `ARCHITECTURE.md` §11 in executable form. This module is the **only**
//! mutation path in pdfce: `pdfce-gui` and `pdfce-cli` both go through
//! [`EditSession`], and nothing anywhere constructs a
//! [`DirtySet`](crate::writer::DirtySet) with real changes except
//! [`EditSession::dirty_set`].
//!
//! §11.4 is why this lands in Pass 3.1 rather than later: *"the first
//! Pass that introduces any editing capability must build the
//! command-log/undo-stack mechanism as part of that Pass, not after"*.
//! Retrofitting undo onto edit code written for direct mutation is the
//! expensive move, so the very first two edits pdfce ever performs go
//! through the stack.
//!
//! ## The design in one picture
//!
//! ```text
//!   Document (the BASE revision)          ← immutable for the session's life
//!      buf: the retained source bytes     ← every ByteSpan indexes into this
//!      objects: id -> parsed value
//!      trailer
//!            │
//!            │  overlay
//!            ▼
//!   EditSession
//!      state:   id -> CURRENT value       ← only for objects an edit touched
//!      trailer: working copy
//!      undo:    Vec<Command>              ← each carries its own inverse
//!      redo:    Vec<Command>
//!            │
//!            │  dirty_set()  =  structural diff(state, base), at SAVE time
//!            ▼
//!   DirtySet  ──►  writer::save_incremental / save_full
//! ```
//!
//! The base document is **never mutated**. An edit writes into `state`;
//! an undo writes the recorded prior value back into `state`. That is
//! not merely convenient — it is what makes §5's verbatim-re-emission
//! path keep working while edits are in flight, because every untouched
//! object still has its original bytes and its original `ByteSpan`.
//!
//! ## THE bug this module exists to prevent (§11.1)
//!
//! > "the 'dirty set' … is computed as a **structural diff against the
//! > base revision at save time** — it is *not* the union of every object
//! > any command ever touched during the session. If a user edits an
//! > object and then undoes that specific edit before saving, that object
//! > must **not** appear in the incremental update."
//!
//! §7.5.6 requirement 1 is the spec-side reason: an update section
//! *"shall contain entries **only for** objects that have been changed,
//! replaced, or deleted"* — a restriction, not permission.
//!
//! [`EditSession::dirty_set`] enforces it **structurally rather than by
//! discipline**: it iterates `state` and *skips every entry whose value
//! equals the base document's*. An edit-then-undo leaves an entry in
//! `state` holding a value equal to the base, so it is skipped, and the
//! save is byte-identical to the input. There is no code path that could
//! do otherwise, because history is never consulted at save time at all.
//!
//! Three properties fall out of that, and each is worth naming because
//! each would need its own defensive code under a history-replay design:
//!
//! - **Bounding the undo stack is free.** [`MAX_UNDO_DEPTH`] can drop the
//!   oldest command without affecting what gets saved: the state is the
//!   truth, and the dropped command only cost the operator the ability to
//!   step back that far.
//! - **Coalescing is free.** N edits to one object leave one entry in
//!   `state`, so the update section carries that object once, not N
//!   times. Not a special case — a consequence of keying by id.
//! - **A "modified?" indicator cannot lie.** [`EditSession::is_modified`]
//!   asks the same question the writer will: *does anything currently
//!   differ from the base?* An edit-then-undo reports unmodified, which
//!   is what the operator sees on screen.
//!
//! ## Why each command stores its prior value rather than an operation
//!
//! §11.1 asks for commands with `apply()` and an inverse. For a *value
//! replacement* — which is all Pass 3.1 has — the inverse **is** the
//! prior value, and storing it is strictly more robust than recomputing
//! it: a recomputed inverse can drift from what was actually replaced if
//! any other code path touches the same object in between.
//!
//! This is not §11.3's snapshot fallback in disguise. A snapshot is a
//! copy of state the command did not itself change; a [`Command`] here
//! records **exactly the entries it wrote**, and nothing else. When Pass
//! 3.2 adds structural operations whose before-image is genuinely large
//! (a page reorder), §11.3 permits a coarse before/after page-order
//! snapshot as *one entry on this same stack* — the [`Command`] type is
//! where that variant goes. Do not build a second, parallel undo system
//! for it.
//!
//! ## Pass 3.1's mutation surface, and why it is this small
//!
//! Two edits only — document `/Info` metadata (§14.3.3) and page
//! `/Rotate` (Table 30). Decision 007 §5.1 chose them deliberately:
//! *"dictionary values only — no content stream, no appearance stream,
//! no font. That isolates the dirty-set machinery so it can be tested
//! without content re-emission confounding it."* Everything in this
//! module is therefore about **when** an object is written, never about
//! how its bytes are produced.
//!
//! One consequence to hold on to for Pass 3.2: because no Pass 3.1 edit
//! touches a content stream, a resource, or the page tree's *shape*, the
//! renderer may keep reading the **base** document's object graph and
//! still be correct — the one rendering-relevant value an edit can change
//! is `/Rotate`, and that travels in the [`Page`] value
//! [`EditSession::pages`] hands out. The moment an edit can alter a
//! content stream, that shortcut is gone and the renderer needs an
//! overlay-aware view.
//!
//! ## Spec sources
//!
//! - `iso32000__s__7.5.6.md` — update sections carry changed objects only
//! - `iso32000__s__14.4.md` — `/ID[1]` refreshes when something changed
//! - `iso32000__s__7.7.3.md` — Table 30 `/Rotate`: *"shall be a multiple
//!   of 90"*, clockwise, inheritable
//! - `iso32000__s__7.5.5.md` — Table 15: `/Info` *"shall be an indirect
//!   reference"*
//! - `iso32000__s__7.3.4.md` — string syntax (the *interpretation* of the
//!   bytes is §7.9.2, which is a **recorded gap** in the spec RAG; see
//!   [`encode_text_string`])

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::annot::AnnotFlags;
use crate::annot_author::{self, MarkupSpec, TextAnnotSpec};
use crate::dimension::{
    AUTHORED_ANNOT_KEYS, AUTHORED_MEASURE_KEY, DEFAULT_GROUP_ID, DimStandard, DimensionId,
    DimensionKind, DimensionModel, DimensionStyle, GroupId, NumberFormat, ScaleState, Unit,
    author_dimension, build_ocg, build_ocproperties, deserialize_model, serialize_model,
};
use crate::document::Document;
use crate::fontdata::Std14;
use crate::forms::{self, ButtonKind, Field, FieldType};
use crate::forms_author::{self, FieldPath, FieldShape, FormAuthorError};
use crate::graph::ObjectGraph;
use crate::object::{Dict, IndirectObject, Name, ObjId, Object, Stream};
use crate::page_tree::{self, Page, PageSlot, PageTreeError};
use crate::pageops::references::{DanglingReport, census_dangling};
use crate::signature::{SaveMode, SignatureCensus, SignatureImpact, census, impact_of};
use crate::span::ByteSpan;
use crate::vartext::FontResource;
use crate::view::{DocumentView, StreamSource};
use crate::writer::content::ContentBuilder;
use crate::writer::{DirtySet, SaveOptions, SaveReport, WriteError};

/// How many commands the undo stack keeps before the oldest is dropped.
///
/// `ARCHITECTURE.md` §11.1: *"Bound the undo history (a configurable max
/// operation count) rather than keeping it unbounded — large documents
/// with long editing sessions shouldn't accumulate unbounded command-
/// object memory. Acrobat itself bounds undo."*
///
/// Dropping the oldest command is safe **only** because the dirty set is
/// a diff against the base rather than a replay of history (module
/// docs). Under a replay design, dropping a command would corrupt what
/// gets saved; here it costs exactly what it appears to cost — the
/// operator can no longer step back past that point.
pub const MAX_UNDO_DEPTH: usize = 256;

/// The document-information-dictionary fields an operator may edit
/// (§14.3.3, Table 317).
///
/// A closed enum rather than an arbitrary key, for two reasons. The
/// first is ordinary API hygiene: a typo'd key would silently create a
/// dead entry. The second is R41 — `/Producer` is deliberately **absent
/// from this list**, because pdfce's producer identity is governed by
/// [`ProducerPolicy`](crate::writer::ProducerPolicy) and is the one
/// field whose no-fingerprint rule must not be reachable through a
/// general-purpose metadata editor.
///
/// `/CreationDate` and `/ModDate` are absent for a different reason:
/// they are §7.9.4 date strings, which need their own encoder and their
/// own "does pdfce set `/ModDate` automatically?" policy question (it
/// should not, silently — that is a fingerprint). Both belong to a later
/// Pass with the Acrobat-parity scoping the metadata bucket will get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum InfoField {
    /// `/Title` — the document's title.
    Title,
    /// `/Author` — the name of the person who created the document.
    Author,
    /// `/Subject` — the subject of the document.
    Subject,
    /// `/Keywords` — keywords associated with the document.
    Keywords,
}

impl InfoField {
    /// The dictionary key this field is stored under.
    #[must_use]
    pub const fn key(self) -> &'static [u8] {
        match self {
            Self::Title => b"Title",
            Self::Author => b"Author",
            Self::Subject => b"Subject",
            Self::Keywords => b"Keywords",
        }
    }

    /// Every editable field, in the order a properties panel should show
    /// them. Provided so a front end enumerates the real list instead of
    /// hard-coding one that drifts when a field is added.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Title, Self::Author, Self::Subject, Self::Keywords]
    }
}

/// What an undo-stack entry did, in structured form.
///
/// Deliberately **not** a display string. Decision 002 R1 keeps every
/// user-facing string in `pdfce-gui`'s `ui_text` catalog, and R4 makes
/// core diagnostics structured data a front end maps to its own text —
/// so `pdfce-core` returning "Set title" would put an English string in
/// the wrong crate and make it invisible to a future localization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandKind {
    /// A document-information field was given a value.
    SetInfoField(InfoField),
    /// A document-information field was removed.
    ClearInfoField(InfoField),
    /// One page's `/Rotate` was set. The index is into
    /// [`EditSession::pages`]' document-order list.
    SetPageRotation {
        /// 0-based page index.
        page_index: usize,
        /// The rotation that was applied, normalized to {0, 90, 180, 270}.
        degrees: u16,
    },
    /// Pages were removed from the document (Pass 3.2).
    DeletePages {
        /// How many pages the one operation removed.
        count: usize,
    },
    /// The document's page order changed (Pass 3.2).
    ///
    /// Carries only a count, because §11.3 makes a reorder **one**
    /// undo entry however many pages moved (*"reordering 50 pages in one
    /// drag operation"* is its named example) and a front end labelling
    /// the Undo control needs a magnitude, not a permutation.
    ReorderPages {
        /// How many pages ended up somewhere different.
        count: usize,
    },
    /// Several pages were rotated in one operation (Pass 3.2).
    RotatePages {
        /// How many pages the one operation turned.
        count: usize,
        /// The turn that was applied, in degrees, signed.
        delta: i32,
    },
    /// A geometric-markup annotation was authored onto a page (Pass 6.1).
    /// One gesture — one annotation — is one undo entry (§11.3, R49): the
    /// created appearance stream, the created annotation dictionary, and
    /// the page's `/Annots` patch all undo together.
    AddAnnotation {
        /// Which markup subtype was added, for an undo-control label.
        kind: AnnotKind,
    },
    /// A NEW form field was authored onto a page: the merged field/widget
    /// dictionary, its baked `/AP`, the page's `/Annots` patch and the
    /// `/AcroForm` `/Fields` registration, all as ONE undo entry.
    ///
    /// Additive (R46): no existing content stream or object is rewritten.
    /// Carries no payload: `CommandKind` is `Copy`, and a `String` name
    /// would force that off for the sake of an undo LABEL — a bad trade,
    /// since the label can be generic while `Copy` is relied on throughout.
    AddFormField,
    /// A form field, or ONE of its widgets, was deleted (decision 020
    /// §3.6.3): the widget(s) un-listed from their pages' `/Annots`, the
    /// field's `/Kids` or `/AcroForm /Fields` registration patched, any
    /// grouping node the removal emptied pruned, and — when the deleted
    /// widget held the field's value — `/V` and the survivors' `/AS` cleared
    /// to `/Off`, all as ONE undo entry.
    ///
    /// SUBTRACTIVE, and therefore the deliberate counterpart to
    /// [`Self::AddFormField`]'s R46 additivity: this is one of the few
    /// commands that removes objects rather than appending. Everything it
    /// touches goes in one entry for the same reason creation does — a field
    /// registered but not annotated, or annotated but not registered, is a
    /// document no undo can repair.
    ///
    /// Carries no payload, matching `AddFormField`: `CommandKind` is `Copy`.
    DeleteFormField,
    /// A field's partial name `/T` was changed (decision 020's F6).
    ///
    /// Exactly one dictionary is written, however many fields the operator
    /// sees renamed: §12.7.3.2 derives every descendant's fully-qualified
    /// name from this node's, so the subtree follows without being touched.
    /// One undo entry restores the whole apparent rename for the same reason.
    RenameFormField,
    /// A text or choice field's value was set and its appearance
    /// regenerated (Pass 7, §12.7.3.3). One fill — the field's `/V` and
    /// every widget's `/AP` — is one undo entry.
    FillTextField,
    /// A check-box or radio-button field's state was selected (Pass 7,
    /// §12.7.4.2.3): the field's `/V` and the widgets' `/AS` set together,
    /// with no appearance regeneration (state selection, not generation).
    SetButtonState,
    /// A choice (list/combo) field's selection was set (Pass 7.1,
    /// §12.7.4.4): the field's `/V` (a string, or an array under
    /// `MultiSelect`), its `/I` selected-index array, and every widget's
    /// `/AP` regenerated to show the selected display value(s).
    SetChoiceValue,
    /// All widget appearances that needed regeneration were rebuilt and
    /// `/NeedAppearances` cleared (Pass 7.1, R51). One save-side operation,
    /// one undo entry.
    RegenerateAppearances {
        /// How many widget appearances were regenerated.
        count: usize,
    },
    /// Form fields were flattened — their appearances burned into page
    /// content and the fields removed from `/AcroForm` and `/Annots`
    /// (Pass 7.1, R48). Destructive; one undo entry.
    FlattenFields {
        /// How many fields were flattened.
        count: usize,
    },
    /// One `/Redact` **mark** was removed from a page before it was ever
    /// applied (Pass 8.1 review surface, R52).
    ///
    /// Deliberately its own kind rather than a generic "delete annotation":
    /// the operator-facing meaning is *"I decided not to redact that after
    /// all"*, and an Undo control that says so is a different sentence from
    /// one that says "delete annotation". It is also the narrowest possible
    /// command — [`EditSession::delete_redaction_mark`] refuses any
    /// annotation that is not a `/Redact` — so this kind cannot be
    /// repurposed later into a general annotation delete without the
    /// compiler pointing at every reader.
    ///
    /// **Removing a mark removes nothing from the document's content.** The
    /// mark was never applied; the covered content was always still there.
    /// This command is the *reverse of marking*, not a reverse of redacting
    /// (which has none — see [`crate::redact::apply_redactions`]).
    DeleteRedactionMark,
    /// One in-place page-text REPLACE edit (Pass 14.3 §0.2): the page's
    /// content stream object (+ any collapsed extra content streams) was
    /// rewritten by the 14.1 advance-preserving surgery, recorded as ONE
    /// undo-able command so the GUI's per-keystroke-accepted edits undo one
    /// at a time exactly like every other mutation. The mutation lands on the
    /// session's in-memory content object (staged bytes + `state` overlay),
    /// NOT as pre-saved bytes — see
    /// [`EditSession::edit_text`]. The verbatim disclosure/report is returned
    /// by that method, not carried on the command (a front end labels its
    /// Undo control from this kind alone).
    EditText,
    /// One in-place page-text FORMAT edit (size / fill-colour model / font
    /// family; Pass 14.3 §0.2), recorded as ONE undo-able command by the same
    /// session-integrated 14.2 surgery. See [`EditSession::format_text`].
    FormatText,
    /// One within-block reflow (Pass 15.1): a recognized paragraph was
    /// re-wrapped to a new width/alignment/leading and its own
    /// content-stream object re-emitted at the new per-line origins/breaks
    /// (justified lines via `TJ` slack), recorded as ONE undo-able command so
    /// the operator's accepted re-wrap undoes atomically exactly like every
    /// other mutation (decision 015 §3.4, R75). Undo restores the
    /// byte-identical pre-reflow stream. The line-count magnitudes label a
    /// front end's Undo control; the verbatim disclosure/report is returned by
    /// [`EditSession::reflow_block`], not carried on the command.
    ReflowBlock {
        /// Line count before the re-wrap.
        lines_before: usize,
        /// Line count after the re-wrap.
        lines_after: usize,
    },
    /// One add-new-text operation (Pass 16.0 / FF-D): a fresh `BT…ET` run was
    /// synthesized at operator coordinates and APPENDED as a new content stream
    /// in the page `/Contents` array (§7.7.3.3), leaving every original content
    /// stream byte-identical, with one new Standard-14 font dict added to the
    /// page `/Resources` `/Font` (no embedding, R79). Recorded as ONE undo-able
    /// command so the operator's placed run undoes atomically exactly like
    /// every other mutation (decision 016 §3.5, R78); undo removes the two
    /// created objects and restores the byte-identical original page dict. It
    /// is page-content surgery, NEVER a FreeText annotation (R78) — see
    /// [`EditSession::add_text`]. The verbatim disclosure/report (font
    /// provenance, tagged-untagged, inheritance-safe resources) is returned by
    /// that method, not carried on the command.
    AddText,
    /// A dimension (Pass 12.M2) was authored onto a page: the `/Line`
    /// `/IT /LineDimension` annotation + its baked `/AP`, its group's `/OCG`
    /// (allocated on first use) registered in the catalog `/OCProperties`,
    /// and the authoritative `/PieceInfo` sidecar updated — all ONE undo
    /// entry (decision 011 §2.4, §12.9 / §8.11 / §14.5).
    AddDimension,
    /// A ce dimension was MOVED (Pass 25.5): its stored geometry was
    /// translated and its annotation + baked `/AP` regenerated from it. ONE
    /// undoable command. See [`EditSession::move_dimension`].
    MoveDimension,
    /// A ce dimension was PLACED (Pass 27.1): its standoff and/or its text
    /// position along the dimension line changed, and its appearance was
    /// regenerated. What it MEASURES is untouched — this writes only fields the
    /// value function does not read. ONE undoable command. See
    /// [`EditSession::place_dimension`].
    PlaceDimension,
    /// A ce dimension's radius-versus-diameter DISPLAY changed (Pass 34.2):
    /// the label now reports `2r` where it reported `r`, or the reverse, and
    /// the baked `/AP` was regenerated to say so. ONE undoable command.
    ///
    /// The fitted circle itself — centre, radius, fit residual — is untouched.
    /// This is a display property layered over immutable measured geometry,
    /// exactly like [`Self::SetGroupScale`]'s number format is, which is why
    /// changing it can never change what the ce dimension measures. See
    /// [`EditSession::set_dimension_display`].
    SetDimensionDisplay {
        /// The resulting display: `true` ⇒ diameter, `false` ⇒ radius.
        show_diameter: bool,
    },
    /// A ce dimension GROUP's drafting standard changed (Pass 27.2) and every
    /// wired member was regenerated to it. ONE undoable command. See
    /// [`EditSession::set_group_standard`].
    SetGroupStandard {
        /// How many members were regenerated.
        members: usize,
    },
    /// A ce dimension was DELETED (Pass 25.6): its `/Annots` reference, its
    /// annotation dictionary, its `/AP` stream and its sidecar record were all
    /// removed together. ONE undoable command. See
    /// [`EditSession::delete_dimension`].
    DeleteDimension,
    /// A dimension group's scale/units/format changed and every wired
    /// member's baked `/AP` label was regenerated (Pass 12.M2, the Pass 7.1
    /// regenerate pattern) — ONE undo entry however many members updated.
    SetGroupScale {
        /// How many member dimensions had their appearance regenerated.
        members: usize,
    },
    /// A dimension group's optional-content layer visibility was toggled
    /// (Pass 12.M2, §8.11 `/D` config `/OFF`) — ONE undo entry.
    ToggleDimensionLayer {
        /// The resulting default visibility.
        visible: bool,
    },
    /// One vector object was **moved** (Pass 9c-min, decision 011 §2.5): all
    /// of its path-construction operands were translated by a page-space
    /// `(dx, dy)` through content-stream surgery (the R46/§5.7 named
    /// exception — the mirror of redaction). ONE undoable command; undo
    /// restores the byte-identical pre-move content stream (Pass 3.1 command
    /// log). See [`EditSession::move_object`].
    MoveObject,
    /// One vector object was **deleted** (Pass 9c-min, decision 011 §2.5):
    /// its construction + painting operators were removed from the content
    /// stream (surgery, R46/§5.7). ONE undoable command; undo restores the
    /// byte-identical pre-delete content stream. See
    /// [`EditSession::delete_object`].
    DeleteObject,
    /// One anchor **node** of a path object was dragged (Pass 9c-min,
    /// decision 011 §2.5): exactly one coordinate pair was rewritten in an
    /// `m`/`l`/`c`/`v`/`y` operand list (surgery, R46/§5.7). ONE undoable
    /// command; undo restores the byte-identical pre-drag content stream.
    /// See [`EditSession::move_node`].
    MoveNode,
    /// One Bézier **handle** (control point) of a path object was dragged
    /// (Pass 30.1): one control-point pair was rewritten, or a `v`/`y`
    /// segment re-spelled as the equivalent `c` so a control point it left
    /// implicit could hold its own value (surgery, R46/§5.7). ONE undoable
    /// command; undo restores the byte-identical pre-drag content stream.
    /// Distinct from [`CommandKind::MoveNode`] because it changes a curve's
    /// SHAPE while the on-curve node stays put — a front end labelling its
    /// Undo control from the kind alone must not call it "move point".
    /// See [`EditSession::move_handle`].
    MoveHandle,
    /// ONE **subpath** of a path object was deleted (Pass 25.2): its
    /// construction operators were removed from the content stream while the
    /// object's other subpaths kept their exact bytes (surgery, R46/§5.7).
    ///
    /// A distinct kind from [`CommandKind::DeleteObject`] even though the
    /// mechanism is the same splice, because the two are different things to
    /// undo and different things to *read in a history*: "deleted a drawing
    /// view" and "deleted one line of a drawing view" must not look alike to
    /// an operator scanning what they did. See
    /// [`EditSession::delete_subpath`].
    DeleteSubpath,
    /// ONE **subpath** of a path object was moved (Pass 28.0): its
    /// construction operands were translated while the object's other
    /// subpaths kept their exact bytes. See [`EditSession::move_subpath`].
    MoveSubpath,
    /// ONE **anchor** was removed from a path object (Pass 36.1): the segment
    /// operator that produced it was excised (or, for a subpath's first
    /// anchor, its follower was promoted to the new `m`), joining its
    /// neighbours directly.
    ///
    /// A distinct kind from [`CommandKind::DeleteSubpath`] for the same reason
    /// that one is distinct from `DeleteObject`: "removed a point" and
    /// "removed a line" must not read alike in a history. The operator who
    /// pressed Delete meant exactly one of them, and before Pass 36.0 the GUI
    /// gave them the wrong one.
    DeleteNode,
}

/// Which geometric-markup subtype [`EditSession::add_markup`] authored,
/// for a [`CommandKind::AddAnnotation`] undo label. A projection of
/// [`crate::annot_author::MarkupSpec`]'s variant, kept `Copy` so it fits
/// the `Copy` [`CommandKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnnotKind {
    /// `/Square`.
    Square,
    /// `/Circle`.
    Circle,
    /// `/Line`.
    Line,
    /// `/Ink`.
    Ink,
    /// `/Polygon`.
    Polygon,
    /// `/PolyLine`.
    PolyLine,
    /// `/Highlight`.
    Highlight,
    /// `/Underline`.
    Underline,
    /// `/StrikeOut`.
    StrikeOut,
    /// `/Squiggly`.
    Squiggly,
    /// `/FreeText` (Pass 6.2).
    FreeText,
    /// `/Text` (sticky note, Pass 6.2).
    Text,
    /// `/Stamp` (Pass 6.2).
    Stamp,
    /// `/Redact` — a redaction mark (Pass 8, §12.5.6.23). The
    /// non-destructive MARK phase; removal is
    /// [`crate::redact::apply_redactions`].
    Redact,
}

/// One entry on the undo stack: the set of writes it performed, each
/// paired with the value that was there before.
///
/// `before`/`after` are `Option<Object>` because "absent from the
/// session state" is a real, distinct value: it means *fall through to
/// the base document*, and for a created object it means *this object
/// does not exist at all*. Collapsing the two into a sentinel `Object`
/// would make undoing a creation impossible to express.
#[derive(Debug, Clone)]
struct Command {
    kind: CommandKind,
    objects: Vec<ObjectWrite>,
    /// Objects whose *existence* this command changed, each with the
    /// before/after flag.
    ///
    /// A separate axis from `objects` because deletion is a separate
    /// axis: an object can be edited without being deleted, deleted
    /// without being edited, or — when a page is removed and its parent
    /// node rewritten in the same command — both, on different objects.
    /// Folding "deleted" into `ObjectWrite`'s `Option<Object>` would
    /// collide with the meaning that option already carries ("absent
    /// from the overlay, read through to the base").
    removals: Vec<Removal>,
    /// The whole working trailer before/after, when the command changed
    /// it. Whole rather than per-key because the only Pass 3.1 command
    /// that touches the trailer adds exactly one key to it, so a whole
    /// copy of a handful of entries is cheaper than the bookkeeping to
    /// track which — and it cannot get the restore subtly wrong.
    trailer: Option<(Dict, Dict)>,
}

/// One object-level write inside a [`Command`].
#[derive(Debug, Clone)]
struct ObjectWrite {
    id: ObjId,
    before: Option<Object>,
    after: Option<Object>,
}

/// What to author when creating a new **text** form field (§12.7.4.3).
///
/// Construct with [`NewTextField::new`] and refine with the `with_*`
/// builders. `#[non_exhaustive]` so a struct literal is not usable
/// out-of-crate and later fields never break callers — the same shape
/// [`crate::text_edit::AddTextRequest`] uses.
///
/// # The defaults are Acrobat's, deliberately
///
/// `Acrobat_Features/forms__field_creation_minimums.md` records that Acrobat
/// *"never leaves a newly-created field in a bare/incomplete state"* — it
/// writes a complete `/DA` and `/MK` at placement time. Its text-field floor
/// is **Helvetica, size 0 (auto), black, thin solid border, no fill**, and
/// these defaults match it.
///
/// Size **0** is not a placeholder: §12.7.3.3 makes it the auto-size trigger,
/// and [`crate::vartext::build_variable_text`] already implements that path
/// (it is what a fill uses), so a field created here auto-sizes its text the
/// same way a filled one does.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewTextField {
    /// 0-based page index the widget is placed on.
    pub page_index: usize,
    /// The field's partial name `/T`, which is also its fully-qualified name
    /// for a top-level field (§12.7.3.2).
    pub name: String,
    /// The widget's `/Rect` in default user space.
    pub rect: page_tree::Rect,
    /// Initial value `/V`. Empty means the field is created unfilled.
    pub value: String,
    /// `/MaxLen` — the maximum character count (§12.7.4.3 Table 229).
    pub max_len: Option<i64>,
    /// `/TU`, the alternate (accessibility / UI) name — as an explicit
    /// DECISION, not an option (R105). Screen readers announce this in
    /// preference to `/T`, and for form fields they read it INSTEAD of the
    /// tag tree, which is why leaving it undecided is refused rather than
    /// defaulted. See [`TooltipChoice`].
    pub tooltip: TooltipChoice,
    /// `/Ff` bit 13 — the field accepts multiple lines.
    pub multiline: bool,
    /// `/Ff` bit 1 — the value may not be changed by the operator.
    pub read_only: bool,
    /// `/Ff` bit 2 — the field must have a value when the form is submitted.
    pub required: bool,
}

/// Whether the operator has decided about `/TU`, the accessibility name
/// (**R105**, decision 020 §3.5.3).
///
/// # Why this is a three-state decision and not an `Option<String>`
///
/// `Option<String>` cannot distinguish *"the operator chose not to have
/// one"* from *"nobody thought about it"*, and for `/TU` those are not the
/// same situation. For form fields specifically, `/TU` — **not** the
/// structure tree — is what assistive technology actually reads: screen
/// readers announce fields through the interactive-field layer and bypass
/// the tag tree entirely. So a missing `/TU` is invisible to the sighted
/// person who created the field and load-bearing for the person who cannot
/// see the form.
///
/// That asymmetry is the whole argument for making it mandatory-or-declined
/// rather than warning about it. A warning is read by the person for whom
/// nothing is wrong.
///
/// Declining is a legitimate answer and is recorded in the operation's
/// disclosure, so *"I decided not to"* leaves a trace and *"I never
/// considered it"* cannot happen silently.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TooltipChoice {
    /// Nobody has decided. Field creation REFUSES this
    /// ([`EditError::TooltipDecisionRequired`]) — it is never a silent
    /// default, which is the entire point of R105.
    #[default]
    Undecided,
    /// The operator supplied an accessibility name; it is written as `/TU`.
    Text(String),
    /// The operator explicitly declined one. No `/TU` is written, and the
    /// declination is reported in [`FieldAuthorDisclosures`].
    Declined,
}

impl TooltipChoice {
    /// The text to write as `/TU`, or `None` when declined.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            Self::Undecided | Self::Declined => None,
        }
    }
}

/// What field creation did, and everything about it the operator must be
/// told (decision 020 §3.4.3, §3.5.3; R105).
///
/// # Why creation returns a struct rather than an id
///
/// Three of these are things pdfce KNOWS and the operator cannot see: that a
/// document is tagged and the new field is not in its tag tree, that a page
/// uses structure tab order and the new field therefore has no tab position
/// at all, and that an accessibility name was declined. None of them is an
/// error — each is a true statement about a document that was created
/// exactly as asked — and none of them is discoverable by looking at the
/// result. That combination is what a disclosure is for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldAuthorOutcome {
    /// The field's object id — the NEW field on a create, or the EXISTING
    /// field a merge attached a widget to.
    pub field_id: ObjId,
    /// Whether this attached a widget to an existing field (a merge) rather
    /// than creating one. The operator asked for "a field here" either way,
    /// so which happened is worth saying: a merge means the new widget
    /// SHARES a value with the one already on the form.
    pub merged: bool,
    /// Everything about the result the operator cannot see.
    pub disclosures: FieldAuthorDisclosures,
}

/// The disclosures field creation owes (decision 020 §3.4.3 / §3.5.3, R105).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FieldAuthorDisclosures {
    /// The operator explicitly declined an accessibility name, so this field
    /// has no `/TU` (R105). Recorded so the declination leaves a trace.
    pub tooltip_declined: bool,
    /// The document carries `/StructTreeRoot` (§14.7, Tagged PDF) and the
    /// new field is **not** in the structure tree.
    ///
    /// pdfce has no structure-tree writer (FF-I), and decision 020 §3.5.3
    /// deliberately ships this disclosure rather than a partial one — a
    /// half-written tag tree is worse than an honestly absent one. This is
    /// already stricter than Acrobat, whose own workflow leaves new fields
    /// untagged and says nothing about it.
    pub tagged_document: bool,
    /// The target page carries `/Tabs /S` (structure tab order, Table 30)
    /// while the new field is untagged.
    ///
    /// # Why this is its own disclosure and not folded into the one above
    ///
    /// §14.7 derives structure tab order from the TAG TREE. An untagged
    /// field on such a page therefore has no tab position **at all** — not
    /// "last", *undefined* — and different viewers will do different things
    /// with it. That is a functional defect in the form, not merely an
    /// accessibility gap, and it needs naming as one.
    ///
    /// Not hypothetical: `/Tabs /S` is Acrobat's own recommended default for
    /// well-tagged forms, so the forms most likely to carry it are exactly
    /// the ones where this bites.
    pub structure_tab_order: bool,
    /// A choice field was created with an EMPTY `/Opt` and therefore cannot
    /// be filled until options are added. Always `false` for other types.
    pub has_no_options: bool,
    /// A radio member MERGED into an existing group carried group-behaviour
    /// flags (`NoToggleToOff` / `RadiosInUnison`) that disagreed with the
    /// group's own, and the group's won.
    ///
    /// # Why disclosed rather than refused, or silently applied
    ///
    /// The flags live on the FIELD (§12.7.4.2.1 Table 226), so the group has
    /// exactly one set of them and the call that created it decided them.
    /// Honouring a later member's flags would silently change how every
    /// EXISTING member behaves — the second member quietly rewriting the
    /// first — which is the sneaky outcome. Refusing outright would make the
    /// obvious script (`--no-toggle-to-off` passed to every call in a loop)
    /// fail on its second iteration for no real reason.
    ///
    /// So the group's flags stand and the divergence is REPORTED, which is
    /// rule 4 applied exactly: pdfce made a choice the operator did not
    /// specify, so the operator gets told.
    pub group_flags_ignored: bool,
    /// `--defaults-from` named a template of a **different field type**, so
    /// **nothing was copied**.
    ///
    /// Not a partial copy — a total one. Every property the four creation
    /// specs share is a boolean, and every boolean is excluded (see
    /// [`EditSession::field_defaults`]), so the only copyable properties are
    /// type-specific. A text template therefore contributes nothing at all to
    /// a choice field.
    ///
    /// Disclosed rather than refused: the operator's other flags are still
    /// valid and the field they asked for is still the field they get. But
    /// they asked for defaults and received none, and rule 4 says that gets
    /// said.
    pub defaults_type_mismatch: bool,
    /// The check-box template's widgets define **different on-state names**,
    /// and the first widget's was used.
    ///
    /// A check box's widgets normally share one on-state — that is what makes
    /// the box mean the same thing on every page it appears on — so a
    /// template where they disagree is saying something unusual. pdfce picks
    /// one, which is a choice the operator did not specify.
    pub defaults_on_state_ambiguous: bool,
}

/// What applying a [`FieldDefaults`] actually did, for the caller to fold
/// into its disclosures.
///
/// Returned rather than mutated-in-place-and-forgotten: the two facts here
/// are the ones rule 4 obliges the operator to be told, and returning them
/// makes ignoring them a visible act rather than an omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DefaultsApplied {
    /// The template was a different type, so nothing transferred.
    pub type_mismatch: bool,
    /// The check-box template's widgets disagreed on the on-state name.
    pub on_state_ambiguous: bool,
}

/// The copyable properties of an existing field, for `--defaults-from`.
///
/// Produced by [`EditSession::field_defaults`], which documents what is in
/// here and — more importantly — what is deliberately not.
///
/// Carries the source's type so the applier can tell a real copy from a
/// mismatch: the type is the whole gate, since every copyable property is
/// type-specific.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldDefaults {
    /// The template's `/FT`, or `None` for a malformed field.
    pub field_type: Option<FieldType>,
    /// Which button the template is, when it is one — a check box's
    /// on-state must not be handed to a radio member.
    pub button_kind: Option<ButtonKind>,
    /// `/MaxLen`, for a text template.
    pub max_len: Option<i64>,
    /// `/Opt`, for a choice template. The case that makes the flag worth
    /// having: a fifty-entry option list is the thing nobody wants to retype.
    pub options: Vec<forms::ChoiceOption>,
    /// The on-state name, for a check-box template, read from the first
    /// widget.
    pub on_state: Option<Vec<u8>>,
    /// Whether the template's widgets disagreed about that name.
    pub on_state_ambiguous: bool,
}

impl FieldAuthorDisclosures {
    /// Whether there is anything at all to tell the operator.
    ///
    /// # Destructured deliberately, so a new field cannot be forgotten
    ///
    /// This was written as a chain of `self.field ||` and **omitted
    /// [`Self::group_flags_ignored`]** — the newest field, added by the radio
    /// slice. A caller gating its whole disclosure block on `any()` (which is
    /// exactly what this predicate exists for) would therefore have shown
    /// NOTHING for a radio member whose only disclosure was that its
    /// group-behaviour flags were overridden. Rule 4 states the obligation as
    /// *pdfce made a choice the operator did not specify, so the operator gets
    /// told*; a silent `false` here is that obligation failing closed.
    ///
    /// The same omission had already happened once, one field away: the CLI's
    /// `report_field_disclosures` had no arm for `group_flags_ignored` while
    /// the machine-readable line did. Fixing that occurrence did not reach
    /// this one — R162's shape, where the second instance survives the fix for
    /// the first, and the reason this is a destructuring rather than a
    /// corrected `||` chain.
    ///
    /// **Destructuring makes the next omission a compile error.** Adding a
    /// field to the struct without adding it here fails to build, so the
    /// discipline is enforced by the type rather than by remembering.
    #[must_use]
    pub const fn any(self) -> bool {
        let Self {
            tooltip_declined,
            tagged_document,
            structure_tab_order,
            has_no_options,
            group_flags_ignored,
            defaults_type_mismatch,
            defaults_on_state_ambiguous,
        } = self;
        tooltip_declined
            || tagged_document
            || structure_tab_order
            || has_no_options
            || group_flags_ignored
            || defaults_type_mismatch
            || defaults_on_state_ambiguous
    }
}

impl NewTextField {
    /// Copy this type's shareable properties out of a template field.
    ///
    /// `/MaxLen` only — see [`EditSession::field_defaults`] for why the list
    /// is this short and what is deliberately excluded. Returns what it did
    /// so the caller can disclose it.
    ///
    /// A template of any other type sets `type_mismatch` and changes
    /// nothing: there is no property a text field shares with a check box
    /// that is not a boolean, and booleans do not copy.
    pub fn apply_defaults(&mut self, defaults: &FieldDefaults) -> DefaultsApplied {
        if defaults.field_type != Some(FieldType::Text) {
            return DefaultsApplied {
                type_mismatch: true,
                on_state_ambiguous: false,
            };
        }
        // Only fills a gap. An explicit `--max-len` on the command line is
        // the operator speaking about THIS field, and a template must not
        // overrule it.
        if self.max_len.is_none() {
            self.max_len = defaults.max_len;
        }
        DefaultsApplied::default()
    }
    /// A field at `rect` on `page_index`, named `name`, with Acrobat's
    /// creation-floor defaults and no value.
    #[must_use]
    pub fn new(page_index: usize, name: impl Into<String>, rect: page_tree::Rect) -> Self {
        Self {
            page_index,
            name: name.into(),
            rect,
            value: String::new(),
            max_len: None,
            tooltip: TooltipChoice::Undecided,
            multiline: false,
            read_only: false,
            required: false,
        }
    }

    /// Set the initial value.
    #[must_use]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    /// Set `/MaxLen`.
    #[must_use]
    pub const fn with_max_len(mut self, max_len: i64) -> Self {
        self.max_len = Some(max_len);
        self
    }

    /// Set `/TU`, the accessibility name.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = TooltipChoice::Text(tooltip.into());
        self
    }

    /// Explicitly DECLINE an accessibility name (R105).
    ///
    /// A legitimate answer, and the only alternative to supplying one —
    /// leaving the decision unmade is refused. The declination is reported
    /// in [`FieldAuthorDisclosures::tooltip_declined`], so "I decided not
    /// to" leaves a trace and "I never considered it" cannot happen quietly.
    #[must_use]
    pub fn declining_tooltip(mut self) -> Self {
        self.tooltip = TooltipChoice::Declined;
        self
    }

    /// Set the three `/Ff` bits this type offers at creation.
    #[must_use]
    pub const fn with_flags(mut self, multiline: bool, read_only: bool, required: bool) -> Self {
        self.multiline = multiline;
        self.read_only = read_only;
        self.required = required;
        self
    }

    /// The resolved `/Ff` value (§12.7.3.1 Table 221, §12.7.4.3 Table 228).
    fn field_flags(&self) -> i64 {
        let mut ff = 0i64;
        if self.read_only {
            ff |= i64::from(forms::FieldFlags::READ_ONLY);
        }
        if self.required {
            ff |= i64::from(forms::FieldFlags::REQUIRED);
        }
        if self.multiline {
            ff |= i64::from(forms::FieldFlags::MULTILINE);
        }
        ff
    }
}

/// A check box to be created by [`EditSession::add_check_box`].
///
/// # What makes a check box structurally unlike slice 1's text field
///
/// Three things, all of them consequences of §12.7.4.2.3, and each one a
/// place where copying the text-field code would produce a field that parses
/// and does not work:
///
/// 1. **`/V` is a NAME, not a string.** A text field's value is
///    `/V (Ken Mantle)`; a check box's is `/V /Yes`. Writing a string here
///    produces a field whose value no conforming reader recognises as "on".
/// 2. **`/AP` `/N` is a sub-dictionary keyed by state**, not a stream:
///    `<< /N << /Yes 12 0 R /Off 13 0 R >> >>`. Both states exist in the
///    file at once and `/AS` selects between them.
/// 3. **The appearance is CHOSEN, never generated.** A text or choice field
///    regenerates its `/AP` from `/DA` + `/V` on every change (§12.7.3.3);
///    a button just repoints `/AS`. Nothing here goes near the variable-text
///    generator.
///
/// # The on-state name is exposed, and deliberately so
///
/// §12.7.4.2.3 says the off state *shall* be `Off` and the on state
/// *should* be `Yes` — a **should**, not a **shall**, and Acrobat treats the
/// on-state export value as independently overridable per field. It is
/// offered here because that name is the field's exported data: a form that
/// submits `Colour=Red` needs the on state named `Red`, and a creator that
/// hard-coded `Yes` could not author one. `Yes` is the default, so the
/// common case does not have to know any of this.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewCheckBox {
    /// 0-based page index the widget is placed on.
    pub page_index: usize,
    /// The field's partial name `/T` (§12.7.3.2).
    pub name: String,
    /// The widget's `/Rect` in default user space.
    pub rect: page_tree::Rect,
    /// The name of the ON state — `/V` and `/AS` when checked, and the
    /// value the form exports. `Off` is reserved (§12.7.4.2.3).
    pub on_state: String,
    /// Whether the box is created already ticked.
    pub checked: bool,
    /// `/TU`, the alternate (accessibility / UI) name — as an explicit
    /// DECISION, not an option (R105). Screen readers announce this in
    /// preference to `/T`, and for form fields they read it INSTEAD of the
    /// tag tree, which is why leaving it undecided is refused rather than
    /// defaulted. See [`TooltipChoice`].
    pub tooltip: TooltipChoice,
    /// `/Ff` bit 1 — the value may not be changed by the operator.
    pub read_only: bool,
    /// `/Ff` bit 2 — the field must have a value when the form is submitted.
    pub required: bool,
}

/// The facts about an existing radio group that a joining member is checked
/// against — see [`EditSession::radio_group_state`].
struct RadioGroupState {
    /// `/Ff` bit 26, read through the type-gated predicate.
    in_unison: bool,
    /// `/Ff` bit 15.
    no_toggle_to_off: bool,
    /// The group carries `/Opt` (Table 227), so its `/AP /N` keys are array
    /// indices rather than export values.
    positional_opt: bool,
    /// Every on-state name any member widget currently offers.
    on_states: Vec<Vec<u8>>,
}

/// One member of a **radio group** (§12.7.4.2.1) to be authored.
///
/// # A group is built by repeating this, not by declaring it
///
/// There is deliberately no `NewRadioGroup` taking a list of members. Decision
/// 020 builds radio grouping **through the F1 merge primitive**, not a
/// radio-specific path: the first [`EditSession::add_radio_button`] with a
/// given `name` creates the field, and each later one with the SAME name
/// merges another widget into it. That is the identical mechanism a check box
/// repeated across pages uses, and it is why a radio group needs no code that
/// knows what a radio group is.
///
/// The consequence worth stating: a one-member "group" is a legitimate
/// intermediate state, not an error. A group under construction has to pass
/// through it, and there is no point at which pdfce could know the operator
/// was finished.
///
/// # `export_value` is the member's identity
///
/// It is simultaneously the `/AP /N` key, the `/AS` value when this member is
/// chosen, and the field's `/V` — one string doing all three jobs
/// (§12.7.4.2.1). Members are told apart by it and by nothing else, which is
/// why two members may not share one unless [`radios_in_unison`] says so.
///
/// pdfce always writes these as NAMES. It never authors the positional `/Opt`
/// form (Table 227) — see [`EditError::RadioGroupUsesPositionalOpt`].
///
/// [`radios_in_unison`]: NewRadioButton::radios_in_unison
#[derive(Debug, Clone, PartialEq)]
pub struct NewRadioButton {
    /// 0-based page index the widget is placed on.
    pub page_index: usize,
    /// The GROUP's partial name `/T` (§12.7.3.2) — shared by every member.
    pub name: String,
    /// This member's widget `/Rect` in default user space.
    pub rect: page_tree::Rect,
    /// This member's export value: its `/AP /N` key, its `/AS` when chosen,
    /// and the `/V` the group takes. `Off` is reserved (§12.7.4.2.3).
    pub export_value: String,
    /// Whether this member is the group's initial selection.
    pub selected: bool,
    /// `/TU`, the alternate (accessibility / UI) name — an explicit DECISION,
    /// not an option (R105). See [`TooltipChoice`].
    pub tooltip: TooltipChoice,
    /// `/Ff` bit 15 `NoToggleToOff` — once a member is chosen, clicking it
    /// again does not clear the group (§12.7.4.2.1 Table 226).
    ///
    /// Only meaningful on the call that CREATES the group; a merge cannot
    /// change a flag that already exists. See
    /// [`FieldAuthorDisclosures::group_flags_ignored`].
    pub no_toggle_to_off: bool,
    /// `/Ff` bit 26 `RadiosInUnison` — members sharing an export value turn
    /// on together (Table 226).
    ///
    /// **Bit 26 is overloaded**: on a text field the same bit means
    /// `RichText`. It is only ever read back through
    /// [`forms::Field::radios_in_unison`], which gates on the field type, so
    /// the two can never be confused.
    ///
    /// Only meaningful on the call that creates the group.
    pub radios_in_unison: bool,
    /// `/Ff` bit 1 — the value may not be changed by the operator.
    pub read_only: bool,
    /// `/Ff` bit 2 — the field must have a value when the form is submitted.
    pub required: bool,
}

impl NewRadioButton {
    /// A radio template contributes **nothing**, and this says so rather
    /// than leaving the absence to be inferred.
    ///
    /// Its only non-boolean property is the export value, and there is no
    /// field-level one to copy: on-states live per widget
    /// ([`forms::Widget::on_states`]), so a radio *field* has one export
    /// value per member while `--defaults-from <field>` names a field. A
    /// copy would either collide with
    /// [`FormAuthorError::RadioExportValueTaken`] inside the same group or
    /// be arbitrary across groups.
    ///
    /// So this always reports `type_mismatch` — including for a radio
    /// template, where the types DO match but the copyable set is empty.
    /// "You asked for defaults and got none" is the fact the operator needs,
    /// and it is true either way.
    pub fn apply_defaults(&mut self, _defaults: &FieldDefaults) -> DefaultsApplied {
        DefaultsApplied {
            type_mismatch: true,
            on_state_ambiguous: false,
        }
    }
    /// A member at `rect` on `page_index`, in group `name`, exporting
    /// `export_value`, not selected.
    #[must_use]
    pub fn new(
        page_index: usize,
        name: impl Into<String>,
        rect: page_tree::Rect,
        export_value: impl Into<String>,
    ) -> Self {
        Self {
            page_index,
            name: name.into(),
            rect,
            export_value: export_value.into(),
            selected: false,
            tooltip: TooltipChoice::Undecided,
            no_toggle_to_off: false,
            radios_in_unison: false,
            read_only: false,
            required: false,
        }
    }

    /// Make this member the group's initial selection.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set `/TU`, the accessibility name.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = TooltipChoice::Text(tooltip.into());
        self
    }

    /// Explicitly DECLINE an accessibility name (R105).
    #[must_use]
    pub fn declining_tooltip(mut self) -> Self {
        self.tooltip = TooltipChoice::Declined;
        self
    }

    /// Set the two group-behaviour flags. Only honoured on the call that
    /// creates the group.
    #[must_use]
    pub const fn with_group_flags(mut self, no_toggle_to_off: bool, in_unison: bool) -> Self {
        self.no_toggle_to_off = no_toggle_to_off;
        self.radios_in_unison = in_unison;
        self
    }

    /// Set the two general `/Ff` bits this type offers at creation.
    #[must_use]
    pub const fn with_flags(mut self, read_only: bool, required: bool) -> Self {
        self.read_only = read_only;
        self.required = required;
        self
    }

    /// The resolved `/Ff` (§12.7.3.1 Table 221 + §12.7.4.2.1 Table 226).
    ///
    /// Bit 16 `Radio` is what makes this a radio group rather than a check
    /// box — with both `Radio` and `Pushbutton` clear a `/Btn` IS a check box
    /// (§12.7.4.2.1), so the bit is the entire type declaration.
    fn field_flags(&self) -> i64 {
        let mut ff = i64::from(forms::FieldFlags::RADIO);
        if self.read_only {
            ff |= i64::from(forms::FieldFlags::READ_ONLY);
        }
        if self.required {
            ff |= i64::from(forms::FieldFlags::REQUIRED);
        }
        if self.no_toggle_to_off {
            ff |= i64::from(forms::FieldFlags::NO_TOGGLE_TO_OFF);
        }
        if self.radios_in_unison {
            ff |= i64::from(forms::FieldFlags::RADIOS_IN_UNISON);
        }
        ff
    }
}

impl NewCheckBox {
    /// Copy this type's shareable properties out of a template field.
    ///
    /// The on-state name only, read from the template's first widget. See
    /// [`EditSession::field_defaults`] for the exclusions — in particular
    /// `checked`, which is a *value* and does not copy.
    ///
    /// Propagates the template's widget disagreement so the caller discloses
    /// it: pdfce picked one of several names, which is a choice the operator
    /// did not make.
    pub fn apply_defaults(&mut self, defaults: &FieldDefaults) -> DefaultsApplied {
        if defaults.field_type != Some(FieldType::Button)
            || defaults.button_kind != Some(ButtonKind::Check)
        {
            return DefaultsApplied {
                type_mismatch: true,
                on_state_ambiguous: false,
            };
        }
        let mut applied = DefaultsApplied {
            type_mismatch: false,
            on_state_ambiguous: defaults.on_state_ambiguous,
        };
        // The spec's on_state is a String and the model's is raw bytes: a
        // name that is not UTF-8 is left behind rather than lossily
        // converted, because a mangled on-state names a state the `/AP /N`
        // subdictionary does not have, and the box would never tick.
        match defaults.on_state.as_deref().map(std::str::from_utf8) {
            Some(Ok(name)) if self.on_state.is_empty() => name.clone_into(&mut self.on_state),
            Some(Err(_)) => applied.type_mismatch = true,
            _ => {}
        }
        applied
    }
    /// An unchecked box at `rect` on `page_index`, on-state `Yes`.
    #[must_use]
    pub fn new(page_index: usize, name: impl Into<String>, rect: page_tree::Rect) -> Self {
        Self {
            page_index,
            name: name.into(),
            rect,
            on_state: "Yes".to_owned(),
            checked: false,
            tooltip: TooltipChoice::Undecided,
            read_only: false,
            required: false,
        }
    }

    /// Override the on-state name (the exported value).
    #[must_use]
    pub fn with_on_state(mut self, on_state: impl Into<String>) -> Self {
        self.on_state = on_state.into();
        self
    }

    /// Create the box already ticked.
    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set `/TU`, the accessibility name.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = TooltipChoice::Text(tooltip.into());
        self
    }

    /// Explicitly DECLINE an accessibility name (R105).
    ///
    /// A legitimate answer, and the only alternative to supplying one —
    /// leaving the decision unmade is refused. The declination is reported
    /// in [`FieldAuthorDisclosures::tooltip_declined`], so "I decided not
    /// to" leaves a trace and "I never considered it" cannot happen quietly.
    #[must_use]
    pub fn declining_tooltip(mut self) -> Self {
        self.tooltip = TooltipChoice::Declined;
        self
    }

    /// Set the two `/Ff` bits this type offers at creation.
    #[must_use]
    pub const fn with_flags(mut self, read_only: bool, required: bool) -> Self {
        self.read_only = read_only;
        self.required = required;
        self
    }

    /// The resolved `/Ff` (§12.7.3.1 Table 221).
    ///
    /// Neither `Radio` (bit 16) nor `Pushbutton` (bit 17) is set: with both
    /// clear, a `/Btn` field **is** a check box (§12.7.4.2.1). That is the
    /// whole of the type declaration — there is no positive "I am a check
    /// box" flag to set.
    fn field_flags(&self) -> i64 {
        let mut ff = 0i64;
        if self.read_only {
            ff |= i64::from(forms::FieldFlags::READ_ONLY);
        }
        if self.required {
            ff |= i64::from(forms::FieldFlags::REQUIRED);
        }
        ff
    }
}

/// One entry in a choice field's `/Opt` array: what the form SUBMITS and what
/// the operator SEES.
///
/// §12.7.4.4 lets an `/Opt` element be either a single text string (the two
/// coincide) or a two-element array `[(export) (display)]`. Keeping them as
/// separate fields here — rather than as one string with a convention — is
/// what stops the pair being collapsed by accident. The Acrobat reference
/// names that collapse as something that *"would silently break forms"*: the
/// document still opens, the drop-down still reads correctly, and the
/// submitted data is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    /// The export value — what the form submits for this item.
    pub export: String,
    /// The display string — what the operator sees in the list.
    pub display: String,
}

impl ChoiceOption {
    /// An option whose export value and display string are the same.
    ///
    /// Emitted as a **bare string** rather than a one-element-repeated array,
    /// which is both what §12.7.4.4 intends and what keeps a round-tripped
    /// file byte-comparable with a hand-written one.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            export: text.clone(),
            display: text,
        }
    }

    /// An option whose submitted value differs from its label.
    #[must_use]
    pub fn new(export: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            export: export.into(),
            display: display.into(),
        }
    }

    /// Whether this option can be written as a bare string.
    fn is_plain(&self) -> bool {
        self.export == self.display
    }
}

/// A list box or combo box to be created by
/// [`EditSession::add_choice_field`].
///
/// # Created UNSELECTED, and that is a decision rather than an omission
///
/// No `/V` is written. §12.7.4.4 gives `/V` a default of `null` (nothing
/// selected), so this is a well-formed field — but the reason is more
/// specific than "the default was convenient".
///
/// pdfce's spec reference and pdfce's own code **disagree about what `/V`
/// holds for a choice field**. The §12.7.4.4 summary says `/V` is the
/// *display* name of the selected item; [`EditSession::set_choice_value`]
/// writes the *export* value and paints the display string into the
/// appearance. Real-world files and Acrobat's own documentation side with
/// the code — an export value that never got exported would not be an
/// export value — but the discrepancy is real and is not this slice's to
/// settle.
///
/// Creating the field unselected means this constructor **takes no position
/// on it**. Whatever convention `set_choice_value` uses is the one the field
/// ends up with, because that verb is what puts the first value in. If the
/// convention is later found to be wrong it is wrong in exactly one place,
/// and fixing it there fixes fields created here too.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewChoiceField {
    /// 0-based page index the widget is placed on.
    pub page_index: usize,
    /// The field's partial name `/T` (§12.7.3.2).
    pub name: String,
    /// The widget's `/Rect` in default user space.
    pub rect: page_tree::Rect,
    /// The `/Opt` entries, in the order they will be displayed.
    ///
    /// **Order is presentation order.** §12.7.4.4 is explicit that a
    /// conforming reader *"shall display the options in the order in which
    /// they occur in the `Opt` array"* — the `Sort` flag is a note to
    /// authoring tools, not an instruction to viewers, so pdfce sorts the
    /// array itself if asked rather than setting a flag and hoping.
    pub options: Vec<ChoiceOption>,
    /// `/Ff` bit 18 — a drop-down (combo) rather than a scrolling list box.
    pub combo: bool,
    /// `/Ff` bit 19 — the operator may type a value not in the list.
    /// Valid only with `combo` (§12.7.4.4 Table 230).
    pub editable: bool,
    /// `/Ff` bit 22 — more than one item may be selected at a time.
    pub multi_select: bool,
    /// Sort the options alphabetically by display string before writing.
    ///
    /// This SORTS THE ARRAY, and also sets `/Ff` bit 20 to record that the
    /// list is meant to stay sorted. Setting the flag alone would do nothing
    /// visible, because readers display `/Opt` order regardless.
    pub sort: bool,
    /// `/TU`, the alternate (accessibility / UI) name — as an explicit
    /// DECISION, not an option (R105). Screen readers announce this in
    /// preference to `/T`, and for form fields they read it INSTEAD of the
    /// tag tree, which is why leaving it undecided is refused rather than
    /// defaulted. See [`TooltipChoice`].
    pub tooltip: TooltipChoice,
    /// `/Ff` bit 1 — the value may not be changed by the operator.
    pub read_only: bool,
    /// `/Ff` bit 2 — the field must have a value when the form is submitted.
    pub required: bool,
}

impl NewChoiceField {
    /// Copy this type's shareable properties out of a template field.
    ///
    /// `/Opt` only — the flagship case for `--defaults-from`, because a
    /// fifty-entry option list is the thing nobody wants to retype. See
    /// [`EditSession::field_defaults`] for the exclusions.
    ///
    /// The `Sort`, `Combo`, `Edit` and `MultiSelect` flags do NOT come with
    /// it: they are booleans, and a presence flag cannot express "off".
    pub fn apply_defaults(&mut self, defaults: &FieldDefaults) -> DefaultsApplied {
        if defaults.field_type != Some(FieldType::Choice) {
            return DefaultsApplied {
                type_mismatch: true,
                on_state_ambiguous: false,
            };
        }
        // Only fills a gap — explicit `--option` arguments win. Note this
        // means an operator who wants a template's list MINUS one entry has
        // to supply the whole list; copying is all-or-nothing by design,
        // since a partial merge would have no rule for ordering and §12.7.4.4
        // says readers never re-sort `/Opt`.
        if !self.options.is_empty() {
            return DefaultsApplied::default();
        }
        // `/Opt` entries are §7.9.2 text strings on the model side and
        // `String`s on the spec side. A list containing a name that is not
        // UTF-8 is NOT copied and is reported: a lossily-converted export
        // value is a value the form would submit and no consumer expects,
        // which is worse than copying nothing. All-or-nothing, so the list
        // that lands is either the template's or the operator's.
        let converted: Option<Vec<ChoiceOption>> = defaults
            .options
            .iter()
            .map(|o| {
                Some(ChoiceOption {
                    export: std::str::from_utf8(&o.export).ok()?.to_owned(),
                    display: std::str::from_utf8(&o.display).ok()?.to_owned(),
                })
            })
            .collect();
        match converted {
            Some(options) => {
                self.options = options;
                DefaultsApplied::default()
            }
            None => DefaultsApplied {
                type_mismatch: true,
                on_state_ambiguous: false,
            },
        }
    }
    /// A list box at `rect` on `page_index` offering `options`.
    #[must_use]
    pub fn new(
        page_index: usize,
        name: impl Into<String>,
        rect: page_tree::Rect,
        options: Vec<ChoiceOption>,
    ) -> Self {
        Self {
            page_index,
            name: name.into(),
            rect,
            options,
            combo: false,
            editable: false,
            multi_select: false,
            sort: false,
            tooltip: TooltipChoice::Undecided,
            read_only: false,
            required: false,
        }
    }

    /// Make this a drop-down (combo box), optionally free-text editable.
    #[must_use]
    pub const fn as_combo(mut self, editable: bool) -> Self {
        self.combo = true;
        self.editable = editable;
        self
    }

    /// Allow more than one selection at a time.
    #[must_use]
    pub const fn multi_select(mut self, multi: bool) -> Self {
        self.multi_select = multi;
        self
    }

    /// Sort the options alphabetically by display string.
    #[must_use]
    pub const fn sorted(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }

    /// Set `/TU`, the accessibility name.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = TooltipChoice::Text(tooltip.into());
        self
    }

    /// Explicitly DECLINE an accessibility name (R105).
    ///
    /// A legitimate answer, and the only alternative to supplying one —
    /// leaving the decision unmade is refused. The declination is reported
    /// in [`FieldAuthorDisclosures::tooltip_declined`], so "I decided not
    /// to" leaves a trace and "I never considered it" cannot happen quietly.
    #[must_use]
    pub fn declining_tooltip(mut self) -> Self {
        self.tooltip = TooltipChoice::Declined;
        self
    }

    /// Set the two general `/Ff` bits this type offers at creation.
    #[must_use]
    pub const fn with_flags(mut self, read_only: bool, required: bool) -> Self {
        self.read_only = read_only;
        self.required = required;
        self
    }

    /// The resolved `/Ff` (§12.7.3.1 Table 221, §12.7.4.4 Table 230).
    fn field_flags(&self) -> i64 {
        let mut ff = 0i64;
        if self.read_only {
            ff |= i64::from(forms::FieldFlags::READ_ONLY);
        }
        if self.required {
            ff |= i64::from(forms::FieldFlags::REQUIRED);
        }
        if self.combo {
            ff |= i64::from(forms::FieldFlags::COMBO);
        }
        if self.editable {
            ff |= i64::from(forms::FieldFlags::EDIT);
        }
        if self.sort {
            ff |= i64::from(forms::FieldFlags::SORT);
        }
        if self.multi_select {
            ff |= i64::from(forms::FieldFlags::MULTI_SELECT);
        }
        ff
    }
}

/// One object-existence change inside a [`Command`].
#[derive(Debug, Clone, Copy)]
struct Removal {
    id: ObjId,
    was_deleted: bool,
    is_deleted: bool,
}

/// Why an edit could not be performed.
///
/// Every variant names a condition the operator (or the calling front
/// end) can act on. There is deliberately no catch-all "edit failed".
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The page index is past the end of the document.
    #[error("page index {index} is out of range (the document has {count} page(s))")]
    PageOutOfRange {
        /// The 0-based index that was asked for.
        index: usize,
        /// How many pages the document actually has.
        count: usize,
    },
    /// The page tree could not be walked, so no page could be named.
    #[error("the page tree could not be resolved: {0}")]
    PageTree(#[from] PageTreeError),
    /// Table 30: a page's `/Rotate` *"shall be a multiple of 90"*.
    /// Refused rather than rounded — silently turning 45° into 90° would
    /// be pdfce deciding what the operator meant.
    #[error("rotation {degrees}° is not a multiple of 90 (ISO 32000-1 Table 30)")]
    RotationNotMultipleOf90 {
        /// The value that was rejected.
        degrees: i32,
    },
    /// The object that must carry the edited entry is not a dictionary.
    ///
    /// A page object that is not a dictionary, or an `/Info` that points
    /// at a string, is a malformed file. pdfce refuses rather than
    /// replacing the value wholesale, which would destroy whatever the
    /// object actually held.
    #[error("object {id} is not a dictionary, so /{key} cannot be set on it")]
    NotADictionary {
        /// The offending object.
        id: ObjId,
        /// The key the edit wanted to write.
        key: &'static str,
    },
    /// No object number is left to allocate (see
    /// [`Document::next_object_number`]).
    #[error("the document has no unused object number left")]
    ObjectNumbersExhausted,
    /// An object would have to be created, but this file's trailer
    /// `/Size` is **suppressing** cross-reference entries, and creating
    /// an object raises `/Size` — which would expose them.
    ///
    /// Refused rather than performed, because the exposed objects are
    /// ones the operator did not touch and may not even parse; the
    /// document is frequently loadable *only* because the filter is
    /// hiding them (§7.5.5: an object at or above `/Size` *"shall be
    /// ignored and defined to be missing"*). See
    /// [`Document::suppressed_object_count`].
    ///
    /// Editing an existing object in such a file is unaffected — only
    /// creation is refused.
    #[error(
        "creating an object would raise /Size and expose {count} cross-reference \
         entr{} this file's /Size currently hides; edit an existing object instead",
        if *count == 1 { "y" } else { "ies" }
    )]
    ObjectCreationWouldExposeHiddenObjects {
        /// How many entries would be exposed.
        count: usize,
    },
    /// A ce-dimension operation named a dimension the sidecar model does not
    /// contain (Pass 25.5).
    ///
    /// Reachable from a stale selection: an id survives an undo that removed
    /// the dimension it named. Refused by name rather than ignored, so a
    /// caller finds out its handle went stale instead of watching a move
    /// silently do nothing.
    #[error("no ce dimension with id {id} exists in this document")]
    DimensionNotFound {
        /// The dimension id that was asked for.
        id: u32,
    },
    /// This document's ce-dimension sidecar was written by a **newer** pdfce
    /// than this build understands, so writing to it would destroy what this
    /// build cannot represent (Pass 27.2).
    ///
    /// The alternative is what used to happen and is far worse: the sidecar
    /// failed an exact-equality version check, the session silently started a
    /// FRESH model, and the next save wrote that empty model over the
    /// operator's groups, calibrated scales and memberships. Nothing looked
    /// wrong in the meantime — the `/Line` annotations kept rendering — so the
    /// loss would not be noticed until it was permanent.
    ///
    /// Reading is unaffected: the dimensions this build understands are still
    /// listed and still render. Only ce-dimension WRITES are refused.
    #[error(
        "this document's measurement data was written by a newer version of pdfce (format {found}, this build understands {supported}); editing measurements here would discard what this version cannot read, so it is refused"
    )]
    SidecarWrittenByNewerBuild {
        /// The schema version found in the document.
        found: i64,
        /// The newest schema version this build understands.
        supported: i64,
    },
    /// A placement operation named a ce dimension that is not linear
    /// (Pass 27.1).
    ///
    /// A circular dimension has no axis to stand off from or slide along, so
    /// there is nothing for `offset`/`text_along` to mean. Refused by name
    /// rather than ignored, so a caller learns its assumption was wrong
    /// instead of watching a drag do nothing.
    #[error(
        "ce dimension {id} is circular, and only a linear one has a standoff and a text position"
    )]
    NotALinearDimension {
        /// The dimension id.
        id: u32,
    },
    /// A display-mode operation named a ce dimension that is not circular
    /// (Pass 34.2) — the mirror of [`Self::NotALinearDimension`].
    ///
    /// Radius-versus-diameter is a property of a fitted CIRCLE: it asks
    /// whether the label reports `r` or `2r` for the same stored geometry. A
    /// linear ce dimension has no circle and no radius, so there is nothing
    /// for the flag to select between. Refused by name rather than ignored, on
    /// the same reasoning [`Self::NotALinearDimension`] gives: a caller that
    /// aimed the verb at the wrong kind should learn that, not watch a control
    /// appear to do nothing.
    #[error("ce dimension {id} is linear, and only a circular one has a radius/diameter display")]
    NotACircularDimension {
        /// The dimension id.
        id: u32,
    },
    /// Field authoring was asked for on a document carrying an **XFA**
    /// layer (§12.7.8).
    ///
    /// # Refused by name, on pdfce's own capability boundary
    ///
    /// Decision 020 settled this and it is not a scope cut. A hybrid
    /// XFA/AcroForm document describes its fields TWICE — once in the
    /// AcroForm dictionaries and once in the XFA XML. pdfce can write the
    /// AcroForm half and cannot write the XFA half, so adding a field would
    /// leave an XFA-aware viewer and a plain viewer reporting **different
    /// field counts for the same file**.
    ///
    /// A one-sided add is therefore worse than no add: it produces a
    /// document whose two descriptions of itself disagree, and neither
    /// viewer can tell it has happened.
    #[error(
        "document {name:?} carries an XFA form layer; pdfce can author the AcroForm half but not the XFA half, and adding only one would make XFA-aware and plain viewers disagree about this document's fields"
    )]
    FieldAuthoringRefusedXfa {
        /// A short description of where the XFA was found.
        name: String,
    },
    /// A new field's `/Rect` is degenerate (zero or negative extent).
    ///
    /// A zero-area rectangle is meaningful for a **signature** field
    /// (§12.7.4.5 makes it deliberate invisibility) but not for one pdfce is
    /// authoring for an operator to type into: it would create a field that
    /// exists, accepts a value, and can never be seen or clicked.
    #[error(
        "the new field's rectangle has no area ({w} x {h}); it would be invisible and unclickable"
    )]
    FieldRectDegenerate {
        /// Width in user-space units.
        w: f64,
        /// Height in user-space units.
        h: f64,
    },
    /// An authoring write could not resolve its name against the field tree.
    ///
    /// # What replaced a refusal here, and why it is not a loosening
    ///
    /// This used to be `FieldNameAlreadyUsed`: a same-name, same-type add was
    /// REFUSED, because merging needs a write-side resolver pdfce did not
    /// have. That refusal was correct while it stood — the alternative was
    /// appending a second top-level field with the same `/T`, and §12.7.3.2
    /// makes the fully-qualified name a field's IDENTITY, so two of them have
    /// one identity and no disambiguator. Nothing records which the operator
    /// meant, so it cannot be un-authored.
    ///
    /// The resolver now exists ([`crate::forms_author::resolve_field_path`]),
    /// so a same-name same-type add MERGES — which is what §12.7.3.2 says it
    /// means, and is how a check box appears on every page of a form and how
    /// a radio group is built. The duplicate-identity document is not merely
    /// refused any more; it is unreachable, because every authoring write
    /// resolves the name against the graph before it decides what to write.
    ///
    /// What remains here are the collisions that are still genuinely
    /// impossible — a different TYPE under the same name, and a name that
    /// belongs to a grouping node — plus the malformed-path refusals.
    #[error(transparent)]
    FieldAuthoring(#[from] crate::forms_author::FormAuthorError),
    /// `Edit` was asked for on a list box rather than a combo box.
    ///
    /// §12.7.4.4 Table 230 says the `Edit` flag is *"used only if the Combo
    /// flag is on"* — a list box has no text entry to make editable. Refused
    /// rather than silently dropped so the caller learns their combination
    /// was impossible instead of quietly getting a field that ignores it.
    #[error("the editable flag applies only to a combo box (drop-down), not to a list box")]
    ChoiceEditRequiresCombo,
    /// A check box's on-state name was empty or was `Off`.
    ///
    /// §12.7.4.2.3: the off state *shall* be named `Off`, so it cannot also
    /// name the on state — a box whose two states share a name has no way to
    /// express "checked". An empty name is not a valid PDF name object here.
    /// A widget index was past the end of the field's widget list.
    ///
    /// The index is into the field's widgets as [`forms::Field::widgets`]
    /// reports them — the same order `list-fields` shows — so an operator can
    /// see what they are choosing between before choosing.
    #[error(
        "field `{name}` has {widgets} widget(s), so there is no widget {index} to delete (they are numbered from 0)"
    )]
    WidgetIndexOutOfRange {
        /// The field's fully-qualified name.
        name: String,
        /// The index asked for.
        index: usize,
        /// How many widgets the field actually has.
        widgets: usize,
    },
    /// Two members of one radio group were given the same export value, and
    /// the group is not `RadiosInUnison`.
    ///
    /// §12.7.4.2.1: a radio group's members are distinguished BY their
    /// on-state names — that name is both the appearance key and the value
    /// `/V` takes when that member is chosen. Two members sharing one means
    /// `/V` cannot say which was chosen, and `set_button_state` would light
    /// both.
    ///
    /// That is exactly what `RadiosInUnison` (`/Ff` bit 26) exists to request
    /// deliberately, so this is refused only when the flag is absent — the
    /// error names the flag rather than leaving the caller to find it.
    #[error(
        "radio group `{fqn}` already has a member exporting {state:?}; give this one a different export value, or create the group with radios-in-unison if they are meant to select together"
    )]
    RadioExportValueTaken {
        /// The group's fully-qualified name.
        fqn: String,
        /// The export value already in use.
        state: String,
    },
    /// A radio group already in the document names its states POSITIONALLY
    /// (`/1`, `/2`, …) through `/Opt`, so pdfce cannot add a member to it.
    ///
    /// §12.7.4.2.1 Table 227 allows a button field's `/Opt` to supply export
    /// values positionally: the `/AP /N` keys are then array INDICES and the
    /// real export value is `/Opt[i]`. pdfce parses `/Opt` but has never
    /// consulted it on the write side, so it cannot compute what index a new
    /// member should take, nor what the existing members actually export.
    ///
    /// Decision 020 §8.3 requires this be implemented or explicitly refused,
    /// and refusal is the honest half: authoring into such a group would
    /// produce members whose export values pdfce itself cannot resolve.
    /// Groups pdfce creates are always NAMED, never positional, so this can
    /// only be reached on a foreign document.
    #[error(
        "radio group `{fqn}` names its states positionally through /Opt, which pdfce cannot extend; its export values are array positions pdfce does not write"
    )]
    RadioGroupUsesPositionalOpt {
        /// The group's fully-qualified name.
        fqn: String,
    },
    /// A check box's on-state name was empty or was `Off`.
    ///
    /// §12.7.4.2.3: the off state *shall* be named `Off`, so it cannot also
    /// name the on state — a box whose two states share a name has no way to
    /// express "checked". An empty name is not a valid PDF name object here.
    #[error("{name:?} cannot name a check box's on state (\"Off\" is reserved for the off state)")]
    CheckBoxOnStateInvalid {
        /// The rejected name.
        name: String,
    },
    /// A choice field listed the same export value twice.
    ///
    /// §12.7.4.4 does not forbid duplicates, but pdfce's own fill verb
    /// resolves a requested value to the FIRST matching option, so a
    /// duplicate export makes the later one unselectable — a field with an
    /// option in it the operator can see and can never choose.
    #[error("option {value:?} is listed more than once; the duplicate could never be selected")]
    ChoiceOptionDuplicate {
        /// The repeated export value.
        value: String,
    },
    /// Field creation was asked for without deciding about `/TU` (R105).
    ///
    /// # Why an undecided accessibility name is an error and not a default
    ///
    /// For form fields, `/TU` — not the structure tree — is what assistive
    /// technology actually reads: screen readers announce fields through the
    /// interactive-field layer and bypass the tag tree. So a field with no
    /// `/TU` is perfectly usable for the person who created it and
    /// unnavigable for the person who cannot see the form.
    ///
    /// That asymmetry is why this is not a warning. A warning is read by the
    /// person for whom nothing is wrong. Declining is a legitimate answer —
    /// it is simply required to be an ANSWER, and it is recorded in the
    /// operation's disclosure so it leaves a trace.
    #[error(
        "field {name:?}: decide about the accessibility name (tooltip) — supply one, or decline it explicitly; it is what screen readers announce for a form field, so it is never defaulted silently"
    )]
    TooltipDecisionRequired {
        /// The field being created.
        name: String,
    },
    /// A new field was given an empty name.
    ///
    /// §12.7.3.2 builds the fully-qualified name from `/T`; a terminal field
    /// with no `/T` anywhere on its path has no addressable name, so nothing
    /// could later fill, export or delete it by name.
    #[error("a new field needs a name — without one it cannot be filled, exported or referred to")]
    FieldNameEmpty,
    /// A ce-dimension operation named a group the sidecar model does not
    /// contain (Pass 25.5).
    #[error("no ce dimension group with id {id} exists in this document")]
    DimensionGroupNotFound {
        /// The group id that was asked for.
        id: u32,
    },
    /// A **certification signature** with an enforced permissions entry
    /// forbids this structural change (§12.8.4 Table 258).
    ///
    /// Not a warning — a refusal. Table 258 says *"consumer applications
    /// **shall enforce** the permissions specified by the `P`
    /// attribute"*, and pdfce is a consumer application, so performing
    /// the edit and reporting the resulting invalidation would be pdfce
    /// declining to do something the spec requires of it. Table 254's
    /// permitted-change lists are closed at every `P` value and contain
    /// no operation pdfce can perform, so every Pass-3.2 structural
    /// operation lands here. See [`crate::signature`].
    ///
    /// A certification signature **without** the `/Perms` entry is
    /// detection only; those edits proceed and report
    /// [`SignatureImpact::Invalidated`] instead.
    #[error(
        "this document carries a certification signature whose permissions are enforced          (ISO 32000-1 §12.8.4, /Perms /DocMDP, P={permission}); structural page changes are          not among the changes it permits, so pdfce refuses rather than silently breaking it"
    )]
    CertificationForbidsChange {
        /// The certification's `/P` access permission (Table 254: 1–3).
        /// **2 when the transform parameters omit `/P`**, which is that
        /// table's documented default.
        permission: u8,
    },
    /// The operation would leave the document with no pages.
    ///
    /// §7.7.3.3 requires a page tree to contain at least one page, and
    /// Acrobat refuses the same operation for the same reason
    /// (`core_ops__delete_pages.md`: *"Cannot delete the only remaining
    /// page"*).
    #[error("removing {removing} of {total} page(s) would leave the document with none")]
    WouldRemoveEveryPage {
        /// How many pages the operation asked to remove.
        removing: usize,
        /// How many the document has.
        total: usize,
    },
    /// Annotation authoring was attempted on an **encrypted** document
    /// (§7.6). Refused **by name** (X10, R27 posture): an annotation's
    /// `/Contents`/`/T`/`/Subj` strings are encrypted per object, and
    /// writing them plaintext into an encrypted file would produce a
    /// document that opens and shows mojibake — a plausible, working,
    /// wrong file.
    ///
    /// Encryption support is Pass 5. The [`crate::writer::encoder`]
    /// object-encoder seam (R37) already exists, so the eventual fix is a
    /// plug-in, not a retrofit. Until then, authoring is declined rather
    /// than attempted.
    #[error(
        "this document is encrypted (/Encrypt); pdfce cannot yet author annotations into an \
         encrypted file (encryption is Pass 5) — authoring is refused rather than corrupting \
         the file's per-object string encryption"
    )]
    DocumentEncrypted,
    /// A markup annotation was authored with geometry that names no point
    /// (an empty `/InkList`, an empty vertex list, or no quads). Refused
    /// rather than emit an empty appearance for a non-empty subtype, which
    /// would be an invisible annotation the operator could not find.
    #[error("the annotation has no geometry to draw")]
    EmptyGeometry,
    /// A text-bearing annotation's variable-text appearance could not be
    /// generated (Pass 6.2, §12.7.3.3) — e.g. a symbolic font was chosen
    /// for a Latin text body. Named, never a silent blank appearance.
    #[error("the text annotation's appearance could not be generated: {0}")]
    VariableText(#[from] crate::vartext::VarTextError),
    /// The target page's `/Annots` is present but is neither an array nor
    /// an indirect reference to one (§12.5.2: `/Annots` is an array).
    /// Refused rather than clobber whatever the malformed value really is.
    #[error(
        "page {page} has an /Annots entry that is not an array, so an annotation cannot be appended to it"
    )]
    AnnotsNotAnArray {
        /// The offending page object.
        page: ObjId,
    },
    /// [`EditSession::delete_redaction_mark`] was given an object that is
    /// not a `/Redact` annotation listed on some page's `/Annots`.
    ///
    /// Refused by name rather than "removing whatever that id is": the
    /// review surface addresses marks by object id, and a stale id (a mark
    /// already removed, an id from an undone command) must produce a
    /// refusal the operator can read, never the silent deletion of some
    /// unrelated annotation that happened to inherit the number.
    #[error("object {id} is not an unapplied /Redact mark on any page of this document")]
    NotARedactionMark {
        /// The object that was asked for.
        id: ObjId,
    },
    /// A form-fill edit named a field the document's `/AcroForm` does not
    /// contain (or the document has no `/AcroForm` at all). Refused by name
    /// rather than silently creating a field — pdfce fills existing fields,
    /// it does not invent form structure from a fill.
    #[error("the document has no fillable form field with the fully-qualified name {name:?}")]
    FieldNotFound {
        /// The fully-qualified name that was requested.
        name: String,
    },
    /// A plain-text fill named a **rich-text** field (`/Ff` bit 26).
    ///
    /// # This refusal prevents a WRONG VALUE ON SCREEN, not merely lost bold
    ///
    /// It would be easy to read this as a fidelity compromise — "pdfce can
    /// only write plain text, so the formatting is dropped". It is worse than
    /// that, and the reason is a `shall`.
    ///
    /// §12.7.3.4: the rich text string *"in addition to the `RV` or `RC`
    /// entry, **shall** be used to generate the appearance"*, and §12.7.3.3
    /// requires the appearance to be regenerated on **every** value change for
    /// these fields. Appearance generation for a rich-text field is bound to
    /// `/RV`, **not** to `/V`. So writing a fresh `/V` while leaving `/RV`
    /// stale produces a field whose appearance a conforming reader rebuilds
    /// **from the OLD text** — the document then displays a value nobody
    /// entered, which is a correctness defect rather than a formatting one.
    ///
    /// Hence a refusal by name rather than a lossy best effort.
    ///
    /// # The way through
    ///
    /// [`EditSession::fill_text_field_downgrading_rich_text`] — clear bit 26,
    /// DELETE `/RV`, write `/V`, regenerate a plain appearance. That is fully
    /// conformant (the field simply stops being a rich-text field), needs no
    /// XHTML engine, and is a deliberate act the operator asks for rather than
    /// something that happens to their document silently.
    #[error(
        "field {name:?} holds rich (formatted) text; writing plain text into it would leave its stored formatting in charge of what readers display — use the explicit convert-to-plain-text fill instead"
    )]
    FieldIsRichText {
        /// The fully-qualified name that was requested.
        name: String,
    },
    /// A fill named a field that cannot hold a fillable value: a
    /// `ReadOnly` field, a pushbutton, or a signature field.
    #[error("field {name:?} is not fillable (it is read-only, a pushbutton, or a signature field)")]
    FieldNotFillable {
        /// The field's fully-qualified name.
        name: String,
    },
    /// A check-box/radio fill named an appearance state the field's widgets
    /// do not define (§12.7.4.2.3). Refused rather than writing a `/V`/`/AS`
    /// no widget can display — that would be an invisible, unselectable
    /// state.
    #[error("field {name:?} has no selectable state {state:?} (its widgets define {available:?})")]
    FieldStateUnknown {
        /// The field's fully-qualified name.
        name: String,
        /// The state that was requested.
        state: String,
        /// The on-states the field's widgets actually define (plus `Off`).
        available: Vec<String>,
    },
    /// A fill was attempted on a field locked by a `/FieldMDP` signature
    /// transform (§12.8.2.4). Refused **by name**: filling a field a
    /// signature locks would break that signature's field-lock guarantee.
    ///
    /// pdfce does not yet resolve *which* fields a `/FieldMDP` transform
    /// locks, so it refuses fill conservatively whenever any `/FieldMDP`
    /// transform is present — a fail-clean-safe over-refusal (worst case a
    /// fillable field is declined, never a locked one silently filled). A
    /// per-field `/FieldMDP` lock resolution is a named follow-up.
    #[error(
        "a /FieldMDP signature transform is present (§12.8.2.4); pdfce refuses form fill \
         conservatively rather than risk breaking a field-lock signature"
    )]
    FieldLockedBySignature,
    /// A choice fill supplied several values but the field is not
    /// `MultiSelect` (§12.7.4.4 bit 22). Refused by name rather than
    /// silently keeping only the first — a data file that lists two values
    /// for a single-select field is a mismatch the operator should see.
    #[error(
        "field {name:?} is a single-select choice but {count} values were supplied \
         (only a MultiSelect choice may hold several)"
    )]
    ChoiceRequiresMultiSelect {
        /// The field's fully-qualified name.
        name: String,
        /// How many values were supplied.
        count: usize,
    },
    /// A choice fill named a value that is not among the field's `/Opt`
    /// options (by export or display value) and the field is not an editable
    /// combo (§12.7.4.4 bit 19 `Edit`). Refused by name rather than storing
    /// a value no option can display.
    #[error(
        "field {name:?} has no option {value:?} (its options are {available:?}); \
         only an editable combo box accepts a free-text value"
    )]
    ChoiceValueNotInOptions {
        /// The field's fully-qualified name.
        name: String,
        /// The value that was not found.
        value: String,
        /// The export/display values the field's `/Opt` actually offers.
        available: Vec<String>,
    },
    /// A regenerate/flatten/import operation found no `/AcroForm` at all.
    /// Refused by name rather than silently doing nothing.
    #[error("the document has no interactive form (/AcroForm)")]
    NoInteractiveForm,
    /// An FDF/XFDF data file could not be parsed on import.
    #[error("the form-data file could not be parsed: {0}")]
    FormData(#[from] crate::fdf::FdfError),
    /// A reorder was given something that is not a permutation of the
    /// document's pages.
    ///
    /// Refused rather than repaired: a "reorder" that silently dropped a
    /// page the caller forgot to list, or duplicated one they listed
    /// twice, would be a **delete** or a **duplicate** wearing a
    /// reorder's name.
    #[error("the new order must list each of the {expected} page(s) exactly once, and lists {got}")]
    NotAPermutation {
        /// How many pages the document has.
        expected: usize,
        /// How many distinct, in-range indices the caller supplied.
        got: usize,
    },
    /// Search-and-redact could not extract the document's text, so no
    /// matches could be located (Pass 8).
    #[error("text could not be extracted for search-and-redact: {0}")]
    TextExtraction(String),
    /// A basic vector edit (move / delete / drag-node, Pass 9c-min) was
    /// refused by the surgery planner — an out-of-range object/node, a
    /// non-path target, a singular CTM, or a malformed operator. The inner
    /// [`crate::vector::VectorEditError`] names which.
    #[error(transparent)]
    VectorEdit(#[from] crate::vector::VectorEditError),
    /// A basic vector edit could not read the page's content stream to
    /// decompose it (Pass 9c-min) — a decode/tokenize failure identical to
    /// the one the renderer would hit on the same page.
    #[error("the page content could not be read for a vector edit: {0}")]
    VectorEditContent(#[source] crate::content::ContentError),
    /// A basic vector edit named a page that has no `/Contents` stream to
    /// edit (an empty page).
    #[error("page {page_index} has no /Contents stream to vector-edit")]
    VectorEditNoContents {
        /// The 0-based page index.
        page_index: usize,
    },
}

/// Find every occurrence of `needle` in `hay`, returned as `(start, end)`
/// byte ranges into `hay`. Case-insensitive matching (when requested) is
/// ASCII-only and **byte-offset preserving** — it compares windows with
/// [`u8::eq_ignore_ascii_case`] rather than lower-casing (which would
/// shift offsets for non-ASCII text and break the glyph mapping).
fn find_matches(hay: &str, needle: &str, case_insensitive: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let hb = hay.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() || nb.len() > hb.len() {
        return out;
    }
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        let matched = hb.get(i..i + nb.len()).is_some_and(|w| {
            if case_insensitive {
                w.eq_ignore_ascii_case(nb)
            } else {
                w == nb
            }
        });
        if matched && hay.is_char_boundary(i) && hay.is_char_boundary(i + nb.len()) {
            out.push((i, i + nb.len()));
            i += nb.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Find every match of a simple pattern in `hay`, as `(start, end)` byte
/// ranges. `#` matches any ASCII digit, `?` matches any single character,
/// every other pattern character is a literal (ASCII case-insensitive when
/// requested). Matches are non-overlapping. Character-aligned, so a
/// multi-byte code point is never split.
fn find_pattern_matches(hay: &str, pattern: &str, case_insensitive: bool) -> Vec<(usize, usize)> {
    let pchars: Vec<char> = pattern.chars().collect();
    let hchars: Vec<(usize, char)> = hay.char_indices().collect();
    let mut out = Vec::new();
    if pchars.is_empty() {
        return out;
    }
    let mut i = 0;
    while i < hchars.len() {
        let mut k = i;
        let mut matched = true;
        for pc in &pchars {
            let Some(&(_, hc)) = hchars.get(k) else {
                matched = false;
                break;
            };
            let ok = match pc {
                '#' => hc.is_ascii_digit(),
                '?' => true,
                _ => {
                    if case_insensitive {
                        hc.eq_ignore_ascii_case(pc)
                    } else {
                        hc == *pc
                    }
                }
            };
            if !ok {
                matched = false;
                break;
            }
            k += 1;
        }
        if matched {
            let start = hchars.get(i).map_or(0, |&(o, _)| o);
            let end = hchars.get(k).map_or(hay.len(), |&(o, _)| o);
            out.push((start, end));
            i = k.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

/// Narrow a selectable [`VectorObject`](crate::vector::VectorObject) to a
/// **path** for the move / drag-node surgeries, or name the refusal.
///
/// Text and image/form objects are selectable-for-delete but not
/// path-operand editable in the 9c-min cut (decision 011 §2.1) — moving them
/// needs `Tm`/`cm`-operand surgery, a different operator family, deferred to
/// a fast-follow. Deletion does not go through this narrowing (it is a pure
/// byte-span removal that works on any kind).
fn vector_object_as_path(
    obj: &crate::vector::VectorObject,
    index: usize,
) -> Result<&crate::vector::PathObject, crate::vector::VectorEditError> {
    match obj {
        crate::vector::VectorObject::Path(p) => Ok(p),
        crate::vector::VectorObject::Text(_) => Err(crate::vector::VectorEditError::NotAPath {
            index,
            kind: "text",
        }),
        crate::vector::VectorObject::Image(_) => Err(crate::vector::VectorEditError::NotAPath {
            index,
            kind: "image",
        }),
    }
}

/// A decoded PDF text string, and whether the decode was exact.
///
/// Returned by [`EditSession::info_text`] so a front end can show the
/// value **and** know whether showing it back to the user and saving it
/// again would be lossless — see [`decode_text_string`] for why that is
/// currently a real question.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InfoText {
    /// The text. Bytes with no known mapping appear as U+FFFD.
    pub text: String,
    /// `true` when every byte was decoded with certainty. When `false`,
    /// re-encoding [`InfoText::text`] would **not** reproduce the
    /// original bytes, so a front end must not write the field back
    /// unless the operator actually changed it.
    pub exact: bool,
}

/// An open document plus the operator's unsaved edits.
///
/// See the module docs for the overlay design and the §11.1 rule it
/// makes structural. Construct with [`EditSession::new`]; every mutation
/// goes through a `set_*` method, which is what puts it on the undo
/// stack.
///
/// # Examples
///
/// The Pass's headline contract — edit, undo, save, byte-identical:
///
/// ```
/// use pdfce_core::document::Document;
/// use pdfce_core::edit::EditSession;
/// use pdfce_core::writer::{SaveOptions, save_incremental};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes: Vec<u8> =
///     include_bytes!("../../../fixtures/synthetic/hello.pdf").to_vec();
/// let mut session = EditSession::new(Document::from_bytes(bytes.clone())?);
///
/// session.set_page_rotation(0, 90)?;
/// assert!(session.is_modified());
///
/// session.undo();
/// assert!(!session.is_modified());
///
/// let (out, report) = save_incremental(
///     session.document(),
///     &session.dirty_set(),
///     &SaveOptions::identity(),
/// )?;
/// assert_eq!(out, bytes);
/// assert!(report.byte_identical);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct EditSession {
    /// The base revision. Never mutated; every `ByteSpan` in the
    /// document indexes into its retained buffer, which is what keeps
    /// verbatim re-emission available for untouched objects.
    base: Document,
    /// Current value of every object an edit has touched. An id absent
    /// here reads through to `base`.
    state: BTreeMap<ObjId, Object>,
    /// Objects the session has **deleted**. Read as absent by
    /// [`EditSession::value`], emitted as §7.5.4 free entries by the
    /// writer, and restored by undo.
    ///
    /// A set rather than a sentinel in `state`, because `state`'s
    /// `Option`-shaped absence already means *"read through to the
    /// base"* — the opposite of what deletion means.
    deleted: BTreeSet<ObjId>,
    /// Working copy of the trailer, seeded from the base.
    trailer: Dict,
    /// The next object number to hand out for a created object. Cached
    /// so two creations in one session cannot collide.
    next_number: Option<u32>,
    /// Authored stream bytes (R45, Pass 6.1) — the appearance content
    /// streams of annotations added this session. A created appearance
    /// [`Stream`](crate::object::Stream) keeps the span model: its
    /// `data_span` points into this buffer, expressed in the combined
    /// `base.len() + local` coordinate system (see
    /// [`EditSession::stage_bytes`] and
    /// [`crate::writer::DirtySet::combined_source`]). Empty until the
    /// first annotation is authored, so a session that only performs
    /// pre-6.1 edits carries no staging and its save path is unchanged.
    staging: Vec<u8>,
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl EditSession {
    /// Open an editing session over `doc`.
    ///
    /// Takes the document by value: the session **is** the open
    /// document from this point on, and a second handle to the same
    /// `Document` would be a second, stale view of it. Recover it with
    /// [`EditSession::into_document`].
    #[must_use]
    pub fn new(doc: Document) -> Self {
        let trailer = doc.trailer().clone();
        let next_number = doc.next_object_number();
        Self {
            base: doc,
            state: BTreeMap::new(),
            deleted: BTreeSet::new(),
            trailer,
            next_number,
            staging: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The base document — the parse result, and the writer's source of
    /// verbatim bytes.
    ///
    /// **This is the base revision, not the edited state.** Reading
    /// `session.document().get(id)` gives the value as loaded, not as
    /// edited; use [`EditSession::value`] for the current one. The
    /// accessor exists because everything that reads *unedited* structure
    /// — the renderer, the page-tree walk, the writer's span lookups —
    /// wants exactly this.
    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.base
    }

    /// Give the base document back, discarding any unsaved edits.
    #[must_use]
    pub fn into_document(self) -> Document {
        self.base
    }

    /// The current value of `id`: the session's overlay if an edit
    /// touched it, otherwise the base document's.
    /// A deleted object reads as absent, exactly as a freed
    /// cross-reference entry does after the save (§7.5.4, §7.3.10) — so
    /// the in-memory view and the saved file agree about what exists.
    #[must_use]
    pub fn value(&self, id: ObjId) -> Option<&Object> {
        if self.deleted.contains(&id) {
            return None;
        }
        self.state
            .get(&id)
            .or_else(|| self.base.get(id).map(|io: &IndirectObject| &io.value))
    }

    /// A read view of the document **as the operator currently has it**:
    /// the base revision with every unsaved edit applied.
    ///
    /// This is what makes [`EditSession::pages`] correct after a
    /// structural edit. Pass 3.1's page list was produced by walking the
    /// base tree and patching each leaf's rotation, and its own doc
    /// comment recorded the expiry date on that shortcut: *"the moment an
    /// edit can add or remove a `Kids` entry, patching a base-derived
    /// list is not an approximation — it is wrong."* Pass 3.2 is that
    /// moment.
    #[must_use]
    pub const fn graph(&self) -> SessionGraph<'_> {
        SessionGraph { session: self }
    }

    /// A read view of the document **as the operator currently has it**,
    /// for every consumer that also needs stream BYTES — the rasterizer,
    /// the vector object model, `pageops`' cross-document copier.
    ///
    /// This is the fix for the defect recorded in
    /// `docs/decisions/018-edited-state-is-what-the-canvas-renders.md`:
    /// from Pass 3.1 to Pass 16.2 the GUI rasterized
    /// [`EditSession::document`] — the BASE revision — so every edit the
    /// operator made was authored correctly and displayed not at all.
    /// Passing `&session.view()` where `&session.document()` used to go is
    /// the whole of the read-path half of that fix.
    ///
    /// Two halves, matching [`crate::view`]'s two:
    ///
    /// - **Graph:** `self`, via this type's [`ObjectGraph`] impl — the base
    ///   with the overlay applied and deletions honoured, identical to
    ///   [`EditSession::graph`].
    /// - **Bytes:** a [`StreamSource::Split`] over the base file and the
    ///   R45 `staging` buffer. NOT
    ///   [`EditSession::authored_source`], which would memcpy the entire
    ///   file on every call (see that method's own warning) — the split
    ///   form resolves a staged span with one integer comparison and no
    ///   allocation, which is what makes this callable once per rendered
    ///   frame.
    ///
    /// The version is the base document's: a session cannot currently
    /// raise `/Version`, and if one ever can, this is the line that has to
    /// learn about it.
    ///
    /// # ⚠️ Read-only
    ///
    /// The returned view must never reach the writer — see
    /// [`DocumentView`]'s own doc comment and decision 018 §10 hazard 1.
    /// Saving goes through [`EditSession::dirty_set`], which hands the
    /// writer the staging buffer under its own contract.
    #[must_use]
    pub fn view(&self) -> DocumentView<'_> {
        DocumentView::with_source(
            self,
            StreamSource::Split {
                base: self.base.bytes(),
                staged: &self.staging,
            },
            self.base.version(),
        )
    }

    // -- the save-time diff ------------------------------------------

    /// Compute the dirty set: **what currently differs from the base
    /// revision**, right now, at save time.
    ///
    /// This is the function §11.1 is about. It never consults the undo
    /// history; it compares values. An object edited and then undone
    /// holds a value equal to the base's and is skipped, so it does not
    /// appear in the update section — which is what makes
    /// *edit → undo → save* produce a byte-identical file.
    ///
    /// Equality uses `Object`'s derived `PartialEq`, and that is correct
    /// **here specifically**: both sides come from the same retained
    /// buffer, so a `Stream`'s `ByteSpan` is comparable rather than
    /// misleading (see [`crate::object::equivalent_across_buffers`] for
    /// the cross-buffer case, which this is not).
    #[must_use]
    pub fn dirty_set(&self) -> DirtySet {
        let mut dirty = DirtySet::empty();
        for (id, value) in &self.state {
            match self.base.get(*id) {
                // Net-zero against the base: NOT dirty. The one line
                // this whole module exists to make unavoidable.
                Some(io) if io.value == *value => {}
                _ => dirty.replace(*id, value.clone()),
            }
        }
        // Deletions are a diff against the base too: an id the base
        // never defined cannot be "deleted" into it, and emitting a free
        // entry for one would put a cross-reference entry in the update
        // section for an object §7.5.6 says has not changed.
        for id in &self.deleted {
            if self.base.get(*id).is_some() {
                dirty.delete(*id);
            }
        }
        for (key, value) in self.trailer.iter() {
            if self.base.trailer().get(key.as_bytes()) != Some(value) {
                dirty.patch_trailer(key.clone(), value.clone());
            }
        }
        // R45: hand the writer the authored-stream staging buffer so a
        // replacement value that is an authored appearance Stream resolves
        // its span (which points past the base into this buffer). Cloned
        // because the DirtySet is the writer's owned input; empty for any
        // session that authored nothing, which keeps the pre-6.1 save path
        // byte-for-byte unchanged.
        if !self.staging.is_empty() {
            dirty.set_staging(self.staging.clone());
        }
        dirty
    }

    /// The buffer this session's stream spans index into (R45): the base
    /// file alone when nothing has been authored, or `base ++ staging`
    /// when the session carries authored appearance streams.
    ///
    /// This is what a [`DocumentView`] built
    /// over an editing session must use as its `bytes`, so that
    /// extract/merge/split reading an authored appearance's `data_span`
    /// (which points past the base) resolves it — the X5 hazard the
    /// `DocumentView` assertion was written to catch. Returns a borrowed
    /// slice (zero-copy) in the common no-authoring case.
    ///
    /// # ⚠️ NOT for per-frame use
    ///
    /// Once anything has been authored this returns `Cow::Owned` — a full
    /// `base ++ staging` memcpy, which on decision 018's benchmark document
    /// is ~14 MB **per call**. That is fine for its intended
    /// once-per-operation `pageops` callers (extract/merge/split, which
    /// then serialize a whole new file anyway) and completely unacceptable
    /// on a render loop.
    ///
    /// Anything that reads stream bytes repeatedly — the rasterizer, the
    /// vector decomposer, the GUI hit-test provider — must use
    /// [`EditSession::view`] instead, whose
    /// [`StreamSource::Split`] resolves the same spans against the same two
    /// buffers with one comparison and no allocation. Decision 018 §4
    /// records this rejection explicitly so the cheap-looking call does not
    /// get reintroduced on the hot path later.
    #[must_use]
    pub fn authored_source(&self) -> std::borrow::Cow<'_, [u8]> {
        if self.staging.is_empty() {
            std::borrow::Cow::Borrowed(self.base.bytes())
        } else {
            let base = self.base.bytes();
            let mut combined = Vec::with_capacity(base.len() + self.staging.len());
            combined.extend_from_slice(base);
            combined.extend_from_slice(&self.staging);
            std::borrow::Cow::Owned(combined)
        }
    }

    /// Whether anything currently differs from the base revision.
    ///
    /// Asks the same question [`EditSession::dirty_set`] does, so a
    /// "unsaved changes" indicator can never disagree with what a save
    /// would actually write. In particular an edit-then-undo reports
    /// `false`.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        !self.dirty_set().is_empty()
    }

    // -- undo / redo --------------------------------------------------

    /// Whether there is a command to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is a command to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// What [`EditSession::undo`] would undo, without doing it — for a
    /// front end that labels its Undo control with the operation name.
    #[must_use]
    pub fn undo_kind(&self) -> Option<CommandKind> {
        self.undo.last().map(|c| c.kind)
    }

    /// What [`EditSession::redo`] would redo, without doing it.
    #[must_use]
    pub fn redo_kind(&self) -> Option<CommandKind> {
        self.redo.last().map(|c| c.kind)
    }

    /// Undo the most recent command, returning what was undone.
    ///
    /// Restores each write's recorded `before` value; a `None` before
    /// means the entry is removed from the overlay, which either
    /// restores read-through to the base or — for a created object —
    /// makes it not exist again.
    pub fn undo(&mut self) -> Option<CommandKind> {
        let command = self.undo.pop()?;
        for write in &command.objects {
            Self::write_state(&mut self.state, write.id, write.before.clone());
        }
        for removal in &command.removals {
            Self::write_deleted(&mut self.deleted, removal.id, removal.was_deleted);
        }
        if let Some((before, _)) = &command.trailer {
            self.trailer = before.clone();
        }
        let kind = command.kind;
        self.redo.push(command);
        Some(kind)
    }

    /// Redo the most recently undone command, returning what was redone.
    pub fn redo(&mut self) -> Option<CommandKind> {
        let command = self.redo.pop()?;
        for write in &command.objects {
            Self::write_state(&mut self.state, write.id, write.after.clone());
        }
        for removal in &command.removals {
            Self::write_deleted(&mut self.deleted, removal.id, removal.is_deleted);
        }
        if let Some((_, after)) = &command.trailer {
            self.trailer = after.clone();
        }
        let kind = command.kind;
        self.undo.push(command);
        Some(kind)
    }

    /// How many commands are currently undoable (bounded by
    /// [`MAX_UNDO_DEPTH`]).
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    // -- edits ---------------------------------------------------------

    /// Set or clear one document-information field (§14.3.3).
    ///
    /// `Some(text)` sets it; `None` removes the entry entirely (see
    /// [`Dict::remove`] for why removal rather than an explicit `null`).
    /// Text is encoded by [`encode_text_string`].
    ///
    /// ## Creating `/Info` when the file has none
    ///
    /// A file with no document information dictionary gets one, together
    /// with the trailer's `/Info` reference (Table 15: *"shall be an
    /// indirect reference"*). That is **not** in tension with R41's
    /// no-fingerprint rule, which
    /// [`ProducerPolicy::Set`](crate::writer::ProducerPolicy::Set)
    /// deliberately narrows itself to avoid: R41 forbids pdfce writing
    /// *its own* identity into a file unasked. An operator who types a
    /// title has asked, by name, for exactly this dictionary to exist.
    /// The distinction is authorship, not novelty.
    ///
    /// ## No-ops do not reach the undo stack
    ///
    /// Setting a field to the value it already has, or clearing an
    /// absent one, changes nothing — so no command is recorded and the
    /// redo stack is left alone. An Undo control that steps through
    /// entries which visibly do nothing is worse than useless.
    ///
    /// # Errors
    ///
    /// [`EditError::NotADictionary`] if `/Info` points at a non-dictionary;
    /// [`EditError::ObjectNumbersExhausted`] if a new object is needed
    /// and no number is free.
    pub fn set_info_field(
        &mut self,
        field: InfoField,
        value: Option<&str>,
    ) -> Result<(), EditError> {
        let kind = match value {
            Some(_) => CommandKind::SetInfoField(field),
            None => CommandKind::ClearInfoField(field),
        };
        let key = Name::from(field.key());

        match self.info_id() {
            // The document already has an /Info dictionary: edit it.
            Some(id) => {
                let Some(Object::Dict(current)) = self.value(id) else {
                    // No /Info dictionary value to edit. If the operator
                    // is CLEARING a field on a malformed /Info, doing
                    // nothing is right; setting one must refuse rather
                    // than overwrite whatever is really there.
                    if value.is_none() {
                        return Ok(());
                    }
                    return Err(EditError::NotADictionary {
                        id,
                        key: field_key_str(field),
                    });
                };
                let mut updated = current.clone();
                match value {
                    Some(text) => {
                        updated.insert(key, Object::String(encode_text_string(text)));
                    }
                    None => {
                        updated.remove(field.key());
                    }
                }
                if updated == *current {
                    return Ok(()); // no-op
                }
                let before = self.state.get(&id).cloned();
                self.commit(Command {
                    kind,
                    objects: vec![ObjectWrite {
                        id,
                        before,
                        after: Some(Object::Dict(updated)),
                    }],
                    removals: Vec::new(),
                    trailer: None,
                });
                Ok(())
            }
            // No /Info at all. Clearing a field is already true; setting
            // one creates the dictionary and the trailer reference in a
            // single undoable command.
            None => {
                let Some(text) = value else {
                    return Ok(()); // no-op
                };
                // Creating an object raises `/Size`, which exposes every
                // entry this file's `/Size` is suppressing. Refuse by
                // name rather than resurrect objects nobody touched —
                // see `Document::suppressed_object_count`.
                let suppressed = self.base.suppressed_object_count();
                if suppressed > 0 {
                    return Err(EditError::ObjectCreationWouldExposeHiddenObjects {
                        count: suppressed,
                    });
                }
                let id = ObjId::new(
                    self.next_number.ok_or(EditError::ObjectNumbersExhausted)?,
                    0,
                );
                let mut info = Dict::new();
                info.insert(key, Object::String(encode_text_string(text)));

                let trailer_before = self.trailer.clone();
                let mut trailer_after = self.trailer.clone();
                trailer_after.insert(Name::from(b"Info"), Object::Reference(id));

                self.next_number = self.next_number.and_then(|n| n.checked_add(1));
                self.commit(Command {
                    kind,
                    objects: vec![ObjectWrite {
                        id,
                        before: None, // did not exist
                        after: Some(Object::Dict(info)),
                    }],
                    removals: Vec::new(),
                    trailer: Some((trailer_before, trailer_after)),
                });
                Ok(())
            }
        }
    }

    /// The current raw bytes of a document-information field, or `None`
    /// if it is absent. Reflects unsaved edits.
    #[must_use]
    pub fn info_bytes(&self, field: InfoField) -> Option<Vec<u8>> {
        let id = self.info_id()?;
        let Some(Object::Dict(dict)) = self.value(id) else {
            return None;
        };
        match dict.get(field.key()) {
            Some(Object::String(bytes)) => Some(bytes.clone()),
            _ => None,
        }
    }

    /// The current value of a document-information field as displayable
    /// text, or `None` if it is absent. Reflects unsaved edits.
    ///
    /// Check [`InfoText::exact`] before writing the text back: a
    /// non-exact decode means re-encoding would change the bytes, so a
    /// front end must only save the field if the operator actually edited
    /// it. See [`decode_text_string`].
    #[must_use]
    pub fn info_text(&self, field: InfoField) -> Option<InfoText> {
        self.info_bytes(field).as_deref().map(decode_text_string)
    }

    /// Set one page's `/Rotate` to an absolute value (Table 30).
    ///
    /// `degrees` must be a multiple of 90; negative and ≥360 values are
    /// accepted and normalized with a **positive** modulo, because Table
    /// 30 says only *"a multiple of 90"* and `-90` is one (the spec RAG
    /// records this explicitly as a real-world shape).
    ///
    /// The entry is written on the **page object itself**, which
    /// overrides any value inherited from an ancestor `Pages` node
    /// (§7.7.3.4). That is both the correct semantics and the
    /// minimal-diff choice: rotating one page must not rewrite an
    /// ancestor node that governs its siblings.
    ///
    /// ## Rotating back to where you started writes nothing
    ///
    /// When the requested rotation equals the page's **base** effective
    /// rotation, the page object's own `/Rotate` is restored to exactly
    /// what the base file carried — present, absent, or oddly spelled —
    /// rather than an explicit normalized entry being written. Three
    /// things follow, and all three matter:
    ///
    /// - Four quarter-turns net to nothing, so the document reports
    ///   itself unmodified and a save is byte-identical. That is the same
    ///   §11.1 net-zero rule undo relies on, reached without undo.
    /// - A page whose 90° is *inherited* from an ancestor and is set to
    ///   90° does not gain a redundant explicit entry. Writing one would
    ///   be modifying an object pdfce was not asked to modify — §5's
    ///   invariant, not a nicety.
    /// - A base `/Rotate 450` set to 90° keeps its `450`. It means the
    ///   same thing (Table 30 constrains only "a multiple of 90"), and
    ///   rewriting it to `90` would be exactly the silent normalization
    ///   R33 forbids.
    ///
    /// # Errors
    ///
    /// [`EditError::RotationNotMultipleOf90`], [`EditError::PageOutOfRange`],
    /// [`EditError::PageTree`], or [`EditError::NotADictionary`].
    pub fn set_page_rotation(&mut self, page_index: usize, degrees: i32) -> Result<(), EditError> {
        let write = self.rotation_write(page_index, degrees)?;
        let Some((write, normalized)) = write else {
            return Ok(()); // no-op
        };
        self.commit(Command {
            kind: CommandKind::SetPageRotation {
                page_index,
                degrees: normalized,
            },
            objects: vec![write],
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    /// Work out the single object write that setting `page_index`'s
    /// rotation to `degrees` requires, or `None` when it requires none.
    ///
    /// Extracted from [`EditSession::set_page_rotation`] so that
    /// [`EditSession::rotate_pages`] can build **one** command holding N
    /// writes rather than pushing N commands — §11.3's rule that one
    /// operator gesture is one undo entry.
    ///
    /// ## The three-way choice, and why the order matters
    ///
    /// 1. **The base file's own spelling wins if it means the same
    ///    thing.** A base `/Rotate 450` asked for 90° keeps its `450`:
    ///    Table 30 constrains only *"a multiple of 90"*, the two mean the
    ///    same, and rewriting one to the other is exactly the silent
    ///    normalization R33 forbids. Looked up **physically** rather than
    ///    through [`Dict::get`], which collapses a `null` value to absent
    ///    (§7.3.7) — a base `/Rotate null` must be restored as `null`, or
    ///    "restore" is still a byte change.
    /// 2. **Otherwise, if the target equals what the page would
    ///    *inherit*, the page's own entry is removed** rather than an
    ///    explicit one written. Writing a redundant `/Rotate 90` onto a
    ///    page whose ancestor already says 90 would be modifying an
    ///    object pdfce was not asked to modify (§5). The inherited value
    ///    is read from the **current** slot walk, not the base one, so
    ///    this stays correct after a reorder has moved the page under a
    ///    different ancestor.
    /// 3. **Otherwise** an explicit normalized entry is written.
    ///
    /// Rule 1 before rule 2 is what makes four quarter-turns net to
    /// nothing on a page that had an explicit entry, *and* leaves an
    /// unusual-but-legal spelling alone.
    ///
    /// # Errors
    ///
    /// [`EditError::RotationNotMultipleOf90`], [`EditError::PageOutOfRange`],
    /// [`EditError::PageTree`], or [`EditError::NotADictionary`].
    fn rotation_write(
        &self,
        page_index: usize,
        degrees: i32,
    ) -> Result<Option<(ObjectWrite, u16)>, EditError> {
        if degrees % 90 != 0 {
            return Err(EditError::RotationNotMultipleOf90 { degrees });
        }
        let normalized = normalize_rotation(i64::from(degrees));

        let slots = self.page_slots()?;
        let count = slots.len();
        let slot = slots.get(page_index).ok_or(EditError::PageOutOfRange {
            index: page_index,
            count,
        })?;
        let id = slot.id;
        // What this page would show with no entry of its own — from the
        // CURRENT tree, so a page moved under a different ancestor by a
        // reorder is judged against its new ancestry.
        let inherited = slot
            .inherited
            .rotate
            .as_ref()
            .map(|value| self.graph().resolve(value).clone())
            .and_then(|value| value.as_int())
            .map_or(0, normalize_rotation);

        let Some(Object::Dict(current)) = self.value(id) else {
            return Err(EditError::NotADictionary { id, key: "Rotate" });
        };
        let mut updated = current.clone();

        // Rule 1: the base file's physical entry, if it means the target.
        let base_own = self
            .base
            .get(id)
            .and_then(|io| io.value.as_dict())
            .and_then(|d| d.0.iter().find(|(k, _)| k.as_bytes() == b"Rotate"))
            .map(|(_, value)| value.clone());
        let base_own_means_target = base_own
            .as_ref()
            .and_then(Object::as_int)
            .is_some_and(|v| normalize_rotation(v) == normalized);

        if let (true, Some(original)) = (base_own_means_target, base_own.as_ref()) {
            updated.insert(Name::from(b"Rotate"), original.clone());
        } else if normalized == inherited {
            updated.remove(b"Rotate");
        } else {
            updated.insert(
                Name::from(b"Rotate"),
                Object::Integer(i64::from(normalized)),
            );
        }

        if updated == *current {
            return Ok(None); // no-op — nothing reaches the undo stack
        }
        Ok(Some((
            ObjectWrite {
                id,
                before: self.state.get(&id).cloned(),
                after: Some(Object::Dict(updated)),
            },
            normalized,
        )))
    }

    /// Rotate one page **relative to its current effective rotation**.
    ///
    /// The turn-it-90°-clockwise operation a toolbar button performs.
    /// "Effective" matters: the base value may be inherited from an
    /// ancestor `Pages` node rather than written on the page, and a
    /// relative rotation computed from a missing entry (i.e. from 0)
    /// would visibly jump.
    ///
    /// # Errors
    ///
    /// As [`EditSession::set_page_rotation`].
    pub fn rotate_page_by(&mut self, page_index: usize, delta: i32) -> Result<(), EditError> {
        if delta % 90 != 0 {
            return Err(EditError::RotationNotMultipleOf90 { degrees: delta });
        }
        let pages = self.pages()?;
        let count = pages.len();
        let current = pages
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count,
            })?
            .rotate;
        let target = i64::from(current) + i64::from(delta);
        self.set_page_rotation(page_index, i32::from(normalize_rotation(target)))
    }

    /// The document's pages in document order, **as the operator
    /// currently has them** — every unsaved structural and rotation edit
    /// applied.
    ///
    /// Pass 3.1 produced this by walking the *base* page tree and
    /// patching each leaf's rotation, and recorded the expiry date on
    /// that shortcut in its own doc comment: *"Pass 3.2 must replace this
    /// with an overlay-aware walk. The moment an edit can add or remove a
    /// `Kids` entry, patching a base-derived list is not an
    /// approximation — it is wrong."* It now walks
    /// [`EditSession::graph`], which is the overlay, through the same
    /// [`page_tree::pages_in`] every other consumer uses. There is one
    /// page-tree walk in pdfce, and this is a caller of it.
    ///
    /// # Errors
    ///
    /// [`PageTreeError`] — structural damage or a guard violation in the
    /// page tree as currently edited.
    pub fn pages(&self) -> Result<Vec<Page>, PageTreeError> {
        page_tree::pages_in(&self.graph())
    }

    /// The document's pages as **structural slots** — parent node, index
    /// within it, ancestor chain, inherited raw attributes.
    ///
    /// What every structural operation actually needs, and deliberately
    /// separate from [`EditSession::pages`]: resolving a page's
    /// appearance can fail on a damaged file
    /// ([`PageTreeError::MissingRequired`]), and a page with no
    /// `MediaBox` anywhere is still a page that can be deleted.
    ///
    /// # Errors
    ///
    /// [`PageTreeError`] — structural damage or a guard violation.
    pub fn page_slots(&self) -> Result<Vec<page_tree::PageSlot>, PageTreeError> {
        page_tree::page_slots(&self.graph())
    }

    // -- saving ---------------------------------------------------------

    /// Save an incremental update (§7.5.6) and return the bytes.
    ///
    /// pdfce's default save mode. The dirty set is computed here, at save
    /// time, from the current state — which is the whole point (module
    /// docs).
    ///
    /// ⚠️ Incremental save **structurally preserves superseded content**
    /// (`ARCHITECTURE.md` §5.2): the old bytes of every replaced object
    /// stay in the file by construction. Any operation whose contract is
    /// *removal* must use [`EditSession::to_full_bytes`] instead — and
    /// must read that method's note about compressed objects.
    ///
    /// # Errors
    ///
    /// [`WriteError`].
    pub fn to_incremental_bytes(
        &self,
        options: &SaveOptions,
    ) -> Result<(Vec<u8>, SaveReport), WriteError> {
        crate::writer::save_incremental(&self.base, &self.dirty_set(), options)
    }

    /// Rewrite the whole document as one revision, applying the current
    /// edits, and return the bytes.
    ///
    /// ⚠️ Destroys every existing digital signature (§12.8.1), and does
    /// **not** by itself remove the superseded value of a compressed
    /// object that an edit promoted out of its object stream — see
    /// `crate::writer::save`'s "Promotion, and the stale copy it leaves".
    ///
    /// # Errors
    ///
    /// [`WriteError`], notably a refusal for a hybrid-reference input.
    pub fn to_full_bytes(
        &self,
        options: &SaveOptions,
    ) -> Result<(Vec<u8>, SaveReport), WriteError> {
        crate::writer::save_full(&self.base, &self.dirty_set(), options)
    }

    // -- in-place page-text editing (Pass 14.3 §0.2) ------------------

    /// Apply one in-place page-text REPLACE edit as a single undo-able
    /// command — the session-integrated sibling of the free function
    /// [`text_edit::edit_text`](crate::text_edit::edit_text).
    ///
    /// It reuses the EXACT locate / re-encode / relayout / font-on-edit-gate
    /// surgery (through the shared
    /// [`plan_edit`](crate::text_edit::edit::plan_edit)) but applies the
    /// result to THIS session's in-memory content-stream object as one
    /// command on the undo stack — returning the same
    /// [`EditReport`](crate::text_edit::EditReport) the free function
    /// produces, NOT already-saved bytes. `to_incremental_bytes` is called
    /// later, at real Save, exactly like every other command.
    ///
    /// ## Why both this and the free function exist
    ///
    /// The free function returns already-incrementally-saved bytes — the
    /// right shape for `pdfce-cli edit-text` (one-shot batch), the WRONG
    /// shape for an interactive GUI that must let the operator make five
    /// edits and Ctrl+Z them one at a time. Reloading the whole document from
    /// the saved bytes after every accepted keystroke would force a full
    /// save + reparse + re-extract per edit and splice two structurally
    /// different undo-entry kinds together (Pass 14.3 UI spec §0.2). This
    /// method is the addition that avoids that; the free function is
    /// unchanged for the CLI.
    ///
    /// ## Multi-edit accumulation + minimal-diff (R32/R46)
    ///
    /// The surgery walks the page's CURRENT content: the session's own edited
    /// raw stream when a prior text/format edit already rewrote this content
    /// object this session (read back from the staging buffer — see
    /// [`Self::current_page_content`]), else the base document's decoded
    /// content. So five sequential edits compose correctly, and each is one
    /// undo-stack entry whose `before` restores the exact prior
    /// content-object value (a prior edit's raw stream, or
    /// read-through-to-base on the first). Saved output stays minimal-diff:
    /// only the content stream object (+ any collapsed extras) differs from
    /// the base — everything else is byte-verbatim (incremental append),
    /// identical to the free function's output for a text-edit-only session.
    ///
    /// # Errors
    ///
    /// The same [`text_edit::EditError`](crate::text_edit::EditError) the free
    /// function raises: a named font-on-edit refusal, no match, an
    /// unsupported run, an encrypted document, or a content-parse failure. A
    /// refusal happens BEFORE any mutation — the session is left untouched
    /// (rule 4), because `plan_edit` returns `Err` before any `commit`.
    pub fn edit_text(
        &mut self,
        req: &crate::text_edit::EditRequest,
        opts: &crate::text_edit::EditOptions,
    ) -> Result<crate::text_edit::EditReport, crate::text_edit::EditError> {
        use crate::text_edit::EditError as TeError;
        use crate::text_edit::edit::plan_edit;

        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(TeError::Encrypted);
        }
        let pages = page_tree::pages(&self.base)?;
        let page = pages
            .get(req.page_index)
            .ok_or(TeError::PageIndex(req.page_index))?;
        let content_id = *page
            .contents
            .first()
            .ok_or_else(|| TeError::Unsupported("the page has no /Contents to edit".to_owned()))?;

        let stream = self
            .current_page_content(content_id, page)
            .map_err(TeError::Content)?;
        let plan = plan_edit(&self.base, page, &stream, req, opts)?;

        let command =
            self.text_edit_command(CommandKind::EditText, content_id, page, plan.new_content);
        self.commit(command);
        Ok(plan.report)
    }

    /// Apply one in-place page-text FORMAT edit (size / fill-colour model /
    /// font family) as a single undo-able command — the session-integrated
    /// sibling of [`text_edit::set_format`](crate::text_edit::set_format),
    /// reusing the shared [`plan_format`](crate::text_edit::format::plan_format)
    /// surgery. See [`Self::edit_text`] for the accumulation / minimal-diff /
    /// why-both-exist rationale (identical, one class of command up).
    ///
    /// # Errors
    ///
    /// The same [`text_edit::FormatError`](crate::text_edit::FormatError) the
    /// free function raises: a no-op request, an invalid colour, a missing
    /// target font, a named coverage/classification refusal, no match, an
    /// unsupported run, an encrypted document, or a content-parse failure.
    /// A refusal happens BEFORE any mutation (rule 4).
    pub fn format_text(
        &mut self,
        req: &crate::text_edit::FormatRequest,
        opts: &crate::text_edit::FormatOptions,
    ) -> Result<crate::text_edit::FormatReport, crate::text_edit::FormatError> {
        use crate::text_edit::FormatError as FmtError;
        use crate::text_edit::format::plan_format;

        // The pre-checks `plan_format` assumes (mirrors the free
        // `set_format`): a no-op request and encryption are refused before
        // any surgery.
        //
        // The emptiness test is delegated to `FormatRequest::is_empty`
        // rather than re-listed here. It used to be re-listed, and Pass
        // 19.1 found out why that was a bug: three new operation fields
        // were added, the copy here did not learn about them, and a
        // spacing-only request became a phantom `NoOp` on the session path
        // — the very path the GUI drives — while succeeding through the
        // free function. One predicate, two callers.
        if req.is_empty() {
            return Err(FmtError::NoOp);
        }
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(FmtError::Encrypted);
        }
        let pages = page_tree::pages(&self.base)?;
        let page = pages
            .get(req.page_index)
            .ok_or(FmtError::PageIndex(req.page_index))?;
        let content_id = *page
            .contents
            .first()
            .ok_or_else(|| FmtError::Unsupported("the page has no /Contents to edit".to_owned()))?;

        let stream = self
            .current_page_content(content_id, page)
            .map_err(FmtError::Content)?;
        let plan = plan_format(&self.base, page, &stream, req, opts)?;

        let command =
            self.text_edit_command(CommandKind::FormatText, content_id, page, plan.new_content);
        self.commit(command);
        Ok(plan.report)
    }

    /// Ask what a synthetic bold/italic request **would** do to one run,
    /// without doing it (Pass 19.3, ui-spec §1.1 "Option B").
    ///
    /// `&self`, and side-effect-free: it reads the session-current content
    /// for the page, locates the same anchor [`Self::format_text`] would, and
    /// runs the R90 gate as a query. Nothing is planned, staged, committed or
    /// cached. Calling it a hundred times changes nothing.
    ///
    /// # Why a session method and not a free function over `Document`
    ///
    /// The answer must be about the page **as the operator is looking at it**,
    /// which after any accepted edit is the session's staged content, not the
    /// base document's (the same trap `build_text_edit_state` documents on the
    /// GUI side: a query answered against `session.document()` describes a
    /// page that no longer exists). Reusing `current_page_content` is what
    /// makes the preview and the subsequent commit agree.
    ///
    /// # Errors
    ///
    /// [`FormatError::Encrypted`](crate::text_edit::FormatError::Encrypted),
    /// a bad page index, a page with no `/Contents`, a content-parse failure,
    /// or the location failures the planner reports (no match, unsupported
    /// anchor, unresolvable font resource).
    /// [`RealFaceAvailable`](crate::text_edit::FormatError::RealFaceAvailable)
    /// is never returned — it is the *answer*, delivered as
    /// [`StyleOutcome::RealFaceResolves`](crate::text_edit::StyleOutcome::RealFaceResolves).
    pub fn preview_style_resolution(
        &self,
        page_index: usize,
        find: &str,
        pinned_span: Option<crate::span::ByteSpan>,
        want: crate::text_edit::StyleSynthesis,
    ) -> Result<crate::text_edit::StyleResolution, crate::text_edit::FormatError> {
        use crate::text_edit::FormatError as FmtError;
        use crate::text_edit::format::preview_style_resolution;

        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(FmtError::Encrypted);
        }
        let pages = page_tree::pages(&self.base)?;
        let page = pages
            .get(page_index)
            .ok_or(FmtError::PageIndex(page_index))?;
        let content_id = *page
            .contents
            .first()
            .ok_or_else(|| FmtError::Unsupported("the page has no /Contents to edit".to_owned()))?;
        let stream = self
            .current_page_content(content_id, page)
            .map_err(FmtError::Content)?;
        preview_style_resolution(&self.base, page, &stream, find, pinned_span, want)
    }

    /// Apply one within-block reflow (Pass 15.1) as a single undo-able
    /// command — the session-integrated sibling of the free-function
    /// [`apply_reflow`](crate::text_edit::apply_reflow), reusing the shared
    /// [`plan_reflow_from_doc`](crate::text_edit::reflow_apply::plan_reflow_from_doc)
    /// surgery. The block `block_index` on page `page_index` is re-wrapped
    /// under `req` (width/alignment/leading, all optional) and its own
    /// content-stream object re-emitted at the new origins/breaks; the change
    /// lands as ONE [`CommandKind::ReflowBlock`] whose `before` restores the
    /// byte-identical pre-reflow stream on undo (decision 015 §3.4/R75).
    ///
    /// The reflow is planned against the **base** document's content (it
    /// extracts + recognises the page fresh, needing provenance the staging
    /// buffer does not carry), so — unlike the accumulating
    /// [`Self::edit_text`]/[`Self::format_text`] — it refuses when the page's
    /// content object was **already** rewritten this session (a prior text or
    /// format edit): the base-relative byte offsets would not match the staged
    /// content. Save and reopen to reflow after an in-session edit of the same
    /// page. This is a clean, named refusal, never a silent mis-splice
    /// (rule 4).
    ///
    /// # Errors
    ///
    /// The same [`ReflowApplyError`](crate::text_edit::ReflowApplyError) the
    /// free function raises — a named composite refusal, a
    /// rotated/shared/non-contiguous block, a missing-provenance or
    /// bad-index/width error — plus an [`ReflowApplyError::Unsupported`] when
    /// the page's content was already edited this session. A refusal happens
    /// BEFORE any mutation (rule 4): the session is left untouched.
    pub fn reflow_block(
        &mut self,
        page_index: usize,
        block_index: usize,
        req: &crate::text_edit::ReflowRequest,
    ) -> Result<crate::text_edit::ReflowApplyReport, crate::text_edit::ReflowApplyError> {
        use crate::text_edit::ReflowApplyError as RErr;
        use crate::text_edit::reflow_apply::plan_reflow_from_doc;

        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(RErr::Encrypted);
        }
        let pages = page_tree::pages(&self.base)?;
        let page = pages.get(page_index).ok_or(RErr::PageIndex(page_index))?;
        let content_id = *page
            .contents
            .first()
            .ok_or_else(|| RErr::Unsupported("the page has no /Contents to reflow".to_owned()))?;

        // Reflow is planned from base content; refuse if this content object
        // was already rewritten this session (see the method docs).
        if self.state.contains_key(&content_id) {
            return Err(RErr::Unsupported(
                "the page's content was already edited this session; reflow is planned against \
                 the base content, so save and reopen before reflowing this page"
                    .to_owned(),
            ));
        }

        let plan = plan_reflow_from_doc(&self.base, page_index, block_index, req)?;
        let kind = CommandKind::ReflowBlock {
            lines_before: plan.report.lines_before,
            lines_after: plan.report.lines_after,
        };
        let command = self.text_edit_command(kind, content_id, page, plan.new_content);
        self.commit(command);
        Ok(plan.report)
    }

    /// Add a NEW single-line text run at operator coordinates as one undo-able
    /// command (Pass 16.0 / FF-D) — the session-integrated sibling of the
    /// free-function [`add_text`](crate::text_edit::add_text), reusing the
    /// shared [`plan_add_text`](crate::text_edit::addtext::plan_add_text)
    /// planner so the two never drift.
    ///
    /// Synthesizes a fresh `BT…ET` run and APPENDS it as a new content stream
    /// in the page `/Contents` array (§7.7.3.3), leaving every original content
    /// stream **byte-identical** (R32/R46); adds one Standard-14 font dict to
    /// the page `/Resources` `/Font` (no embedding, R79; the §7.7.3.4
    /// inheritance trap handled by the planner). It is genuine page content —
    /// editable afterward by [`Self::edit_text`] / [`Self::format_text`] —
    /// **never** a FreeText annotation (R78).
    ///
    /// Lands as ONE [`CommandKind::AddText`]: the two created objects
    /// (before `None`) and the modified page dict (before = the current value)
    /// are one undo entry, so undo removes the created objects and restores the
    /// byte-identical original page dict. The planner reads the SESSION-current
    /// page dict (via [`Self::graph`]), so an `/Annots` a prior op added to the
    /// same page is preserved.
    ///
    /// # Errors
    ///
    /// The same [`AddTextError`](crate::text_edit::AddTextError) the free
    /// function raises — a named font refusal (R71), an out-of-range page,
    /// empty text, an invalid size, encryption, or an object-creation/`/Size`
    /// conflict. A refusal happens BEFORE any mutation (rule 4): the session is
    /// left untouched, because [`plan_add_text`](crate::text_edit::addtext::plan_add_text)
    /// returns `Err` before any `commit`.
    pub fn add_text(
        &mut self,
        req: &crate::text_edit::AddTextRequest,
    ) -> Result<crate::text_edit::AddTextReport, crate::text_edit::AddTextError> {
        use crate::text_edit::AddTextError as AtError;
        use crate::text_edit::addtext::plan_add_text;
        use crate::text_edit::edit::make_raw_stream;

        // Guards mirror the free function and `add_markup`, in the SAME order
        // (encryption → certification → /Size-hides-objects): each is a named
        // refusal made before any work.
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(AtError::Encrypted);
        }
        // An enforced-DocMDP certification forbids adding page content
        // (ISO 32000-1 §12.8.4 Table 258). This is the add-text mirror of the
        // `self.check_certification()?` guard `EditSession::add_markup` runs
        // before authoring an annotation — the SAME census machinery, placed
        // in the SAME position relative to the encryption/suppressed guards.
        // Delegating to the shared `refuse_if_certification_forbids` (which the
        // free `add_text` also calls) keeps the GUI and CLI paths in lockstep,
        // so neither operator-facing entry can add page content to an
        // enforced-certified document unguarded.
        crate::text_edit::addtext::refuse_if_certification_forbids(&self.graph())?;
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(AtError::HiddenObjects { count: suppressed });
        }

        // Plan against the SESSION overlay (session-aware page dict + resources),
        // then drop the immutable graph borrow before mutating.
        let prep = {
            let pages = self.pages()?;
            let page = pages
                .get(req.page_index)
                .ok_or(AtError::PageIndex(req.page_index))?;
            plan_add_text(req, page, &self.graph())?
        };

        let content_num = self
            .alloc_number()
            .map_err(|_| AtError::ObjectNumbersExhausted)?;
        let font_num = self
            .alloc_number()
            .map_err(|_| AtError::ObjectNumbersExhausted)?;
        let content_id = ObjId::new(content_num, 0);
        let font_id = ObjId::new(font_num, 0);

        let new_page = prep.build_page_dict(content_id, font_id);
        let content_len = prep.content_data.len();
        let span = self.stage_bytes(&prep.content_data);
        let content_stream = make_raw_stream(span, content_len);
        let page_before = self.value(prep.page_id).cloned();

        let objects = vec![
            ObjectWrite {
                id: content_id,
                before: None,
                after: Some(content_stream),
            },
            ObjectWrite {
                id: font_id,
                before: None,
                after: Some(prep.font_dict.clone()),
            },
            ObjectWrite {
                id: prep.page_id,
                before: page_before,
                after: Some(Object::Dict(new_page)),
            },
        ];
        self.commit(Command {
            kind: CommandKind::AddText,
            objects,
            removals: Vec::new(),
            trailer: None,
        });

        let mut report = prep.report;
        report.content_object = content_num;
        report.font_object = font_num;
        Ok(report)
    }

    // -- basic vector editing (Pass 9c-min, decision 011 §2.5) --------

    /// **Move** the object at paint-order `object_index` on page
    /// `page_index` by the page-space displacement `(dx, dy)`, as one
    /// undoable command (decision 011 §2.5 operation 1).
    ///
    /// Content-stream surgery via [`crate::vector::plan_move`]: the object's
    /// path-construction operands are translated (CTM-aware — the page-space
    /// drag is mapped to the object's user space by its captured CTM's linear
    /// inverse), and ONLY the edited content stream is re-emitted (R46/§5.7
    /// named exception); every other object stays byte-verbatim. Lands as one
    /// [`CommandKind::MoveObject`]; undo restores the byte-identical pre-move
    /// stream.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] (out-of-range object, non-path target,
    /// singular CTM, malformed operator — see
    /// [`crate::vector::VectorEditError`]), [`EditError::PageOutOfRange`],
    /// [`EditError::VectorEditNoContents`], [`EditError::VectorEditContent`]
    /// (undecodable page), [`EditError::DocumentEncrypted`],
    /// or [`EditError::CertificationForbidsChange`]. Every refusal happens
    /// **before** any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless it had to change the *form* of an
    /// operator to express the request (expanding an `re` rectangle whose
    /// corner was dragged out of square, materializing the `m` an
    /// implicitly-started subpath never had). The caller must surface them:
    /// the drawing is unchanged but the bytes are not recoverable by
    /// reversing the gesture, and rule 4 forbids letting the operator find
    /// that out from a diff.
    pub fn move_object(
        &mut self,
        page_index: usize,
        object_index: usize,
        dx: f64,
        dy: f64,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::MoveObject, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_move(stream, path, dx, dy)?)
        })
    }

    /// **Delete** the object at paint-order `object_index` on page
    /// `page_index`, as one undoable command (decision 011 §2.5 operation
    /// 2).
    ///
    /// Content-stream surgery via [`crate::vector::plan_delete`]: the
    /// object's construction + painting operators are removed from the
    /// content stream (R46/§5.7 named exception); every other object stays
    /// byte-verbatim. Works on any object kind (path/text/image — it is a
    /// pure byte-span removal). Lands as one [`CommandKind::DeleteObject`];
    /// undo restores the byte-identical pre-delete stream.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] (out-of-range object),
    /// [`EditError::PageOutOfRange`], [`EditError::VectorEditNoContents`],
    /// [`EditError::VectorEditContent`],
    /// [`EditError::DocumentEncrypted`], or
    /// [`EditError::CertificationForbidsChange`]. Every refusal happens
    /// before any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless it had to change the *form* of an
    /// operator to express the request (expanding an `re` rectangle whose
    /// corner was dragged out of square, materializing the `m` an
    /// implicitly-started subpath never had). The caller must surface them:
    /// the drawing is unchanged but the bytes are not recoverable by
    /// reversing the gesture, and rule 4 forbids letting the operator find
    /// that out from a diff.
    pub fn delete_object(
        &mut self,
        page_index: usize,
        object_index: usize,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::DeleteObject, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            Ok(crate::vector::plan_delete(stream, obj)?)
        })
    }

    /// **Delete one subpath** of the path object at paint-order `object_index`
    /// on page `page_index`, as one undoable command (Pass 25.2).
    ///
    /// # Why this is not just `delete_object` with a smaller target
    ///
    /// A CAD producer commonly emits an entire drawing view as a single path
    /// object — measured on a real SolidWorks export, one stroked path with
    /// 1194 subpaths covering a whole isometric view. On such a file
    /// [`EditSession::delete_object`] can only remove the entire view, and an
    /// operator asking to delete "this line" means one of its subpaths. This
    /// is that operation.
    ///
    /// `subpath_index` is into the object's subpaths in **decomposition
    /// order** — the same order [`crate::vector::hit_test_subpaths`] returns
    /// indices in, so a subpath picked by a click can be handed straight here.
    /// The planner re-derives the subpaths from the operator bytes and refuses
    /// if the counts disagree, so an index can never silently name a different
    /// line from the one that was picked.
    ///
    /// Content-stream surgery via [`crate::vector::plan_delete_subpath`]: only
    /// the edited stream is re-emitted; every other object in the file stays
    /// byte-verbatim (R46/§5.7 named exception). Lands as one
    /// [`CommandKind::DeleteSubpath`]; undo restores the byte-identical
    /// pre-delete stream.
    ///
    /// **Deleting the only subpath deletes the object**, because a painting
    /// operator with no path is not a smaller object — it is meaningless.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] wrapping
    /// [`SubpathOutOfRange`](crate::vector::VectorEditError::SubpathOutOfRange),
    /// [`ClippingPath`](crate::vector::VectorEditError::ClippingPath) (deleting
    /// part of a clip would change what OTHER content is visible — refused
    /// under rule 4), or
    /// [`SubpathStructureMismatch`](crate::vector::VectorEditError::SubpathStructureMismatch);
    /// [`EditError::VectorEdit`] with `NotAPath` for a text/image target; plus
    /// [`EditError::PageOutOfRange`], [`EditError::VectorEditNoContents`],
    /// [`EditError::VectorEditContent`],
    /// [`EditError::DocumentEncrypted`], or
    /// [`EditError::CertificationForbidsChange`]. Every refusal happens before
    /// any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless it had to change the *form* of an
    /// operator to express the request (expanding an `re` rectangle whose
    /// corner was dragged out of square, materializing the `m` an
    /// implicitly-started subpath never had). The caller must surface them:
    /// the drawing is unchanged but the bytes are not recoverable by
    /// reversing the gesture, and rule 4 forbids letting the operator find
    /// that out from a diff.
    /// **Delete one anchor** of the path object at paint-order `object_index`
    /// on page `page_index`, as one undoable command (Pass 36.1).
    ///
    /// Content-stream surgery via [`crate::vector::plan_delete_node`]: the one
    /// segment operator that produced the anchor is excised — or, when the
    /// anchor is its subpath's first, the following operator is rewritten into
    /// the new `m` — and every other object stays byte-verbatim (R46/§5.7
    /// named exception). Lands as one [`CommandKind::DeleteNode`]; undo
    /// restores the byte-identical pre-delete stream.
    ///
    /// `node_index` is the anchor's 0-based index in decomposition order, the
    /// same numbering [`Self::move_node`] takes.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] wrapping
    /// [`NodeOutOfRange`](crate::vector::VectorEditError::NodeOutOfRange),
    /// [`NodeDeleteWouldEmptySubpath`](crate::vector::VectorEditError::NodeDeleteWouldEmptySubpath),
    /// [`NodeDeleteRectangleCorner`](crate::vector::VectorEditError::NodeDeleteRectangleCorner),
    /// [`NodeDeleteImplicitStart`](crate::vector::VectorEditError::NodeDeleteImplicitStart)
    /// or [`ClippingPath`](crate::vector::VectorEditError::ClippingPath);
    /// `NotAPath` for a text/image target; plus
    /// [`EditError::PageOutOfRange`], [`EditError::VectorEditNoContents`],
    /// [`EditError::VectorEditContent`], [`EditError::DocumentEncrypted`], or
    /// [`EditError::CertificationForbidsChange`]. Every refusal happens before
    /// any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — non-empty when a **curve** was discarded along with
    /// the point, which is a shape change the operator cannot reverse by
    /// re-adding a point. Rule 4 forbids letting them find that out from a
    /// diff, so the caller must surface these.
    pub fn delete_node(
        &mut self,
        page_index: usize,
        object_index: usize,
        node_index: usize,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::DeleteNode, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_delete_node(stream, path, node_index)?)
        })
    }

    pub fn delete_subpath(
        &mut self,
        page_index: usize,
        object_index: usize,
        subpath_index: usize,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::DeleteSubpath, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_delete_subpath(
                stream,
                path,
                subpath_index,
            )?)
        })
    }

    /// **Move one subpath** of the path object at paint-order `object_index`
    /// on page `page_index` by a page-space `(dx, dy)`, as one undoable
    /// command (Pass 28.0).
    ///
    /// The companion to [`Self::delete_subpath`], and the operation the
    /// roadmap has owed since Pass 25.2: on a CAD export where one path object
    /// holds a whole drawing view, moving "this line" means moving one of its
    /// subpaths, not the view.
    ///
    /// `subpath_index` is into the object's subpaths in decomposition order —
    /// the order [`crate::vector::hit_test_subpaths`] returns.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] wrapping `SubpathOutOfRange`, `ImplicitNode`
    /// (a subpath whose start is inherited rather than written cannot be
    /// translated without tearing it), `MalformedOperand` or `DegenerateCtm`;
    /// plus the page/encryption/certification guards.
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless it had to change the *form* of an
    /// operator to express the request (expanding an `re` rectangle whose
    /// corner was dragged out of square, materializing the `m` an
    /// implicitly-started subpath never had). The caller must surface them:
    /// the drawing is unchanged but the bytes are not recoverable by
    /// reversing the gesture, and rule 4 forbids letting the operator find
    /// that out from a diff.
    pub fn move_subpath(
        &mut self,
        page_index: usize,
        object_index: usize,
        subpath_index: usize,
        dx: f64,
        dy: f64,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::MoveSubpath, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_move_subpath(
                stream,
                path,
                subpath_index,
                dx,
                dy,
            )?)
        })
    }

    /// **Drag** the anchor node `node_index` of the path object at paint-order
    /// `object_index` on page `page_index` to the page-space point `to`, as
    /// one undoable command (decision 011 §2.5 operation 3).
    ///
    /// Content-stream surgery via [`crate::vector::plan_move_node`]: exactly
    /// one coordinate pair (the anchor of an `m`/`l`/`c`/`v`/`y` operator) is
    /// rewritten (the target is mapped from page space to the object's user
    /// space by its CTM's affine inverse); adjacent Bézier control points are
    /// left in place (handle editing is a named fast-follow). `node_index` is
    /// into the object's anchors in **decomposition order** (the order
    /// [`crate::vector::PathObject::page_subpaths`]'
    /// [`Subpath::anchors`](crate::vector::Subpath::anchors) flatten to, and
    /// the count [`crate::vector::anchor_count`] reports). Only the one edited
    /// operator's bytes change; every other object stays byte-verbatim. Lands
    /// as one [`CommandKind::MoveNode`]; undo restores the byte-identical
    /// pre-drag stream.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] (out-of-range object/node, non-path target,
    /// an `re`-rectangle corner or an implicit `h`-reopened start that cannot
    /// be node-edited, singular CTM), [`EditError::PageOutOfRange`],
    /// [`EditError::VectorEditNoContents`], [`EditError::VectorEditContent`],
    /// [`EditError::DocumentEncrypted`],
    /// or [`EditError::CertificationForbidsChange`]. Every refusal happens
    /// before any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless it had to change the *form* of an
    /// operator to express the request (expanding an `re` rectangle whose
    /// corner was dragged out of square, materializing the `m` an
    /// implicitly-started subpath never had). The caller must surface them:
    /// the drawing is unchanged but the bytes are not recoverable by
    /// reversing the gesture, and rule 4 forbids letting the operator find
    /// that out from a diff.
    pub fn move_node(
        &mut self,
        page_index: usize,
        object_index: usize,
        node_index: usize,
        to: crate::vector::Point,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::MoveNode, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_move_node(stream, path, node_index, to)?)
        })
    }

    /// **Drag a Bézier handle** of the path object at paint-order
    /// `object_index` on page `page_index`: move one control point of node
    /// `node_index` to the page-space point `to`, leaving the on-curve node
    /// itself exactly where it is (Pass 30.1).
    ///
    /// This is what makes a curve's SHAPE editable. [`Self::move_node`] can
    /// only move points the curve passes THROUGH, so without this the
    /// curvature between two anchors could not be changed at all.
    ///
    /// Content-stream surgery via [`crate::vector::plan_move_handle`]: one
    /// control-point pair is rewritten in place, or — where the segment is a
    /// `v`/`y` whose requested handle is implied by another point
    /// (§8.5.2.1 Table 59) — that segment is re-spelled as the equivalent `c`
    /// so the handle can hold its own value. Either way exactly one operator's
    /// bytes change; every other object stays byte-verbatim. Lands as one
    /// [`CommandKind::MoveHandle`]; undo restores the byte-identical pre-drag
    /// stream.
    ///
    /// # Errors
    ///
    /// [`EditError::VectorEdit`] — including
    /// [`VectorEditError::NoHandleHere`](crate::vector::VectorEditError::NoHandleHere)
    /// when the segment on that side is straight or absent, which is refused
    /// rather than converted (turning a line into a curve is a different
    /// operation and is not inferred from a drag) — plus
    /// [`EditError::PageOutOfRange`], [`EditError::VectorEditNoContents`],
    /// [`EditError::VectorEditContent`], [`EditError::DocumentEncrypted`], or
    /// [`EditError::CertificationForbidsChange`]. Every refusal happens
    /// before any mutation (rule 4).
    ///
    /// # Returns
    ///
    /// The operator-facing [disclosures](crate::vector::PlannedEdit::disclosures)
    /// the surgery owes — **empty** unless a `v`/`y` had to be re-spelled as
    /// `c`. The curve draws identically, but the bytes are not recoverable by
    /// dragging back, and rule 4 forbids letting the operator find that out
    /// from a diff.
    pub fn move_handle(
        &mut self,
        page_index: usize,
        object_index: usize,
        node_index: usize,
        handle: crate::vector::Handle,
        to: crate::vector::Point,
    ) -> Result<Vec<String>, EditError> {
        self.vector_surgery(CommandKind::MoveHandle, page_index, |stream, model| {
            let count = model.objects.len();
            let obj = model.objects.get(object_index).ok_or(
                crate::vector::VectorEditError::ObjectOutOfRange {
                    index: object_index,
                    count,
                },
            )?;
            let path = vector_object_as_path(obj, object_index)?;
            Ok(crate::vector::plan_move_handle(
                stream, path, node_index, handle, to,
            )?)
        })
    }

    /// The shared skeleton of the three 9c-min vector edits: guard, locate
    /// the page's first content stream, decompose the **base** content, let
    /// `plan` produce the rewritten content bytes, then stage them as one
    /// undoable [`text_edit_command`](Self::text_edit_command) command (the
    /// SAME staging + minimal-diff + objstm-promotion path text edit uses, so
    /// the writer handles the §5.7/§5.9 promotion for free and the byte
    /// contract is inherited).
    ///
    /// Decomposing the **session-current** content (this session's staged
    /// stream if the page has already been rewritten, else the base) is what
    /// makes `object_index` line up with the caller that supplied it: the GUI
    /// object provider decomposes `session.view()`, i.e. the edited revision
    /// (decision 018, Pass 17.0), and the CLI's fresh load has no session edits
    /// so the two agree by definition. It is also what makes successive vector
    /// edits ACCUMULATE, through the same [`Self::current_page_content`] helper
    /// [`Self::edit_text`] uses.
    ///
    /// This previously read the base unconditionally and refused once a page
    /// had been touched. See the comment at the read site for why that was
    /// backwards.
    fn vector_surgery(
        &mut self,
        kind: CommandKind,
        page_index: usize,
        plan: impl FnOnce(
            &crate::content::ContentStream,
            &crate::vector::PageObjects,
        ) -> Result<crate::vector::PlannedEdit, EditError>,
    ) -> Result<Vec<String>, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        // An enforced-DocMDP certification forbids editing page content
        // (§12.8.4 Table 258) — the same guard the markup/add-text/dimension
        // authoring paths run, so no operator-facing entry can surgically
        // rewrite a certified page unguarded.
        self.check_certification()?;

        let pages = page_tree::pages(&self.base)?;
        let count = pages.len();
        let page = pages.get(page_index).ok_or(EditError::PageOutOfRange {
            index: page_index,
            count,
        })?;
        let content_id = *page
            .contents
            .first()
            .ok_or(EditError::VectorEditNoContents { page_index })?;
        // Decompose the page's CURRENT content — the session's own staged
        // stream if a prior edit already rewrote it, else the base.
        //
        // This used to read the BASE unconditionally and refuse outright
        // (`VectorEditNeedsReopen`) once the page had been touched, on the
        // reasoning that `object_index` must line up with whoever decomposed
        // the base. The reasoning was sound and the conclusion was backwards:
        // the caller that supplies the index is the GUI's object provider,
        // which decomposes `session.view()` — the EDITED revision (decision
        // 018, Pass 17.0). So base-decomposing here was the thing out of step,
        // and the refusal existed to stop a mismatch this function was itself
        // creating.
        //
        // Reading current content aligns the two and makes vector edits
        // ACCUMULATE, exactly as `edit_text` / `format_text` already do
        // through this same helper. That is not a bonus: on a CAD drawing
        // where one object holds 1194 subpaths, "one vector edit per page per
        // session, then save and reopen" makes deleting stray lines — the
        // operator's actual task — unusable. Reported 2026-08-04: "After
        // clicking and deleting an object I couldn't delete another one after
        // selecting it."
        //
        // Undo is unaffected: each command still records its own before/after
        // for `content_id`, and `text_edit_command`'s `first_edit` gate
        // already distinguishes the first rewrite from later ones.
        let new_content = {
            let stream = self
                .current_page_content(content_id, page)
                .map_err(EditError::VectorEditContent)?;
            // The XObject resolver reads the BASE view deliberately: page
            // `/Resources` are not rewritten by content surgery, so base and
            // session agree on them, and the base view is the one guaranteed
            // to be borrowable here.
            let base_view = self.base.view();
            let resolver = crate::vector::DocumentXObjects {
                view: &base_view,
                resources: &page.resources,
            };
            let model =
                crate::vector::decompose(&stream, crate::vector::Matrix::IDENTITY, &resolver);
            let planned = plan(&stream, &model)?;
            (planned.content, planned.disclosures)
        };
        let (new_content, disclosures) = new_content;

        let command = self.text_edit_command(kind, content_id, page, new_content);
        self.commit(command);
        Ok(disclosures)
    }

    /// The page's CURRENT decoded content, tokenized: the session's own
    /// edited raw stream (read back from the staging buffer) when a prior
    /// text/format edit already rewrote `content_id` this session, else the
    /// base document's decoded page content.
    ///
    /// This is what makes sequential edits ACCUMULATE (Pass 14.3 §0.2 /
    /// UI spec §2.1's post-edit rebuild): a second edit must compose on top
    /// of the first, not re-splice the base. A page's first `/Contents`
    /// object is touched in `state` ONLY by [`Self::edit_text`] /
    /// [`Self::format_text`] (no other `EditSession` operation rewrites a page
    /// content stream), so `state.contains_key(content_id)` reliably means "a
    /// prior text edit this session," and its overlay value is the raw
    /// (unfiltered) stream those methods staged — read directly, no decode.
    fn current_page_content(
        &self,
        content_id: ObjId,
        page: &Page,
    ) -> Result<crate::content::ContentStream, crate::content::ContentError> {
        use crate::content::{ContentError, ContentStream};
        if let Some(Object::Stream(s)) = self.state.get(&content_id) {
            // A prior edit rewrote this content object into a raw stream whose
            // data lives in the staging buffer; walk THAT.
            let span = s.data_span;
            let src = self.authored_source();
            let raw = span
                .slice(src.as_ref())
                .ok_or(ContentError::NotAStream)?
                .to_vec();
            return ContentStream::parse(raw);
        }
        // BASE READ, deliberately (decision 018 caller audit). The session
        // case is the branch above, which reads the RAW staged stream
        // directly — a staged content stream is stored unfiltered, so
        // routing it through `from_page` (which runs `decode_stream`) would
        // double-handle it. This branch is only reached when the session has
        // NOT rewritten this content object, where base and session agree by
        // definition.
        ContentStream::from_page(&self.base.view(), page)
    }

    /// Build the one [`Command`] that replaces `content_id` with the freshly
    /// spliced `new_content` (staged into this session's buffer as a raw
    /// stream) and, on the FIRST edit to this content object, empties any
    /// extra content streams on a multi-stream page.
    ///
    /// Shared by [`Self::edit_text`] and [`Self::format_text`]; the only
    /// difference between a REPLACE and a FORMAT command is the
    /// [`CommandKind`] label. On the first edit the whole edited content lives
    /// in the first object and the extras are emptied so byte offsets stay
    /// coherent (mirrors `write_incremental`); on a subsequent edit the extras
    /// are already emptied, so no redundant no-op write is recorded (kept out
    /// of the undo entry by the `first_edit` gate — the offset in an empty
    /// stream's span would otherwise make an all-empty re-write look like a
    /// change).
    fn text_edit_command(
        &mut self,
        kind: CommandKind,
        content_id: ObjId,
        page: &Page,
        new_content: Vec<u8>,
    ) -> Command {
        use crate::text_edit::edit::make_raw_stream;
        let content_before = self.state.get(&content_id).cloned();
        let first_edit = content_before.is_none();

        let len = new_content.len();
        let span = self.stage_bytes(&new_content);
        let mut objects = vec![ObjectWrite {
            id: content_id,
            before: content_before,
            after: Some(make_raw_stream(span, len)),
        }];

        if first_edit {
            for id in page.contents.iter().skip(1) {
                let empty = self.stage_bytes(&[]);
                objects.push(ObjectWrite {
                    id: *id,
                    before: self.state.get(id).cloned(),
                    after: Some(make_raw_stream(empty, 0)),
                });
            }
        }

        Command {
            kind,
            objects,
            removals: Vec::new(),
            trailer: None,
        }
    }

    // -- internals ------------------------------------------------------

    /// The `/Info` object id from the **working** trailer, so a
    /// just-created dictionary is visible immediately.
    fn info_id(&self) -> Option<ObjId> {
        self.trailer.get(b"Info").and_then(Object::as_reference)
    }

    /// Apply a command, push it on the undo stack, and invalidate redo.
    ///
    /// Clearing the redo stack on a new edit is standard editor
    /// behaviour (§11.1 states it for completeness rather than because
    /// it is subtle): the redone future no longer exists once history
    /// diverges.
    fn commit(&mut self, command: Command) {
        for write in &command.objects {
            Self::write_state(&mut self.state, write.id, write.after.clone());
        }
        for removal in &command.removals {
            Self::write_deleted(&mut self.deleted, removal.id, removal.is_deleted);
        }
        if let Some((_, after)) = &command.trailer {
            self.trailer = after.clone();
        }
        self.redo.clear();
        self.undo.push(command);
        // Bound the history (§11.1). Safe to drop the oldest ONLY
        // because the dirty set is a diff, never a replay — see the
        // module docs.
        if self.undo.len() > MAX_UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    /// Write one overlay slot, where `None` means "remove the entry".
    ///
    /// Removing rather than storing a copy of the base value keeps the
    /// overlay minimal, but note that it is **not** required for
    /// correctness: [`EditSession::dirty_set`] skips any entry equal to
    /// the base regardless. Both halves of that belt-and-braces are
    /// deliberate, because the equality check is the one that has to hold
    /// for the edit → undo → save contract, and it must not depend on
    /// this function having tidied up.
    fn write_state(state: &mut BTreeMap<ObjId, Object>, id: ObjId, value: Option<Object>) {
        match value {
            Some(v) => {
                state.insert(id, v);
            }
            None => {
                state.remove(&id);
            }
        }
    }

    /// Set or clear one object's deleted flag.
    fn write_deleted(deleted: &mut BTreeSet<ObjId>, id: ObjId, is_deleted: bool) {
        if is_deleted {
            deleted.insert(id);
        } else {
            deleted.remove(&id);
        }
    }
}

/// The `/Rotate` key name for an [`EditError::NotADictionary`] on an
/// `/Info` field.
const fn field_key_str(field: InfoField) -> &'static str {
    match field {
        InfoField::Title => "Title",
        InfoField::Author => "Author",
        InfoField::Subject => "Subject",
        InfoField::Keywords => "Keywords",
    }
}

/// Normalize a `/Rotate` value to {0, 90, 180, 270}.
///
/// Table 30 requires a multiple of 90 and gives 0 as the default, but
/// says nothing about range — so `-90` and `450` are both conforming and
/// both mean 270 and 90 respectively. A **positive** modulo is required:
/// Rust's `%` keeps the sign of the dividend, so `-90 % 360` is `-90`,
/// and using it directly would produce a rotation no renderer expects.
///
/// A value that is not a multiple of 90 cannot reach here through
/// [`EditSession::set_page_rotation`] (which refuses it), but can arrive
/// from a malformed file through [`EditSession::pages`]; rounding it down
/// to the enclosing quarter turn matches what
/// [`crate::page_tree`] already does on load.
const fn normalize_rotation(degrees: i64) -> u16 {
    let wrapped = degrees.rem_euclid(360);
    // `rem_euclid` on a positive modulus yields 0..=359, so the cast is
    // exact; the quarter-turn floor handles a malformed non-multiple.
    ((wrapped / 90) * 90) as u16
}

/// Encode operator-entered text as a PDF text string (§7.9.2).
///
/// Two forms, chosen by content:
///
/// - **Pure ASCII** (0x20–0x7E) is emitted as those bytes directly.
///   Those code points are identical in PDFDocEncoding, so the value is
///   unambiguous, and it keeps a title like `Quarterly Report` readable
///   in a hex dump — which matters for a format whose files get
///   diagnosed by eye.
/// - **Anything else** becomes **UTF-16BE with a leading U+FEFF byte
///   order mark**, which §7.9.2 defines as the escape hatch for text
///   outside PDFDocEncoding and which every reader implements.
///
/// ## Why not PDFDocEncoding for the middle ground
///
/// A Latin-1-ish title (`Café`) is representable in PDFDocEncoding, and
/// encoding it that way would be two bytes shorter. pdfce does not,
/// because **the PDFDocEncoding table is a recorded gap in the spec
/// RAG** — Annex D.3 is listed as not yet ingested, and §7.9.2 itself is
/// not ingested at all (only §7.3.4's byte *syntax* is). Guessing the
/// table from memory is exactly the failure mode the spec-fidelity rule
/// exists to prevent, and the cost of not guessing is a handful of bytes
/// on a metadata string. The ASCII subset used above is the part that is
/// certain.
///
/// # Examples
///
/// ```
/// use pdfce_core::edit::encode_text_string;
///
/// assert_eq!(encode_text_string("Report"), b"Report".to_vec());
///
/// // Non-ASCII takes the UTF-16BE + BOM form (§7.9.2).
/// let cafe = encode_text_string("Café");
/// assert_eq!(cafe.get(..2), Some(&[0xFE, 0xFF][..]));
/// assert_eq!(cafe.len(), 2 + 4 * 2);
/// ```
#[must_use]
pub fn encode_text_string(text: &str) -> Vec<u8> {
    if text.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return text.as_bytes().to_vec();
    }
    let mut out = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

/// Decode a PDF text string (§7.9.2) for display, reporting whether the
/// decode was exact.
///
/// - A leading U+FEFF BOM selects UTF-16BE; unpaired surrogates become
///   U+FFFD and mark the result inexact.
/// - Otherwise bytes 0x20–0x7E (plus tab, LF and CR) decode as
///   themselves. **Any other byte becomes U+FFFD and marks the result
///   inexact**, because decoding it correctly needs the PDFDocEncoding
///   table from Annex D.3, which the spec RAG does not yet carry (see
///   [`encode_text_string`]).
///
/// Guessing Latin-1 for those bytes would look right most of the time
/// and be silently wrong for the 0x80–0x9F range, where PDFDocEncoding
/// and Latin-1 disagree — and "silently wrong most of the time" is worse
/// than a visible U+FFFD, because a front end can *see* the `exact` flag
/// and decline to write the field back.
///
/// # Examples
///
/// ```
/// use pdfce_core::edit::decode_text_string;
///
/// let plain = decode_text_string(b"Report");
/// assert_eq!(plain.text, "Report");
/// assert!(plain.exact);
///
/// let utf16 = decode_text_string(&[0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]);
/// assert_eq!(utf16.text, "Hi");
/// assert!(utf16.exact);
///
/// // A byte with no certain mapping is shown, and flagged.
/// let unknown = decode_text_string(&[0x91]);
/// assert!(!unknown.exact);
/// ```
#[must_use]
pub fn decode_text_string(bytes: &[u8]) -> InfoText {
    if let (Some(0xFE), Some(0xFF)) = (bytes.first(), bytes.get(1)) {
        let body = bytes.get(2..).unwrap_or(&[]);
        let units: Vec<u16> = body
            .chunks_exact(2)
            .map(|pair| {
                u16::from_be_bytes([*pair.first().unwrap_or(&0), *pair.get(1).unwrap_or(&0)])
            })
            .collect();
        let text = String::from_utf16_lossy(&units);
        // An odd trailing byte, or a lone surrogate, means the value was
        // not a well-formed UTF-16BE string.
        let exact = body.len() % 2 == 0 && !text.contains('\u{FFFD}');
        return InfoText { text, exact };
    }
    let mut text = String::with_capacity(bytes.len());
    let mut exact = true;
    for &b in bytes {
        if (0x20..=0x7E).contains(&b) || matches!(b, b'\t' | b'\n' | b'\r') {
            text.push(b as char);
        } else {
            text.push('\u{FFFD}');
            exact = false;
        }
    }
    InfoText { text, exact }
}

// ---------------------------------------------------------------------------
// Pass 3.2 — structural page operations
// ---------------------------------------------------------------------------

/// A read view of an [`EditSession`]: the base revision with the
/// operator's unsaved edits applied.
///
/// Exists so that the **one** page-tree walk in pdfce
/// ([`page_tree::pages_in`]) can run over the edited document as easily
/// as over a loaded file. See [`crate::graph`] for why that is a trait
/// rather than a second walk.
#[derive(Debug, Clone, Copy)]
pub struct SessionGraph<'a> {
    session: &'a EditSession,
}

/// The session itself, as a graph.
///
/// [`SessionGraph`] already expressed exactly this, and for the page-tree
/// walk it remains the ergonomic handle (`Copy`, no lifetime juggling).
/// This impl exists because [`DocumentView`] holds a `&'a dyn ObjectGraph`,
/// and a `SessionGraph` constructed inside [`EditSession::view`] would be a
/// **temporary** — it could not outlive the call that built it, so the
/// borrow would not compile. `&self` can, and does.
///
/// Behaviour is identical to [`SessionGraph`]'s by construction:
/// `SessionGraph` now delegates here rather than reimplementing, so the two
/// views of a session can never drift apart the way two hand-written copies
/// of a walk would (the reason [`crate::graph`] is a trait in the first
/// place).
impl ObjectGraph for EditSession {
    fn value(&self, id: ObjId) -> Option<&Object> {
        // Disambiguated: the inherent `EditSession::value` is the real
        // implementation (overlay, then deletions, then base) and this
        // trait method is its projection, not the other way round.
        Self::value(self, id)
    }

    fn trailer_entry(&self, key: &[u8]) -> Option<&Object> {
        // The session's WORKING trailer, not the base's: an operator who
        // created `/Info` this session must see it here (Pass 3.1).
        self.trailer.get(key)
    }
}

impl ObjectGraph for SessionGraph<'_> {
    fn value(&self, id: ObjId) -> Option<&Object> {
        self.session.value(id)
    }

    fn trailer_entry(&self, key: &[u8]) -> Option<&Object> {
        // Delegates to `<EditSession as ObjectGraph>` so there is exactly
        // one definition of "the session's trailer" (see that impl).
        ObjectGraph::trailer_entry(self.session, key)
    }
}

/// A read view of a session **plus pending, uncommitted writes**.
///
/// Used for exactly one thing: computing what is still reachable *after*
/// a structural splice, before that splice is committed. A delete has to
/// know which objects the removed pages owned exclusively, and that is a
/// question about the document as it will be, not as it is.
///
/// Private, because a half-applied document is not a thing any caller
/// outside this module should be able to hold.
struct PendingGraph<'a> {
    session: &'a EditSession,
    scratch: &'a BTreeMap<ObjId, Object>,
    removed: &'a HashSet<ObjId>,
}

impl ObjectGraph for PendingGraph<'_> {
    fn value(&self, id: ObjId) -> Option<&Object> {
        if self.removed.contains(&id) {
            return None;
        }
        self.scratch.get(&id).or_else(|| self.session.value(id))
    }

    fn trailer_entry(&self, key: &[u8]) -> Option<&Object> {
        self.session.trailer.get(key)
    }
}

/// Maximum objects a reachability walk will visit (pdfce policy,
/// `ARCHITECTURE.md` §10). Bounds the delete-time garbage sweep on a
/// hostile object graph.
pub const MAX_REACHABLE_OBJECTS: usize = 5_000_000;

/// What a delete actually did.
///
/// Everything here is something the operator cannot see by looking at the
/// result, which is the test for whether a counter earns its place.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeleteOutcome {
    /// Pages removed.
    pub pages_removed: usize,
    /// Objects that left the document graph entirely and will be written
    /// as §7.5.4 free entries — the removed pages, any page-tree node
    /// left empty by their removal, and everything the removed pages
    /// owned exclusively (their content streams, their annotations).
    pub objects_freed: usize,
    /// What the removal broke elsewhere in the document.
    ///
    /// pdfce **exceeds** Acrobat here on purpose.
    /// `core_ops__delete_pages.md` records that Acrobat *"does not
    /// auto-delete, auto-repoint, or warn by default"* and recommends
    /// pdfce *"surface (don't silently leave) dangling
    /// bookmarks/links/destinations as a reviewable post-delete
    /// report … rather than silently leaving them broken the way Acrobat
    /// does. … Acrobat's native behavior here is a low bar, not a target
    /// to literally copy."*
    ///
    /// pdfce reports and does **not** repair. Repointing a bookmark at
    /// "whatever page now occupies that index" would be pdfce deciding
    /// what the author meant — the fuzzy-never-sneaky rule cuts against
    /// silent repair exactly as hard as it cuts against silent breakage.
    pub dangling: DanglingReport,
    /// What saving this edit will do to the document's signatures.
    pub signature: SignatureImpact,
}

/// What a text/choice form fill actually did (Pass 7).
///
/// The disclosures a fuzzy-never-sneaky fill owes the operator: how many
/// widgets it repainted, and the two variable-text caveats the §12.7.3.3
/// generator surfaces (an applied auto-size, and any characters it could not
/// encode in `WinAnsi`).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct FillOutcome {
    /// The primary field object that was filled.
    pub field_id: ObjId,
    /// How many widget appearances were regenerated (≥1 per filled field,
    /// more when the field is presented in several places).
    pub widgets_updated: usize,
    /// `Some(size)` when the field's `/DA` requested auto-size (`0 Tf`) and a
    /// size was chosen — surfaced for disclosure (VT1), never presented as
    /// spec-mandated.
    pub applied_autosize: Option<f64>,
    /// How many characters of the filled text had no `WinAnsi` code and were
    /// substituted with `?` (the named Base-14-Latin limit, disclosed).
    pub unencodable_chars: usize,
}

/// What a [`regenerate_appearances`](EditSession::regenerate_appearances)
/// operation did (Pass 7.1, R51).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RegenOutcome {
    /// How many fields had a widget appearance regenerated.
    pub regenerated: usize,
    /// Whether `/NeedAppearances` was present and was cleared on output.
    pub need_appearances_cleared: bool,
    /// An applied auto-size, if any (disclosure; VT1).
    pub applied_autosize: Option<f64>,
    /// Characters that had no `WinAnsi` code (disclosure).
    pub unencodable_chars: usize,
}

/// What an [`import_form_data`](EditSession::import_form_data) operation did
/// (Pass 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportOutcome {
    /// How many fields the data file set on the document.
    pub applied: usize,
    /// How many named fields the document did not have (counted + skipped,
    /// never an error — a data file may name a superset).
    pub skipped: usize,
}

/// What a [`flatten_fields`](EditSession::flatten_fields) operation did
/// What a field or widget deletion actually did (decision 020 §3.6.3).
///
/// Returned rather than inferred, because the two facts an operator cannot
/// see afterwards — that a selection was cleared, and that emptied grouping
/// nodes went with it — are exactly the ones that change what the document
/// means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldDeletion {
    /// Widget annotations un-listed from their pages and deleted.
    pub widgets_removed: usize,
    /// The field itself was removed from the form — either because the whole
    /// field was the target, or because its last widget was.
    pub field_removed: bool,
    /// **§3.6.3's disclosure.** The deleted widget held the field's `/V`, so
    /// the value pointed at a state no remaining widget could display. `/V`
    /// was set to `/Off` and every remaining kid's `/AS` with it.
    ///
    /// Silently leaving the dangling `/V` would be sneaky; silently clearing
    /// it would also be. Hence a fact the caller must surface.
    pub selection_cleared: bool,
    /// Grouping nodes that became childless and were pruned with the field.
    ///
    /// Not cosmetic: a named node with nothing under it still occupies its
    /// slot in the §12.7.3.2 FQN space, so leaving one behind would refuse a
    /// later field that wanted the name.
    pub emptied_parents: usize,
}

/// What a [`EditSession::rename_field`] changed.
///
/// # Why a rename is ONE object write, and this struct exists to say what
/// else moved
///
/// §12.7.3.2 builds the fully-qualified name by walking DOWN the tree and
/// appending each node's partial name `/T`. So changing one node's `/T`
/// re-derives the FQN of that node **and of every descendant**, without
/// touching a single descendant object. The write is one dictionary; the
/// consequence is a subtree.
///
/// That is exactly the shape rule 4 exists for. An operator who renames
/// `Address` and is not told that `Address.City` is now `Location.City` has
/// had six fields renamed by a one-field request, and every FDF, every
/// JavaScript reference and every submit mapping that named them is now
/// pointing at nothing. [`descendants_renamed`](Self::descendants_renamed)
/// is that disclosure, and it is why the count is returned rather than
/// discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRename {
    /// The fully-qualified name before the rename.
    pub from: String,
    /// The fully-qualified name after it.
    pub to: String,
    /// Descendant fields whose fully-qualified name changed as a consequence,
    /// **without any of their own objects being written**.
    ///
    /// Zero for a terminal field with no children — the common case, and the
    /// one where the operator's mental model and the effect coincide.
    pub descendants_renamed: usize,
}

/// (Pass 7.1, R48).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FlattenOutcome {
    /// How many fields were flattened (removed from the form).
    pub fields_flattened: usize,
    /// How many widget appearances were burned into page content.
    pub widgets_burned: usize,
    /// How many pages had content appended.
    pub pages_touched: usize,
}

/// Per-page accumulation of a flatten: the `q cm /Name Do Q` invocations to
/// author into the overlay content stream, and the `(name, appearance-stream)`
/// pairs to add to the page's `/Resources` `/XObject`.
#[derive(Debug, Default)]
struct PageFlatten {
    /// `(resource-name, placement-cm)` per burned widget, in page order.
    invocations: Vec<(Vec<u8>, [f64; 6])>,
    /// `(resource-name, appearance-stream-id)` to register in `/XObject`.
    xobjects: Vec<(Vec<u8>, ObjId)>,
}

/// Match a choice selection against a field's `/Opt` options, by export
/// value first then display value (§12.7.4.4). Returns the option index and a
/// reference to it. Comparison is on the §7.9.2-decoded text so a UTF-16
/// option matches a decoded selection string.
fn match_option<'a>(
    options: &'a [forms::ChoiceOption],
    sel: &str,
) -> Option<(usize, &'a forms::ChoiceOption)> {
    options
        .iter()
        .enumerate()
        .find(|(_, o)| decode_text_string(&o.export).text == sel)
        .or_else(|| {
            options
                .iter()
                .enumerate()
                .find(|(_, o)| decode_text_string(&o.display).text == sel)
        })
}

/// The display text a choice field's current `/V` should show: each stored
/// **export** value mapped to its `/Opt` **display** value, joined by newline
/// (§12.7.4.4 — the appearance shows the display, `/V` stores the export).
/// `None` when the field has no value.
fn choice_display_text(field: &Field) -> Option<String> {
    let exports: Vec<Vec<u8>> = match &field.value {
        forms::FieldValue::Choice(items) => items.clone(),
        _ => return None,
    };
    if exports.is_empty() {
        return None;
    }
    let lines: Vec<String> = exports
        .iter()
        .map(|ex| {
            let ex_text = decode_text_string(ex).text;
            field
                .options
                .iter()
                .find(|o| decode_text_string(&o.export).text == ex_text)
                .map_or(ex_text, |o| decode_text_string(&o.display).text)
        })
        .collect();
    Some(lines.join("\n"))
}

/// The §12.5.5 placement matrix **A** that maps an appearance form XObject's
/// `/BBox` (transformed by its `/Matrix`) onto a widget's `/Rect`, emitted as
/// the `cm` before a `/Name Do` (the `Do` procedure itself re-applies
/// `/Matrix`, so this is A alone — never A×Matrix, the double-apply trap).
///
/// Step a: transform the four `/BBox` corners by `/Matrix`, take the upright
/// bounding box. Step b: A scales-and-translates that box onto `/Rect`
/// (anisotropic — aspect ratio is not preserved; normative). A degenerate
/// transformed box (either extent ≈ 0) yields the identity translated to the
/// rect origin, so a sliver appearance still lands rather than dividing by
/// zero.
fn fit_matrix_for(bbox: [f64; 4], matrix: [f64; 6], rect: crate::page_tree::Rect) -> [f64; 6] {
    let [a, b, c, d, e, f] = matrix;
    // Normalise BBox corners (§7.9.5 — BBox may be given in any corner order).
    let (bx0, by0, bx1, by1) = (bbox[0], bbox[1], bbox[2], bbox[3]);
    let corners = [(bx0, by0), (bx1, by0), (bx1, by1), (bx0, by1)];
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for (x, y) in corners {
        let tx = a * x + c * y + e;
        let ty = b * x + d * y + f;
        minx = minx.min(tx);
        miny = miny.min(ty);
        maxx = maxx.max(tx);
        maxy = maxy.max(ty);
    }
    let tw = maxx - minx;
    let th = maxy - miny;
    let (rw, rh) = (rect.width(), rect.height());
    let sx = if tw.abs() > 1e-6 { rw / tw } else { 1.0 };
    let sy = if th.abs() > 1e-6 { rh / th } else { 1.0 };
    let tx = rect.llx - sx * minx;
    let ty = rect.lly - sy * miny;
    [sx, 0.0, 0.0, sy, tx, ty]
}

/// Read a four-number rectangle array (each element possibly indirect) as
/// `[x0, y0, x1, y1]` (not normalised — the caller's fit handles corner
/// order). `None` when it is not four numbers.
fn read_rect_array<G: ObjectGraph + ?Sized>(graph: &G, obj: &Object) -> Option<[f64; 4]> {
    let arr = graph.resolve(obj).as_array()?;
    let nums: Vec<f64> = arr
        .iter()
        .filter_map(|o| graph.resolve(o).as_number())
        .collect();
    match nums.as_slice() {
        &[a, b, c, d] => Some([a, b, c, d]),
        _ => None,
    }
}

/// Map a `/BaseFont` name to the standard-14 face it denotes, for resolving
/// a field `/DA` font against the AcroForm `/DR` (§12.7.3.3).
///
/// Handles the canonical §9.6.2.2 spellings and the common producer
/// shorthands (`Helv`, `HeBo`, `Cour`, `TiRo`, `Symb`, `ZaDb`) Acrobat's
/// default `/DR` uses. A subset-prefixed name (`ABCDEF+Helvetica`) is
/// matched on the suffix. `None` for anything not a standard-14 face — the
/// caller then falls back to Helvetica (the Base-14 generator cannot lay out
/// an embedded/CID font).
fn basefont_to_std14(name: &[u8]) -> Option<Std14> {
    // Strip a subset prefix `ABCDEF+`.
    let bare = match name.iter().position(|&b| b == b'+') {
        Some(i) if i == 6 => name.get(i + 1..).unwrap_or(name),
        _ => name,
    };
    Some(match bare {
        b"Helvetica" | b"Helv" | b"Arial" | b"ArialMT" => Std14::Helvetica,
        b"Helvetica-Bold" | b"HeBo" | b"Arial-Bold" | b"Arial-BoldMT" => Std14::HelveticaBold,
        b"Helvetica-Oblique" | b"Arial-Italic" | b"Arial-ItalicMT" => Std14::HelveticaOblique,
        b"Helvetica-BoldOblique" | b"Arial-BoldItalic" => Std14::HelveticaBoldOblique,
        b"Times-Roman" | b"TiRo" | b"TimesNewRoman" | b"TimesNewRomanPSMT" => Std14::TimesRoman,
        b"Times-Bold" | b"TimesNewRomanPS-BoldMT" => Std14::TimesBold,
        b"Times-Italic" | b"TimesNewRomanPS-ItalicMT" => Std14::TimesItalic,
        b"Times-BoldItalic" | b"TimesNewRomanPS-BoldItalicMT" => Std14::TimesBoldItalic,
        b"Courier" | b"Cour" | b"CourierNew" | b"CourierNewPSMT" => Std14::Courier,
        b"Courier-Bold" => Std14::CourierBold,
        b"Courier-Oblique" => Std14::CourierOblique,
        b"Courier-BoldOblique" => Std14::CourierBoldOblique,
        b"Symbol" | b"Symb" => Std14::Symbol,
        b"ZapfDingbats" | b"ZaDb" => Std14::ZapfDingbats,
        _ => return None,
    })
}

impl EditSession {
    /// Refuse a structural change that an **enforced** certification
    /// signature forbids (§12.8.4 Table 258).
    ///
    /// The distinction this enforces, from `iso32000__s__12.8.md`'s
    /// VALIDATION MODEL: a DocMDP transform in a signature's
    /// `/Reference` is **detection** — pdfce performs the edit and
    /// reports the invalidation. The catalog's `/Perms → /DocMDP` entry
    /// upgrades it to **prevention**: *"consumer applications shall
    /// enforce the permissions specified by the `P` attribute"*. For an
    /// editor, enforcing means declining, and Table 254's permitted lists
    /// are closed at every `P` value with nothing pdfce can do inside
    /// them.
    ///
    /// ## What is deliberately **not** gated here, and why
    ///
    /// Document `/Info` metadata edits (Pass 3.1). Table 254's `P=1`
    /// forbids *any* change, so a strict reading gates those too — but
    /// that would silently narrow a contract Pass 3.1 already shipped,
    /// on a reading rather than on a measurement, and the RAG's
    /// permitted-change vocabulary is phrased in document-content terms
    /// that `/Info` sits awkwardly against. Recorded as an owed decision
    /// rather than made in passing.
    fn check_certification(&self) -> Result<SignatureCensus, EditError> {
        let found = census(&self.graph());
        if found.forbids_structural_change() {
            return Err(EditError::CertificationForbidsChange {
                permission: found.certification_permission.unwrap_or(2),
            });
        }
        Ok(found)
    }

    /// What saving right now would do to the document's signatures.
    ///
    /// `mode` is required because the answer genuinely differs by save
    /// path — see [`crate::signature::impact_of`]'s table. A front end
    /// asks this **immediately before Save**, not at edit time: per
    /// §11.1 the dirty set is a diff computed at save time, so "does this
    /// save change structure?" is not knowable when the edit is made.
    ///
    /// ⚠️ [`SignatureImpact::ByteRangePreserved`] must never be rendered
    /// on its own as "the signature is still valid" — read
    /// [`crate::signature`]'s module docs before writing that string.
    #[must_use]
    pub fn signature_impact_of_save(&self, mode: SaveMode) -> SignatureImpact {
        impact_of(&census(&self.graph()), mode, self.changes_structure())
    }

    /// A census of the signatures the open document carries.
    #[must_use]
    pub fn signature_census(&self) -> SignatureCensus {
        census(&self.graph())
    }

    /// Whether the unsaved edits include a **structural** change — a page
    /// added, removed, or moved.
    ///
    /// Computed from the page tree rather than from the command history,
    /// for the same reason [`EditSession::dirty_set`] is: an edit that
    /// has been undone is not a change, and asking history would say it
    /// was.
    #[must_use]
    pub fn changes_structure(&self) -> bool {
        if !self.deleted.is_empty() {
            return true;
        }
        let (Ok(before), Ok(after)) = (page_tree::page_slots(&self.base), self.page_slots()) else {
            // A tree that cannot be walked on one side is a change worth
            // reporting conservatively; the alternative is claiming
            // "nothing structural happened" about a document pdfce
            // cannot describe.
            return true;
        };
        before.len() != after.len()
            || before
                .iter()
                .zip(after.iter())
                .any(|(a, b)| a.id != b.id || a.parent != b.parent)
    }

    /// **Author a new text form field** onto a page — the field dictionary,
    /// its widget annotation, its baked `/AP`, the page's `/Annots` entry and
    /// the `/AcroForm` `/Fields` registration, as ONE undoable command.
    ///
    /// Returns the created field/widget object id.
    ///
    /// # A field is three things, and all three must land together
    ///
    /// §12.7.2 requires the field in `/AcroForm` `/Fields`; §12.5.6.19
    /// requires a widget annotation in the page's `/Annots` for it to appear
    /// on a page at all. A field registered but not annotated is invisible; a
    /// widget annotated but not registered is not a form field. Both halves
    /// plus the appearance go in one `Command`, so an undo cannot leave the
    /// document in either broken state.
    ///
    /// # MERGED field+widget (§12.5.6.19), not two objects
    ///
    /// *"when a field has only a single associated widget annotation, the
    /// contents of the field dictionary and the annotation dictionary may be
    /// merged into a single dictionary."* A field created here has exactly
    /// one widget, so the merged form is the correct and simpler shape — and
    /// it is the shape `forms.rs` already calls Shape A and reports as
    /// `Widget::merged`.
    ///
    /// # It authors an `/AP`, and does NOT set `/NeedAppearances`
    ///
    /// This is forced by pdfce's own standing rules rather than chosen.
    /// **R43**: a widget with `/MK` but no `/AP` is the canonical
    /// named-not-painted case, and pdfce does not build the dynamic
    /// appearance at display time. **R51**: `/NeedAppearances` is *reported,
    /// not auto-generated*. So a field created without an `/AP` would be
    /// invisible in pdfce's own renderer — and setting `/NeedAppearances`
    /// would be asking every other viewer to do work pdfce has told itself
    /// not to do.
    ///
    /// The appearance is built by [`crate::vartext::build_variable_text`] —
    /// the SAME path a fill uses (R92's one-regenerator discipline), so a
    /// created field and a filled one cannot disagree about how a value is
    /// drawn.
    ///
    /// # Errors
    ///
    /// [`EditError::FieldNameEmpty`] for a nameless field;
    /// [`EditError::FieldRectDegenerate`] for a zero-area rectangle;
    /// [`EditError::FieldAuthoringRefusedXfa`] on a hybrid XFA document
    /// (decision 020 — see that variant for why a one-sided add is worse
    /// than none); [`crate::forms_author::FormAuthorError::FieldTypeCollision`] when the name is
    /// already used by a field of a different type; plus the encryption,
    /// **strict** certification and `/Size`-suppression guards.
    ///
    /// # Why the STRICT certification gate, not the fill gate
    ///
    /// [`Self::fill_refusal`] uses the `/P`-aware gate, which permits filling
    /// a certified document at `/P >= 2` — because filling is what such a
    /// document is often certified TO allow. Creating a field is a
    /// **structural** change to the form itself, which is precisely what a
    /// certification signature exists to freeze. So this takes
    /// `check_certification`, the same gate `add_markup` and `flatten_fields`
    /// take.
    pub fn add_text_field(&mut self, spec: &NewTextField) -> Result<FieldAuthorOutcome, EditError> {
        let (w, h) = (spec.rect.urx - spec.rect.llx, spec.rect.ury - spec.rect.lly);
        let (page_id, slots, path, disclosures) = self.field_authoring_preflight(
            &spec.name,
            spec.rect,
            spec.page_index,
            forms::FieldType::Text,
            None,
            &spec.tooltip,
        )?;

        // The `/DA` Acrobat's floor specifies: Helvetica, size 0 (auto), black.
        let da = crate::vartext::default_appearance_string(
            b"Helv",
            0.0,
            crate::vartext::TextColor::Gray(0.0),
        );
        let resources = [crate::vartext::FontResource {
            name: b"Helv".to_vec(),
            font: crate::fontdata::Std14::Helvetica,
        }];
        // THE SAME builder a fill uses (R92: one regenerator, never two).
        // A created field and a filled one therefore cannot disagree about
        // how a value is drawn — which they could if this hand-assembled its
        // own appearance dict.
        let appearance = annot_author::build_field_text_appearance(
            w,
            h,
            &spec.value,
            &da,
            crate::vartext::Quadding::Left,
            spec.multiline,
            &resources,
        )?;

        let ap_id = ObjId::new(self.alloc_number()?, 0);

        let ap_span = self.stage_bytes(&appearance.content);
        let mut ap_dict = appearance.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(appearance.content.len()).unwrap_or(i64::MAX)),
        );
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        // THE MERGE BRANCH. An existing same-type terminal gets ANOTHER
        // WIDGET, not another field: §12.7.3.2 makes two widgets sharing a
        // name two views of ONE field, sharing one `/V`. This is how a
        // reference number repeats in a header and how a check box appears on
        // every page — and it is a capability, not a tolerated degeneracy.
        if let FieldPath::Terminal { id, shape, .. } = path {
            let mut w = Self::widget_base_dict(&spec.name, spec.rect, page_id, &spec.tooltip);
            // The widget carries the LOOK; the field keeps the name and the
            // value. `widget_base_dict` writes a `/T` because a merged
            // (Shape A) field needs one — a widget kid must not have one
            // (R101), and `merge_widget_into_field` strips it.
            let mut ap = Dict::new();
            ap.insert(Name::from(b"N"), Object::Reference(ap_id));
            w.insert(Name::from(b"AP"), Object::Dict(ap));
            let mut mk = Dict::new();
            mk.insert(
                Name::from(b"BC"),
                Object::Array(vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                ]),
            );
            w.insert(Name::from(b"MK"), Object::Dict(mk));

            let mut objects = vec![ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(ap_stream),
            }];
            let (merge_writes, _widget_id) =
                self.merge_widget_into_field(id, shape, w, page_id, &slots)?;
            objects.extend(merge_writes);
            self.commit(Command {
                kind: CommandKind::AddFormField,
                objects,
                removals: Vec::new(),
                trailer: None,
            });
            return Ok(FieldAuthorOutcome {
                field_id: id,
                merged: true,
                disclosures,
            });
        }

        // THE CREATE BRANCH. Any intermediate grouping nodes a dotted path
        // needs are created first, so the terminal can hang from the right
        // parent and carry only its OWN partial name.
        let FieldPath::Vacant { deepest, remaining } = path else {
            // Unreachable: the preflight refuses `Grouping` and a mismatched
            // `Terminal`, and the matching `Terminal` returned above. Stated
            // as a refusal rather than a panic — this crate is panic-free.
            return Err(FormAuthorError::NameIsGroupingNode {
                fqn: spec.name.clone(),
            }
            .into());
        };
        let field_id = ObjId::new(self.alloc_number()?, 0);
        let (parent_writes, parent, partial) =
            self.place_new_field(deepest, &remaining, field_id)?;

        // The MERGED field + widget dictionary (§12.5.6.19).
        let mut d = Dict::new();
        d.insert(Name::from(b"Type"), Object::Name(Name::from(b"Annot")));
        d.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Widget")));
        d.insert(Name::from(b"FT"), Object::Name(Name::from(b"Tx")));
        // The terminal's OWN partial name — the last path segment, never
        // the dotted string. §12.7.3.2 composes the FQN from the ancestors'
        // `/T`s, so writing `Personal.Address.Zip` as a `/T` under `Address`
        // would yield the FQN `Personal.Address.Personal.Address.Zip`.
        d.insert(
            Name::from(b"T"),
            Object::String(encode_text_string(&partial)),
        );
        if let Some(p) = parent {
            d.insert(Name::from(b"Parent"), Object::Reference(p));
        }
        d.insert(
            Name::from(b"Rect"),
            Object::Array(vec![
                Object::Real(spec.rect.llx),
                Object::Real(spec.rect.lly),
                Object::Real(spec.rect.urx),
                Object::Real(spec.rect.ury),
            ]),
        );
        d.insert(Name::from(b"P"), Object::Reference(page_id));
        d.insert(Name::from(b"DA"), Object::String(da.clone()));
        d.insert(Name::from(b"Ff"), Object::Integer(spec.field_flags()));
        // §12.5.3 Table 165 bit 3 (Print). Without it the field is on screen
        // and absent from paper, which is not what an operator placing a form
        // field means — and is a difference they would not see until printing.
        d.insert(Name::from(b"F"), Object::Integer(4));
        d.insert(
            Name::from(b"V"),
            Object::String(encode_text_string(&spec.value)),
        );
        if let Some(max) = spec.max_len {
            d.insert(Name::from(b"MaxLen"), Object::Integer(max));
        }
        // See `widget_base_dict`: declined writes nothing, and is disclosed.
        if let Some(tu) = spec.tooltip.text() {
            d.insert(Name::from(b"TU"), Object::String(encode_text_string(tu)));
        }
        // `/MK` with a black border colour and no fill — Acrobat's documented
        // creation floor.
        //
        // HONEST LIMIT, verified by rendering: **pdfce does not paint this.**
        // R43 makes `/MK`-without-`/AP` the canonical named-not-painted case
        // and pdfce declines to build the dynamic appearance at display time,
        // so the border is present in the FILE for viewers that honour `/MK`
        // and is invisible in pdfce's own renderer. A created field therefore
        // shows its value but no box around it here.
        //
        // Writing it anyway is still right: the alternative is a file that is
        // less complete than Acrobat's for no gain. Painting the border into
        // the `/AP` instead would mean either changing the SHARED appearance
        // builder — which fill also uses, so every refilled field in every
        // document would gain a border it never had — or building a second
        // appearance generator, which is exactly what R92 forbids. Neither is
        // a slice-1 trade.
        let mut mk = Dict::new();
        mk.insert(
            Name::from(b"BC"),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        d.insert(Name::from(b"MK"), Object::Dict(mk));
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        d.insert(Name::from(b"AP"), Object::Dict(ap));

        let mut objects = vec![
            ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(ap_stream),
            },
            ObjectWrite {
                id: field_id,
                before: None,
                after: Some(Object::Dict(d)),
            },
        ];
        objects.extend(parent_writes);
        objects.extend(self.annots_writes(page_id, field_id, &slots)?);

        self.commit(Command {
            kind: CommandKind::AddFormField,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FieldAuthorOutcome {
            field_id,
            merged: false,
            disclosures,
        })
    }

    /// The guard sequence EVERY field-authoring verb runs before it writes
    /// anything, returning the resolved page object and the page slots.
    ///
    /// # Why this is shared rather than copied per field type
    ///
    /// Eight refusals in a fixed order, and the order is load-bearing: the
    /// cheap structural checks come first so a malformed request never
    /// reaches the parser, and `check_certification` comes before anything
    /// that inspects the form so a certified document is refused for being
    /// certified rather than for whatever the form happens to contain.
    ///
    /// Three copies of that sequence — one per field type — is how the third
    /// one ends up missing a guard that the first two have. There would be no
    /// test failure: the field would be created, would parse, and would be
    /// refused by nothing. Keeping it in one place makes "check boxes forgot
    /// the encryption guard" unrepresentable rather than merely unlikely.
    ///
    /// `want` is the type being created. **Any** existing field with the
    /// requested fully-qualified name is refused — see below.
    ///
    /// # Same-name is REFUSED, and the resolver is owed
    ///
    /// §12.7.3.2 makes two widgets sharing a `/T` two views of ONE field, and
    /// that is a real feature — it is how a check box appears on every page
    /// of a multi-page form, and it is the mechanism a radio group is built
    /// from. pdfce cannot do it: performing the merge needs a write-side
    /// resolver that answers *what does this name currently name?* and
    /// promotes a merged single-widget field into a field-with-`/Kids`.
    ///
    /// Without that resolver the only alternative to refusing is appending a
    /// second top-level field with the same name, which emits a document
    /// whose two fields have the same identity and no disambiguator. That is
    /// not a deferred refactor — it is corrupt output, and it is unrecoverable
    /// in a way a refusal is not: a missing capability can be added later and
    /// the operator loses nothing, whereas a duplicate-named pair cannot be
    /// resolved after the fact because nothing records which one was meant.
    ///
    /// So this refuses, by name, and the merge is owed. Both refusals live
    /// here rather than in the three callers for the reason given above.
    ///
    /// # Errors
    ///
    /// In evaluation order: [`EditError::FieldNameEmpty`],
    /// [`EditError::FieldRectDegenerate`], [`EditError::DocumentEncrypted`],
    /// a certification refusal, [`EditError::FieldAuthoringRefusedXfa`],
    /// a [`crate::forms_author::FormAuthorError`] collision refusal,
    /// [`EditError::PageOutOfRange`],
    /// and [`EditError::ObjectCreationWouldExposeHiddenObjects`].
    fn field_authoring_preflight(
        &mut self,
        name: &str,
        rect: page_tree::Rect,
        page_index: usize,
        want: forms::FieldType,
        want_button: Option<forms::ButtonKind>,
        tooltip: &TooltipChoice,
    ) -> Result<
        (
            ObjId,
            Vec<PageSlot>,
            forms_author::FieldPath,
            FieldAuthorDisclosures,
        ),
        EditError,
    > {
        if name.trim().is_empty() {
            return Err(EditError::FieldNameEmpty);
        }
        // R105, and FIRST among the content checks: an undecided
        // accessibility name is refused before pdfce spends any effort on a
        // field it is not going to create.
        if *tooltip == TooltipChoice::Undecided {
            return Err(EditError::TooltipDecisionRequired {
                name: name.to_owned(),
            });
        }
        let (w, h) = (rect.urx - rect.llx, rect.ury - rect.lly);
        if w <= 0.0 || h <= 0.0 {
            return Err(EditError::FieldRectDegenerate { w, h });
        }
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        // XFA and same-name checks both read the CURRENT form, so they see
        // fields added earlier in this session, not just those in the file.
        let form = forms::parse_acroform(&self.graph());
        if let Some(f) = &form
            && f.xfa.is_present()
        {
            return Err(EditError::FieldAuthoringRefusedXfa {
                name: "/AcroForm /XFA".to_owned(),
            });
        }
        // THE RESOLVER (R100). Every authoring write learns what its name
        // currently denotes here and nowhere else, so the collision branch
        // exists once and cannot drift between the verbs that use it.
        let path = forms_author::resolve_field_path(&self.graph(), name)?;
        match &path {
            // A grouping node cannot become a field: Table 220 gives a
            // non-terminal no type of its own, and it is the container the
            // fields beneath it hang from.
            forms_author::FieldPath::Grouping { .. } => {
                return Err(forms_author::FormAuthorError::NameIsGroupingNode {
                    fqn: name.to_owned(),
                }
                .into());
            }
            // A terminal of a DIFFERENT type. Not merged, because
            // §12.7.3.2 makes same-FQN nodes representations of ONE field,
            // and one field has one type — one `/V` cannot simultaneously be
            // a text string and a button on-state.
            forms_author::FieldPath::Terminal { ft, kind, .. }
                if !Self::types_merge(*ft, *kind, want, want_button) =>
            {
                return Err(forms_author::FormAuthorError::FieldTypeCollision {
                    fqn: name.to_owned(),
                    existing: Self::type_token(*ft, *kind),
                    requested: Self::type_token(Some(want), want_button),
                }
                .into());
            }
            // Vacant (create) or a same-type terminal (merge). Both proceed.
            _ => {}
        }

        let slots = self.page_slots()?;
        let page_id = slots
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count: slots.len(),
            })?
            .id;
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        let disclosures = FieldAuthorDisclosures {
            tooltip_declined: *tooltip == TooltipChoice::Declined,
            tagged_document: self.document_is_tagged(),
            structure_tab_order: self.page_uses_structure_tab_order(page_id),
            has_no_options: false,
            group_flags_ignored: false,
            // Set by the CALLER, not the preflight: `--defaults-from` is
            // applied to the spec before the spec reaches this point, so the
            // preflight cannot see whether a template contributed anything.
            defaults_type_mismatch: false,
            defaults_on_state_ambiguous: false,
        };
        Ok((page_id, slots, path, disclosures))
    }

    /// Whether the document carries `/StructTreeRoot` (§14.7, Tagged PDF).
    ///
    /// Field creation discloses this because pdfce has no structure-tree
    /// writer, so a field it adds to a tagged document is **not** in that
    /// document's tag tree. Decision 020 §3.5.3 ships the disclosure rather
    /// than a partial writer: a half-written tag tree claims a completeness
    /// the document does not have, which is worse than an honestly absent
    /// one. The same R73 posture redaction already takes.
    fn document_is_tagged(&self) -> bool {
        self.graph()
            .catalog_dict()
            .is_some_and(|c| c.contains_key(b"StructTreeRoot"))
    }

    /// Whether a page declares `/Tabs /S` — structure tab order (Table 30).
    ///
    /// # Why this is worth its own check
    ///
    /// §14.7 derives structure tab order from the TAG TREE. A pdfce-authored
    /// field is untagged, so on such a page it has no tab position **at
    /// all** — not "last", *undefined* — and viewers are free to differ.
    /// That is a functional defect in the form rather than only an
    /// accessibility gap, so it is disclosed separately from
    /// [`Self::document_is_tagged`].
    ///
    /// `/Tabs` is inheritable through the page tree (Table 30), so an
    /// absent entry on the page itself is not the answer — the ancestors
    /// are walked, bounded by the page tree's own depth guard.
    fn page_uses_structure_tab_order(&self, page_id: ObjId) -> bool {
        let graph = self.graph();
        let mut current = Some(page_id);
        for _ in 0..page_tree::MAX_TREE_DEPTH {
            let Some(id) = current else { return false };
            let Some(d) = graph.resolved(id).as_dict() else {
                return false;
            };
            if let Some(tabs) = d
                .get(b"Tabs")
                .map(|o| graph.resolve(o))
                .and_then(Object::as_name)
            {
                return tabs.as_bytes() == b"S";
            }
            current = d.get(b"Parent").and_then(Object::as_reference);
        }
        false
    }

    /// Whether an existing field's type accepts a merge from a requested one.
    ///
    /// `/FT` alone does not decide it for buttons: a check box and a radio
    /// group are both `/FT /Btn`, and merging one into the other would give a
    /// single field widgets that disagree about what they are — a check box
    /// toggles independently, a radio member is mutually exclusive with its
    /// siblings, and one `/V` cannot mean both.
    ///
    /// A field with NO resolvable `/FT` never accepts a merge. That is
    /// malformed input, and attaching a widget to it would make pdfce a
    /// second author of a field it cannot classify.
    fn types_merge(
        existing_ft: Option<forms::FieldType>,
        existing_kind: Option<forms::ButtonKind>,
        want: forms::FieldType,
        want_kind: Option<forms::ButtonKind>,
    ) -> bool {
        if existing_ft != Some(want) {
            return false;
        }
        if want == forms::FieldType::Button {
            return existing_kind == want_kind;
        }
        true
    }

    /// A short operator-facing token for a field type, distinguishing the
    /// three button kinds — because "a button field already exists" would not
    /// tell an operator why their radio button collided with a check box.
    fn type_token(ft: Option<forms::FieldType>, kind: Option<forms::ButtonKind>) -> &'static str {
        match ft {
            Some(forms::FieldType::Button) => match kind {
                Some(forms::ButtonKind::Radio) => "radio button",
                Some(forms::ButtonKind::Push) => "push button",
                _ => "check box",
            },
            Some(forms::FieldType::Choice) => "choice",
            Some(forms::FieldType::Signature) => "signature",
            Some(forms::FieldType::Text) => "text",
            None => "untyped",
        }
    }

    /// Attach a widget to an EXISTING field, promoting Shape A to Shape B if
    /// it is still merged (decision 020 §3.1.5 — the load-bearing primitive).
    ///
    /// # What a merge is, and why it is not an append
    ///
    /// §12.7.3.2 makes two widgets sharing a fully-qualified name **two views
    /// of one field**. That is not a degenerate case to be tolerated — it is
    /// how a check box appears on every page of a form, how a reference
    /// number repeats in a header, and how a radio group is built. The whole
    /// group shares one `/V`, so setting it changes every view at once.
    ///
    /// §12.5.6.19 lets a field and its SOLE widget live in one dictionary
    /// (Shape A, "merged"), and Table 220 permits that **only** while there
    /// is exactly one widget. So attaching a second is a **split**:
    ///
    /// 1. Allocate a widget object and MOVE the annotation keys onto it
    ///    ([`crate::forms_author::WIDGET_KEYS_TO_MOVE`]). Field keys — `/FT`,
    ///    `/T`, `/Ff`, `/V`, `/DV`, `/AA`, `/Opt`, `/MaxLen`, `/Q` — stay,
    ///    because the field is what owns a name and a value.
    /// 2. Remove those keys from the field dictionary.
    /// 3. Write `/Kids [widget1 widget2]` on the field, `/Parent` on each.
    /// 4. **Retarget the page's `/Annots`.** This is the step that is easy to
    ///    miss and expensive to miss: the existing `/Annots` entry references
    ///    the merged dict, which after step 2 is no longer an annotation. Left
    ///    alone, the page points at a dictionary with no `/Subtype /Widget` —
    ///    and `dict_is_widget`'s defensive "or it has `/Rect` or `/AP`"
    ///    fallback would PARTIALLY mask it, giving a document that half-works
    ///    in pdfce and misbehaves elsewhere.
    /// 5. Add the new widget to its page's `/Annots`.
    ///
    /// A field already in Shape B skips steps 1-4: it has `/Kids` already, so
    /// this is a genuine append.
    ///
    /// # Never collapsed back (R102)
    ///
    /// Nothing here ever turns Shape B back into Shape A. `ARCHITECTURE.md`
    /// §5.6 — "never normalize" — forbids rewriting two objects for a purely
    /// cosmetic tidy-up, and a 2->1 deletion leaving Shape B intact is
    /// correct, not a leftover.
    ///
    /// Returns the writes plus the new widget's id.
    fn merge_widget_into_field(
        &mut self,
        field_id: ObjId,
        shape: FieldShape,
        widget_extra: Dict,
        page_id: ObjId,
        slots: &[PageSlot],
    ) -> Result<(Vec<ObjectWrite>, ObjId), EditError> {
        let mut writes = Vec::new();
        let new_widget_id = ObjId::new(self.alloc_number()?, 0);
        // Whether the promotion's page write already carried the new widget's
        // `/Annots` entry, so it is not appended a second time below.
        let mut appended_here = false;

        let Some(Object::Dict(field_dict)) = self.value(field_id) else {
            return Err(EditError::NotADictionary {
                id: field_id,
                key: "Kids",
            });
        };
        let field_dict = field_dict.clone();
        let mut updated_field = field_dict.clone();

        // Build the new widget: the caller's keys (rect, appearance, /MK, /P)
        // plus the wiring that makes it a kid of this field.
        let mut new_widget = widget_extra;
        new_widget.insert(Name::from(b"Type"), Object::Name(Name::from(b"Annot")));
        new_widget.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Widget")));
        new_widget.insert(Name::from(b"Parent"), Object::Reference(field_id));
        // STRIP EVERY FIELD KEY. The callers hand this path a dictionary they
        // built to be a MERGED field+widget (§12.5.6.19); it is about to be
        // only the widget half, and the field half already exists.
        //
        // `/T`, `/FT` and `/Kids` are the load-bearing three (R101): the
        // reader classifies a `/Kids` entry as a child FIELD when it carries
        // any of them, so a widget written with a `/T` would not be a second
        // view of this field — it would be a second field underneath it,
        // silently, with the FQN `Ref.Ref`.
        //
        // The rest matter to the operator rather than to the parser. `/Opt`
        // is the one that surfaced the list: a choice field's options belong
        // to the FIELD, and a copy on each widget means two `add-choice-field`
        // calls under one name leave two disagreeing option lists in one
        // document with no rule for which wins.
        for key in forms_author::FIELD_ONLY_KEYS {
            new_widget.remove(key);
        }

        let mut kids: Vec<Object> = match shape {
            FieldShape::MergedSingleWidget => {
                // THE PROMOTION. Move the annotation keys off the field.
                let promoted_id = ObjId::new(self.alloc_number()?, 0);
                let mut promoted = Dict::new();
                promoted.insert(Name::from(b"Type"), Object::Name(Name::from(b"Annot")));
                promoted.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Widget")));
                promoted.insert(Name::from(b"Parent"), Object::Reference(field_id));
                for key in forms_author::WIDGET_KEYS_TO_MOVE {
                    if let Some(v) = field_dict.get(key) {
                        promoted.insert(Name::from(*key), v.clone());
                    }
                    updated_field.remove(key);
                }
                // `/Type /Annot` goes too. It is not in the move list because
                // the new widget gets a FRESH one written rather than a moved
                // one (so a document that omitted it does not have the
                // omission propagated) — but leaving it on the field dict
                // would label a dictionary that is no longer an annotation as
                // one, which is exactly the confusion `dict_is_widget`'s
                // defensive fallback then has to guess its way out of.
                updated_field.remove(b"Type");
                writes.push(ObjectWrite {
                    id: promoted_id,
                    before: None,
                    after: Some(Object::Dict(promoted)),
                });

                // STEP 4 — retarget every page that referenced the field
                // dict as an annotation. `/P` names the page it was on; when
                // the dict had no `/P`, every page is checked, because an
                // `/Annots` entry with no back-reference is still an entry.
                let old_page = field_dict.get(b"P").and_then(Object::as_reference);
                let candidates: Vec<ObjId> = match old_page {
                    Some(p) => vec![p],
                    None => slots.iter().map(|sl| sl.id).collect(),
                };
                for page in candidates {
                    // THE RETARGET AND THE APPEND ARE ONE WRITE when they
                    // land on the same page — and they usually do, because
                    // the common merge puts the second widget on the page the
                    // first is already on.
                    //
                    // Two separate whole-dictionary writes to one page in one
                    // command do not compose: each is computed from the
                    // pre-command state, so applying both leaves only the
                    // last. That is not hypothetical — it is precisely the
                    // defect the R85 preview-equals-saved oracle caught in
                    // `flatten_fields`, where three page writes in one command
                    // meant every flattened form lost its visible values. Here
                    // it showed as `/Annots [<field> <new widget>]`: the
                    // append had silently replaced the retarget, so the page
                    // still pointed at a dictionary that was no longer an
                    // annotation.
                    let also_append = (page == page_id).then_some(new_widget_id);
                    writes.extend(self.retarget_annot(page, field_id, promoted_id, also_append)?);
                    if also_append.is_some() {
                        appended_here = true;
                    }
                }
                vec![Object::Reference(promoted_id)]
            }
            FieldShape::KidsWidgets { .. } => field_dict
                .get(b"Kids")
                .map(|o| self.graph().resolve(o).clone())
                .and_then(|o| o.as_array().map(<[Object]>::to_vec))
                .unwrap_or_default(),
        };

        kids.push(Object::Reference(new_widget_id));
        updated_field.insert(Name::from(b"Kids"), Object::Array(kids));

        writes.push(ObjectWrite {
            id: new_widget_id,
            before: None,
            after: Some(Object::Dict(new_widget)),
        });
        writes.push(ObjectWrite {
            id: field_id,
            before: self.state.get(&field_id).cloned(),
            after: Some(Object::Dict(updated_field)),
        });
        if !appended_here {
            writes.extend(self.annots_writes(page_id, new_widget_id, slots)?);
        }
        Ok((writes, new_widget_id))
    }

    /// Replace one `/Annots` entry with another and — optionally, in the
    /// SAME write — append a further one.
    ///
    /// Used by the Shape A→B promotion to point the page at the widget that
    /// took over the field dictionary's annotation role.
    ///
    /// # Two reasons the shape is what it is
    ///
    /// **A replace, not a remove-plus-append.** `/Annots` order is paint
    /// order, and (absent `/Tabs`) tab order — so removing the old entry and
    /// pushing a new one would silently move the field to the end of both.
    /// The operator asked to add a second view of a field, not to reorder the
    /// page.
    ///
    /// **The append is folded in rather than left to a second call.** Two
    /// whole-dictionary writes to one page dict in one command do not
    /// compose: both are computed from the pre-command state, so applying
    /// them leaves only the last. This is the same failure the R85 oracle
    /// caught in `flatten_fields`, and it showed here as a page whose
    /// `/Annots` still named the field dictionary that had just stopped being
    /// an annotation.
    fn retarget_annot(
        &mut self,
        page_id: ObjId,
        from: ObjId,
        to: ObjId,
        append: Option<ObjId>,
    ) -> Result<Vec<ObjectWrite>, EditError> {
        let Some(Object::Dict(page)) = self.value(page_id) else {
            return Ok(Vec::new());
        };
        let page = page.clone();
        let swap = |entries: &[Object]| -> (Vec<Object>, bool) {
            let mut hit = false;
            let mut out: Vec<Object> = entries
                .iter()
                .map(|o| {
                    if o.as_reference() == Some(from) {
                        hit = true;
                        Object::Reference(to)
                    } else {
                        o.clone()
                    }
                })
                .collect();
            if let Some(extra) = append {
                out.push(Object::Reference(extra));
                hit = true;
            }
            (out, hit)
        };
        match page.get(b"Annots").cloned() {
            Some(Object::Array(entries)) => {
                let (kept, hit) = swap(&entries);
                if !hit {
                    return Ok(Vec::new());
                }
                let mut updated = page;
                updated.insert(Name::from(b"Annots"), Object::Array(kept));
                Ok(vec![ObjectWrite {
                    id: page_id,
                    before: self.state.get(&page_id).cloned(),
                    after: Some(Object::Dict(updated)),
                }])
            }
            Some(Object::Reference(arr_id)) => {
                let entries = self
                    .value(arr_id)
                    .and_then(Object::as_array)
                    .map(<[Object]>::to_vec)
                    .unwrap_or_default();
                let (kept, hit) = swap(&entries);
                if !hit {
                    return Ok(Vec::new());
                }
                Ok(vec![ObjectWrite {
                    id: arr_id,
                    before: self.state.get(&arr_id).cloned(),
                    after: Some(Object::Array(kept)),
                }])
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Build everything a new terminal field needs to HANG somewhere: the
    /// intermediate grouping nodes a dotted path requires, the `/Kids` links
    /// down to the field, and the `/AcroForm /Fields` registration of
    /// whichever node is the root (§3.1.4).
    ///
    /// # Why this is one function and not "create parents, then register"
    ///
    /// The first draft split it, and the split was wrong in a way worth
    /// recording: the registration step read the parent back with
    /// `self.value(parent_id)` to append to its `/Kids`, and a parent this
    /// same call had just created is NOT there to read — its `ObjectWrite` is
    /// still pending in the command being assembled, and `self.value` sees
    /// committed state. Creating `Personal.Address.Zip` on a formless page
    /// therefore failed with *"object 6 0 is not a dictionary, so /Kids
    /// cannot be set on it"*.
    ///
    /// So the chain is wired at BUILD time instead: each created node is
    /// emitted with its `/Kids` already pointing at the next one down, and
    /// the only dictionary read back is one that genuinely pre-existed.
    ///
    /// # What comes back
    ///
    /// The writes, the parent the terminal hangs from (`None` ⇒ it is itself
    /// a `/Fields` root), and the terminal's OWN partial name — the LAST path
    /// segment, never the dotted string. §12.7.3.2 composes the FQN from the
    /// ancestors' `/T`s, so writing `Personal.Address.Zip` as a `/T` under
    /// `Address` would yield `Personal.Address.Personal.Address.Zip`.
    ///
    /// # The registration rule
    ///
    /// §12.7.3.1 makes `/Fields` the ROOT list. A node that has a `/Parent`
    /// must NOT also appear there: the walk would reach it twice and give it
    /// two fully-qualified names. So exactly one node is registered — the
    /// topmost one this call created — and nothing is registered at all when
    /// the path hangs off an existing node, because that node's own root is
    /// already listed.
    fn place_new_field(
        &mut self,
        deepest: Option<ObjId>,
        remaining: &[String],
        field_id: ObjId,
    ) -> Result<(Vec<ObjectWrite>, Option<ObjId>, String), EditError> {
        let mut writes = Vec::new();
        let Some((terminal, groups)) = remaining.split_last() else {
            return Err(EditError::FieldNameEmpty);
        };

        // Allocate every grouping node first, so each can name the next.
        let mut group_ids = Vec::with_capacity(groups.len());
        for _ in groups {
            group_ids.push(ObjId::new(self.alloc_number()?, 0));
        }

        for (i, segment) in groups.iter().enumerate() {
            let Some(id) = group_ids.get(i).copied() else {
                continue;
            };
            let mut d = Dict::new();
            d.insert(
                Name::from(b"T"),
                Object::String(encode_text_string(segment)),
            );
            // NO `/FT`: Table 220 — a non-terminal field has no type of its
            // own. Writing one would make every terminal beneath it inherit a
            // type the operator never asked these siblings to share.
            let child = group_ids.get(i + 1).copied().unwrap_or(field_id);
            d.insert(
                Name::from(b"Kids"),
                Object::Array(vec![Object::Reference(child)]),
            );
            // The first group's parent is whatever already existed; the rest
            // chain to the group above them.
            let parent = if i == 0 {
                deepest
            } else {
                group_ids.get(i - 1).copied()
            };
            if let Some(p) = parent {
                d.insert(Name::from(b"Parent"), Object::Reference(p));
            }
            writes.push(ObjectWrite {
                id,
                before: None,
                after: Some(Object::Dict(d)),
            });
        }

        // The node the terminal hangs from: the last group created, or the
        // pre-existing `deepest` when the path needed no new groups.
        let field_parent = group_ids.last().copied().or(deepest);

        // The topmost node this call introduced — the one that needs a home.
        let root_of_new_chain = group_ids.first().copied().unwrap_or(field_id);

        match deepest {
            // Hanging off something that already exists: append to ITS
            // `/Kids`. That object is committed, so reading it back is safe.
            Some(existing) => {
                let Some(Object::Dict(pd)) = self.value(existing) else {
                    return Err(EditError::NotADictionary {
                        id: existing,
                        key: "Kids",
                    });
                };
                let mut updated = pd.clone();
                let mut kids: Vec<Object> = pd
                    .get(b"Kids")
                    .map(|o| self.graph().resolve(o).clone())
                    .and_then(|o| o.as_array().map(<[Object]>::to_vec))
                    .unwrap_or_default();
                kids.push(Object::Reference(root_of_new_chain));
                updated.insert(Name::from(b"Kids"), Object::Array(kids));
                let before = self.state.get(&existing).cloned();
                writes.push(ObjectWrite {
                    id: existing,
                    before,
                    after: Some(Object::Dict(updated)),
                });
            }
            // Nothing on the path existed: the chain's top is a new root.
            None => writes.push(self.acroform_register_write(root_of_new_chain)?),
        }

        Ok((writes, field_parent, terminal.clone()))
    }

    /// The `/Rect`, `/P`, `/T`, `/F` and `/TU` entries every authored widget
    /// carries, as a merged field+widget dictionary (§12.5.6.19).
    ///
    /// `/F 4` is §12.5.3 Table 165 bit 3 (Print). Without it the field is on
    /// screen and absent from paper, which is not what an operator placing a
    /// form field means — and is a difference they would not see until
    /// printing.
    fn widget_base_dict(
        name: &str,
        rect: page_tree::Rect,
        page_id: ObjId,
        tooltip: &TooltipChoice,
    ) -> Dict {
        let mut d = Dict::new();
        d.insert(Name::from(b"Type"), Object::Name(Name::from(b"Annot")));
        d.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Widget")));
        d.insert(Name::from(b"T"), Object::String(encode_text_string(name)));
        d.insert(
            Name::from(b"Rect"),
            Object::Array(vec![
                Object::Real(rect.llx),
                Object::Real(rect.lly),
                Object::Real(rect.urx),
                Object::Real(rect.ury),
            ]),
        );
        d.insert(Name::from(b"P"), Object::Reference(page_id));
        d.insert(Name::from(b"F"), Object::Integer(4));
        // `/TU` only when the operator SUPPLIED one. A declined tooltip
        // writes nothing and is reported instead (R105) — an empty `/TU`
        // would be worse than none, because a screen reader would announce
        // an empty accessibility name rather than falling back to `/T`.
        if let Some(tu) = tooltip.text() {
            d.insert(Name::from(b"TU"), Object::String(encode_text_string(tu)));
        }
        d
    }

    /// Create a **check box** on a page (§12.7.4.2), returning the new
    /// field's object id.
    ///
    /// The box is a merged field+widget dictionary (§12.5.6.19) with
    /// `/FT /Btn` and neither `Radio` nor `Pushbutton` set — which is what
    /// makes a `/Btn` field a check box (§12.7.4.2.1). Both appearance
    /// states are written at creation, so the box is immediately usable by
    /// [`EditSession::set_button_state`] and immediately correct in a viewer;
    /// there is no `/NeedAppearances` fallback involved (R51).
    ///
    /// # Everything here is one undoable command
    ///
    /// The two appearance streams, the field dictionary, the page's
    /// `/Annots`, and the `/AcroForm` `/Fields` registration all go into a
    /// single [`Command`]. A half-created field — registered in the form but
    /// absent from the page, or on the page and unregistered — is a document
    /// that no undo can repair, so the operation is atomic by construction.
    ///
    /// # Errors
    ///
    /// [`EditError::CheckBoxOnStateInvalid`] when the on state is empty or
    /// `Off`, plus every refusal from
    /// [`Self::field_authoring_preflight`].
    pub fn add_check_box(&mut self, spec: &NewCheckBox) -> Result<FieldAuthorOutcome, EditError> {
        // §12.7.4.2.3: `Off` names the off state, so it cannot also name the
        // on state — a box whose states share a name cannot express "ticked".
        if spec.on_state.trim().is_empty() || spec.on_state == "Off" {
            return Err(EditError::CheckBoxOnStateInvalid {
                name: spec.on_state.clone(),
            });
        }
        let (w, h) = (spec.rect.urx - spec.rect.llx, spec.rect.ury - spec.rect.lly);
        // A check box is `/Btn` with neither `Radio` nor `Pushbutton`
        // (§12.7.4.2.1), and the KIND is part of merge compatibility: a radio
        // group is also `/FT /Btn`, and merging a check box into one would
        // give a single field widgets that disagree about whether they toggle
        // independently or exclusively.
        let (page_id, slots, path, disclosures) = self.field_authoring_preflight(
            &spec.name,
            spec.rect,
            spec.page_index,
            forms::FieldType::Button,
            Some(forms::ButtonKind::Check),
            &spec.tooltip,
        )?;

        // VECTOR artwork, not a ZapfDingbats glyph — see
        // `build_check_box_appearances` for why the shared text generator
        // cannot draw a check mark.
        let (off, on) = annot_author::build_check_box_appearances(w, h);
        let off_id = ObjId::new(self.alloc_number()?, 0);
        let on_id = ObjId::new(self.alloc_number()?, 0);

        let mut stream_of = |state: annot_author::CheckBoxStateAppearance| {
            let span = self.stage_bytes(&state.content);
            let mut dict = state.ap_dict;
            dict.insert(
                Name::from(b"Length"),
                Object::Integer(i64::try_from(state.content.len()).unwrap_or(i64::MAX)),
            );
            Object::Stream(Stream {
                dict,
                data_span: span,
            })
        };
        let off_stream = stream_of(off);
        let on_stream = stream_of(on);

        let mut d = Self::widget_base_dict(&spec.name, spec.rect, page_id, &spec.tooltip);

        // `/V` and `/AS` are NAMES here, not strings — the single most
        // common way a hand-written check box comes out unrecognisable.
        let state = if spec.checked {
            Name::from(spec.on_state.as_bytes())
        } else {
            Name::from(b"Off")
        };
        d.insert(Name::from(b"V"), Object::Name(state.clone()));
        // `/AS` (§12.5.5) is what actually selects the painted stream. It is
        // written even though it equals `/V`, because they are answering
        // different questions — `/V` is the field's value, `/AS` is the
        // annotation's current appearance — and a viewer paints from `/AS`.
        d.insert(Name::from(b"AS"), Object::Name(state));

        // `/AP` `/N` is a SUB-DICTIONARY keyed by state name, not a stream.
        let mut n = Dict::new();
        n.insert(
            Name::from(spec.on_state.as_bytes()),
            Object::Reference(on_id),
        );
        n.insert(Name::from(b"Off"), Object::Reference(off_id));
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Dict(n));
        d.insert(Name::from(b"AP"), Object::Dict(ap));

        let mut objects = vec![
            ObjectWrite {
                id: off_id,
                before: None,
                after: Some(off_stream),
            },
            ObjectWrite {
                id: on_id,
                before: None,
                after: Some(on_stream),
            },
        ];

        // THE MERGE BRANCH — the same box on a second page, sharing one
        // `/V`, which is what makes ticking it tick both.
        //
        // `/V` and `/AS` are removed from the incoming widget dict: the VALUE
        // belongs to the field (it is shared), and the merged field already
        // has one. `/AS` is re-derived per widget by `set_button_state`, and
        // seeding it here from this call's `--checked` would let a second add
        // silently change the state the first one set.
        if let FieldPath::Terminal { id, shape, .. } = path {
            let mut w = d;
            w.remove(b"V");
            w.remove(b"AS");
            let existing_state = self
                .value(id)
                .and_then(Object::as_dict)
                .and_then(|fd| fd.get(b"V"))
                .and_then(Object::as_name)
                .map_or_else(|| Name::from(b"Off"), |n| Name::from(n.as_bytes()));
            w.insert(Name::from(b"AS"), Object::Name(existing_state));
            let (merge_writes, _widget) =
                self.merge_widget_into_field(id, shape, w, page_id, &slots)?;
            objects.extend(merge_writes);
            self.commit(Command {
                kind: CommandKind::AddFormField,
                objects,
                removals: Vec::new(),
                trailer: None,
            });
            return Ok(FieldAuthorOutcome {
                field_id: id,
                merged: true,
                disclosures,
            });
        }

        let FieldPath::Vacant { deepest, remaining } = path else {
            return Err(FormAuthorError::NameIsGroupingNode {
                fqn: spec.name.clone(),
            }
            .into());
        };
        let field_id = ObjId::new(self.alloc_number()?, 0);
        let (parent_writes, parent, partial) =
            self.place_new_field(deepest, &remaining, field_id)?;

        d.insert(Name::from(b"FT"), Object::Name(Name::from(b"Btn")));
        d.insert(Name::from(b"Ff"), Object::Integer(spec.field_flags()));
        d.insert(
            Name::from(b"T"),
            Object::String(encode_text_string(&partial)),
        );
        if let Some(p) = parent {
            d.insert(Name::from(b"Parent"), Object::Reference(p));
        }

        objects.push(ObjectWrite {
            id: field_id,
            before: None,
            after: Some(Object::Dict(d)),
        });
        objects.extend(parent_writes);
        objects.extend(self.annots_writes(page_id, field_id, &slots)?);

        self.commit(Command {
            kind: CommandKind::AddFormField,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FieldAuthorOutcome {
            field_id,
            merged: false,
            disclosures,
        })
    }

    /// Add one member to a **radio group** (§12.7.4.2.1), creating the group
    /// if this is its first member.
    ///
    /// # The group is the merge primitive, not a radio feature
    ///
    /// This function contains no code that knows what a radio group is beyond
    /// setting `/Ff` bit 16 and drawing a round widget. Grouping comes
    /// entirely from F1's [`forms_author::resolve_field_path`]: a second call
    /// with the same `name` resolves to
    /// [`FieldPath::Terminal`](forms_author::FieldPath::Terminal) and merges,
    /// promoting the field from Shape A to Shape B exactly as a repeated
    /// check box does. Decision 020 requires this specifically — a
    /// radio-specific grouping path would be a second mechanism for what
    /// §12.7.3.2 already says a shared name means.
    ///
    /// Mutual exclusion likewise needs nothing here:
    /// [`Self::set_button_state`] already sets each widget's `/AS` to the
    /// requested state if that widget offers it and `/Off` otherwise, which
    /// **is** radio behaviour. Authoring a real group is enough to make the
    /// existing, unmodified fill path behave correctly.
    ///
    /// # What this refuses, and why each refusal exists
    ///
    /// - **A different field type under the name** — F1's collision rule,
    ///   and buttons compare KIND, so a check box cannot join a radio group
    ///   (they disagree about whether widgets toggle independently).
    /// - **A duplicate export value** without `RadiosInUnison`
    ///   ([`EditError::RadioExportValueTaken`]) — members are identified by
    ///   that string alone.
    /// - **A group whose states are positional `/Opt`**
    ///   ([`EditError::RadioGroupUsesPositionalOpt`]) — decision 020 §8.3.
    ///   Unreachable on pdfce-authored groups, which are always named.
    ///
    /// # Errors
    ///
    /// The three above, plus everything
    /// [`Self::field_authoring_preflight`] raises: an empty name, an
    /// undecided `/TU` (R105), a degenerate `/Rect`, a hybrid-XFA document,
    /// and the encryption, **strict** certification and `/Size` guards.
    /// `Off` as an export value is refused through
    /// [`EditError::CheckBoxOnStateInvalid`] — the same reserved-name rule,
    /// shared rather than duplicated.
    pub fn add_radio_button(
        &mut self,
        spec: &NewRadioButton,
    ) -> Result<FieldAuthorOutcome, EditError> {
        // §12.7.4.2.3 reserves `Off`; a member exporting it could never be
        // told apart from the group being empty.
        if spec.export_value.trim().is_empty() || spec.export_value == "Off" {
            return Err(EditError::CheckBoxOnStateInvalid {
                name: spec.export_value.clone(),
            });
        }
        let (w, h) = (spec.rect.urx - spec.rect.llx, spec.rect.ury - spec.rect.lly);
        let (page_id, slots, path, mut disclosures) = self.field_authoring_preflight(
            &spec.name,
            spec.rect,
            spec.page_index,
            forms::FieldType::Button,
            Some(forms::ButtonKind::Radio),
            &spec.tooltip,
        )?;

        let (off, on) = annot_author::build_radio_button_appearances(w, h);
        let off_id = ObjId::new(self.alloc_number()?, 0);
        let on_id = ObjId::new(self.alloc_number()?, 0);

        let mut stream_of = |state: annot_author::CheckBoxStateAppearance| {
            let span = self.stage_bytes(&state.content);
            let mut dict = state.ap_dict;
            dict.insert(
                Name::from(b"Length"),
                Object::Integer(i64::try_from(state.content.len()).unwrap_or(i64::MAX)),
            );
            Object::Stream(Stream {
                dict,
                data_span: span,
            })
        };
        let off_stream = stream_of(off);
        let on_stream = stream_of(on);

        let mut d = Self::widget_base_dict(&spec.name, spec.rect, page_id, &spec.tooltip);

        // `/AP /N` keyed by state name — the member's export value and `Off`.
        // NAMES, not strings: the single most common way a hand-built button
        // comes out unrecognisable to a viewer.
        let mut n = Dict::new();
        n.insert(
            Name::from(spec.export_value.as_bytes()),
            Object::Reference(on_id),
        );
        n.insert(Name::from(b"Off"), Object::Reference(off_id));
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Dict(n));
        d.insert(Name::from(b"AP"), Object::Dict(ap));

        let mut objects = vec![
            ObjectWrite {
                id: off_id,
                before: None,
                after: Some(off_stream),
            },
            ObjectWrite {
                id: on_id,
                before: None,
                after: Some(on_stream),
            },
        ];

        // ---- THE MERGE BRANCH: joining an existing group ----
        if let FieldPath::Terminal { id, shape, .. } = path {
            let existing = self.radio_group_state(id, &spec.name)?;

            // Decision 020 §8.3: a positional-`/Opt` group cannot be extended,
            // because pdfce does not write `/Opt` and cannot compute what
            // index a new member would occupy.
            if existing.positional_opt {
                return Err(EditError::RadioGroupUsesPositionalOpt {
                    fqn: spec.name.clone(),
                });
            }
            // Members are told apart by export value alone, unless the group
            // was deliberately created to select in unison.
            if !existing.in_unison
                && existing
                    .on_states
                    .iter()
                    .any(|s| s == spec.export_value.as_bytes())
            {
                return Err(EditError::RadioExportValueTaken {
                    fqn: spec.name.clone(),
                    state: spec.export_value.clone(),
                });
            }
            // The group's flags stand; a disagreement is reported, not applied
            // — see `FieldAuthorDisclosures::group_flags_ignored`.
            if spec.no_toggle_to_off != existing.no_toggle_to_off
                || spec.radios_in_unison != existing.in_unison
            {
                disclosures.group_flags_ignored = true;
            }

            let mut widget = d;
            // `/V` belongs to the FIELD, which already has one. `/AS` is this
            // widget's own, and it is `Off` unless this call is choosing this
            // member — a merge must not silently re-point a selection the
            // earlier calls made.
            let this_as: &[u8] = if spec.selected {
                spec.export_value.as_bytes()
            } else {
                b"Off"
            };
            widget.insert(Name::from(b"AS"), Object::Name(Name::from(this_as)));

            let (merge_writes, _new_widget) =
                self.merge_widget_into_field(id, shape, widget, page_id, &slots)?;
            objects.extend(merge_writes);

            // Choosing this member on the way in re-points the group's `/V`
            // and must clear every SIBLING's `/AS`, or two widgets paint as
            // selected at once. That is `set_button_state`'s job and it is
            // reached below, after the merge is committed, so it sees the
            // group the merge actually produced rather than a prediction of
            // it (R92: one path decides what "selected" looks like).
            self.commit(Command {
                kind: CommandKind::AddFormField,
                objects,
                removals: Vec::new(),
                trailer: None,
            });
            if spec.selected {
                self.set_button_state(&spec.name, &spec.export_value)?;
            }
            return Ok(FieldAuthorOutcome {
                field_id: id,
                merged: true,
                disclosures,
            });
        }

        // ---- CREATING THE GROUP: this is its first member ----
        let FieldPath::Vacant { deepest, remaining } = path else {
            return Err(FormAuthorError::NameIsGroupingNode {
                fqn: spec.name.clone(),
            }
            .into());
        };
        let field_id = ObjId::new(self.alloc_number()?, 0);
        let (parent_writes, parent, partial) =
            self.place_new_field(deepest, &remaining, field_id)?;

        let state = if spec.selected {
            Name::from(spec.export_value.as_bytes())
        } else {
            Name::from(b"Off")
        };
        d.insert(Name::from(b"V"), Object::Name(state.clone()));
        d.insert(Name::from(b"AS"), Object::Name(state));
        d.insert(Name::from(b"FT"), Object::Name(Name::from(b"Btn")));
        d.insert(Name::from(b"Ff"), Object::Integer(spec.field_flags()));
        d.insert(
            Name::from(b"T"),
            Object::String(encode_text_string(&partial)),
        );
        if let Some(p) = parent {
            d.insert(Name::from(b"Parent"), Object::Reference(p));
        }

        objects.push(ObjectWrite {
            id: field_id,
            before: None,
            after: Some(Object::Dict(d)),
        });
        objects.extend(parent_writes);
        objects.extend(self.annots_writes(page_id, field_id, &slots)?);

        self.commit(Command {
            kind: CommandKind::AddFormField,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FieldAuthorOutcome {
            field_id,
            merged: false,
            disclosures,
        })
    }

    /// Delete a whole field: every widget, the field dictionary, its
    /// registration, and any grouping node it leaves empty (§3.6.3).
    ///
    /// # One command, for the same reason creation is one
    ///
    /// A field un-listed from `/AcroForm /Fields` but still annotated on a
    /// page is a widget no form owns; annotated nowhere but still registered
    /// is a field with no way to reach it. Either half alone is a document no
    /// undo can repair, so both go in a single [`Command`].
    ///
    /// # Errors
    ///
    /// [`EditError::FieldNotFound`] when nothing bears the name; plus the
    /// encryption and **strict** certification guards — deleting a field is a
    /// structural change to the form, which is precisely what a certification
    /// signature freezes.
    pub fn delete_field(&mut self, fqn: &str) -> Result<FieldDeletion, EditError> {
        let (field, _) = self.deletion_preflight(fqn)?;
        let slots = self.page_slots()?;

        let widget_ids: Vec<ObjId> = field
            .widgets
            .iter()
            .filter(|w| !w.merged)
            .map(|w| w.id)
            .collect();
        let objects = self.unlist_widgets(&field.widgets, &slots)?;

        let (form_writes, emptied) = self.remove_fields_from_form(&[field.id])?;
        let mut objects = objects;
        objects.extend(form_writes);

        // The field dict, its non-merged widgets, and every grouping node the
        // removal emptied. A merged widget IS the field dict, so it is not
        // deleted twice.
        let removals: Vec<Removal> = std::iter::once(field.id)
            .chain(widget_ids.iter().copied())
            .chain(emptied.iter().copied())
            .filter(|id| self.base.get(*id).is_some() || self.state.contains_key(id))
            .map(|id| Removal {
                id,
                was_deleted: self.deleted.contains(&id),
                is_deleted: true,
            })
            .collect();

        self.commit(Command {
            kind: CommandKind::DeleteFormField,
            objects,
            removals,
            trailer: None,
        });
        Ok(FieldDeletion {
            widgets_removed: field.widgets.len(),
            field_removed: true,
            selection_cleared: false,
            emptied_parents: emptied.len(),
        })
    }

    /// Delete ONE widget of a field, by its index in the field's widget list
    /// (§3.6.3's mid-group and last-member rules).
    ///
    /// # The three rules, and which one fires
    ///
    /// 1. **Mid-group.** The widget leaves the field's `/Kids` and its page's
    ///    `/Annots`, and its object is deleted. Remaining members are
    ///    otherwise untouched.
    /// 2. **The deleted widget held the selection.** Its on-state equalled
    ///    the field's `/V`, so `/V` now names a state **no remaining widget
    ///    can display** — a malformed field. `/V` becomes `/Off`, every
    ///    remaining kid's `/AS` becomes `/Off`, and
    ///    [`FieldDeletion::selection_cleared`] says so.
    /// 3. **Last member.** There is no field left to hold, so this becomes
    ///    [`Self::delete_field`] — including its grouping-node prune. The
    ///    same rule for any field type, not a radio special case.
    ///
    /// **No Shape B→A collapse** (R102): a group that falls from three
    /// members to one stays a `/Kids` parent. Collapsing it would rewrite
    /// object identities the operator never asked to change, and the shape is
    /// legal either way.
    ///
    /// # Errors
    ///
    /// [`EditError::FieldNotFound`] when nothing bears the name;
    /// [`EditError::WidgetIndexOutOfRange`] when the field has no such
    /// widget; plus the encryption and strict certification guards.
    pub fn delete_widget(&mut self, fqn: &str, index: usize) -> Result<FieldDeletion, EditError> {
        let (field, _) = self.deletion_preflight(fqn)?;
        let Some(widget) = field.widgets.get(index).cloned() else {
            return Err(EditError::WidgetIndexOutOfRange {
                name: fqn.to_owned(),
                index,
                widgets: field.widgets.len(),
            });
        };

        // Rule 3: the last widget takes the whole field with it. Delegated
        // rather than reimplemented, so the two paths cannot disagree about
        // what "gone" means.
        if field.widgets.len() == 1 {
            return self.delete_field(fqn);
        }

        let slots = self.page_slots()?;
        let mut objects = self.unlist_widgets(std::slice::from_ref(&widget), &slots)?;

        // Rule 2: did this widget hold the field's value?
        let held_selection = match &field.value {
            forms::FieldValue::Name(v) => widget.on_states.iter().any(|s| s == v),
            _ => false,
        };

        // The field dict loses this kid from /Kids, and (rule 2) its /V.
        let Some(Object::Dict(field_dict)) = self.value(field.id) else {
            return Err(EditError::NotADictionary {
                id: field.id,
                key: "Kids",
            });
        };
        let mut updated = field_dict.clone();
        if let Some(Object::Array(kids)) = updated.get(b"Kids").cloned() {
            let pruned: Vec<Object> = kids
                .into_iter()
                .filter(|o| o.as_reference() != Some(widget.id))
                .collect();
            updated.insert(Name::from(b"Kids"), Object::Array(pruned));
        }
        if held_selection {
            updated.insert(Name::from(b"V"), Object::Name(Name::from(b"Off")));
        }
        objects.push(ObjectWrite {
            id: field.id,
            before: self.state.get(&field.id).cloned(),
            after: Some(Object::Dict(updated)),
        });

        // Rule 2 continued: every REMAINING kid goes to /Off, because the
        // value they were agreeing with no longer exists.
        if held_selection {
            for w in field.widgets.iter().filter(|w| w.id != widget.id) {
                objects.push(self.set_widget_as(w.id, b"Off")?);
            }
        }

        let removals = if widget.merged {
            // Unreachable while the single-widget case is delegated above — a
            // merged widget IS the field dict, so a field with two widgets
            // has none. Guarded rather than assumed: deleting the field dict
            // here would take the surviving members with it.
            Vec::new()
        } else {
            vec![Removal {
                id: widget.id,
                was_deleted: self.deleted.contains(&widget.id),
                is_deleted: true,
            }]
        };

        self.commit(Command {
            kind: CommandKind::DeleteFormField,
            objects,
            removals,
            trailer: None,
        });
        Ok(FieldDeletion {
            widgets_removed: 1,
            field_removed: false,
            selection_cleared: held_selection,
            emptied_parents: 0,
        })
    }

    /// The guards and lookup both deletion entry points share.
    ///
    /// Shared so the two cannot drift about which documents refuse deletion —
    /// a certification gate honoured by one verb and not the other is worse
    /// than neither having it, because it reads as protection.
    /// Rename a field, changing its partial name `/T` (decision 020's F6).
    ///
    /// # One write, a subtree of consequence
    ///
    /// §12.7.3.2 constructs the fully-qualified name by walking DOWN from the
    /// `/AcroForm /Fields` roots and appending each node's `/T`. A rename
    /// therefore writes **exactly one dictionary** — the target's — and every
    /// descendant's FQN re-derives from the new prefix with no object of
    /// theirs touched.
    ///
    /// This is why [`FieldRename::descendants_renamed`] is returned rather
    /// than discarded: a one-field request can rename six fields, and an
    /// operator not told so has silently broken every FDF, JavaScript
    /// reference and submit mapping that named them (rule 4).
    ///
    /// `new_partial` is a **partial** name, not an FQN — the one path segment
    /// this node contributes. Renaming `Address.City` to `Town` yields
    /// `Address.Town`; the field does not move in the tree, and this verb
    /// deliberately cannot re-parent it.
    ///
    /// # Errors
    ///
    /// [`crate::forms_author::FormAuthorError::PeriodInPartialName`] when
    /// `new_partial` contains a period — §12.7.3.2 reserves it as the path
    /// separator, so a `/T` holding one has no unambiguous FQN, and there is
    /// no escape;
    /// [`crate::forms_author::FormAuthorError::EmptyName`] for an empty one;
    /// [`crate::forms_author::FormAuthorError::RenameCollision`] when
    /// something already bears the destination name — refused, never merged,
    /// for the reason on that variant;
    /// [`EditError::FieldNotFound`] when nothing bears `fqn`; plus the
    /// encryption and **strict** certification guards, because a rename is a
    /// structural change to the form and that is precisely what a
    /// certification signature freezes.
    pub fn rename_field(&mut self, fqn: &str, new_partial: &str) -> Result<FieldRename, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        // A partial name is ONE segment. Routing it through the same splitter
        // the authoring paths use means the period rule, the empty rule and
        // the depth rule cannot drift between create and rename — and a
        // caller who passes a dotted name here gets the period refusal rather
        // than a silently re-parented field.
        let segments = forms_author::split_field_path(new_partial)?;
        // Destructured rather than length-checked-then-indexed: "exactly one
        // segment" IS the requirement, and binding it that way removes the
        // panic path instead of guarding it.
        let [only_segment] = segments.as_slice() else {
            return Err(FormAuthorError::DottedPartialName {
                supplied: new_partial.to_owned(),
            }
            .into());
        };
        let only_segment = only_segment.clone();

        // What does the OLD name denote? A grouping node is renameable too —
        // it is the case that moves a whole subtree — so this accepts both,
        // and only `Vacant` is a failure.
        let target = match forms_author::resolve_field_path(&self.graph(), fqn)? {
            FieldPath::Terminal { id, .. } | FieldPath::Grouping { id } => id,
            FieldPath::Vacant { .. } => {
                return Err(EditError::FieldNotFound {
                    name: fqn.to_owned(),
                });
            }
        };

        // The destination FQN: the same path with its LAST segment replaced.
        let mut path = forms_author::split_field_path(fqn)?;
        path.pop();
        path.push(only_segment.clone());
        let new_fqn = path.join(".");

        if new_fqn == fqn {
            // A no-op rename reaches neither the undo stack nor the file.
            return Ok(FieldRename {
                from: fqn.to_owned(),
                to: new_fqn,
                descendants_renamed: 0,
            });
        }

        // Refused, not merged. See `RenameCollision`.
        if !matches!(
            forms_author::resolve_field_path(&self.graph(), &new_fqn)?,
            FieldPath::Vacant { .. }
        ) {
            return Err(FormAuthorError::RenameCollision {
                from: fqn.to_owned(),
                to: new_fqn,
            }
            .into());
        }

        // The blast radius, counted BEFORE the write, off the reader's
        // projection. A descendant is anything whose FQN sits under this
        // node's path — `Address` renames `Address.City`, but not `Addressed`,
        // which is why the prefix carries the separator.
        let prefix = format!("{fqn}.");
        let descendants_renamed = forms::parse_acroform(&self.graph())
            .map(|form| {
                form.fields
                    .iter()
                    .filter(|f| f.fully_qualified_name.starts_with(&prefix))
                    .count()
            })
            .unwrap_or(0);

        let Some(Object::Dict(dict)) = self.value(target) else {
            return Err(EditError::FieldNotFound {
                name: fqn.to_owned(),
            });
        };
        let mut dict = dict.clone();
        dict.insert(Name::from(b"T"), Object::String(only_segment.into_bytes()));

        self.commit(Command {
            kind: CommandKind::RenameFormField,
            objects: vec![ObjectWrite {
                id: target,
                before: self.state.get(&target).cloned(),
                after: Some(Object::Dict(dict)),
            }],
            removals: Vec::new(),
            trailer: None,
        });

        Ok(FieldRename {
            from: fqn.to_owned(),
            to: new_fqn,
            descendants_renamed,
        })
    }

    fn deletion_preflight(&mut self, fqn: &str) -> Result<(forms::Field, ()), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        // STRICT, not the `/P`-aware fill gate: removing a field changes the
        // form's structure, which is what certification exists to freeze.
        self.check_certification()?;
        let form =
            forms::parse_acroform(&self.graph()).ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        let field = form
            .fields
            .iter()
            .find(|f| f.fully_qualified_name == fqn)
            .cloned()
            .ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        Ok((field, ()))
    }

    /// Un-list `widgets` from whichever pages carry them, returning the page
    /// writes.
    ///
    /// Grouped by page so each page is written ONCE. Two writes to one page
    /// dict computed from the same pre-command state overwrite rather than
    /// compose — the defect that broke `flatten_fields` and again F1's
    /// promotion, both of which produced documents that parsed and drew the
    /// wrong picture.
    fn unlist_widgets(
        &self,
        widgets: &[forms::Widget],
        slots: &[PageSlot],
    ) -> Result<Vec<ObjectWrite>, EditError> {
        let mut by_page: BTreeMap<ObjId, Vec<ObjId>> = BTreeMap::new();
        for w in widgets {
            if let Some(page_id) = self.page_of_widget(w, slots) {
                by_page.entry(page_id).or_default().push(w.id);
            }
        }
        let mut writes = Vec::new();
        for (page_id, ids) in by_page {
            let Some(Object::Dict(page_dict)) = self.value(page_id) else {
                continue;
            };
            let mut updated = page_dict.clone();
            // An indirect `/Annots` array shared between pages is its own
            // object and returns its own write; an inline one is composed
            // into the page dict here.
            if let Some(shared) = self.remove_from_annots(&mut updated, &ids)? {
                writes.push(shared);
            } else {
                writes.push(ObjectWrite {
                    id: page_id,
                    before: self.state.get(&page_id).cloned(),
                    after: Some(Object::Dict(updated)),
                });
            }
        }
        Ok(writes)
    }

    /// Read the copyable properties of an existing field, to pre-fill a new
    /// field's spec (decision 020's F6 `--defaults-from`).
    ///
    /// # What copies, and why the list is this short
    ///
    /// Only **non-boolean data**, and only within a matching field type:
    /// `/MaxLen` for text, `/Opt` for choice, the on-state name for a check
    /// box. A radio field contributes **nothing**. Everything else is
    /// excluded for a stated reason, and the exclusions are the design:
    ///
    /// **Every boolean is excluded.** The CLI's flags are *presence* flags
    /// (`#[arg(long)] multiline: bool`), so absence and explicit-false are
    /// the same token. Copying them would let `--defaults-from` **add** a
    /// property but never turn one off — a single-line field could not be
    /// created from a multiline template. That is a one-way trap, it is
    /// operator-facing, and it is expensive to reverse once scripts depend
    /// on it. **If `--no-*` pairs are ever added across the creation verbs,
    /// this exclusion should be revisited** — the trap is a property of the
    /// flag shape, not of the idea.
    ///
    /// **`/TU` is excluded**, and this one is load-bearing. R105 exists so
    /// an accessibility name is never a silent default — *"'I never
    /// considered it' cannot happen silently"*. A copied tooltip would
    /// satisfy R105's mechanism while defeating its purpose, and that holds
    /// for a copied *declination* too: inheriting "no tooltip" is still a
    /// decision the operator never made.
    ///
    /// **`/V`, `checked` and `selected` are excluded** — a value is content,
    /// not a default.
    ///
    /// **`/AA` is excluded** — decision 020 §F3 rules that push-button
    /// creation authors no action under decision 009 posture A, so copying
    /// `/AA` would author actions through the back door.
    ///
    /// **The radio export value is excluded because there is nothing to
    /// copy.** On-states live per *widget* ([`forms::Widget::on_states`]),
    /// so a radio *field* has N export values, one per member, and
    /// `--defaults-from <field>` names a field. A copy would either collide
    /// with [`FormAuthorError::RadioExportValueTaken`] within the same group
    /// or be arbitrary across groups.
    ///
    /// # The consequence worth stating
    ///
    /// Because every shared property is a boolean and every copyable one is
    /// type-specific, **there is no common subset**: a template of a
    /// different type contributes nothing at all. That is disclosed rather
    /// than silently producing a bare field
    /// ([`FieldAuthorDisclosures::defaults_type_mismatch`]).
    ///
    /// # Errors
    ///
    /// [`EditError::FieldNotFound`] when `source` names no terminal field.
    /// A grouping node is not a field and is refused the same way.
    pub fn field_defaults(&self, source: &str) -> Result<FieldDefaults, EditError> {
        let not_found = || EditError::FieldNotFound {
            name: source.to_owned(),
        };
        let form = forms::parse_acroform(&self.graph()).ok_or_else(not_found)?;
        let field = form
            .fields
            .iter()
            .find(|f| f.fully_qualified_name == source)
            .ok_or_else(not_found)?;

        // The on-state is read from the FIRST widget, and the disagreement is
        // reported rather than resolved. A check box's widgets normally share
        // one on-state name — that is what makes the box mean the same thing
        // on every page it appears on — so a document where they differ is
        // saying something unusual, and picking one silently would hide it.
        let mut on_state = None;
        let mut on_state_ambiguous = false;
        if matches!(field.button_kind, Some(ButtonKind::Check)) {
            let mut states = field.widgets.iter().filter_map(|w| w.on_states.first());
            on_state = states.next().cloned();
            on_state_ambiguous = states.any(|s| Some(s) != on_state.as_ref());
        }

        Ok(FieldDefaults {
            field_type: field.field_type,
            button_kind: field.button_kind,
            max_len: field.max_len,
            options: field.options.clone(),
            on_state,
            on_state_ambiguous,
        })
    }

    /// Read the facts about an existing radio group that a new member has to
    /// be checked against.
    ///
    /// Read through [`forms::parse_acroform`] rather than off the raw dict so
    /// that inherited flags and `/Kids` widgets are resolved the same way
    /// every other consumer resolves them — a second, private reading of the
    /// same graph is how two parts of a program come to disagree about what a
    /// document says.
    fn radio_group_state(&self, field_id: ObjId, fqn: &str) -> Result<RadioGroupState, EditError> {
        let form =
            forms::parse_acroform(&self.graph()).ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        let field = form
            .fields
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        Ok(RadioGroupState {
            // `radios_in_unison()` is the TYPE-GATED predicate: `/Ff` bit 26
            // is `RichText` on a text field and only means unison on a radio,
            // so the raw bit is never tested directly.
            in_unison: field.radios_in_unison(),
            no_toggle_to_off: field.flags.has(forms::FieldFlags::NO_TOGGLE_TO_OFF),
            // A button field carrying `/Opt` is using Table 227's positional
            // form: its `/AP /N` keys are indices into that array.
            positional_opt: !field.options.is_empty(),
            on_states: field
                .widgets
                .iter()
                .flat_map(|w| w.on_states.iter().cloned())
                .collect(),
        })
    }

    /// Create a **list box or combo box** on a page (§12.7.4.4), returning
    /// the new field's object id.
    ///
    /// The field is created with its options and **no selection** — see
    /// [`NewChoiceField`] for why that is deliberate rather than a default
    /// that happened to be convenient. Filling it is
    /// [`EditSession::set_choice_value`]'s job.
    ///
    /// Unlike a check box, a choice field's appearance IS generated, through
    /// the same §12.7.3.3 variable-text path a text field uses (R92: one
    /// regenerator, never two) — so an empty appearance is generated here for
    /// the same reason slice 1 generates one for an empty text field.
    ///
    /// # The border asymmetry with check boxes, named rather than hidden
    ///
    /// A check box created by [`Self::add_check_box`] shows a visible box in
    /// pdfce's own renderer; a choice field created here does NOT, and
    /// neither does a text field from slice 1. Verified by rendering, not
    /// assumed.
    ///
    /// The cause is not inconsistency about what a field should look like —
    /// it is WHERE the border lives. A check box's border is vector artwork
    /// inside its `/AP` stream, because its appearance is hand-drawn anyway.
    /// A choice or text field's border is `/MK` `/BC`, which R43 makes the
    /// canonical named-not-painted case: pdfce declines to synthesise a
    /// dynamic appearance from `/MK` at display time, so the border is in the
    /// FILE for viewers that honour it and invisible here.
    ///
    /// Closing the gap would mean putting a border into the SHARED
    /// variable-text appearance — which fill also uses, so every refilled
    /// field in every document would gain a border it never had — or writing
    /// a second generator, which R92 forbids. Neither is a slice-2 trade, and
    /// this note exists so the difference is a known limit rather than a
    /// surprise.
    ///
    /// # An empty option list is ALLOWED, and disclosed
    ///
    /// A choice field with no options saves, and the returned
    /// [`FieldAuthorDisclosures::has_no_options`] says so, because a zero-option
    /// field **cannot be filled** — [`Self::set_choice_value`] refuses any
    /// value not in `/Opt`, and pdfce has no verb that adds options later.
    ///
    /// Allowed rather than refused because the option list is a thing the
    /// operator populates, and a form under construction legitimately passes
    /// through the empty state; the spec permits it and Acrobat does not
    /// block it. Disclosing rather than staying silent because "this field
    /// exists and nothing can fill it" is exactly the sort of thing an
    /// operator must not discover later (R4).
    ///
    /// # Errors
    ///
    /// [`EditError::ChoiceEditRequiresCombo`] for `editable` without `combo`,
    /// [`EditError::ChoiceOptionDuplicate`] for a repeated export value,
    /// a [`VarTextError`](crate::vartext::VarTextError) from the appearance
    /// generator, plus every refusal from
    /// [`Self::field_authoring_preflight`].
    pub fn add_choice_field(
        &mut self,
        spec: &NewChoiceField,
    ) -> Result<FieldAuthorOutcome, EditError> {
        if spec.editable && !spec.combo {
            return Err(EditError::ChoiceEditRequiresCombo);
        }
        // A duplicate export is unselectable, because the fill verb resolves
        // to the first match. Checked before any write so the refusal is
        // total rather than partial.
        let mut seen = std::collections::BTreeSet::new();
        for opt in &spec.options {
            if !seen.insert(opt.export.as_str()) {
                return Err(EditError::ChoiceOptionDuplicate {
                    value: opt.export.clone(),
                });
            }
        }
        let (w, h) = (spec.rect.urx - spec.rect.llx, spec.rect.ury - spec.rect.lly);
        let (page_id, slots, path, disclosures) = self.field_authoring_preflight(
            &spec.name,
            spec.rect,
            spec.page_index,
            forms::FieldType::Choice,
            None,
            &spec.tooltip,
        )?;

        // SORT THE ARRAY, do not merely flag it. §12.7.4.4: a reader "shall
        // display the options in the order in which they occur in the Opt
        // array", so the flag alone changes nothing an operator can see.
        let mut options = spec.options.clone();
        if spec.sort {
            options.sort_by(|a, b| a.display.cmp(&b.display));
        }

        let da = crate::vartext::default_appearance_string(
            b"Helv",
            0.0,
            crate::vartext::TextColor::Gray(0.0),
        );
        let resources = [crate::vartext::FontResource {
            name: b"Helv".to_vec(),
            font: crate::fontdata::Std14::Helvetica,
        }];
        // THE SAME builder a fill and a text field use (R92).
        let appearance = annot_author::build_field_text_appearance(
            w,
            h,
            "",
            &da,
            crate::vartext::Quadding::Left,
            false,
            &resources,
        )?;

        let ap_id = ObjId::new(self.alloc_number()?, 0);
        let ap_span = self.stage_bytes(&appearance.content);
        let mut ap_dict = appearance.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(appearance.content.len()).unwrap_or(i64::MAX)),
        );
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        let mut d = Self::widget_base_dict(&spec.name, spec.rect, page_id, &spec.tooltip);
        d.insert(Name::from(b"DA"), Object::String(da.clone()));
        // §12.7.4.4: an element is `(Display)` when export and display
        // coincide, or `[(export) (display)]` when they differ. Writing the
        // short form where it applies keeps the file the shape a hand-written
        // one would be, rather than uniformly verbose.
        d.insert(
            Name::from(b"Opt"),
            Object::Array(
                options
                    .iter()
                    .map(|o| {
                        if o.is_plain() {
                            Object::String(encode_text_string(&o.display))
                        } else {
                            Object::Array(vec![
                                Object::String(encode_text_string(&o.export)),
                                Object::String(encode_text_string(&o.display)),
                            ])
                        }
                    })
                    .collect(),
            ),
        );
        // NO `/V`: §12.7.4.4 defaults it to null (nothing selected).
        let mut mk = Dict::new();
        mk.insert(
            Name::from(b"BC"),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        d.insert(Name::from(b"MK"), Object::Dict(mk));
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        d.insert(Name::from(b"AP"), Object::Dict(ap));

        let mut objects = vec![ObjectWrite {
            id: ap_id,
            before: None,
            after: Some(ap_stream),
        }];

        // THE MERGE BRANCH. The second widget carries the LOOK; `/Opt` and
        // `/Ff` stay on the field, because the option list and the combo/
        // list-box mode belong to the field, not to a view of it. A second
        // add with a DIFFERENT `--option` set therefore does not silently
        // rewrite the first one's options — it adds a place to show them.
        if let FieldPath::Terminal { id, shape, .. } = path {
            let (merge_writes, _widget) =
                self.merge_widget_into_field(id, shape, d, page_id, &slots)?;
            objects.extend(merge_writes);
            self.commit(Command {
                kind: CommandKind::AddFormField,
                objects,
                removals: Vec::new(),
                trailer: None,
            });
            // A merge adds a WIDGET, never options — `/Opt` stays on the
            // field — so the empty-options disclosure reports the field's
            // existing state, not this call's argument list.
            let has_no_options = self
                .value(id)
                .and_then(Object::as_dict)
                .and_then(|fd| fd.get(b"Opt").map(|o| self.graph().resolve(o).clone()))
                .and_then(|o| o.as_array().map(<[Object]>::len))
                .is_none_or(|n| n == 0);
            return Ok(FieldAuthorOutcome {
                field_id: id,
                merged: true,
                disclosures: FieldAuthorDisclosures {
                    has_no_options,
                    ..disclosures
                },
            });
        }

        let FieldPath::Vacant { deepest, remaining } = path else {
            return Err(FormAuthorError::NameIsGroupingNode {
                fqn: spec.name.clone(),
            }
            .into());
        };
        let field_id = ObjId::new(self.alloc_number()?, 0);
        let (parent_writes, parent, partial) =
            self.place_new_field(deepest, &remaining, field_id)?;

        d.insert(Name::from(b"FT"), Object::Name(Name::from(b"Ch")));
        d.insert(Name::from(b"Ff"), Object::Integer(spec.field_flags()));
        d.insert(
            Name::from(b"T"),
            Object::String(encode_text_string(&partial)),
        );
        if let Some(p) = parent {
            d.insert(Name::from(b"Parent"), Object::Reference(p));
        }

        objects.push(ObjectWrite {
            id: field_id,
            before: None,
            after: Some(Object::Dict(d)),
        });
        objects.extend(parent_writes);
        objects.extend(self.annots_writes(page_id, field_id, &slots)?);

        self.commit(Command {
            kind: CommandKind::AddFormField,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FieldAuthorOutcome {
            field_id,
            merged: false,
            disclosures: FieldAuthorDisclosures {
                has_no_options: options.is_empty(),
                ..disclosures
            },
        })
    }

    /// Register `field_id` in the catalog's `/AcroForm` `/Fields`, creating
    /// the `/AcroForm` dictionary (with `/DR` and `/DA`) when the document
    /// has none.
    ///
    /// # Why `/DR` matters and is not optional
    ///
    /// §12.7.3.3: a variable-text field's `/DA` names a font (`/Helv` here)
    /// that must be resolvable in the AcroForm's `/DR` `/Font`. Writing the
    /// `/DA` without the matching `/DR` entry produces a field whose
    /// appearance pdfce can regenerate (its `/AP` carries its own resources)
    /// but which another viewer re-generating from `/DA` cannot resolve —
    /// a document that works here and not elsewhere.
    fn acroform_register_write(&mut self, field_id: ObjId) -> Result<ObjectWrite, EditError> {
        let graph = self.graph();
        let catalog_id = graph.catalog_id().ok_or(EditError::NotADictionary {
            id: ObjId::new(0, 0),
            key: "Root",
        })?;
        let catalog = graph
            .resolved(catalog_id)
            .as_dict()
            .ok_or(EditError::NotADictionary {
                id: catalog_id,
                key: "AcroForm",
            })?
            .clone();
        let existing = catalog.get(b"AcroForm").cloned();

        match existing {
            // An /AcroForm that is an indirect object: append to its /Fields.
            Some(Object::Reference(af_id)) => {
                let graph = self.graph();
                let mut af = graph
                    .resolved(af_id)
                    .as_dict()
                    .ok_or(EditError::NotADictionary {
                        id: af_id,
                        key: "AcroForm",
                    })?
                    .clone();
                let mut fields = match af.get(b"Fields") {
                    Some(Object::Array(a)) => a.clone(),
                    _ => Vec::new(),
                };
                fields.push(Object::Reference(field_id));
                af.insert(Name::from(b"Fields"), Object::Array(fields));
                Self::ensure_default_resources(&mut af);
                let before = self.state.get(&af_id).cloned();
                Ok(ObjectWrite {
                    id: af_id,
                    before,
                    after: Some(Object::Dict(af)),
                })
            }
            // Inline /AcroForm, or none at all: write it into the catalog.
            other => {
                let mut cat = catalog;
                let mut af = match other {
                    Some(Object::Dict(d)) => d,
                    _ => Dict::new(),
                };
                let mut fields = match af.get(b"Fields") {
                    Some(Object::Array(a)) => a.clone(),
                    _ => Vec::new(),
                };
                fields.push(Object::Reference(field_id));
                af.insert(Name::from(b"Fields"), Object::Array(fields));
                Self::ensure_default_resources(&mut af);
                cat.insert(Name::from(b"AcroForm"), Object::Dict(af));
                let before = self.state.get(&catalog_id).cloned();
                Ok(ObjectWrite {
                    id: catalog_id,
                    before,
                    after: Some(Object::Dict(cat)),
                })
            }
        }
    }

    /// Ensure an `/AcroForm` carries a `/DA` and a `/DR` `/Font` `/Helv` the
    /// authored fields' `/DA` can resolve against (§12.7.3.3).
    ///
    /// Only ADDS what is missing — an existing `/DR` or `/DA` belongs to the
    /// document's own author and is left exactly as found.
    fn ensure_default_resources(af: &mut Dict) {
        if af.get(b"DA").is_none() {
            af.insert(
                Name::from(b"DA"),
                Object::String(crate::vartext::default_appearance_string(
                    b"Helv",
                    0.0,
                    crate::vartext::TextColor::Gray(0.0),
                )),
            );
        }
        let mut dr = match af.get(b"DR") {
            Some(Object::Dict(d)) => d.clone(),
            _ => Dict::new(),
        };
        let mut fonts = match dr.get(b"Font") {
            Some(Object::Dict(d)) => d.clone(),
            _ => Dict::new(),
        };
        if fonts.get(b"Helv").is_none() {
            fonts.insert(
                Name::from(b"Helv"),
                Object::Dict(crate::vartext::standard14_font_dict(
                    crate::fontdata::Std14::Helvetica,
                )),
            );
        }
        dr.insert(Name::from(b"Font"), Object::Dict(fonts));
        af.insert(Name::from(b"DR"), Object::Dict(dr));
    }

    /// Author a geometric-markup annotation onto a page (Pass 6.1).
    ///
    /// Generates a full `/AP` `/N` appearance (R44 — never a private
    /// pdfce-only rendering) with [`crate::annot_author::build_appearance`],
    /// creates the appearance stream and the annotation dictionary as new
    /// indirect objects, and patches the page's `/Annots` array — **without
    /// touching the page's content stream** (R47, the minimal-diff best
    /// case). The whole thing is **one** undoable command (§11.3, R49): the
    /// appearance, the annotation, and the `/Annots` patch undo together.
    ///
    /// Returns the new annotation object's id.
    ///
    /// ## Guards, in order
    ///
    /// 1. **Encrypted document ⇒ refused by name** ([`EditError::DocumentEncrypted`],
    ///    X10). Authoring strings into an encrypted file needs per-object
    ///    encryption, which is Pass 5; the R37 encoder seam already exists,
    ///    so the future fix is a plug-in.
    /// 2. **Enforced certification signature ⇒ refused**
    ///    ([`EditError::CertificationForbidsChange`], X11). This reuses the
    ///    Pass 3.2 [`EditSession::check_certification`] machinery unchanged
    ///    rather than re-deriving §12.8.2.2's per-`/P` gradation. It is
    ///    **deliberately conservative and may over-refuse**: DocMDP Table
    ///    254 permits annotation *addition* at `P = 3`, but the existing
    ///    check treats every enforced certification as forbidding. Over-
    ///    refusal is fail-clean-safe (worst case pdfce declines an edit
    ///    DocMDP would allow, never the reverse); a per-`/P` annotation-
    ///    permission refinement is a named residual for a spec-verified
    ///    follow-up.
    /// 3. **Object creation that would expose `/Size`-hidden objects ⇒
    ///    refused** ([`EditError::ObjectCreationWouldExposeHiddenObjects`]),
    ///    the same guard [`EditSession::set_info_field`] applies.
    /// 4. **Geometry with no points ⇒ refused** ([`EditError::EmptyGeometry`]).
    ///
    /// # Errors
    ///
    /// Any of the guards above, plus [`EditError::PageOutOfRange`],
    /// [`EditError::PageTree`], [`EditError::ObjectNumbersExhausted`],
    /// [`EditError::AnnotsNotAnArray`], or [`EditError::NotADictionary`]
    /// (a page that is not a dictionary).
    pub fn add_markup(&mut self, page_index: usize, spec: &MarkupSpec) -> Result<ObjId, EditError> {
        // Guard 1 (X10): encryption. Checked against the base trailer —
        // pdfce does not yet load most encrypted files, but a defensive
        // named refusal here is the R37 seam the Pass-5 fix plugs into.
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        // Guard 2 (X11): enforced certification. Conservative reuse.
        self.check_certification()?;
        // Guard 4: geometry must draw something.
        validate_geometry(spec)?;

        // Resolve the target page.
        let slots = self.page_slots()?;
        let count = slots.len();
        let page_id = slots
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count,
            })?
            .id;

        // Guard 3: creating objects raises /Size, which would expose any
        // entries a filtering /Size is hiding (§7.5.5). Refuse by name.
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        // Generate the appearance + annotation dictionary.
        let authored = annot_author::build_appearance(spec);

        // Allocate object numbers: appearance stream, then annotation.
        let ap_id = ObjId::new(self.alloc_number()?, 0);
        let annot_id = ObjId::new(self.alloc_number()?, 0);

        // Stage the appearance content (R45) and build the stream object.
        // A correct /Length is written for well-formedness; the serializer
        // recomputes it from the emitted bytes to the same value.
        let mut ap_dict = authored.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(authored.ap_content.len()).unwrap_or(i64::MAX)),
        );
        let ap_span = self.stage_bytes(&authored.ap_content);
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        // Complete the annotation dictionary: wire the appearance (/AP /N),
        // the back-reference to the page (/P), and /F Print (Table 165 bit
        // 3) so the markup prints, matching Acrobat's default for markup.
        let mut annot = authored.annot;
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        annot.insert(Name::from(b"AP"), Object::Dict(ap));
        annot.insert(Name::from(b"P"), Object::Reference(page_id));
        annot.insert(
            Name::from(b"F"),
            Object::Integer(i64::from(AnnotFlags::PRINT)),
        );

        // Patch the page's /Annots (X7: create / append / copy-on-write a
        // shared array). May allocate one more object number.
        let mut annots_writes = self.annots_writes(page_id, annot_id, &slots)?;

        let mut objects = vec![
            ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(ap_stream),
            },
            ObjectWrite {
                id: annot_id,
                before: None,
                after: Some(Object::Dict(annot)),
            },
        ];
        objects.append(&mut annots_writes);

        self.commit(Command {
            kind: CommandKind::AddAnnotation {
                kind: annot_kind_of(spec),
            },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(annot_id)
    }

    /// Author a `/Redact` redaction **mark** onto a page (Pass 8,
    /// §12.5.6.23) — the non-destructive first phase.
    ///
    /// The mark is a reviewable, saveable, round-trippable annotation
    /// (fuzzy-never-sneaky): the document can be saved marked-but-not-
    /// applied, and the operator can move or delete the mark before
    /// committing. The destructive removal is a **separate** operation
    /// ([`crate::redact::apply_redactions`], R52 — apply is never a side
    /// effect of marking, and is separately confirmed).
    ///
    /// Shares every guard, the R45 staging buffer, and the X7 `/Annots`
    /// copy-on-write path with [`EditSession::add_markup`]. The authored
    /// preview appearance is a **red outline** (never a solid fill), so a
    /// marked region can never be mistaken for a completed redaction.
    ///
    /// # Errors
    ///
    /// The same guards as [`EditSession::add_markup`].
    pub fn add_redaction(
        &mut self,
        page_index: usize,
        spec: &annot_author::RedactSpec,
    ) -> Result<ObjId, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;
        if spec.quads.is_empty() {
            return Err(EditError::EmptyGeometry);
        }

        let slots = self.page_slots()?;
        let count = slots.len();
        let page_id = slots
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count,
            })?
            .id;

        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        let authored = annot_author::build_redact_mark(spec);
        let ap_id = ObjId::new(self.alloc_number()?, 0);
        let annot_id = ObjId::new(self.alloc_number()?, 0);

        let mut ap_dict = authored.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(authored.ap_content.len()).unwrap_or(i64::MAX)),
        );
        let ap_span = self.stage_bytes(&authored.ap_content);
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        let mut annot = authored.annot;
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        annot.insert(Name::from(b"AP"), Object::Dict(ap));
        annot.insert(Name::from(b"P"), Object::Reference(page_id));
        // No /F Print: a redaction MARK is transient review state, not
        // page content — it must not print, and it is removed on apply.

        let mut annots_writes = self.annots_writes(page_id, annot_id, &slots)?;
        let mut objects = vec![
            ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(ap_stream),
            },
            ObjectWrite {
                id: annot_id,
                before: None,
                after: Some(Object::Dict(annot)),
            },
        ];
        objects.append(&mut annots_writes);

        self.commit(Command {
            kind: CommandKind::AddAnnotation {
                kind: AnnotKind::Redact,
            },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(annot_id)
    }

    /// Remove ONE unapplied `/Redact` mark from the document, as a single
    /// undoable command (Pass 8.1 — the review surface's reject half).
    ///
    /// # Why this exists, and why it is scoped this narrowly
    ///
    /// [`Self::mark_redactions_by_search`] authors marks in **bulk**: one
    /// click can produce forty. Rule 4 (fuzzy, never sneaky) says an
    /// algorithmically-produced batch must be *reviewable* — which means the
    /// operator can reject an individual member of it, not merely undo the
    /// whole batch. Undo alone cannot do that: it is a stack, so rejecting
    /// the 3rd of 40 marks would mean undoing 38 good ones. This is the
    /// per-mark reject that makes the batch genuinely reviewable.
    ///
    /// It is deliberately **not** a general `delete_annotation`. The guard
    /// below refuses anything whose `/Subtype` is not `/Redact`, so this
    /// command cannot become the back door through which a UI deletes an
    /// operator's highlights or a form's widgets without those features
    /// designing their own deletion semantics (dangling `/AcroForm`
    /// `/Fields`, `/Popup` companions, `/IRT` reply chains — none of which
    /// a redaction mark has).
    ///
    /// # What it changes
    ///
    /// 1. the annotation reference is dropped from its page's `/Annots`
    ///    (via the same [`Self::remove_from_annots`] helper flattening uses,
    ///    so the inline-array / shared-indirect-array / copy-on-write cases
    ///    are handled once, not twice);
    /// 2. the annotation dictionary and its `/AP` `/N` appearance stream are
    ///    marked deleted — the `/AP` too, because a redaction mark's
    ///    appearance stream is authored by
    ///    [`Self::add_redaction`] solely for that mark and is referenced by
    ///    nothing else, so leaving it would orphan a stream in every
    ///    subsequent save.
    ///
    /// **It changes nothing about the page's content.** A mark that was
    /// never applied never removed anything, so removing the mark restores
    /// no content — there is nothing to restore. This is the exact
    /// asymmetry the redaction feature turns on, and the reason this method
    /// is safe to offer with no confirmation while
    /// [`crate::redact::apply_redactions`] is not.
    ///
    /// # Errors
    ///
    /// [`EditError::NotARedactionMark`] if `annot_id` is not a `/Redact`
    /// annotation listed on a page (including a stale id from an already-
    /// removed mark); [`EditError::DocumentEncrypted`];
    /// [`EditError::CertificationForbidsChange`];
    /// [`EditError::PageTree`]. Every refusal happens before any mutation.
    pub fn delete_redaction_mark(&mut self, annot_id: ObjId) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        // Locate the mark by walking the SESSION's own page list and
        // annotations — never the base document — so a mark authored this
        // session is findable. `crate::redact::redaction_marks` is the one
        // definition of "what is a /Redact mark", reused rather than
        // re-derived, which is what keeps the review list, the status-bar
        // census and this deletion agreeing about the same set of objects.
        let marks = crate::redact::redaction_marks(&self.graph());
        let mark = marks
            .iter()
            .find(|m| m.annot_id == annot_id)
            .ok_or(EditError::NotARedactionMark { id: annot_id })?;
        let slots = self.page_slots()?;
        let page_id = slots
            .get(mark.page_index)
            .ok_or(EditError::NotARedactionMark { id: annot_id })?
            .id;

        // The mark's own appearance stream, if it has one. Resolved before
        // any mutation; a mark with no /AP simply contributes nothing.
        let ap_id = self
            .value(annot_id)
            .and_then(Object::as_dict)
            .and_then(|d| d.get(b"AP").cloned())
            .map(|ap| self.graph().resolve(&ap).clone())
            .and_then(|ap| {
                ap.as_dict()
                    .and_then(|d| d.get(b"N").and_then(Object::as_reference))
            });

        let Some(Object::Dict(page_dict)) = self.value(page_id) else {
            return Err(EditError::NotADictionary {
                id: page_id,
                key: "Annots",
            });
        };
        let mut updated = page_dict.clone();
        let mut objects: Vec<ObjectWrite> = Vec::new();
        // `remove_from_annots` returns `Some(write)` when `/Annots` is an
        // indirect array (the patch lands on THAT object, and the page dict
        // is untouched) and `None` when it is inline (the patch is already
        // composed into `updated`). Writing the page dict in the indirect
        // case would be a no-op write that inflates the dirty set.
        match self.remove_from_annots(&mut updated, &[annot_id])? {
            Some(shared) => objects.push(shared),
            None => objects.push(self.page_write(page_id, updated)),
        }

        let mut removals: Vec<Removal> = Vec::new();
        for id in std::iter::once(annot_id).chain(ap_id) {
            if self.base.get(id).is_some() || self.state.contains_key(&id) {
                removals.push(Removal {
                    id,
                    was_deleted: self.deleted.contains(&id),
                    is_deleted: true,
                });
            }
        }

        self.commit(Command {
            kind: CommandKind::DeleteRedactionMark,
            objects,
            removals,
            trailer: None,
        });
        Ok(())
    }

    /// Mark every occurrence of `query` in the document's extracted text
    /// for redaction (search-and-redact, Pass 8). Returns the created
    /// `/Redact` mark ids.
    ///
    /// This is the fuzzy-never-sneaky search half: it authors reviewable
    /// marks over the matched glyphs, never a silent removal. The match
    /// geometry comes from the Pass-4 per-glyph extraction, so a match is
    /// covered exactly where its glyphs sit on the page.
    ///
    /// # Errors
    ///
    /// [`EditError::TextExtraction`] if the document's text cannot be
    /// extracted, plus the guards of [`EditSession::add_redaction`].
    pub fn mark_redactions_by_search(
        &mut self,
        query: &str,
        case_insensitive: bool,
    ) -> Result<Vec<ObjId>, EditError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let q = query.to_string();
        self.author_text_matches(|text| find_matches(text, &q, case_insensitive))
    }

    /// Mark every match of a simple pattern for redaction (Pass 8). In the
    /// pattern, `#` matches any ASCII digit, `?` matches any single
    /// character, and every other character is a literal (case-insensitive
    /// when requested). Example: `###-##-####` marks US-SSN-shaped runs.
    ///
    /// The fuzzy-never-sneaky sibling of
    /// [`EditSession::mark_redactions_by_search`]: it authors reviewable
    /// marks, never a silent removal.
    ///
    /// # Errors
    ///
    /// As [`EditSession::mark_redactions_by_search`].
    pub fn mark_redactions_by_pattern(
        &mut self,
        pattern: &str,
        case_insensitive: bool,
    ) -> Result<Vec<ObjId>, EditError> {
        if pattern.is_empty() {
            return Ok(Vec::new());
        }
        let p = pattern.to_string();
        self.author_text_matches(|text| find_pattern_matches(text, &p, case_insensitive))
    }

    /// Shared engine for search/pattern redaction: extract the document's
    /// text, run `matcher` over each glyph run, turn each match's glyph
    /// geometry into a `/Redact` mark. The view borrow is released
    /// before any mark is authored.
    ///
    /// # SESSION READ, and why it has to be (Pass 17.1, decision 018 §8)
    ///
    /// This extracted from `self.document()` — the BASE revision — until
    /// Pass 17.1, and the page-index mismatch that created was a genuine
    /// mis-targeting hazard, not just staleness:
    ///
    /// - the page indices in `extracted.pages` were **base** page indices;
    /// - [`Self::add_redaction`], immediately below, resolves its
    ///   `page_index` through [`Self::page_slots`] — the **session** page
    ///   list.
    ///
    /// Those two agree only while the session has not deleted, inserted or
    /// reordered a page. After `delete_pages` or `reorder_pages`, a search
    /// redaction would have placed its mark on a **different page than the
    /// one holding the matched text** — silently, with correct-looking
    /// geometry, on the operation whose entire purpose is removing content
    /// the operator must not leak. Reading `self.view()` puts both sides in
    /// the same page-index space by construction.
    ///
    /// It also makes the search itself honest: text the operator typed this
    /// session is findable, and text they deleted is not offered for
    /// redaction.
    fn author_text_matches<F>(&mut self, matcher: F) -> Result<Vec<ObjId>, EditError>
    where
        F: Fn(&str) -> Vec<(usize, usize)>,
    {
        use crate::annot_author::{Quad, RedactSpec};
        use crate::page_tree::Rect;
        use crate::text_extract::{self, ExtractOptions, TextOrigin};
        use crate::vartext::Quadding;

        let extracted =
            text_extract::extract_document_view(&self.view(), &ExtractOptions::default())
                .map_err(|e| EditError::TextExtraction(e.to_string()))?;

        let mut matches: Vec<(usize, Quad)> = Vec::new();
        for page in &extracted.pages {
            for run in &page.runs {
                if run.origin != TextOrigin::Glyphs || run.glyphs.is_empty() {
                    continue;
                }
                for (start, end) in matcher(&run.text) {
                    let matched: Vec<_> = run
                        .glyphs
                        .iter()
                        .filter(|g| {
                            let gs = g.text_start as usize;
                            let ge = gs + g.text_len.max(1) as usize;
                            gs < end && start < ge
                        })
                        .collect();
                    if matched.is_empty() {
                        continue;
                    }
                    let mut llx = f64::INFINITY;
                    let mut lly = f64::INFINITY;
                    let mut urx = f64::NEG_INFINITY;
                    let mut ury = f64::NEG_INFINITY;
                    for g in &matched {
                        let x0 = f64::from(g.x);
                        let x1 = f64::from(g.x + g.advance);
                        let size = f64::from(g.size);
                        let y0 = f64::from(g.y) - 0.22 * size;
                        let y1 = f64::from(g.y) + 0.85 * size;
                        llx = llx.min(x0.min(x1));
                        urx = urx.max(x0.max(x1));
                        lly = lly.min(y0);
                        ury = ury.max(y1);
                    }
                    if llx.is_finite() && urx > llx {
                        matches.push((
                            page.page_index,
                            Quad::from_rect(Rect::from_corners(llx, lly, urx, ury)),
                        ));
                    }
                }
            }
        }

        let mut created = Vec::with_capacity(matches.len());
        for (page_index, quad) in matches {
            let spec = RedactSpec {
                quads: vec![quad],
                fill: None,
                overlay_text: None,
                quadding: Quadding::Left,
            };
            created.push(self.add_redaction(page_index, &spec)?);
        }
        Ok(created)
    }

    /// Author a text-bearing annotation onto a page (Pass 6.2): FreeText,
    /// Text (sticky note) + its `/Popup`, or Stamp.
    ///
    /// The text-family sibling of [`EditSession::add_markup`]. It shares
    /// every guard (X10 encryption, X11 conservative certification, the
    /// `/Size`-suppression refusal), the R45 staging buffer, and the X7
    /// `/Annots` copy-on-write path, and is likewise **one** undoable
    /// command (§11.3, R49) — for a sticky note, the appearance stream, the
    /// annotation, the `/Popup` companion, and the `/Annots` patch all undo
    /// together. It never touches a page content stream (R47).
    ///
    /// Returns the new annotation object's id (the note itself, not its
    /// popup).
    ///
    /// # Errors
    ///
    /// The same guards as [`EditSession::add_markup`], plus
    /// [`EditError::VariableText`] when the §12.7.3.3 appearance generation
    /// fails (e.g. a symbolic font chosen for a Latin text body).
    pub fn add_text_annotation(
        &mut self,
        page_index: usize,
        spec: &TextAnnotSpec,
    ) -> Result<ObjId, EditError> {
        // Guards, identical order to add_markup (X10, X11, /Size).
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        let slots = self.page_slots()?;
        let count = slots.len();
        let page_id = slots
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count,
            })?
            .id;

        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        // Generate the appearance + annotation dictionary (§12.7.3.3).
        let authored = annot_author::build_text_annotation(spec)?;

        // Allocate: appearance stream, annotation, and (if any) popup.
        let ap_id = ObjId::new(self.alloc_number()?, 0);
        let annot_id = ObjId::new(self.alloc_number()?, 0);
        let popup_id = if authored.popup.is_some() {
            Some(ObjId::new(self.alloc_number()?, 0))
        } else {
            None
        };

        // Stage the appearance bytes (R45) and build the /AP /N stream.
        let mut ap_dict = authored.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(authored.ap_content.len()).unwrap_or(i64::MAX)),
        );
        let ap_span = self.stage_bytes(&authored.ap_content);
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        // Complete the annotation dictionary: /AP /N, /P, /F, and — for a
        // sticky note — the /Popup back-link.
        let mut annot = authored.annot;
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        annot.insert(Name::from(b"AP"), Object::Dict(ap));
        annot.insert(Name::from(b"P"), Object::Reference(page_id));
        annot.insert(Name::from(b"F"), Object::Integer(i64::from(authored.flags)));
        if let Some(pid) = popup_id {
            annot.insert(Name::from(b"Popup"), Object::Reference(pid));
        }

        let mut objects = vec![
            ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(ap_stream),
            },
            ObjectWrite {
                id: annot_id,
                before: None,
                after: Some(Object::Dict(annot)),
            },
        ];

        // The /Popup companion object (Text only): /Parent points back at
        // the note; it carries no appearance (never painted — Pass 6.0 X4).
        let mut annot_refs = vec![annot_id];
        if let (Some(mut popup), Some(pid)) = (authored.popup, popup_id) {
            popup.insert(Name::from(b"Parent"), Object::Reference(annot_id));
            objects.push(ObjectWrite {
                id: pid,
                before: None,
                after: Some(Object::Dict(popup)),
            });
            annot_refs.push(pid);
        }

        // Patch /Annots with the note (and popup) in one command (X7).
        let mut annots_writes = self.annots_append(page_id, &annot_refs, &slots)?;
        objects.append(&mut annots_writes);

        self.commit(Command {
            kind: CommandKind::AddAnnotation {
                kind: text_annot_kind_of(spec),
            },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(annot_id)
    }

    /// The `/P`-aware certification gate for **form fill** (§12.8 VALIDATION
    /// MODEL; decision 009 §10).
    ///
    /// Form fill is DocMDP tier 2 — the tier whose documented purpose
    /// (Table 254 `/P = 2`) is *"filling in forms, … and digital
    /// signatures"*. So fill is **permitted at `/P >= 2`, including `/P`
    /// absent (default 2)**, and **refused by name only at `/P = 1`** (which
    /// forbids every change). This is deliberately **less strict** than the
    /// structural gate [`EditSession::check_certification`], which stays
    /// conservative (over-refusing a structural edit is still fail-clean-
    /// safe) — fill is the one operation the spec explicitly permits at the
    /// default tier, so hard-refusing it on every `/P 2` document would
    /// break the headline scenario.
    ///
    /// A `/FieldMDP` transform (§12.8.2.4) locks specific field *values*; a
    /// fill on a locked field would break that lock. pdfce does not yet
    /// resolve *which* fields a `/FieldMDP` locks, so it refuses fill
    /// conservatively whenever any `/FieldMDP` is present — a named
    /// over-refusal, precise per-field resolution a follow-up.
    /// **Would a form fill be refused on this document, and why?**
    ///
    /// `None` when filling is allowed; `Some(err)` carrying the exact refusal
    /// a fill would raise.
    ///
    /// # Why a shell needs to ask BEFORE it offers the control
    ///
    /// Standing rule R83: no affordance without the capability. A form panel
    /// that renders a live, focusable text box over a document whose
    /// certification signature forbids fills is promising something it cannot
    /// deliver — the operator types a value, tabs away, and gets a refusal
    /// instead of an edit. Disabling the row up front with the reason on it is
    /// the honest surface, and that requires knowing the answer before any
    /// mutation is attempted.
    ///
    /// # Why it returns the ERROR, not a `bool`
    ///
    /// The two refusals are different facts with different remedies —
    /// [`EditError::CertificationForbidsChange`] is about the whole document's
    /// permissions entry, [`EditError::FieldLockedBySignature`] about a
    /// `/FieldMDP` transform lock — and a `bool` would force the caller to
    /// invent its own wording for a distinction `pdfce-core` already knows.
    /// That is how a shell's message and the engine's message drift apart.
    ///
    /// This is a **pure query**: it reads the signature census and mutates
    /// nothing, so it is safe to call every frame from a UI.
    ///
    /// ```
    /// # use pdfce_core::{document::Document, edit::EditSession};
    /// # fn demo(doc: Document) {
    /// let session = EditSession::new(doc);
    /// if let Some(err) = session.fill_refusal() {
    ///     // Disable the control and show `err`, rather than offering a box
    ///     // that will reject whatever is typed into it.
    ///     eprintln!("form filling is not available: {err}");
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn fill_refusal(&self) -> Option<EditError> {
        self.check_certification_for_fill().err()
    }

    /// Why field/widget DELETION would refuse right now, or `None`.
    ///
    /// [`Self::fill_refusal`]'s sibling, and deliberately **not** the same
    /// query — a shell that reused `fill_refusal` to gate a delete control
    /// would offer deletion on a document that refuses it.
    ///
    /// # Why the gates differ, argued from what each operation does
    ///
    /// Filling uses the `/P`-aware gate: a certified document at
    /// `/P >= 2` permits form filling, because that is frequently what such
    /// a document was certified TO allow (§12.8.2.2 Table 257). Deleting a
    /// field is a **structural** change to the form itself, which is
    /// precisely what a certification signature exists to freeze — so
    /// [`Self::delete_field`] and [`Self::delete_widget`] take the STRICT
    /// gate through `deletion_preflight`.
    ///
    /// The consequence a caller must not get wrong: **there are documents
    /// where filling is offered and deletion is refused.** They are not
    /// rare — a certified fillable form is the ordinary case.
    ///
    /// A **pure query**: it reads the signature census and the trailer and
    /// mutates nothing, so it is safe to call every frame from a UI.
    ///
    /// ```
    /// # use pdfce_core::{document::Document, edit::EditSession};
    /// # fn demo(doc: Document) {
    /// let session = EditSession::new(doc);
    /// if let Some(err) = session.deletion_refusal() {
    ///     // Disable the delete control and show `err` (R83), rather than
    ///     // offering a button whose every press returns this same error.
    ///     eprintln!("field deletion is not available: {err}");
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn deletion_refusal(&self) -> Option<EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Some(EditError::DocumentEncrypted);
        }
        self.check_certification().err()
    }

    fn check_certification_for_fill(&self) -> Result<(), EditError> {
        let found = census(&self.graph());
        if found.perms_enforced && found.signatures > 0 && found.certification_permission == Some(1)
        {
            return Err(EditError::CertificationForbidsChange { permission: 1 });
        }
        if found.field_mdp > 0 {
            return Err(EditError::FieldLockedBySignature);
        }
        Ok(())
    }

    /// The shared preamble for every fill: the encryption guard, the
    /// `/P`-aware fill certification gate, and the `/Size`-suppression guard
    /// (a fill creates appearance-stream objects for text/choice fields).
    fn fill_guards(&self) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification_for_fill()?;
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }
        Ok(())
    }

    /// Set a **text** or **choice** field's value and regenerate its
    /// appearance (Pass 7, §12.7.3.3).
    ///
    /// Sets the field's `/V` to `text` (a §7.9.2 text string) and rebuilds
    /// the `/AP` `/N` of **every** widget of the field via the shared
    /// variable-text generator (R49 — the same §12.7.3.3 pipeline Pass 6.2's
    /// FreeText uses, reused for widgets). The whole fill — the field `/V`
    /// and every widget appearance — is **one** undoable command (§11.3).
    /// It never touches a page content stream (R47) and never rewrites the
    /// `/AcroForm` dictionary, so `/CO`/`/AA`/JavaScript carriers re-emit
    /// verbatim (decision 009 byte-preservation).
    ///
    /// Every field sharing `fqn` is updated together (§12.7.3.2 same-FQN
    /// representations share `/V`).
    ///
    /// # Errors
    ///
    /// - [`EditError::FieldNotFound`] — no such fillable field.
    /// - [`EditError::FieldNotFillable`] — the field is read-only, a button,
    ///   or a signature field.
    /// - [`EditError::VariableText`] — the appearance could not be generated
    ///   (a malformed/unresolvable `/DA`, or a symbolic font).
    /// - [`EditError::DocumentEncrypted`], the fill certification gate, the
    ///   `/Size`-suppression guard, [`EditError::ObjectNumbersExhausted`].
    pub fn fill_text_field(&mut self, fqn: &str, text: &str) -> Result<FillOutcome, EditError> {
        self.fill_text_field_inner(fqn, text, false)
    }

    /// **Fill a rich-text field with plain text, converting it to a plain
    /// field** — an explicit, conformant downgrade.
    ///
    /// # Why this exists as a SEPARATE, differently-named entry point
    ///
    /// [`Self::fill_text_field`] refuses a rich-text field, because writing
    /// `/V` while leaving `/RV` in place makes conforming readers regenerate
    /// the appearance from the OLD text (see
    /// [`EditError::FieldIsRichText`] for the two `shall`s that make that
    /// true). The refusal is right, but it leaves the field unfillable, and
    /// "unfillable forever" is not an acceptable answer to an operator who
    /// simply wants to type in a box.
    ///
    /// This is the way through, and it needs **no XHTML engine at all**:
    ///
    /// 1. clear `/Ff` bit 26 (`RichText`) — the field stops being one,
    /// 2. **delete `/RV`**, so no stale rich value can drive the appearance,
    /// 3. write `/V` and regenerate a plain appearance the ordinary way.
    ///
    /// The result is a perfectly ordinary text field holding exactly what was
    /// typed. Nothing is left inconsistent, and no reader has to guess.
    ///
    /// # It is LOSSY, and that is the caller's decision to disclose
    ///
    /// The stored formatting is discarded, not preserved-and-hidden. That is
    /// exactly why this is a separate method with an unmissable name rather
    /// than a flag on the ordinary fill: a caller cannot reach it by accident,
    /// and a UI that offers it is obliged to say what it costs (rule 4 —
    /// pdfce may not quietly convert an operator's document).
    ///
    /// Preserving the formatting instead means authoring `/RV` and generating
    /// its appearance — and ISO 32000-1 §12.7.3.3 explicitly switches off its
    /// own appearance conventions for these fields **without replacing them**,
    /// so that path is unspecified by the standard and is a much larger piece
    /// of work. Deferred deliberately; this is the honest, shippable half.
    ///
    /// # Errors
    ///
    /// As [`Self::fill_text_field`], except that
    /// [`EditError::FieldIsRichText`] is precisely the case this accepts.
    pub fn fill_text_field_downgrading_rich_text(
        &mut self,
        fqn: &str,
        text: &str,
    ) -> Result<FillOutcome, EditError> {
        self.fill_text_field_inner(fqn, text, true)
    }

    /// The shared body of the two fills above; `downgrade_rich_text` decides
    /// whether a rich-text field is refused or converted.
    fn fill_text_field_inner(
        &mut self,
        fqn: &str,
        text: &str,
        downgrade_rich_text: bool,
    ) -> Result<FillOutcome, EditError> {
        self.fill_guards()?;

        let form =
            forms::parse_acroform(&self.graph()).ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        let targets: Vec<Field> = form
            .fields
            .iter()
            .filter(|f| f.fully_qualified_name == fqn)
            .cloned()
            .collect();
        // The primary target dictates fillability; all same-FQN reps share it.
        let Some(primary) = targets.first() else {
            return Err(EditError::FieldNotFound {
                name: fqn.to_owned(),
            });
        };
        if !matches!(
            primary.field_type,
            Some(FieldType::Text | FieldType::Choice)
        ) || primary.flags.read_only()
        {
            return Err(EditError::FieldNotFillable {
                name: fqn.to_owned(),
            });
        }
        // THE RICH-TEXT GATE. `Field::is_rich_text()` and not a bare
        // `flags.has(RICH_TEXT)`: bit 26 is the only overloaded position in
        // the whole `/Ff` family (`RadiosInUnison` on a button), so the bare
        // test would misfire. The predicate resolves `/FT` first.
        let is_rich = primary.is_rich_text();
        if is_rich && !downgrade_rich_text {
            return Err(EditError::FieldIsRichText {
                name: fqn.to_owned(),
            });
        }
        let primary_id = primary.id;

        let fonts = self.resolve_dr_fonts(&form);
        let default_da = form
            .default_appearance
            .clone()
            .unwrap_or_else(|| b"/Helv 0 Tf 0 g".to_vec());

        let mut objects: Vec<ObjectWrite> = Vec::new();
        let mut widgets_updated = 0usize;
        let mut applied_autosize = None;
        let mut unencodable_chars = 0usize;
        // The new /V string, shared across every same-FQN representation.
        let v_string = Object::String(encode_text_string(text));

        for field in &targets {
            let multiline = field.flags.has(forms::FieldFlags::MULTILINE);

            // THE SHARED REGENERATOR (R92: one appearance path, never two).
            //
            // This loop used to be written out here — an exact copy of
            // `regen_field_appearance`'s body plus a counter. Two copies of
            // an appearance generator is the shape R92 exists to forbid, and
            // for a concrete reason rather than a tidiness one: a fill and an
            // appearance REGENERATION of the same field would have been free
            // to disagree about how a value is drawn, and the disagreement
            // would only show as "the field changed when I regenerated it
            // without editing it" — a defect with no obvious cause.
            //
            // A merged (Shape A) widget's `/AP` patch comes back rather than
            // being written, so it can be folded into the field-dict patch
            // below rather than written as a second object write.
            let merged_ap = self.regen_field_appearance(
                field,
                text,
                &default_da,
                multiline,
                &fonts,
                &mut objects,
                &mut applied_autosize,
                &mut unencodable_chars,
            )?;
            widgets_updated += field.widgets.len();

            // Patch the field dictionary: set /V, and (Shape A) its own /AP.
            let Some(Object::Dict(field_dict)) = self.value(field.id) else {
                return Err(EditError::NotADictionary {
                    id: field.id,
                    key: "V",
                });
            };
            let before = self.state.get(&field.id).cloned();
            let mut updated = field_dict.clone();
            updated.insert(Name::from(b"V"), v_string.clone());
            if is_rich {
                // THE DOWNGRADE, and both halves are load-bearing.
                //
                // Removing `/RV` alone would leave bit 26 set on a field with
                // no rich value — malformed. Clearing bit 26 alone would leave
                // a stale `/RV` sitting in the dictionary, which is exactly
                // the state a future reader (or a future pdfce) could resurrect
                // the old text from. So the flag and the value go together, in
                // one command, or the field is left in neither state.
                //
                // `/RV` is removed rather than emptied: §12.7.3.4 gives no
                // meaning to an empty rich value, and an absent key is the
                // unambiguous way to say "this field has none".
                updated.remove(b"RV");
                // `/DS` (default style, Table 222) goes with it — it exists to
                // style the rich value and means nothing without one.
                updated.remove(b"DS");
                // Bit 26 cleared on the field's OWN `/Ff` only when it has one.
                // The resolved flags may have been INHERITED through /Parent
                // (§12.7.3.1); writing a synthesised /Ff onto a child that had
                // none would silently sever that inheritance for every OTHER
                // flag too, which is a much larger change than asked for.
                if let Some(own) = field_dict.get(b"Ff").and_then(Object::as_int) {
                    let cleared = own & !i64::from(forms::FieldFlags::RICH_TEXT);
                    updated.insert(Name::from(b"Ff"), Object::Integer(cleared));
                }
            }
            if let Some(ap_id) = merged_ap {
                let mut ap = Dict::new();
                ap.insert(Name::from(b"N"), Object::Reference(ap_id));
                updated.insert(Name::from(b"AP"), Object::Dict(ap));
            }
            objects.push(ObjectWrite {
                id: field.id,
                before,
                after: Some(Object::Dict(updated)),
            });
        }

        self.commit(Command {
            kind: CommandKind::FillTextField,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FillOutcome {
            field_id: primary_id,
            widgets_updated,
            applied_autosize,
            unencodable_chars,
        })
    }

    /// Select a **check-box** or **radio-button** field's state (Pass 7,
    /// §12.7.4.2.3).
    ///
    /// Sets the field's `/V` to the state name and each widget's `/AS` to the
    /// matching state — a widget offering `on_state` shows it, every other
    /// widget shows `Off`. This is **state selection, not generation**: the
    /// per-state appearances are pre-authored, so **no appearance is
    /// regenerated** (the fundamental Btn/Tx split). `RadiosInUnison` falls
    /// out for free — every kid whose on-state name equals `on_state` turns
    /// on together. Pass `Off` (or an empty string) to clear the field.
    ///
    /// The whole selection is **one** undoable command (§11.3).
    ///
    /// # Errors
    ///
    /// - [`EditError::FieldNotFound`] — no such button field.
    /// - [`EditError::FieldNotFillable`] — the field is read-only or not a
    ///   check-box/radio.
    /// - [`EditError::FieldStateUnknown`] — no widget defines `on_state`.
    /// - The fill certification/encryption guards.
    pub fn set_button_state(&mut self, fqn: &str, on_state: &str) -> Result<(), EditError> {
        // Buttons author no new objects, so the /Size guard is not needed;
        // the encryption + certification guards still are.
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification_for_fill()?;

        let form =
            forms::parse_acroform(&self.graph()).ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        let targets: Vec<Field> = form
            .fields
            .iter()
            .filter(|f| f.fully_qualified_name == fqn)
            .cloned()
            .collect();
        let Some(primary) = targets.first() else {
            return Err(EditError::FieldNotFound {
                name: fqn.to_owned(),
            });
        };
        if !matches!(
            primary.button_kind,
            Some(ButtonKind::Check | ButtonKind::Radio)
        ) || primary.flags.read_only()
        {
            return Err(EditError::FieldNotFillable {
                name: fqn.to_owned(),
            });
        }

        // Normalise the requested state; empty ⇒ Off (§12.7.4.2.3 off name).
        let want = if on_state.is_empty() { "Off" } else { on_state };
        let want_bytes = want.as_bytes();

        // Unless clearing to Off, the state must be one some widget defines.
        if want != "Off" {
            let mut available: Vec<String> = Vec::new();
            let mut found = false;
            for field in &targets {
                for w in &field.widgets {
                    for s in &w.on_states {
                        if s == want_bytes {
                            found = true;
                        }
                        available.push(String::from_utf8_lossy(s).into_owned());
                    }
                }
            }
            if !found {
                available.sort();
                available.dedup();
                return Err(EditError::FieldStateUnknown {
                    name: fqn.to_owned(),
                    state: want.to_owned(),
                    available,
                });
            }
        }

        let mut objects: Vec<ObjectWrite> = Vec::new();
        let v_name = Object::Name(Name(want_bytes.to_vec()));

        for field in &targets {
            // Each widget's /AS: the requested state if this widget offers it,
            // else Off (a radio's non-selected kids, or a cleared checkbox).
            for widget in &field.widgets {
                let this_state: &[u8] =
                    if want != "Off" && widget.on_states.iter().any(|s| s == want_bytes) {
                        want_bytes
                    } else {
                        b"Off"
                    };
                if widget.merged {
                    // Shape A: /AS lives on the field dict, patched below with
                    // /V — but a merged checkbox is its own single widget, so
                    // fold /AS in with the field patch.
                    continue;
                }
                objects.push(self.set_widget_as(widget.id, this_state)?);
            }

            // Patch the field dict: /V, and (merged) its own /AS.
            let Some(Object::Dict(field_dict)) = self.value(field.id) else {
                return Err(EditError::NotADictionary {
                    id: field.id,
                    key: "V",
                });
            };
            let before = self.state.get(&field.id).cloned();
            let mut updated = field_dict.clone();
            updated.insert(Name::from(b"V"), v_name.clone());
            if let Some(w) = field.widgets.iter().find(|w| w.merged) {
                let this_state: &[u8] =
                    if want != "Off" && w.on_states.iter().any(|s| s == want_bytes) {
                        want_bytes
                    } else {
                        b"Off"
                    };
                updated.insert(Name::from(b"AS"), Object::Name(Name(this_state.to_vec())));
            }
            objects.push(ObjectWrite {
                id: field.id,
                before,
                after: Some(Object::Dict(updated)),
            });
        }

        self.commit(Command {
            kind: CommandKind::SetButtonState,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    /// An [`ObjectWrite`] setting a Shape-B widget dict's `/AP` `/N` to
    /// `ap_id`, preserving every other key (round-trip; JS carriers, `/MK`,
    /// `/DA`, `/Parent` are untouched).
    fn set_widget_ap(&self, widget_id: ObjId, ap_id: ObjId) -> Result<ObjectWrite, EditError> {
        let Some(Object::Dict(dict)) = self.value(widget_id) else {
            return Err(EditError::NotADictionary {
                id: widget_id,
                key: "AP",
            });
        };
        let before = self.state.get(&widget_id).cloned();
        let mut updated = dict.clone();
        // Preserve /R and /D if the /AP was a dict; overwrite /N only.
        let mut ap = match updated.get(b"AP").and_then(Object::as_dict) {
            Some(existing) => existing.clone(),
            None => Dict::new(),
        };
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        updated.insert(Name::from(b"AP"), Object::Dict(ap));
        Ok(ObjectWrite {
            id: widget_id,
            before,
            after: Some(Object::Dict(updated)),
        })
    }

    /// An [`ObjectWrite`] setting a Shape-B widget dict's `/AS` to `state`,
    /// preserving every other key.
    fn set_widget_as(&self, widget_id: ObjId, state: &[u8]) -> Result<ObjectWrite, EditError> {
        let Some(Object::Dict(dict)) = self.value(widget_id) else {
            return Err(EditError::NotADictionary {
                id: widget_id,
                key: "AS",
            });
        };
        let before = self.state.get(&widget_id).cloned();
        let mut updated = dict.clone();
        updated.insert(Name::from(b"AS"), Object::Name(Name(state.to_vec())));
        Ok(ObjectWrite {
            id: widget_id,
            before,
            after: Some(Object::Dict(updated)),
        })
    }

    /// Regenerate every widget appearance of `field` to display `text`,
    /// pushing the created `/AP` stream writes (and Shape-B widget `/AP`
    /// patches) onto `objects`, and returning the merged-widget `/AP` stream
    /// id (Shape A) to fold into the field-dictionary patch.
    ///
    /// The single §12.7.3.3 appearance engine (R49) — shared by
    /// [`EditSession::fill_text_field`], [`EditSession::set_choice_value`],
    /// [`EditSession::regenerate_appearances`], and form-data import — so
    /// every path that authors a widget appearance authors it identically.
    #[allow(clippy::too_many_arguments)]
    fn regen_field_appearance(
        &mut self,
        field: &Field,
        text: &str,
        default_da: &[u8],
        multiline: bool,
        fonts: &[FontResource],
        objects: &mut Vec<ObjectWrite>,
        applied_autosize: &mut Option<f64>,
        unencodable: &mut usize,
    ) -> Result<Option<ObjId>, EditError> {
        let da = field
            .default_appearance
            .clone()
            .unwrap_or_else(|| default_da.to_vec());
        let quad = field.quadding;
        let mut merged_ap: Option<ObjId> = None;
        for widget in &field.widgets {
            let (w, h) = widget.rect.map_or((0.0, 0.0), |r| (r.width(), r.height()));
            let appearance =
                annot_author::build_field_text_appearance(w, h, text, &da, quad, multiline, fonts)?;
            if appearance.applied_autosize.is_some() {
                *applied_autosize = appearance.applied_autosize;
            }
            *unencodable += appearance.unencodable_chars;

            let ap_id = ObjId::new(self.alloc_number()?, 0);
            let mut ap_dict = appearance.ap_dict;
            ap_dict.insert(
                Name::from(b"Length"),
                Object::Integer(i64::try_from(appearance.content.len()).unwrap_or(i64::MAX)),
            );
            let span = self.stage_bytes(&appearance.content);
            objects.push(ObjectWrite {
                id: ap_id,
                before: None,
                after: Some(Object::Stream(Stream {
                    dict: ap_dict,
                    data_span: span,
                })),
            });
            if widget.merged {
                merged_ap = Some(ap_id);
            } else {
                objects.push(self.set_widget_ap(widget.id, ap_id)?);
            }
        }
        Ok(merged_ap)
    }

    /// Set a **choice** field's selection(s) and regenerate its appearance
    /// (Pass 7.1, §12.7.4.4).
    ///
    /// Each string in `selections` is matched against the field's `/Opt`
    /// options by **export value first, then display value**; the matched
    /// **export** value is stored in `/V` (§12.7.4.4: `/V` holds the export
    /// value), and the matched **display** value drives the regenerated
    /// appearance (that is what the user sees). An editable combo box
    /// (`Combo` + `Edit`, §12.7.4.1 bits 18/19) additionally accepts a
    /// **free-text** value not present in `/Opt`.
    ///
    /// `/V` is a single string for one selection and — only when the field is
    /// `MultiSelect` (bit 22) — an **array** for several. The `/I`
    /// selected-index array (§12.7.4.4) is rewritten to the matched option
    /// indices (a free-text combo value contributes no index), or removed
    /// when nothing matched an option. Every same-FQN representation is
    /// updated together.
    ///
    /// # Errors
    ///
    /// - [`EditError::FieldNotFound`] / [`EditError::FieldNotFillable`] — no
    ///   such choice field, or it is read-only.
    /// - [`EditError::ChoiceRequiresMultiSelect`] — several values on a
    ///   single-select field.
    /// - [`EditError::ChoiceValueNotInOptions`] — an unknown value on a
    ///   non-editable choice.
    /// - The shared fill guards and [`EditError::VariableText`].
    pub fn set_choice_value(
        &mut self,
        fqn: &str,
        selections: &[&str],
    ) -> Result<FillOutcome, EditError> {
        self.fill_guards()?;

        let form =
            forms::parse_acroform(&self.graph()).ok_or_else(|| EditError::FieldNotFound {
                name: fqn.to_owned(),
            })?;
        let targets: Vec<Field> = form
            .fields
            .iter()
            .filter(|f| f.fully_qualified_name == fqn)
            .cloned()
            .collect();
        let Some(primary) = targets.first() else {
            return Err(EditError::FieldNotFound {
                name: fqn.to_owned(),
            });
        };
        if primary.field_type != Some(FieldType::Choice) || primary.flags.read_only() {
            return Err(EditError::FieldNotFillable {
                name: fqn.to_owned(),
            });
        }
        let multi = primary.flags.has(forms::FieldFlags::MULTI_SELECT);
        let editable_combo = primary.flags.has(forms::FieldFlags::COMBO)
            && primary.flags.has(forms::FieldFlags::EDIT);
        let list_box = !primary.flags.has(forms::FieldFlags::COMBO);
        if selections.len() > 1 && !multi {
            return Err(EditError::ChoiceRequiresMultiSelect {
                name: fqn.to_owned(),
                count: selections.len(),
            });
        }

        // Resolve each selection to (export, display, matched-index?).
        let mut export_values: Vec<Vec<u8>> = Vec::new();
        let mut display_values: Vec<String> = Vec::new();
        let mut indices: Vec<i64> = Vec::new();
        for sel in selections {
            match match_option(&primary.options, sel) {
                Some((idx, opt)) => {
                    export_values.push(opt.export.clone());
                    display_values.push(String::from_utf8_lossy(&opt.display).into_owned());
                    indices.push(i64::try_from(idx).unwrap_or(0));
                }
                None if editable_combo => {
                    // Free-text combo value: export == display == the typed text.
                    export_values.push(encode_text_string(sel));
                    display_values.push((*sel).to_owned());
                }
                None => {
                    let available: Vec<String> = primary
                        .options
                        .iter()
                        .map(|o| String::from_utf8_lossy(&o.display).into_owned())
                        .collect();
                    return Err(EditError::ChoiceValueNotInOptions {
                        name: fqn.to_owned(),
                        value: (*sel).to_owned(),
                        available,
                    });
                }
            }
        }

        // /V: single string, or an array under MultiSelect.
        let v_object = if multi {
            Object::Array(
                export_values
                    .iter()
                    .map(|b| Object::String(b.clone()))
                    .collect(),
            )
        } else {
            Object::String(export_values.first().cloned().unwrap_or_default())
        };
        // The appearance shows the display value(s): joined by newline for a
        // (multiline) list box, a single line for a combo.
        let display_text = display_values.join("\n");

        let fonts = self.resolve_dr_fonts(&form);
        let default_da = form
            .default_appearance
            .clone()
            .unwrap_or_else(|| b"/Helv 0 Tf 0 g".to_vec());

        let mut objects: Vec<ObjectWrite> = Vec::new();
        let mut widgets_updated = 0usize;
        let mut applied_autosize = None;
        let mut unencodable_chars = 0usize;

        for field in &targets {
            let before_len = objects.len();
            let merged_ap = self.regen_field_appearance(
                field,
                &display_text,
                &default_da,
                list_box,
                &fonts,
                &mut objects,
                &mut applied_autosize,
                &mut unencodable_chars,
            )?;
            widgets_updated += field.widgets.len().max(objects.len() - before_len);

            let Some(Object::Dict(field_dict)) = self.value(field.id) else {
                return Err(EditError::NotADictionary {
                    id: field.id,
                    key: "V",
                });
            };
            let before = self.state.get(&field.id).cloned();
            let mut updated = field_dict.clone();
            updated.insert(Name::from(b"V"), v_object.clone());
            // /I selected-index array (§12.7.4.4): present when options matched.
            if indices.is_empty() {
                updated.remove(b"I");
            } else {
                updated.insert(
                    Name::from(b"I"),
                    Object::Array(indices.iter().map(|i| Object::Integer(*i)).collect()),
                );
            }
            if let Some(ap_id) = merged_ap {
                let mut ap = Dict::new();
                ap.insert(Name::from(b"N"), Object::Reference(ap_id));
                updated.insert(Name::from(b"AP"), Object::Dict(ap));
            }
            objects.push(ObjectWrite {
                id: field.id,
                before,
                after: Some(Object::Dict(updated)),
            });
        }

        let primary_id = primary.id;
        self.commit(Command {
            kind: CommandKind::SetChoiceValue,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(FillOutcome {
            field_id: primary_id,
            widgets_updated,
            applied_autosize,
            unencodable_chars,
        })
    }

    /// Build the [`FontResource`] set the variable-text generator resolves a
    /// field `/DA` font name against, from the AcroForm `/DR` `/Font`
    /// (§12.7.2 / §12.7.3.3).
    ///
    /// Each `/DR` `/Font` entry maps a resource name to a font dictionary;
    /// pdfce resolves its `/BaseFont` to a standard-14 face (the Base-14
    /// generator's remit — an embedded/CID form font is not laid out by this
    /// path, and its name simply falls through to the Helvetica default). A
    /// synthetic `Helv → Helvetica` entry is always included so the common
    /// `/DA /Helv …` resolves even when a producer omitted `/DR`.
    fn resolve_dr_fonts(&self, _form: &forms::AcroForm) -> Vec<FontResource> {
        let mut out = vec![FontResource {
            name: b"Helv".to_vec(),
            font: Std14::Helvetica,
        }];
        let graph = self.graph();
        let dr_font = graph
            .catalog_dict()
            .and_then(|c| c.get(b"AcroForm").map(|o| graph.resolve(o)))
            .and_then(Object::as_dict)
            .and_then(|af| af.get(b"DR").map(|o| graph.resolve(o)))
            .and_then(Object::as_dict)
            .and_then(|dr| dr.get(b"Font").map(|o| graph.resolve(o)))
            .and_then(Object::as_dict)
            .cloned();
        if let Some(fonts) = dr_font {
            for (name, val) in fonts.iter() {
                let base = graph
                    .resolve(val)
                    .as_dict()
                    .and_then(|fd| fd.get(b"BaseFont"))
                    .and_then(Object::as_name)
                    .and_then(|n| basefont_to_std14(n.as_bytes()))
                    .unwrap_or(Std14::Helvetica);
                let nm = name.as_bytes().to_vec();
                if !out.iter().any(|r| r.name == nm) {
                    out.push(FontResource {
                        name: nm,
                        font: base,
                    });
                }
            }
        }
        out
    }

    // -- Pass 7.1: export/import, regenerate, flatten -------------------

    /// Export the document's filled form-field data (Pass 7.1, §12.7.7).
    ///
    /// A read-only projection of the modelled `/AcroForm` into a
    /// format-independent [`FormData`](crate::fdf::FormData) the caller
    /// serializes to FDF or XFDF. `None` when the document has no form.
    #[must_use]
    pub fn export_form_data(&self) -> Option<crate::fdf::FormData> {
        forms::parse_acroform(&self.graph()).map(|form| crate::fdf::FormData::from_acroform(&form))
    }

    /// Import form-field data (Pass 7.1): set each named field's value and
    /// regenerate its appearance.
    ///
    /// Each [`FieldData`](crate::fdf::FieldData) is dispatched by the
    /// **target** field's modelled type — text via [`fill_text_field`], a
    /// checkbox/radio via [`set_button_state`], a choice via
    /// [`set_choice_value`] — so the same data file applies correctly
    /// whatever the document's field types are. A named field the document
    /// does not have is **counted and skipped**, never an error (a data file
    /// may name a superset), the fuzzy-never-sneaky posture. Each field is
    /// its own undoable command.
    ///
    /// [`fill_text_field`]: EditSession::fill_text_field
    /// [`set_button_state`]: EditSession::set_button_state
    /// [`set_choice_value`]: EditSession::set_choice_value
    ///
    /// # Errors
    ///
    /// [`EditError::NoInteractiveForm`] when the document has no `/AcroForm`;
    /// any fill error raised while applying a matched field (the guards,
    /// [`EditError::VariableText`], …).
    pub fn import_form_data(
        &mut self,
        data: &crate::fdf::FormData,
    ) -> Result<ImportOutcome, EditError> {
        if forms::parse_acroform(&self.graph()).is_none() {
            return Err(EditError::NoInteractiveForm);
        }
        let mut applied = 0usize;
        let mut skipped = 0usize;
        for entry in &data.fields {
            // Re-model each time so later imports see earlier overlay writes.
            let Some(form) = forms::parse_acroform(&self.graph()) else {
                return Err(EditError::NoInteractiveForm);
            };
            let Some(field) = form.field_by_name(&entry.name) else {
                skipped += 1;
                continue;
            };
            match field.field_type {
                Some(FieldType::Button) => {
                    let state = entry.values.first().map_or("Off", String::as_str);
                    self.set_button_state(&entry.name, state)?;
                }
                Some(FieldType::Choice) => {
                    let sel: Vec<&str> = entry.values.iter().map(String::as_str).collect();
                    self.set_choice_value(&entry.name, &sel)?;
                }
                Some(FieldType::Text) => {
                    let text = entry.values.first().map_or("", String::as_str);
                    self.fill_text_field(&entry.name, text)?;
                }
                Some(FieldType::Signature) | None => {
                    skipped += 1;
                    continue;
                }
            }
            applied += 1;
        }
        Ok(ImportOutcome { applied, skipped })
    }

    /// Regenerate widget appearances that need it and clear
    /// `/NeedAppearances` on pdfce's own output (Pass 7.1, R51).
    ///
    /// A save-side operation: for every text/choice field that either has no
    /// usable `/AP` or whose document sets `/NeedAppearances true`, the
    /// widget appearance is rebuilt from the field's current `/V` through the
    /// shared §12.7.3.3 generator, and the AcroForm's `/NeedAppearances` flag
    /// is **removed** — pdfce never emits a stale "appearances need
    /// regenerating" assertion on a file it just regenerated (R51). Buttons
    /// are untouched (their appearances are pre-authored state selections,
    /// not generated). One undoable command.
    ///
    /// This is the one form operation that touches the `/AcroForm`
    /// dictionary (to clear the flag); the JS carriers it also holds (`/CO`,
    /// `/DR`, `/DA`) re-emit unchanged, and no `/JS` is stripped
    /// (decision 009).
    ///
    /// # Errors
    ///
    /// [`EditError::NoInteractiveForm`]; the shared fill guards;
    /// [`EditError::VariableText`].
    pub fn regenerate_appearances(&mut self) -> Result<RegenOutcome, EditError> {
        self.fill_guards()?;
        let form = forms::parse_acroform(&self.graph()).ok_or(EditError::NoInteractiveForm)?;
        let want_all = form.need_appearances;
        let fonts = self.resolve_dr_fonts(&form);
        let default_da = form
            .default_appearance
            .clone()
            .unwrap_or_else(|| b"/Helv 0 Tf 0 g".to_vec());

        let mut objects: Vec<ObjectWrite> = Vec::new();
        let mut regenerated = 0usize;
        let mut applied_autosize = None;
        let mut unencodable = 0usize;

        for field in &form.fields {
            if field.widgets.is_empty() {
                continue;
            }
            // Only regenerate what needs it: a field with no usable /AP, or
            // (when /NeedAppearances is set) every variable-text field.
            if !want_all && field.has_appearance() {
                continue;
            }
            let (display, multiline) = match field.field_type {
                Some(FieldType::Text) => match &field.value {
                    forms::FieldValue::Text(b) => (
                        decode_text_string(b).text,
                        field.flags.has(forms::FieldFlags::MULTILINE),
                    ),
                    _ => continue,
                },
                Some(FieldType::Choice) => match choice_display_text(field) {
                    Some(t) => (t, !field.flags.has(forms::FieldFlags::COMBO)),
                    None => continue,
                },
                _ => continue,
            };
            let merged_ap = self.regen_field_appearance(
                field,
                &display,
                &default_da,
                multiline,
                &fonts,
                &mut objects,
                &mut applied_autosize,
                &mut unencodable,
            )?;
            regenerated += 1;
            // Shape A: fold the new /AP /N into the field dictionary.
            if let Some(ap_id) = merged_ap {
                let Some(Object::Dict(fd)) = self.value(field.id) else {
                    continue;
                };
                let before = self.state.get(&field.id).cloned();
                let mut updated = fd.clone();
                let mut ap = Dict::new();
                ap.insert(Name::from(b"N"), Object::Reference(ap_id));
                updated.insert(Name::from(b"AP"), Object::Dict(ap));
                objects.push(ObjectWrite {
                    id: field.id,
                    before,
                    after: Some(Object::Dict(updated)),
                });
            }
        }

        // Clear /NeedAppearances on our own output (R51).
        let cleared = self.clear_need_appearances_write(&mut objects);

        if objects.is_empty() {
            return Ok(RegenOutcome {
                regenerated: 0,
                need_appearances_cleared: false,
                applied_autosize,
                unencodable_chars: unencodable,
            });
        }
        self.commit(Command {
            kind: CommandKind::RegenerateAppearances { count: regenerated },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(RegenOutcome {
            regenerated,
            need_appearances_cleared: cleared,
            applied_autosize,
            unencodable_chars: unencodable,
        })
    }

    /// Flatten form fields: burn each field's appearance into its page's
    /// content and remove the fields from `/AcroForm` and `/Annots`
    /// (Pass 7.1, R48). **Destructive** — the fields stop being interactive.
    ///
    /// `names = Some(&[…])` flattens exactly the named fields; `None`
    /// flattens every field. This is the project's **first modification of a
    /// page's rendered content**: for each flattened widget, its existing
    /// `/AP` `/N` appearance (an existing form XObject) is added to the
    /// page's `/Resources` `/XObject` and invoked by a `q <cm> /Name Do Q`
    /// overlay authored **as a new content stream appended to the page's
    /// `/Contents` array** (§7.8.2). The page's *original* content streams
    /// stay **byte-verbatim** — only a new stream is added and the page dict
    /// re-pointed — so the R46 content-identity gate is unperturbed (no
    /// existing stream is re-emitted; this is a deliberately more
    /// minimal-diff design than an in-place stream rewrite). The §12.5.5
    /// placement `cm` maps the appearance's `/BBox` (via its `/Matrix`) onto
    /// the widget's `/Rect`.
    ///
    /// The flattened field and widget dictionaries are **deleted** (§7.5.4):
    /// under a **full rewrite** they are omitted entirely (the field data is
    /// physically gone); under the default **incremental** save the prior
    /// revision still holds them, so the pre-flatten value is recoverable in
    /// the earlier revision — the R35 sibling. This is the R48
    /// destructive-disclosure the caller must surface: incremental flatten is
    /// reversible-by-revision until a full rewrite removes it.
    ///
    /// Flatten is **structural** (it removes fields), so it uses the strict
    /// certification gate ([`EditSession::check_certification`]): a certified
    /// document refuses flatten by name, unlike the `/P >= 2` fill gate.
    ///
    /// # Errors
    ///
    /// - [`EditError::NoInteractiveForm`] — the document has no `/AcroForm`.
    /// - [`EditError::FieldNotFound`] — a named field is absent.
    /// - [`EditError::CertificationForbidsChange`] — a certified document.
    /// - [`EditError::DocumentEncrypted`],
    ///   [`EditError::ObjectCreationWouldExposeHiddenObjects`],
    ///   [`EditError::ObjectNumbersExhausted`], [`EditError::PageTree`].
    pub fn flatten_fields(&mut self, names: Option<&[&str]>) -> Result<FlattenOutcome, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        // STRICT gate: flatten removes structure, so it uses the same
        // conservative refusal the page ops use — a certified document is
        // refused by name.
        self.check_certification()?;
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        let form = forms::parse_acroform(&self.graph()).ok_or(EditError::NoInteractiveForm)?;
        let slots = self.page_slots()?;

        // Select the fields to flatten (all, or the named subset).
        let targets: Vec<Field> = match names {
            None => form.fields.clone(),
            Some(ns) => {
                for n in ns {
                    if !form.fields.iter().any(|f| f.fully_qualified_name == *n) {
                        return Err(EditError::FieldNotFound {
                            name: (*n).to_owned(),
                        });
                    }
                }
                form.fields
                    .iter()
                    .filter(|f| ns.iter().any(|n| f.fully_qualified_name == *n))
                    .cloned()
                    .collect()
            }
        };

        // Phase 1 (reads only): plan the per-page burns and the object
        // deletions, cloning everything so no borrow is held into phase 2.
        let mut per_page: BTreeMap<ObjId, PageFlatten> = BTreeMap::new();
        let mut delete_ids: Vec<ObjId> = Vec::new();
        let mut widget_removals: BTreeMap<ObjId, Vec<ObjId>> = BTreeMap::new();
        let mut xobj_counter = 0u32;
        let mut widgets_burned = 0usize;

        for field in &targets {
            for widget in &field.widgets {
                let Some(page_id) = self.page_of_widget(widget, &slots) else {
                    continue;
                };
                let Some((ap_id, bbox, matrix)) = self.burn_target(widget) else {
                    continue;
                };
                let Some(rect) = widget.rect else {
                    continue;
                };
                let cm = fit_matrix_for(bbox, matrix, rect);
                xobj_counter += 1;
                let name = format!("pdfceFm{xobj_counter}").into_bytes();
                let entry = per_page.entry(page_id).or_default();
                entry.invocations.push((name.clone(), cm));
                entry.xobjects.push((name, ap_id));
                widget_removals.entry(page_id).or_default().push(widget.id);
                widgets_burned += 1;
                // A Shape-B widget is its own object to delete; a merged
                // widget's object IS the field dict, deleted with the field.
                if !widget.merged {
                    delete_ids.push(widget.id);
                }
            }
            delete_ids.push(field.id);
        }

        // Phase 2 (writes): overlay content, resource + /Contents patches,
        // /Annots removals, /Fields removal, object deletions.
        let mut objects: Vec<ObjectWrite> = Vec::new();
        let mut removals: Vec<Removal> = Vec::new();

        for (page_id, pf) in &per_page {
            // The overlay content stream: `q cm /Name Do Q` per widget.
            let mut cb = ContentBuilder::new();
            for (name, cm) in &pf.invocations {
                cb.save_state();
                cb.concat_matrix(cm[0], cm[1], cm[2], cm[3], cm[4], cm[5]);
                cb.invoke_xobject(name);
                cb.restore_state();
            }
            let content = cb.into_bytes();
            let overlay_id = ObjId::new(self.alloc_number()?, 0);
            let mut sdict = Dict::new();
            sdict.insert(
                Name::from(b"Length"),
                Object::Integer(i64::try_from(content.len()).unwrap_or(i64::MAX)),
            );
            let span = self.stage_bytes(&content);
            objects.push(ObjectWrite {
                id: overlay_id,
                before: None,
                after: Some(Object::Stream(Stream {
                    dict: sdict,
                    data_span: span,
                })),
            });
            // ONE page-dict write carrying ALL THREE page-dict mutations.
            //
            // ## Why this is one write and not three (Pass 17.2 bug fix)
            //
            // Flatten changes three entries of the same page dictionary:
            // `/Contents` (append the burn stream), `/Resources /XObject`
            // (name the appearance forms it invokes) and `/Annots` (drop the
            // widgets it replaced). Until Pass 17.2 those were three separate
            // [`ObjectWrite`]s for the same `page_id`, each built by cloning
            // the page dict as it stood **before** the command — none of them
            // could see the others, because nothing is committed until the
            // whole command is. Applying them in order therefore did not
            // compose; it **overwrote**, and the last write (`/Annots`) won.
            //
            // The result was silent visual data loss on every flatten: the
            // fields and widgets were deleted as designed, the burn stream
            // and the appearance XObjects were created as designed — and the
            // page referenced neither, so the flattened appearance rendered
            // as nothing at all. Every existing flatten test asserted the
            // outcome COUNTS (`fields_flattened`, `widgets_burned`,
            // `pages_touched`), which were all correct, and no test rendered
            // the result. Found by the R85 preview-equals-saved oracle
            // (`crates/pdfce-render/tests/preview_equals_saved.rs`), whose
            // `flatten` case asserts what an operator assumes without being
            // told: that flattening does not change how the page looks.
            //
            // The rule this encodes: **at most one `ObjectWrite` per object
            // id per command.** A second write to an id already in the batch
            // is not an edit to the first — it is a replacement of it.
            let Some(Object::Dict(page_dict)) = self.value(*page_id) else {
                return Err(EditError::NotADictionary {
                    id: *page_id,
                    key: "Contents",
                });
            };
            let mut updated = page_dict.clone();
            self.append_page_content(&mut updated, overlay_id);
            self.add_page_xobjects(&mut updated, *page_id, &pf.xobjects, &slots);
            // The /Annots removal composes into the SAME dict when the array
            // is inline; when `/Annots` is an indirect array shared with
            // other pages it is a different object, so it returns its own
            // write (which cannot collide with this one).
            if let Some(widget_ids) = widget_removals.get(page_id)
                && let Some(shared) = self.remove_from_annots(&mut updated, widget_ids)?
            {
                objects.push(shared);
            }
            objects.push(ObjectWrite {
                id: *page_id,
                before: self.state.get(page_id).cloned(),
                after: Some(Object::Dict(updated)),
            });
        }

        // Any page that had widgets to un-list but no burn of its own (no
        // appearance to burn, so `per_page` has no entry for it) still needs
        // its /Annots pruned. Handled separately because the loop above owns
        // the page-dict write for every page it touched.
        for (page_id, widget_ids) in &widget_removals {
            if per_page.contains_key(page_id) {
                continue; // already composed into that page's single write
            }
            let Some(Object::Dict(page_dict)) = self.value(*page_id) else {
                continue;
            };
            let mut updated = page_dict.clone();
            let shared = self.remove_from_annots(&mut updated, widget_ids)?;
            if let Some(shared) = shared {
                objects.push(shared);
            } else {
                objects.push(ObjectWrite {
                    id: *page_id,
                    before: self.state.get(page_id).cloned(),
                    after: Some(Object::Dict(updated)),
                });
            }
        }

        // Remove flattened fields from /AcroForm /Fields (and any parent
        // /Kids). Root fields drop from /Fields; nested ones from /Parent.
        let field_ids: Vec<ObjId> = targets.iter().map(|f| f.id).collect();
        let (form_writes, emptied_parents) = self.remove_fields_from_form(&field_ids)?;
        objects.extend(form_writes);
        // A grouping node the flatten emptied is deleted with the fields it
        // held. Leaving it would leave a named node in the field tree that
        // owns nothing — see `remove_fields_from_form`'s cascade.
        let delete_ids: Vec<ObjId> = delete_ids.into_iter().chain(emptied_parents).collect();

        // Delete the flattened field/widget dictionaries (§7.5.4). Their
        // /AP appearance streams survive — they are now page resources.
        for id in delete_ids {
            if self.base.get(id).is_some() || self.state.contains_key(&id) {
                removals.push(Removal {
                    id,
                    was_deleted: self.deleted.contains(&id),
                    is_deleted: true,
                });
            }
        }

        if objects.is_empty() && removals.is_empty() {
            return Ok(FlattenOutcome {
                fields_flattened: 0,
                widgets_burned: 0,
                pages_touched: 0,
            });
        }
        let pages_touched = per_page.len();
        let fields_flattened = targets.len();
        self.commit(Command {
            kind: CommandKind::FlattenFields {
                count: fields_flattened,
            },
            objects,
            removals,
            trailer: None,
        });
        Ok(FlattenOutcome {
            fields_flattened,
            widgets_burned,
            pages_touched,
        })
    }

    /// Resolve a widget's burn target: the object id of the `/AP` `/N`
    /// appearance stream to invoke (honoring `/AS` for a state
    /// sub-dictionary), plus its `/BBox` and `/Matrix` for §12.5.5
    /// placement. `None` when there is no usable indirect appearance stream
    /// (an inline `/N` stream, or a state with no matching entry — flatten
    /// then simply skips that widget rather than fabricating one).
    fn burn_target(&self, widget: &forms::Widget) -> Option<(ObjId, [f64; 4], [f64; 6])> {
        let graph = self.graph();
        let wd = graph.resolved(widget.id).as_dict()?;
        let ap = wd
            .get(b"AP")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_dict)?;
        let n = ap.get(b"N")?;
        let ap_id = match n {
            Object::Reference(r) => *r,
            Object::Dict(states) => {
                let state = widget.appearance_state.as_deref()?;
                match states.get(state) {
                    Some(Object::Reference(r)) => *r,
                    _ => return None,
                }
            }
            _ => return None,
        };
        let stream_dict = graph.resolved(ap_id).as_dict()?;
        let bbox = read_rect_array(&graph, stream_dict.get(b"BBox")?)?;
        let matrix = stream_dict
            .get(b"Matrix")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_array)
            .and_then(|a| {
                let nums: Vec<f64> = a
                    .iter()
                    .filter_map(|o| graph.resolve(o).as_number())
                    .collect();
                match nums.as_slice() {
                    &[a, b, c, d, e, f] => Some([a, b, c, d, e, f]),
                    _ => None,
                }
            })
            .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        Some((ap_id, bbox, matrix))
    }

    /// The page a widget appears on: its `/P` back-reference, or (failing
    /// that) the page whose `/Annots` array lists the widget id.
    fn page_of_widget(&self, widget: &forms::Widget, slots: &[PageSlot]) -> Option<ObjId> {
        if let Some(p) = widget.page
            && slots.iter().any(|s| s.id == p)
        {
            return Some(p);
        }
        let graph = self.graph();
        for slot in slots {
            if let Some(Object::Dict(page)) = self.value(slot.id) {
                let annots = page.get(b"Annots").map(|o| graph.resolve(o));
                if let Some(arr) = annots.and_then(Object::as_array)
                    && arr.iter().any(|o| o.as_reference() == Some(widget.id))
                {
                    return Some(slot.id);
                }
            }
        }
        None
    }

    /// Append a content-stream reference to a page's `/Contents` (§7.8.2),
    /// **mutating the caller's page dictionary in place**. `/Contents`
    /// becomes an array `[…existing, overlay]`; the existing content stream
    /// object(s) are untouched (byte-verbatim).
    ///
    /// # Why this mutates rather than returning an [`ObjectWrite`]
    ///
    /// It used to return a complete page-dict write of its own, and so did
    /// its two siblings ([`Self::add_page_xobjects`],
    /// [`Self::remove_from_annots`]) — three whole-dictionary replacements
    /// of the same object in one command, each computed from the same
    /// pre-command state, so the last silently discarded the other two. See
    /// the fix note at the [`Self::flatten_fields`] call site for the
    /// resulting defect. Taking `&mut Dict` makes the three **compose** and
    /// makes it structurally impossible to reintroduce: there is nothing
    /// left to overwrite.
    fn append_page_content(&self, updated: &mut Dict, overlay_id: ObjId) {
        let new_contents = match updated.get(b"Contents").cloned() {
            None => Object::Array(vec![Object::Reference(overlay_id)]),
            Some(Object::Reference(r)) => {
                Object::Array(vec![Object::Reference(r), Object::Reference(overlay_id)])
            }
            Some(Object::Array(mut arr)) => {
                arr.push(Object::Reference(overlay_id));
                Object::Array(arr)
            }
            // A direct or malformed /Contents: wrap what is referenceable
            // plus the overlay, keeping the original as a reference only if
            // it already was one (handled above). Otherwise append after.
            Some(_) => Object::Array(vec![Object::Reference(overlay_id)]),
        };
        updated.insert(Name::from(b"Contents"), new_contents);
    }

    /// Add form-XObject resources to a page's `/Resources` `/XObject`
    /// sub-dictionary, **mutating the caller's page dictionary in place**,
    /// materializing inherited `/Resources` onto the page if needed (so the
    /// addition never shadows the page's real resources).
    ///
    /// Mutates rather than returning an [`ObjectWrite`] for the reason given
    /// on [`Self::append_page_content`]. Note it reads the *page's* existing
    /// resources from `updated` when they are already there, falling back to
    /// the §7.7.3.4 inherited lookup — so it composes correctly whether or
    /// not a sibling mutation ran first.
    fn add_page_xobjects(
        &self,
        updated: &mut Dict,
        page_id: ObjId,
        xobjects: &[(Vec<u8>, ObjId)],
        slots: &[PageSlot],
    ) {
        // The page's own /Resources (possibly already materialized by an
        // earlier mutation in this same command), or a materialized copy of
        // the inherited one — never a fresh empty dict, which would hide
        // inherited fonts.
        let mut resources = updated
            .get(b"Resources")
            .and_then(Object::as_dict)
            .cloned()
            .unwrap_or_else(|| self.effective_resources(page_id, slots));
        let mut xobj = resources
            .get(b"XObject")
            .and_then(Object::as_dict)
            .cloned()
            .unwrap_or_default();
        for (name, id) in xobjects {
            xobj.insert(Name(name.clone()), Object::Reference(*id));
        }
        resources.insert(Name::from(b"XObject"), Object::Dict(xobj));
        updated.insert(Name::from(b"Resources"), Object::Dict(resources));
    }

    /// The `/Resources` in effect for a page (§7.7.3.4): the page's own, or
    /// the nearest ancestor's, cloned. Empty when none is found.
    fn effective_resources(&self, page_id: ObjId, slots: &[PageSlot]) -> Dict {
        let graph = self.graph();
        let mut chain = vec![page_id];
        if let Some(slot) = slots.iter().find(|s| s.id == page_id) {
            chain.extend(slot.ancestors.iter().copied());
        }
        for id in chain {
            if let Some(dict) = graph.resolved(id).as_dict()
                && let Some(res) = dict
                    .get(b"Resources")
                    .map(|o| graph.resolve(o))
                    .and_then(Object::as_dict)
            {
                return res.clone();
            }
        }
        Dict::new()
    }

    /// Remove annotation references from a page's `/Annots`.
    ///
    /// Two shapes, and they land in different places — which is why this one
    /// both mutates and returns:
    ///
    /// - **inline array** (`/Annots [ … ]`): the pruned array is written into
    ///   the caller's `updated` page dict, composing with the sibling
    ///   mutations (see [`Self::append_page_content`]); returns `None`.
    /// - **indirect array** (`/Annots 12 0 R`): the array is its *own*
    ///   object, possibly shared, so pruning it is a separate
    ///   [`ObjectWrite`] on that object's id — which cannot collide with the
    ///   page-dict write — and the page dict is left alone.
    ///
    /// `Ok(None)` also covers "nothing to do" (no `/Annots`, or a malformed
    /// one): the caller still writes its page dict, which is harmless.
    fn remove_from_annots(
        &self,
        updated: &mut Dict,
        remove: &[ObjId],
    ) -> Result<Option<ObjectWrite>, EditError> {
        match updated.get(b"Annots").cloned() {
            Some(Object::Array(arr)) => {
                let kept: Vec<Object> = arr
                    .into_iter()
                    .filter(|o| o.as_reference().is_none_or(|id| !remove.contains(&id)))
                    .collect();
                updated.insert(Name::from(b"Annots"), Object::Array(kept));
                Ok(None)
            }
            Some(Object::Reference(array_id)) => {
                let entries = self
                    .value(array_id)
                    .and_then(Object::as_array)
                    .map(<[Object]>::to_vec)
                    .unwrap_or_default();
                let kept: Vec<Object> = entries
                    .into_iter()
                    .filter(|o| o.as_reference().is_none_or(|id| !remove.contains(&id)))
                    .collect();
                Ok(Some(ObjectWrite {
                    id: array_id,
                    before: self.state.get(&array_id).cloned(),
                    after: Some(Object::Array(kept)),
                }))
            }
            _ => Ok(None),
        }
    }

    /// Remove flattened fields from the `/AcroForm` `/Fields` array and from
    /// any parent field's `/Kids`. Root fields drop from `/Fields`; a nested
    /// field drops from its `/Parent`'s `/Kids`.
    ///
    /// # Returns
    ///
    /// The container patches, **and** the ids of any additional grouping
    /// nodes the removal emptied — see the cascade below. The caller owns
    /// deleting the field dictionaries themselves and must delete these too,
    /// or an emptied node survives as an unreferenced object.
    fn remove_fields_from_form(
        &self,
        field_ids: &[ObjId],
    ) -> Result<(Vec<ObjectWrite>, Vec<ObjId>), EditError> {
        let mut writes: Vec<ObjectWrite> = Vec::new();
        let graph = self.graph();
        // Group the removals by the container that holds each field id: its
        // /Parent's /Kids, or the AcroForm /Fields root.
        let acro_holder = self.acroform_id();

        // CASCADE FIRST: a parent left with no kids is itself removed.
        //
        // Removing the last child of a non-terminal grouping node leaves a
        // node with `/Kids []` — a field with a name, no type of its own
        // (Table 220), and nothing beneath it. It is not merely untidy: it is
        // a name that still OCCUPIES its slot in the field tree, so
        // §12.7.3.2's FQN space still has `Personal.Address` in it, and a
        // later request to create a terminal field called `Personal.Address`
        // is refused as a grouping-node collision by a node that exists only
        // because a deletion did not finish.
        //
        // Recursive because emptying a parent can empty ITS parent: deleting
        // `Personal.Address.Zip` when `Zip` is `Address`'s only child empties
        // `Address`, which is `Personal`'s only child, which empties
        // `Personal`. A single pass would leave two dead nodes behind.
        //
        // The fixed point is computed BEFORE any write, so the patches below
        // see the final removal set and no container is patched to keep a kid
        // that a later round decided to remove.
        let mut removing: BTreeSet<ObjId> = field_ids.iter().copied().collect();
        loop {
            let mut added = false;
            // Candidate parents: the /Parent of everything currently marked
            // for removal that is not itself already marked.
            let parents: Vec<ObjId> = removing
                .iter()
                .filter_map(|id| {
                    graph
                        .resolved(*id)
                        .as_dict()
                        .and_then(|d| d.get(b"Parent").and_then(Object::as_reference))
                })
                .filter(|p| !removing.contains(p))
                .collect();
            for p in parents {
                let survives = graph
                    .resolved(p)
                    .as_dict()
                    .and_then(|d| d.get(b"Kids").map(|o| graph.resolve(o)))
                    .and_then(Object::as_array)
                    .is_some_and(|kids| {
                        kids.iter()
                            .filter_map(Object::as_reference)
                            .any(|k| !removing.contains(&k))
                    });
                // A parent whose `/Kids` cannot be read at all is LEFT ALONE.
                // `survives == false` there would mean "it has no surviving
                // kids", which is true but useless: the node's structure is
                // not understood, and deleting what is not understood is how
                // a repair becomes a data loss.
                let readable = graph
                    .resolved(p)
                    .as_dict()
                    .and_then(|d| d.get(b"Kids").map(|o| graph.resolve(o)))
                    .and_then(Object::as_array)
                    .is_some();
                if readable && !survives && removing.insert(p) {
                    added = true;
                }
            }
            if !added {
                break;
            }
        }
        // The nodes the cascade ADDED, reported so the caller deletes their
        // dictionaries alongside the fields it already owns.
        let requested: BTreeSet<ObjId> = field_ids.iter().copied().collect();
        let cascaded: Vec<ObjId> = removing.difference(&requested).copied().collect();
        let field_ids: Vec<ObjId> = removing.iter().copied().collect();
        let field_ids = &field_ids[..];

        // Map container-object-id -> (array-key, ids to drop).
        let mut by_parent: BTreeMap<ObjId, Vec<ObjId>> = BTreeMap::new();
        let mut root_drop: Vec<ObjId> = Vec::new();
        for id in field_ids {
            let parent = graph
                .resolved(*id)
                .as_dict()
                .and_then(|d| d.get(b"Parent").and_then(Object::as_reference));
            match parent {
                // A parent that is ITSELF being removed needs no `/Kids`
                // patch — the whole node is going, and patching it would emit
                // a write for an object that is about to be dropped.
                Some(p) if !removing.contains(&p) => by_parent.entry(p).or_default().push(*id),
                Some(_) => {}
                None => root_drop.push(*id),
            }
        }
        // Parent /Kids patches.
        for (parent_id, drop) in &by_parent {
            if let Some(Object::Dict(pd)) = self.value(*parent_id) {
                let mut updated = pd.clone();
                if let Some(kids) = pd
                    .get(b"Kids")
                    .map(|o| graph.resolve(o))
                    .and_then(Object::as_array)
                {
                    let kept: Vec<Object> = kids
                        .iter()
                        .filter(|o| o.as_reference().is_none_or(|id| !drop.contains(&id)))
                        .cloned()
                        .collect();
                    updated.insert(Name::from(b"Kids"), Object::Array(kept));
                    writes.push(ObjectWrite {
                        id: *parent_id,
                        before: self.state.get(parent_id).cloned(),
                        after: Some(Object::Dict(updated)),
                    });
                }
            }
        }
        // AcroForm /Fields root patch.
        //
        // # `holder_id` is not always the AcroForm dictionary
        //
        // `acroform_id` returns the object that HOLDS the form: the
        // referenced object when `/AcroForm` is indirect, and **the catalog**
        // when it is a direct dictionary. Those are two different
        // dictionaries, and the difference is the whole of a defect this
        // slice found by running the shipped flatten over the shipped
        // `demo-form.pdf`:
        //
        //     flatten ... fields_flattened=2
        //     /AcroForm << /Fields [4 0 R 5 0 R] ...
        //
        // Objects 4 and 5 were deleted; `/Fields` still referenced them. The
        // code read `/Fields` off the CATALOG, found nothing there (it lives
        // one level down, inside the direct `/AcroForm`), and the `if let`
        // guarding the patch simply did not fire — so removal silently did
        // nothing and left `/AcroForm /Fields` pointing at deleted objects.
        //
        // No test caught it because every forms test asserted through
        // `parse_acroform`, which resolves each `/Fields` entry and drops the
        // ones that no longer resolve. The projection looked right; the FILE
        // was wrong. `add_field`'s registration path had the direct case
        // right all along — this brings removal into line with it.
        if let Some(holder_id) = acro_holder {
            let catalog_id = graph.catalog_id();
            let holder_is_catalog = Some(holder_id) == catalog_id;
            let holder = graph.resolved(holder_id).as_dict().cloned();
            // The actual AcroForm dict: the holder itself, or the direct
            // `/AcroForm` value inside the catalog.
            let acro: Option<Dict> = if holder_is_catalog {
                holder
                    .as_ref()
                    .and_then(|c| c.get(b"AcroForm"))
                    .and_then(Object::as_dict)
                    .cloned()
            } else {
                holder.clone()
            };
            let fields_owned: Option<Vec<Object>> = acro
                .as_ref()
                .and_then(|d| d.get(b"Fields"))
                .map(|o| graph.resolve(o))
                .and_then(Object::as_array)
                .map(<[Object]>::to_vec);
            if let (Some(mut acro), Some(fields)) = (acro, fields_owned) {
                let fields_holder = match acro.get(b"Fields") {
                    Some(Object::Reference(r)) => Some(*r),
                    _ => None,
                };
                let kept: Vec<Object> = fields
                    .into_iter()
                    .filter(|o| o.as_reference().is_none_or(|id| !root_drop.contains(&id)))
                    .collect();
                match fields_holder {
                    // /Fields is an indirect array object: patch it directly.
                    // Correct whichever dictionary points at it.
                    Some(arr_id) => writes.push(ObjectWrite {
                        id: arr_id,
                        before: self.state.get(&arr_id).cloned(),
                        after: Some(Object::Array(kept)),
                    }),
                    // /Fields is a direct array. Rewrite the dictionary that
                    // contains it — nesting the corrected AcroForm back into
                    // the catalog when that is where it lives.
                    None => {
                        acro.insert(Name::from(b"Fields"), Object::Array(kept));
                        let after = if holder_is_catalog {
                            let mut cat = holder.unwrap_or_default();
                            cat.insert(Name::from(b"AcroForm"), Object::Dict(acro));
                            Object::Dict(cat)
                        } else {
                            Object::Dict(acro)
                        };
                        writes.push(ObjectWrite {
                            id: holder_id,
                            before: self.state.get(&holder_id).cloned(),
                            after: Some(after),
                        });
                    }
                }
            }
        }
        Ok((writes, cascaded))
    }

    /// The object id that holds the `/AcroForm` dictionary: the referenced
    /// object when `/AcroForm` is an indirect reference, else the catalog
    /// (an inline `/AcroForm`).
    fn acroform_id(&self) -> Option<ObjId> {
        let graph = self.graph();
        let catalog_id = graph.catalog_id()?;
        let catalog = graph.resolved(catalog_id).as_dict()?;
        match catalog.get(b"AcroForm") {
            Some(Object::Reference(r)) => Some(*r),
            Some(_) => Some(catalog_id),
            None => None,
        }
    }

    /// Push the write that removes `/NeedAppearances` from the AcroForm
    /// dictionary (R51), returning whether it was present to clear.
    fn clear_need_appearances_write(&self, objects: &mut Vec<ObjectWrite>) -> bool {
        let Some(holder_id) = self.acroform_id() else {
            return false;
        };
        let graph = self.graph();
        // The AcroForm dict may be the referenced object or inline in the
        // catalog; patch whichever holds it.
        let catalog_id = graph.catalog_id();
        if Some(holder_id) == catalog_id {
            // Inline: patch the catalog's /AcroForm dict.
            let Some(Object::Dict(cat)) = self.value(holder_id) else {
                return false;
            };
            let Some(Object::Dict(acro)) = cat.get(b"AcroForm") else {
                return false;
            };
            if !acro.contains_key(b"NeedAppearances") {
                return false;
            }
            let mut acro2 = acro.clone();
            acro2.remove(b"NeedAppearances");
            let mut cat2 = cat.clone();
            cat2.insert(Name::from(b"AcroForm"), Object::Dict(acro2));
            objects.push(ObjectWrite {
                id: holder_id,
                before: self.state.get(&holder_id).cloned(),
                after: Some(Object::Dict(cat2)),
            });
            true
        } else {
            let Some(Object::Dict(acro)) = self.value(holder_id) else {
                return false;
            };
            if !acro.contains_key(b"NeedAppearances") {
                return false;
            }
            let mut acro2 = acro.clone();
            acro2.remove(b"NeedAppearances");
            objects.push(ObjectWrite {
                id: holder_id,
                before: self.state.get(&holder_id).cloned(),
                after: Some(Object::Dict(acro2)),
            });
            true
        }
    }

    /// Allocate the next object number for a created object, advancing the
    /// cached counter so two creations in one session cannot collide.
    ///
    /// The counter is **not** rewound on undo — §7.5.4/§7.5.7 never reuse
    /// a number, so a skipped one is harmless, and rewinding would risk a
    /// collision with a redo. Matches [`EditSession::set_info_field`].
    fn alloc_number(&mut self) -> Result<u32, EditError> {
        let n = self.next_number.ok_or(EditError::ObjectNumbersExhausted)?;
        self.next_number = self.next_number.and_then(|v| v.checked_add(1));
        Ok(n)
    }

    /// Append `content` to the session staging buffer (R45) and return its
    /// span in the combined `base.len() + local` coordinate system, so a
    /// created appearance [`Stream`](crate::object::Stream) keeps the span
    /// model rather than owning bytes.
    fn stage_bytes(&mut self, content: &[u8]) -> ByteSpan {
        let start = self.base.bytes().len() + self.staging.len();
        self.staging.extend_from_slice(content);
        ByteSpan::new(start, content.len())
    }

    /// Compute the object write(s) that add `annot_ref_id` to `page_id`'s
    /// `/Annots` array (X7).
    ///
    /// Three shapes, all handled without perturbing another page:
    ///
    /// - **No `/Annots`** — write a fresh direct array `[annot]` onto the
    ///   page dictionary.
    /// - **Direct array** — clone, append, write back onto the page.
    /// - **Indirect array** — if the array object is referenced by only
    ///   this page, modify it in place (the page dictionary is untouched).
    ///   If it is **shared** by more than one page (malformed per §12.5.2's
    ///   *"referenced from only one page"*, but seen in the wild), **copy
    ///   on write**: a new array object carries the old entries plus the
    ///   new one and this page is repointed at it — so the other sharing
    ///   pages keep their original annotation set.
    fn annots_writes(
        &mut self,
        page_id: ObjId,
        annot_ref_id: ObjId,
        slots: &[PageSlot],
    ) -> Result<Vec<ObjectWrite>, EditError> {
        self.annots_append(page_id, &[annot_ref_id], slots)
    }

    /// Append several annotation references to a page's `/Annots` in one
    /// command (the [`annots_writes`](EditSession::annots_writes)
    /// generalization Pass 6.2 needs: a sticky note plus its `/Popup`
    /// companion are two entries added together). Same three `/Annots`
    /// shapes and the same X7 copy-on-write handling.
    fn annots_append(
        &mut self,
        page_id: ObjId,
        new_ids: &[ObjId],
        slots: &[PageSlot],
    ) -> Result<Vec<ObjectWrite>, EditError> {
        let new_refs = || new_ids.iter().map(|id| Object::Reference(*id));
        let Some(Object::Dict(page)) = self.value(page_id) else {
            return Err(EditError::NotADictionary {
                id: page_id,
                key: "Annots",
            });
        };
        let page = page.clone();
        match page.get(b"Annots").cloned() {
            None => {
                let mut updated = page.clone();
                updated.insert(Name::from(b"Annots"), Object::Array(new_refs().collect()));
                Ok(vec![self.page_write(page_id, updated)])
            }
            Some(Object::Array(mut arr)) => {
                arr.extend(new_refs());
                let mut updated = page.clone();
                updated.insert(Name::from(b"Annots"), Object::Array(arr));
                Ok(vec![self.page_write(page_id, updated)])
            }
            Some(Object::Reference(array_id)) => {
                let mut entries = self
                    .value(array_id)
                    .and_then(Object::as_array)
                    .map(<[Object]>::to_vec)
                    .unwrap_or_default();
                entries.extend(new_refs());
                if self.annots_array_is_shared(array_id, slots) {
                    // Copy-on-write: a new array, this page repointed.
                    let new_id = ObjId::new(self.alloc_number()?, 0);
                    let mut updated = page.clone();
                    updated.insert(Name::from(b"Annots"), Object::Reference(new_id));
                    Ok(vec![
                        ObjectWrite {
                            id: new_id,
                            before: None,
                            after: Some(Object::Array(entries)),
                        },
                        self.page_write(page_id, updated),
                    ])
                } else {
                    // Sole owner: edit the array object in place; the page
                    // dictionary is unchanged (no write for it).
                    let before = self.state.get(&array_id).cloned();
                    Ok(vec![ObjectWrite {
                        id: array_id,
                        before,
                        after: Some(Object::Array(entries)),
                    }])
                }
            }
            Some(_) => Err(EditError::AnnotsNotAnArray { page: page_id }),
        }
    }

    /// An [`ObjectWrite`] replacing `page_id`'s dictionary with `updated`,
    /// carrying the correct `before` (the session overlay's value, so undo
    /// restores exactly what was there — possibly nothing).
    fn page_write(&self, page_id: ObjId, updated: Dict) -> ObjectWrite {
        ObjectWrite {
            id: page_id,
            before: self.state.get(&page_id).cloned(),
            after: Some(Object::Dict(updated)),
        }
    }

    /// Whether an indirect `/Annots` array is referenced by more than one
    /// page — the copy-on-write trigger (X7). Short-circuits at the second
    /// referrer.
    fn annots_array_is_shared(&self, array_id: ObjId, slots: &[PageSlot]) -> bool {
        let mut count = 0usize;
        for slot in slots {
            if let Some(Object::Dict(d)) = self.value(slot.id)
                && matches!(d.get(b"Annots"), Some(Object::Reference(r)) if *r == array_id)
            {
                count += 1;
                if count > 1 {
                    return true;
                }
            }
        }
        false
    }

    /// Remove pages from the document.
    ///
    /// `indices` are 0-based positions in the **current** page order;
    /// duplicates and any ordering are accepted and normalized, because
    /// "delete the pages I selected" is the operation and a selection has
    /// no inherent order.
    ///
    /// ## What deletion is, and what it is not
    ///
    /// It is a **page-tree splice**: the page leaves its parent's
    /// `/Kids`, every ancestor's `/Count` drops, a node left empty is
    /// removed too, and every object the removed pages owned exclusively
    /// is freed (§7.5.4 type-0 entries, with the generation discipline
    /// [`crate::writer`] documents). Objects shared with a surviving page
    /// are untouched — the sweep is a reachability computation against
    /// the document *as it will be*, not a guess.
    ///
    /// It is **not redaction**. Under the default incremental save the
    /// removed page's bytes remain in the file by construction (§7.5.6
    /// appends; it does not erase), and `ARCHITECTURE.md` §5.7 records
    /// that a full rewrite is not sufficient either when the content sits
    /// in an object stream. Front ends must say so; pdfce-gui's delete
    /// tooltip does, at length.
    ///
    /// ## `/PageLabels` is left stale, and reported
    ///
    /// `core_ops__page_labels_and_bates_interaction.md` records that
    /// Acrobat does not adjust an existing label tree for any structural
    /// operation, and recommends pdfce match that baseline for this Pass
    /// (*"leave `/PageLabels` numerically stale exactly as Acrobat
    /// does … recommended baseline for Pass 3.2 acceptance criteria"*).
    /// pdfce matches it **and says so** —
    /// [`DanglingReport::page_labels_stale`] — which is the parity-plus
    /// half: Acrobat leaves them stale *and silent*.
    ///
    /// # Errors
    ///
    /// - [`EditError::CertificationForbidsChange`] — an enforced
    ///   certification signature (§12.8.4).
    /// - [`EditError::WouldRemoveEveryPage`] — §7.7.3.3 requires at least
    ///   one page.
    /// - [`EditError::PageOutOfRange`], [`EditError::PageTree`].
    pub fn delete_pages(&mut self, indices: &[usize]) -> Result<DeleteOutcome, EditError> {
        self.check_certification()?;

        let slots = self.page_slots()?;
        let total = slots.len();
        let mut targets: Vec<usize> = indices.to_vec();
        targets.sort_unstable();
        targets.dedup();
        if let Some(&past) = targets.iter().find(|index| **index >= total) {
            return Err(EditError::PageOutOfRange {
                index: past,
                count: total,
            });
        }
        if targets.is_empty() {
            return Ok(DeleteOutcome {
                pages_removed: 0,
                objects_freed: 0,
                dangling: DanglingReport::default(),
                signature: self.signature_impact_of_save(SaveMode::Incremental),
            });
        }
        if targets.len() >= total {
            return Err(EditError::WouldRemoveEveryPage {
                removing: targets.len(),
                total,
            });
        }

        let removed_pages: HashSet<ObjId> = targets
            .iter()
            .filter_map(|index| slots.get(*index))
            .map(|slot| slot.id)
            .collect();
        let surviving: Vec<ObjId> = slots
            .iter()
            .filter(|slot| !removed_pages.contains(&slot.id))
            .map(|slot| slot.id)
            .collect();

        // Census BEFORE the splice: afterwards the removed pages are
        // gone and nothing can be found to have pointed at them.
        let dangling = census_dangling(&self.graph(), &removed_pages, &surviving);

        // --- splice the tree ------------------------------------------
        let mut scratch: BTreeMap<ObjId, Object> = BTreeMap::new();
        let mut freed: HashSet<ObjId> = removed_pages.clone();

        // Every node that loses pages, and how many. A node's new /Count
        // is derived from the walk (how many leaves are under it now)
        // minus the loss — never from the file's own /Count, which
        // `page_tree` deliberately does not trust.
        let mut leaves_under: HashMap<ObjId, usize> = HashMap::new();
        let mut lost_under: HashMap<ObjId, usize> = HashMap::new();
        for slot in &slots {
            for ancestor in &slot.ancestors {
                *leaves_under.entry(*ancestor).or_insert(0) += 1;
                if removed_pages.contains(&slot.id) {
                    *lost_under.entry(*ancestor).or_insert(0) += 1;
                }
            }
        }
        let touched: Vec<ObjId> = {
            let mut all: Vec<ObjId> = slots
                .iter()
                .flat_map(|slot| slot.ancestors.iter().copied())
                .collect();
            all.sort_unstable();
            all.dedup();
            all
        };
        let root_id = self
            .graph()
            .catalog_dict()
            .and_then(|catalog| catalog.get(b"Pages").and_then(Object::as_reference));

        // Drop the removed pages from their parents' /Kids, then prune
        // any node left empty, repeatedly, until nothing more empties.
        // Bounded by the number of nodes: each pass frees at least one,
        // or stops.
        let mut drop_from_parent: HashSet<ObjId> = removed_pages.clone();
        for _ in 0..=touched.len() {
            let mut newly_empty: HashSet<ObjId> = HashSet::new();
            for node_id in &touched {
                let Some(node) = self.pending_dict(&scratch, &freed, *node_id) else {
                    continue;
                };
                let Some(kids) = node
                    .get(b"Kids")
                    .map(|o| self.resolve_value(o))
                    .and_then(Object::as_array)
                    .map(<[Object]>::to_vec)
                else {
                    continue;
                };
                let kept: Vec<Object> = kids
                    .iter()
                    .filter(|kid| {
                        kid.as_reference()
                            .is_none_or(|id| !drop_from_parent.contains(&id))
                    })
                    .cloned()
                    .collect();
                if kept.len() == kids.len() {
                    continue;
                }
                let mut updated = node.clone();
                let empty = kept.is_empty();
                updated.insert(Name::from(b"Kids"), Object::Array(kept));
                let count = leaves_under
                    .get(node_id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_sub(lost_under.get(node_id).copied().unwrap_or(0));
                updated.insert(
                    Name::from(b"Count"),
                    Object::Integer(i64::try_from(count).unwrap_or(0)),
                );
                scratch.insert(*node_id, Object::Dict(updated));
                // An intermediate node with no kids left is not a legal
                // page-tree node (Table 29 requires `Kids`), so it goes
                // too — unless it is the root, which must survive even
                // empty because the catalog names it.
                if empty && Some(*node_id) != root_id {
                    newly_empty.insert(*node_id);
                }
            }
            if newly_empty.is_empty() {
                break;
            }
            for id in &newly_empty {
                scratch.remove(id);
                freed.insert(*id);
            }
            drop_from_parent = newly_empty;
        }

        // --- sweep what the removed pages owned exclusively ------------
        //
        // Two closures, and the pairing is what makes this safe. The
        // CANDIDATE set is everything the removed pages could reach, so
        // nothing outside their subgraph is ever considered. The LIVE set
        // is everything still reachable from the trailer AFTER the
        // splice. A candidate that is not live was owned exclusively by a
        // page that just left; a candidate that IS live is shared with a
        // surviving page and must not be touched.
        //
        // Restricting to candidates matters as much as the liveness test:
        // sweeping every unreachable object in the file would also free
        // objects that were already orphaned before this edit, which
        // pdfce was not asked to remove (§5).
        let roots: Vec<ObjId> = removed_pages.iter().copied().collect();
        let candidates = reachable(&self.graph(), &roots, &removed_pages);
        let live_after = {
            let pending = PendingGraph {
                session: self,
                scratch: &scratch,
                removed: &freed,
            };
            let mut live_roots: Vec<ObjId> = Vec::new();
            live_roots.extend(pending.catalog_id());
            live_roots.extend(
                pending
                    .trailer_entry(b"Info")
                    .and_then(Object::as_reference),
            );
            reachable(&pending, &live_roots, &HashSet::new())
        };
        for id in candidates {
            if !live_after.contains(&id) {
                freed.insert(id);
            }
        }

        // --- build the one command ------------------------------------
        let objects: Vec<ObjectWrite> = scratch
            .into_iter()
            .map(|(id, value)| ObjectWrite {
                id,
                before: self.state.get(&id).cloned(),
                after: Some(value),
            })
            .collect();
        let removals: Vec<Removal> = freed
            .iter()
            .map(|id| Removal {
                id: *id,
                was_deleted: self.deleted.contains(id),
                is_deleted: true,
            })
            .collect();
        let objects_freed = removals.len();
        let pages_removed = targets.len();

        self.commit(Command {
            kind: CommandKind::DeletePages {
                count: pages_removed,
            },
            objects,
            removals,
            trailer: None,
        });

        Ok(DeleteOutcome {
            pages_removed,
            objects_freed,
            dangling,
            signature: self.signature_impact_of_save(SaveMode::Incremental),
        })
    }

    /// Put the document's pages in a new order.
    ///
    /// `new_order[i]` is the **current** 0-based index of the page that
    /// should end up at position `i`. It must be a permutation of
    /// `0..page_count`.
    ///
    /// ## One command, however many pages moved
    ///
    /// §11.3 names this case explicitly — *"for bulk structural
    /// operations where per-item commands would be awkward (e.g.
    /// reordering 50 pages in one drag operation), a coarser
    /// before/after page-order snapshot command is an acceptable
    /// specialization of the same pattern — still one undo-stack
    /// entry"*. The snapshot here is the set of object writes the reorder
    /// performs, each carrying its own prior value — the same mechanism
    /// every other command uses, rather than a parallel one.
    ///
    /// ## How the tree is rewritten, and what it deliberately does not do
    ///
    /// pdfce **permutes which page sits in each existing leaf slot**
    /// rather than flattening the tree. A document whose 900 pages are
    /// balanced across intermediate `Pages` nodes keeps that shape, and a
    /// reorder touches only the nodes whose `Kids` actually changed. The
    /// obvious alternative — rebuild one flat root `Kids` array — would
    /// rewrite the whole tree for a two-page swap and orphan every
    /// intermediate node, which is normalization by another name (R33).
    ///
    /// The cost of keeping the shape is that a page can land under a
    /// **different ancestor**, and §7.7.3.4's inheritable attributes
    /// resolve from ancestors. So any page whose parent changes has the
    /// attributes it *used* to resolve written onto it explicitly, raw,
    /// and only where they would otherwise change. That is the same
    /// materialization rule [`crate::pageops::assemble`] applies, for the
    /// same reason.
    ///
    /// # Errors
    ///
    /// - [`EditError::CertificationForbidsChange`].
    /// - [`EditError::NotAPermutation`] — `new_order` is not one.
    /// - [`EditError::PageTree`].
    pub fn reorder_pages(&mut self, new_order: &[usize]) -> Result<(), EditError> {
        self.check_certification()?;

        let slots = self.page_slots()?;
        let count = slots.len();
        let distinct: BTreeSet<usize> = new_order.iter().copied().filter(|i| *i < count).collect();
        if new_order.len() != count || distinct.len() != count {
            return Err(EditError::NotAPermutation {
                expected: count,
                got: distinct.len(),
            });
        }
        if new_order.iter().copied().eq(0..count) {
            return Ok(()); // identity — nothing to record
        }

        let mut scratch: BTreeMap<ObjId, Object> = BTreeMap::new();
        let mut moved = 0usize;

        // Each leaf slot keeps its position in the tree; the page that
        // occupies it changes.
        for (position, source) in new_order.iter().copied().enumerate() {
            let (Some(target_slot), Some(source_slot)) = (slots.get(position), slots.get(source))
            else {
                continue;
            };
            if target_slot.id == source_slot.id {
                continue;
            }
            moved += 1;

            // Rewrite the destination slot's parent /Kids entry.
            if let Some(parent_id) = target_slot.parent {
                let current_parent = scratch
                    .get(&parent_id)
                    .and_then(Object::as_dict)
                    .cloned()
                    .or_else(|| self.value(parent_id).and_then(Object::as_dict).cloned());
                if let Some(mut parent) = current_parent {
                    let kids = parent
                        .get(b"Kids")
                        .map(|o| self.resolve_value(o))
                        .and_then(Object::as_array)
                        .map(<[Object]>::to_vec);
                    if let Some(mut kids) = kids {
                        if let Some(entry) = kids.get_mut(target_slot.index_in_parent) {
                            *entry = Object::Reference(source_slot.id);
                        }
                        parent.insert(Name::from(b"Kids"), Object::Array(kids));
                        scratch.insert(parent_id, Object::Dict(parent));
                    }
                }
            }

            // The moved page's own /Parent, and any attributes it is
            // about to stop inheriting.
            if source_slot.parent == target_slot.parent {
                continue;
            }
            let Some(page) = self
                .value(source_slot.id)
                .and_then(Object::as_dict)
                .cloned()
            else {
                continue;
            };
            let mut updated = page.clone();
            if let Some(new_parent) = target_slot.parent {
                updated.insert(Name::from(b"Parent"), Object::Reference(new_parent));
            }
            for (key, replacement) in
                preserve_inherited(&page, &source_slot.inherited, &target_slot.inherited)
            {
                updated.insert(Name::from(key), replacement);
            }
            scratch.insert(source_slot.id, Object::Dict(updated));
        }

        if scratch.is_empty() {
            return Ok(());
        }
        let objects: Vec<ObjectWrite> = scratch
            .into_iter()
            .map(|(id, value)| ObjectWrite {
                id,
                before: self.state.get(&id).cloned(),
                after: Some(value),
            })
            .collect();
        self.commit(Command {
            kind: CommandKind::ReorderPages { count: moved },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    /// Turn several pages by the same amount, as **one** undoable
    /// operation.
    ///
    /// `delta` is a relative turn in degrees (a multiple of 90), applied
    /// to each page's own current effective rotation — so a selection of
    /// pages at 0°, 90° and 180° turned by 90° lands at 90°, 180° and
    /// 270°, not all at 90°. That is what a toolbar turn-right button
    /// means, and `core_ops__rotate_pages.md` confirms Acrobat persists
    /// the *"absolute `/Rotate` (existing value + applied increment, mod
    /// 360) — net effect only, not a stored delta."*
    ///
    /// Returns how many pages actually changed; a page already at the
    /// requested rotation contributes nothing, and if none of them
    /// changed, no command reaches the undo stack.
    ///
    /// # Errors
    ///
    /// - [`EditError::CertificationForbidsChange`] — rotating a page is a
    ///   page-attribute change, which Table 254 permits at no `P` value.
    /// - [`EditError::RotationNotMultipleOf90`] (Table 30),
    ///   [`EditError::PageOutOfRange`], [`EditError::PageTree`].
    pub fn rotate_pages(&mut self, indices: &[usize], delta: i32) -> Result<usize, EditError> {
        self.check_certification()?;
        if delta % 90 != 0 {
            return Err(EditError::RotationNotMultipleOf90 { degrees: delta });
        }
        let mut targets: Vec<usize> = indices.to_vec();
        targets.sort_unstable();
        targets.dedup();

        let pages = self.pages()?;
        let count = pages.len();
        let mut writes: Vec<ObjectWrite> = Vec::new();
        for index in &targets {
            let current = pages
                .get(*index)
                .ok_or(EditError::PageOutOfRange {
                    index: *index,
                    count,
                })?
                .rotate;
            let target = i64::from(current) + i64::from(delta);
            if let Some((write, _)) =
                self.rotation_write(*index, i32::from(normalize_rotation(target)))?
            {
                writes.push(write);
            }
        }
        let changed = writes.len();
        if changed == 0 {
            return Ok(0);
        }
        self.commit(Command {
            kind: CommandKind::RotatePages {
                count: changed,
                delta,
            },
            objects: writes,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(changed)
    }

    /// Follow a reference chain over the session's own view (§7.3.10).
    ///
    /// Identical in behaviour to [`ObjectGraph::resolve`] on
    /// [`EditSession::graph`], and it exists only because the trait
    /// method borrows the *graph*: `self.graph().resolve(o)` builds a
    /// temporary `SessionGraph`, so the returned reference would outlive
    /// it. Binding the graph to a local would work too, and is worse —
    /// it holds a shared borrow of `self` across code that needs a
    /// mutable one.
    fn resolve_value<'a>(&'a self, obj: &'a Object) -> &'a Object {
        const NULL: &Object = &Object::Null;
        let mut current = obj;
        for _ in 0..crate::document::MAX_RESOLVE_DEPTH {
            match current {
                Object::Reference(id) => match self.value(*id) {
                    Some(value) => current = value,
                    None => return NULL,
                },
                other => return other,
            }
        }
        NULL
    }

    /// A dictionary as it stands with `scratch` layered over the session
    /// and `freed` removed — the mid-splice view.
    fn pending_dict(
        &self,
        scratch: &BTreeMap<ObjId, Object>,
        freed: &HashSet<ObjId>,
        id: ObjId,
    ) -> Option<Dict> {
        if freed.contains(&id) {
            return None;
        }
        scratch
            .get(&id)
            .or_else(|| self.value(id))
            .and_then(Object::as_dict)
            .cloned()
    }
}

/// The entries a page must gain to keep resolving the §7.7.3.4
/// inheritable attributes it had, now that it sits under `after` instead
/// of `before`.
///
/// ## The asymmetry that makes this more than a diff
///
/// Two directions, and only the first is obvious:
///
/// 1. **The old chain supplied a value and the new one does not (or
///    supplies a different one).** Write the old raw value onto the page.
///    Raw, not resolved — it is usually a single indirect reference, so
///    the shared resource dictionary stays shared.
/// 2. **The old chain supplied *nothing* and the new one supplies
///    something.** This is the case a naive implementation misses
///    entirely, because there is no old value to copy — and it is the one
///    that silently *changes* the page. A page that inherited no
///    `/Rotate` was displaying at 0°; moved under a node that says
///    `/Rotate 90`, it silently turns. The fix is to write §7.7.3.4's
///    **default** explicitly: `/Rotate 0`, and `/CropBox` = the
///    resolved `/MediaBox` (Table 30's documented default for it).
///
/// `Resources` and `MediaBox` have no "absent" default — §7.7.3.4 says a
/// value *"shall be supplied in an ancestor node"* — so direction 2
/// cannot arise for them in a conforming file, and in a malformed one
/// there is nothing to write that would not be invented. They are left
/// alone in that case, and the page keeps whatever the new chain gives
/// it, which is strictly better than a fabricated box.
///
/// An attribute the page already carries itself is never touched: its own
/// entry wins (§7.7.3.4) and restating it would modify an object pdfce
/// was not asked to modify (§5).
/// Refuse a markup spec whose geometry draws nothing
/// ([`EditError::EmptyGeometry`], Pass 6.1 guard 4). The geometrically
/// closed subtypes (Square/Circle/Line) always have geometry; the
/// list-driven ones (Ink/Polygon/PolyLine/text markup) can be handed empty
/// point lists, which would produce an invisible annotation.
fn validate_geometry(spec: &MarkupSpec) -> Result<(), EditError> {
    let empty = match spec {
        MarkupSpec::Ink { strokes, .. } => {
            strokes.is_empty() || strokes.iter().all(std::vec::Vec::is_empty)
        }
        MarkupSpec::Polygon { vertices, .. } | MarkupSpec::PolyLine { vertices, .. } => {
            vertices.len() < 2
        }
        MarkupSpec::TextMarkup { quads, .. } => quads.is_empty(),
        MarkupSpec::Square { .. } | MarkupSpec::Circle { .. } | MarkupSpec::Line { .. } => false,
    };
    if empty {
        Err(EditError::EmptyGeometry)
    } else {
        Ok(())
    }
}

/// The [`AnnotKind`] undo label for a markup spec.
const fn annot_kind_of(spec: &MarkupSpec) -> AnnotKind {
    use crate::annot_author::TextMarkupKind;
    match spec {
        MarkupSpec::Square { .. } => AnnotKind::Square,
        MarkupSpec::Circle { .. } => AnnotKind::Circle,
        MarkupSpec::Line { .. } => AnnotKind::Line,
        MarkupSpec::Ink { .. } => AnnotKind::Ink,
        MarkupSpec::Polygon { .. } => AnnotKind::Polygon,
        MarkupSpec::PolyLine { .. } => AnnotKind::PolyLine,
        MarkupSpec::TextMarkup { kind, .. } => match kind {
            TextMarkupKind::Highlight => AnnotKind::Highlight,
            TextMarkupKind::Underline => AnnotKind::Underline,
            TextMarkupKind::StrikeOut => AnnotKind::StrikeOut,
            TextMarkupKind::Squiggly => AnnotKind::Squiggly,
        },
    }
}

/// The [`AnnotKind`] undo label for a text-bearing spec (Pass 6.2).
const fn text_annot_kind_of(spec: &TextAnnotSpec) -> AnnotKind {
    match spec {
        TextAnnotSpec::FreeText { .. } => AnnotKind::FreeText,
        TextAnnotSpec::Sticky { .. } => AnnotKind::Text,
        TextAnnotSpec::Stamp { .. } => AnnotKind::Stamp,
    }
}

fn preserve_inherited(
    page: &Dict,
    before: &page_tree::InheritedRaw,
    after: &page_tree::InheritedRaw,
) -> Vec<(&'static [u8], Object)> {
    let mut out: Vec<(&'static [u8], Object)> = Vec::new();
    let attributes: [(&'static [u8], &Option<Object>, &Option<Object>); 4] = [
        (b"Resources", &before.resources, &after.resources),
        (b"MediaBox", &before.media_box, &after.media_box),
        (b"CropBox", &before.crop_box, &after.crop_box),
        (b"Rotate", &before.rotate, &after.rotate),
    ];
    for (key, old, new) in attributes {
        // `contains_key` collapses a null-valued entry to absent
        // (§7.3.7), which is the right test: `/Rotate null` inherits.
        if page.contains_key(key) || old == new {
            continue;
        }
        match old {
            // Direction 1.
            Some(value) => out.push((key, value.clone())),
            // Direction 2 — write the spec default, where there is one.
            None => match key {
                b"Rotate" => out.push((key, Object::Integer(0))),
                b"CropBox" => {
                    if let Some(media) = before.media_box.clone() {
                        out.push((key, media));
                    }
                }
                _ => {}
            },
        }
    }
    out
}

/// Every object reachable from `roots`, over `graph`.
///
/// `skip_parent_of` names objects whose `/Parent` entry must **not** be
/// followed. That single exception is what keeps a page's reachability
/// closure from being "the entire document": §7.7.3.2 gives every page a
/// `/Parent` pointing at its `Pages` node, which points at every sibling.
///
/// Iterative and budgeted ([`MAX_REACHABLE_OBJECTS`]): this runs on
/// untrusted input, and a recursive walk over an object *graph* — as
/// opposed to over one object's small value tree — is a stack overflow
/// waiting for a deep enough file.
fn reachable<G: ObjectGraph + ?Sized>(
    graph: &G,
    roots: &[ObjId],
    skip_parent_of: &HashSet<ObjId>,
) -> HashSet<ObjId> {
    let mut seen: HashSet<ObjId> = HashSet::new();
    let mut stack: Vec<ObjId> = roots.to_vec();
    let mut budget = MAX_REACHABLE_OBJECTS;

    while let Some(id) = stack.pop() {
        if budget == 0 || !seen.insert(id) {
            continue;
        }
        budget -= 1;
        let Some(value) = graph.value(id) else {
            continue;
        };
        collect_references(value, skip_parent_of.contains(&id), 0, &mut stack);
    }
    seen
}

/// Push every reference in one value tree onto `out`.
///
/// `skip_parent` drops a top-level `/Parent` entry only — a nested
/// `/Parent` (an annotation's field parent, say) is a genuine ownership
/// edge and is followed.
fn collect_references(value: &Object, skip_parent: bool, depth: usize, out: &mut Vec<ObjId>) {
    if depth > crate::pageops::assemble::MAX_COPY_DEPTH {
        return;
    }
    match value {
        Object::Reference(id) => out.push(*id),
        Object::Array(items) => {
            for item in items {
                collect_references(item, false, depth + 1, out);
            }
        }
        Object::Dict(dict) => {
            for (key, entry) in dict.iter() {
                if skip_parent && key.as_bytes() == b"Parent" {
                    continue;
                }
                collect_references(entry, false, depth + 1, out);
            }
        }
        Object::Stream(stream) => {
            for (key, entry) in stream.dict.iter() {
                if skip_parent && key.as_bytes() == b"Parent" {
                    continue;
                }
                collect_references(entry, false, depth + 1, out);
            }
        }
        _ => {}
    }
}

/// The fixed `/LastModified` date pdfce writes on the `/PieceInfo` data
/// dictionary (§14.5 Table 319 requires the key in valid §7.9.4 form). A
/// stable placeholder rather than a wall-clock read keeps a re-saved but
/// logically-unchanged sidecar byte-stable; a real authoring clock is a
/// trivial follow-up that does not change the storage contract.
const SIDECAR_DATE: &str = "D:20260801000000Z";

// =====================================================================
// Dimensioning subsystem wiring (Pass 12.M2, decision 011 §2.3/§2.4)
// =====================================================================
//
// The additive in-document integration of the `pdfce-core::dimension`
// subsystem. Each method reads the authoritative model from the catalog
// `/PieceInfo /pdfce /Private` sidecar (or starts fresh), mutates it,
// re-authors any affected `/AP`(s), (re)registers the per-group `/OCG` in
// `/OCProperties`, writes the sidecar back, and commits everything as ONE
// undoable command. All authoring is ADDITIVE (overlay-append, §5.8, R46
// zero-exception): no page content-stream byte is touched — the same
// discipline (and the same private helpers: `alloc_number`, `stage_bytes`,
// `annots_writes`, `commit`) as `add_markup`.
impl EditSession {
    /// The authoritative dimensioning model stored in this document's
    /// catalog `/PieceInfo` sidecar, or a fresh model if none is stored
    /// (Pass 12.M2). Overlay-aware — reflects edits made in this session.
    #[must_use]
    pub fn dimension_model(&self) -> DimensionModel {
        self.read_dimension_model()
    }

    /// Author a dimension onto a page: a `/Line` `/IT /LineDimension`
    /// annotation with a baked `/AP` (leader + value label), placed on its
    /// group's optional-content layer (`/OC` → the group `/OCG`, allocated on
    /// first use and registered in `/OCProperties`), its scale mirrored into
    /// a portable `/Measure` dict, and the authoritative `/PieceInfo` sidecar
    /// updated — ALL additive, ALL one undo entry (decision 011 §2.4).
    ///
    /// Returns the created annotation's [`ObjId`] and the model's
    /// [`DimensionId`]. If `group` is unknown, the dimension joins the
    /// always-present default group (ui-spec §5.3).
    ///
    /// # Errors
    ///
    /// The same guards as [`EditSession::add_markup`]: encryption,
    /// enforced certification, page range, and hidden-object exposure.
    pub fn add_dimension(
        &mut self,
        page_index: usize,
        group: GroupId,
        kind: DimensionKind,
    ) -> Result<(ObjId, DimensionId), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        let slots = self.page_slots()?;
        let count = slots.len();
        let page_id = slots
            .get(page_index)
            .ok_or(EditError::PageOutOfRange {
                index: page_index,
                count,
            })?
            .id;
        let suppressed = self.base.suppressed_object_count();
        if suppressed > 0 {
            return Err(EditError::ObjectCreationWouldExposeHiddenObjects { count: suppressed });
        }

        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let dim_id = model.add_dimension(group, kind);
        let gid = model
            .dimension(dim_id)
            .map_or(DEFAULT_GROUP_ID, |d| d.group);

        // Ensure the group has an OCG (allocate one on first use).
        let mut ocg_writes: Vec<ObjectWrite> = Vec::new();
        let ocg_id = match model.group(gid).and_then(|g| g.ocg) {
            Some(id) => id,
            None => {
                let id = ObjId::new(self.alloc_number()?, 0);
                let name = model
                    .group(gid)
                    .map_or_else(|| "Dimensions".to_owned(), |g| g.name.clone());
                ocg_writes.push(ObjectWrite {
                    id,
                    before: None,
                    after: Some(build_ocg(&name)),
                });
                if let Some(g) = model.group_mut(gid) {
                    g.ocg = Some(id);
                }
                id
            }
        };
        // The group is the authority for every display property — scale,
        // format and (Pass 27.2) drafting standard — so the style is derived
        // from it in one step rather than assembled field by field.
        let style = model.group(gid).map_or(
            DimensionStyle {
                scale: ScaleState::NeverSet,
                format: Unit::Millimeter.default_format(),
                standard: DimStandard::default(),
            },
            DimensionStyle::from,
        );

        // Author the /Line + baked /AP from the geometry and the group style.
        let authored = author_dimension(&kind, style);
        let ap_id = ObjId::new(self.alloc_number()?, 0);
        let annot_id = ObjId::new(self.alloc_number()?, 0);

        let mut ap_dict = authored.ap_dict;
        ap_dict.insert(
            Name::from(b"Length"),
            Object::Integer(i64::try_from(authored.ap_content.len()).unwrap_or(i64::MAX)),
        );
        let ap_span = self.stage_bytes(&authored.ap_content);
        let ap_stream = Object::Stream(Stream {
            dict: ap_dict,
            data_span: ap_span,
        });

        let mut annot = authored.annot;
        let mut ap = Dict::new();
        ap.insert(Name::from(b"N"), Object::Reference(ap_id));
        annot.insert(Name::from(b"AP"), Object::Dict(ap));
        annot.insert(Name::from(b"P"), Object::Reference(page_id));
        annot.insert(
            Name::from(b"F"),
            Object::Integer(i64::from(AnnotFlags::PRINT)),
        );
        // The authored-layer /OC entry (§8.11.3.3) — pdfce's render honours
        // it, and any OCG-aware reader toggles the dimension by its layer.
        annot.insert(Name::from(b"OC"), Object::Reference(ocg_id));

        // Record the wiring handles so a later scale change can regenerate.
        if let Some(d) = model.dimension_mut(dim_id) {
            d.annot = Some(annot_id);
            d.ap = Some(ap_id);
        }

        let catalog_write = self.catalog_dimension_write(&model)?;
        let mut annots_writes = self.annots_writes(page_id, annot_id, &slots)?;

        let mut objects = ocg_writes;
        objects.push(ObjectWrite {
            id: ap_id,
            before: None,
            after: Some(ap_stream),
        });
        objects.push(ObjectWrite {
            id: annot_id,
            before: None,
            after: Some(Object::Dict(annot)),
        });
        objects.push(catalog_write);
        objects.append(&mut annots_writes);

        self.commit(Command {
            kind: CommandKind::AddDimension,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok((annot_id, dim_id))
    }

    /// Create a new named dimension group with `unit`'s default number
    /// format (Pass 12.M2). The group starts scale-never-set and visible;
    /// its OCG is allocated lazily on the first dimension. Returns the new
    /// [`GroupId`]. One undo entry.
    ///
    /// # Errors
    ///
    /// Encryption / enforced-certification guards, as the other dimension
    /// operations.
    pub fn add_dimension_group(&mut self, name: &str, unit: Unit) -> Result<GroupId, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;
        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let id = model.add_group(name, unit);
        let catalog_write = self.catalog_dimension_write(&model)?;
        self.commit(Command {
            kind: CommandKind::AddDimension,
            objects: vec![catalog_write],
            removals: Vec::new(),
            trailer: None,
        });
        Ok(id)
    }

    /// Set a dimension group's scale + number format and **regenerate every
    /// wired member's baked `/AP`** (the "change the group scale → all member
    /// dimensions update" story, decision 011 §2.3, the Pass 7.1 regenerate
    /// pattern). Returns how many members were regenerated. One undo entry.
    ///
    /// # Errors
    ///
    /// Encryption / enforced-certification guards.
    pub fn set_group_scale(
        &mut self,
        group: GroupId,
        scale: ScaleState,
        format: NumberFormat,
    ) -> Result<usize, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        model.set_group_scale(group, scale, format);

        // Regeneration goes through the ONE shared path (R92). This used to
        // rewrite the annotation inline, and touched only `/Rect`,
        // `/Contents` and `/Measure` — correct for a scale change, where the
        // geometry does not move, and silently wrong for anything that DOES
        // move it, because `/L` (the measured line, §12.5.6.7) would keep its
        // old endpoints. Pass 25.5's move needs `/L` rewritten, and having two
        // regenerators disagreeing about which keys authoring owns is exactly
        // how that kind of staleness survives review.
        let members: Vec<DimensionId> = model
            .members(group)
            .filter(|d| d.annot.is_some() && d.ap.is_some())
            .map(|d| d.id)
            .collect();
        let mut objects = self.regenerate_dimension_writes(&model, &members)?;

        let catalog_write = self.catalog_dimension_write(&model)?;
        objects.push(catalog_write);
        self.commit(Command {
            kind: CommandKind::SetGroupScale {
                members: members.len(),
            },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(members.len())
    }

    /// Toggle a dimension group's optional-content layer default visibility
    /// (§8.11 `/D` config `/OFF`, Pass 12.M2). Returns the resulting
    /// visibility (the default group is un-hideable, ui-spec §5.3). One undo
    /// entry.
    ///
    /// # Errors
    ///
    /// Encryption / enforced-certification guards.
    pub fn toggle_dimension_layer(
        &mut self,
        group: GroupId,
        visible: bool,
    ) -> Result<bool, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;
        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let result = model.set_group_visible(group, visible);
        let catalog_write = self.catalog_dimension_write(&model)?;
        self.commit(Command {
            kind: CommandKind::ToggleDimensionLayer { visible: result },
            objects: vec![catalog_write],
            removals: Vec::new(),
            trailer: None,
        });
        Ok(result)
    }

    /// Every ce dimension wired onto `page_index`, with its page-space
    /// `/Rect` as `[llx, lly, urx, ury]` — the query a shell hit-tests
    /// against to let an operator click one (Pass 25.5).
    ///
    /// # Why this lives in the core rather than in the shell
    ///
    /// "Which ce dimensions are on this page, and where" needs the sidecar
    /// model, the annotation objects, and the session overlay resolved
    /// together. A GUI that assembled that itself would be reaching through
    /// three layers to rebuild something the session already knows, and would
    /// go stale the first time any of the three changed shape. It is also the
    /// query a CLI needs to report what is on a page, so it belongs where both
    /// shells can reach it (`ARCHITECTURE.md` §3).
    ///
    /// Returned in sidecar order, which is authoring order. Dimensions not yet
    /// wired into a document, or whose annotation has no readable `/Rect`, are
    /// omitted — there is nothing on the page to click.
    ///
    /// **Overlay-aware**: a dimension moved this session reports its NEW rect,
    /// so a shell hit-tests what the operator can currently see rather than
    /// what the file said on open (decision 018's discipline, applied to
    /// annotations).
    #[must_use]
    pub fn dimension_rects(&self, page_index: usize) -> Vec<(DimensionId, [f64; 4])> {
        let Ok(slots) = self.page_slots() else {
            return Vec::new();
        };
        let Some(page_id) = slots.get(page_index).map(|s| s.id) else {
            return Vec::new();
        };
        let model = self.read_dimension_model();
        model
            .dimensions()
            .iter()
            .filter_map(|record| {
                let annot_id = record.annot?;
                let dict = self.value(annot_id)?.as_dict()?;
                // `/P` names the page the annotation lives on (§12.5.2). A
                // dimension authored onto another page must not be clickable
                // here, and comparing ids is the only honest test — the
                // sidecar deliberately does not duplicate the page.
                if dict.get(b"P").and_then(Object::as_reference) != Some(page_id) {
                    return None;
                }
                let rect = dict.get(b"Rect")?.as_array()?;
                let v: Vec<f64> = rect.iter().filter_map(Object::as_number).collect();
                let [llx, lly, urx, ury] = v[..] else {
                    return None;
                };
                // Normalised: §7.9.5 allows either corner order, and a shell
                // that assumed min/max would silently fail to hit a rect
                // written the other way round.
                Some((
                    record.id,
                    [llx.min(urx), lly.min(ury), llx.max(urx), lly.max(ury)],
                ))
            })
            .collect()
    }

    /// **Place a ce dimension** — set where its line stands off and where its
    /// number sits along that line — as one undoable command (Pass 27.1).
    ///
    /// # This, not `move_dimension`, is what dragging a dimension does
    ///
    /// The operator asked for SolidWorks behaviour, and SolidWorks stores a
    /// dimension's placement as a POINT (its API takes one:
    /// `AddDimension2(x, y, z)`, "the text-placement point"). Dragging a
    /// dimension there never moves what it measures — the attachment points
    /// stay on the geometry and the extension lines stretch. Only where the
    /// dimension is DRAWN changes.
    ///
    /// So this writes two fields the value function does not read, which makes
    /// it **value-preserving by construction** rather than by care: no
    /// placement, however far it is dragged, can change the number. That is a
    /// stronger guarantee than [`Self::move_dimension`] offers — that one
    /// translates the measured points too, and while a rigid motion preserves
    /// the distance, it does take the dimension off the feature it was
    /// measuring.
    ///
    /// Both are kept. Placement is the drag; translation is for moving a
    /// dimension bodily with the thing it annotates.
    ///
    /// # Errors
    ///
    /// [`EditError::DimensionNotFound`], [`EditError::NotALinearDimension`]
    /// for a circular target (which has no axis to place along), plus the
    /// encryption and enforced-certification guards.
    pub fn place_dimension(
        &mut self,
        dimension: DimensionId,
        offset: f64,
        text_along: f64,
    ) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let record = model
            .dimension(dimension)
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;
        let DimensionKind::Linear {
            a, b, constraint, ..
        } = record.kind
        else {
            return Err(EditError::NotALinearDimension { id: dimension.0 });
        };
        if let Some(d) = model.dimension_mut(dimension) {
            d.kind = DimensionKind::Linear {
                a,
                b,
                constraint,
                offset,
                text_along,
            };
        }

        let mut objects = self.regenerate_dimension_writes(&model, &[dimension])?;
        objects.push(self.catalog_dimension_write(&model)?);
        self.commit(Command {
            kind: CommandKind::PlaceDimension,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    /// **Set a placed ce dimension's radius-versus-diameter display** — as one
    /// undoable command (Pass 34.2).
    ///
    /// # The gap this closes, in the operator's own words
    ///
    /// > *"the ce dimensions i add need to be editable as well."*
    ///
    /// Before this verb, radius-versus-diameter existed **only as a tool
    /// setting read at draw time**: the measure tool's property bar carried the
    /// toggle, [`Self::add_dimension`] baked the operator's choice into
    /// [`DimensionKind::Circular::show_diameter`], and nothing could ever
    /// change it again. An operator who placed a radius and wanted a diameter
    /// had exactly one route: delete the ce dimension and draw it a second
    /// time — which also loses its group membership, its placement, and its
    /// id. That is not an editing model, it is a redraw.
    ///
    /// # Value-preserving by construction, like `place_dimension`
    ///
    /// The stored [`FitCircle`](crate::dimension::FitCircle) is not touched:
    /// same centre, same radius, same fit residual. Only the flag that decides
    /// whether the label prints `r` or `2r` moves. So this cannot silently
    /// re-measure anything — the same structural guarantee
    /// [`Self::place_dimension`] gets from writing only fields the value
    /// function does not read, and the reason decision 022 §4.2's
    /// anti-silent-re-measure argument does not bite here.
    ///
    /// # Exactly ONE `/AP` regenerates (R92)
    ///
    /// Unlike [`Self::set_group_scale`] and [`Self::set_group_standard`],
    /// which are group-wide and regenerate every member, this is a
    /// per-ce-dimension property, so `regenerate_dimension_writes` is handed a
    /// single-element slice. It goes through that one shared regeneration path
    /// rather than a second appearance builder (R92) — a second one is how the
    /// baked `/AP` and the `/Contents` string start disagreeing about what the
    /// same ce dimension says.
    ///
    /// # Setting it to what it already is still commits
    ///
    /// Deliberate, and stated because the alternative looks tidier: an
    /// early-return on `show_diameter == current` would make an undo stack that
    /// sometimes gains an entry from a control press and sometimes does not,
    /// with nothing on screen to distinguish the two. Callers that want to
    /// suppress a no-op press should compare before calling — the GUI's
    /// selectable pair does exactly that, and can, because it already reads the
    /// current value to show which half is selected.
    ///
    /// # Errors
    ///
    /// [`EditError::DimensionNotFound`] for an unknown id;
    /// [`EditError::NotACircularDimension`] for a linear target (which has no
    /// circle, so neither reading exists); plus the encryption,
    /// enforced-certification and newer-sidecar guards every ce-dimension
    /// mutation carries.
    pub fn set_dimension_display(
        &mut self,
        dimension: DimensionId,
        show_diameter: bool,
    ) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;
        self.check_dimension_sidecar()?;

        let mut model = self.read_dimension_model();
        let record = model
            .dimension(dimension)
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;
        // Read the fit out BEFORE taking the mutable borrow, and refuse a
        // linear target here rather than inside the mutation — so a refusal
        // never leaves a half-written model behind.
        let DimensionKind::Circular { fit, .. } = record.kind else {
            return Err(EditError::NotACircularDimension { id: dimension.0 });
        };
        if let Some(d) = model.dimension_mut(dimension) {
            d.kind = DimensionKind::Circular { fit, show_diameter };
        }

        let mut objects = self.regenerate_dimension_writes(&model, &[dimension])?;
        objects.push(self.catalog_dimension_write(&model)?);
        self.commit(Command {
            kind: CommandKind::SetDimensionDisplay { show_diameter },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    /// **Set a ce dimension group's drafting standard** and regenerate every
    /// member, as one undoable command (Pass 27.2). Returns how many members
    /// were regenerated.
    ///
    /// # Why every member, immediately
    ///
    /// Exactly the precedent [`Self::set_group_scale`] set: a group exists so
    /// that its members agree. A group whose members are drawn to two
    /// different standards is not a group with a setting, it is a group with a
    /// history — and the operator would have to remember which dimensions
    /// predate the change. This is a LARGER visible change than a scale edit
    /// (a scale edit changes numbers; this changes shapes), so the member
    /// count is what a UI should disclose before applying it.
    ///
    /// # The decimal marker rides along, disclosed
    ///
    /// ISO 129-1:2018 cl. 4.1.1 mandates a comma decimal marker (a verified
    /// "shall", and widely violated in practice). Switching to ISO therefore
    /// sets `format.decimal_marker` as a side effect — but it is a normal
    /// field the operator can set back afterwards, not something welded to the
    /// standard. Switching to ANSI restores the point for the same reason.
    ///
    /// # Errors
    ///
    /// [`EditError::DimensionGroupNotFound`], plus the encryption,
    /// certification and newer-sidecar guards.
    pub fn set_group_standard(
        &mut self,
        group: GroupId,
        standard: DimStandard,
    ) -> Result<usize, EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;
        self.check_dimension_sidecar()?;

        let mut model = self.read_dimension_model();
        let Some(g) = model.group_mut(group) else {
            return Err(EditError::DimensionGroupNotFound { id: group.0 });
        };
        g.standard = standard;
        g.format.decimal_marker = match standard {
            DimStandard::Ansi => crate::dimension::DecimalMarker::Point,
            DimStandard::Iso => crate::dimension::DecimalMarker::Comma,
        };

        let members: Vec<DimensionId> = model
            .members(group)
            .filter(|d| d.annot.is_some() && d.ap.is_some())
            .map(|d| d.id)
            .collect();
        let mut objects = self.regenerate_dimension_writes(&model, &members)?;
        objects.push(self.catalog_dimension_write(&model)?);
        self.commit(Command {
            kind: CommandKind::SetGroupStandard {
                members: members.len(),
            },
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(members.len())
    }

    /// **Delete a ce dimension**, as one undoable command (Pass 25.6).
    ///
    /// Removes three things together, because leaving any one of them is a
    /// different kind of wrong:
    ///
    /// 1. the reference from its page's `/Annots` — otherwise the page points
    ///    at an object that is gone;
    /// 2. the annotation dictionary and its `/AP` `/N` appearance stream —
    ///    the stream is authored for this dimension alone and referenced by
    ///    nothing else, so leaving it orphans a stream in every later save;
    /// 3. the record from the `/PieceInfo` sidecar — otherwise pdfce keeps
    ///    believing in a dimension the file no longer contains, and the next
    ///    group-wide re-format would try to regenerate it.
    ///
    /// The group is left alone even when this was its last member. A group is
    /// a named container with a scale the operator calibrated; deleting it as
    /// a side effect of removing the last dimension would throw that
    /// calibration away silently, and re-measuring is not free.
    ///
    /// # Errors
    ///
    /// [`EditError::DimensionNotFound`] for an unknown id or one never wired
    /// into a document, plus the encryption and enforced-certification guards.
    /// Every refusal happens before any mutation (rule 4).
    pub fn delete_dimension(&mut self, dimension: DimensionId) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let record = model
            .dimension(dimension)
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;
        let annot_id = record
            .annot
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;
        let ap_id = record.ap;

        // The page it lives on, from the annotation's own `/P` (§12.5.2) —
        // the sidecar deliberately does not duplicate the page, so `/P` is the
        // only honest source.
        let page_id = self
            .value(annot_id)
            .and_then(Object::as_dict)
            .and_then(|d| d.get(b"P").and_then(Object::as_reference))
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;

        let Some(Object::Dict(page_dict)) = self.value(page_id) else {
            return Err(EditError::NotADictionary {
                id: page_id,
                key: "Annots",
            });
        };
        let mut updated = page_dict.clone();
        let mut objects: Vec<ObjectWrite> = Vec::new();
        // Same helper, same two `/Annots` shapes, as redaction-mark removal:
        // `Some` when `/Annots` is an indirect array (patch that object, leave
        // the page dict alone), `None` when inline (already composed into
        // `updated`). Writing the page dict in the indirect case would be a
        // no-op write that inflates the dirty set.
        match self.remove_from_annots(&mut updated, &[annot_id])? {
            Some(shared) => objects.push(shared),
            None => objects.push(self.page_write(page_id, updated)),
        }

        model.remove_dimension(dimension);
        objects.push(self.catalog_dimension_write(&model)?);

        let mut removals: Vec<Removal> = Vec::new();
        for id in std::iter::once(annot_id).chain(ap_id) {
            if self.base.get(id).is_some() || self.state.contains_key(&id) {
                removals.push(Removal {
                    id,
                    was_deleted: self.deleted.contains(&id),
                    is_deleted: true,
                });
            }
        }

        self.commit(Command {
            kind: CommandKind::DeleteDimension,
            objects,
            removals,
            trailer: None,
        });
        Ok(())
    }

    /// **Move a ce dimension** by a page-space `(dx, dy)`, as one undoable
    /// command (Pass 25.5).
    ///
    /// Translates the stored geometry and regenerates the annotation and its
    /// baked `/AP` from it. The measured VALUE is unchanged — a translation
    /// preserves every distance — so the label reads the same before and
    /// after; what moves is where the dimension sits on the page.
    ///
    /// # Why regeneration rather than patching `/Rect`
    ///
    /// A ce dimension's appearance is baked at absolute coordinates:
    /// leader lines, ticks, arrowheads and the label are all drawn into the
    /// `/AP` stream in the annotation's own space. Nudging `/Rect` alone would
    /// slide the box and leave the drawing inside it exactly where it was —
    /// visibly wrong at the first pixel. `author_dimension` is pure and
    /// deterministic precisely so it can be re-run (`add_dimension` records
    /// the `annot`/`ap` handles "so a later scale change can regenerate"), and
    /// this is the first caller to take it up on that.
    ///
    /// # What is preserved
    ///
    /// The regenerated annotation starts from the EXISTING dictionary and
    /// overwrites only the keys authoring owns (`/Rect`, `/L`, `/Contents`,
    /// `/C`, `/Measure`). `/P`, `/OC`, `/F`, and any key another product
    /// added survive untouched — the round-trip discipline applied inside a
    /// dictionary rather than across a file.
    ///
    /// # Errors
    ///
    /// [`EditError::DimensionNotFound`] for an unknown id or one never wired
    /// into a document, plus the usual encryption / enforced-certification
    /// guards. Every refusal happens before any mutation (rule 4).
    pub fn move_dimension(
        &mut self,
        dimension: DimensionId,
        dx: f64,
        dy: f64,
    ) -> Result<(), EditError> {
        if self.base.trailer().contains_key(b"Encrypt") {
            return Err(EditError::DocumentEncrypted);
        }
        self.check_certification()?;

        self.check_dimension_sidecar()?;
        let mut model = self.read_dimension_model();
        let record = model
            .dimension(dimension)
            .ok_or(EditError::DimensionNotFound { id: dimension.0 })?;
        let moved = record.kind.translated(dx, dy);
        if let Some(d) = model.dimension_mut(dimension) {
            d.kind = moved;
        }

        let mut objects = self.regenerate_dimension_writes(&model, &[dimension])?;
        objects.push(self.catalog_dimension_write(&model)?);
        self.commit(Command {
            kind: CommandKind::MoveDimension,
            objects,
            removals: Vec::new(),
            trailer: None,
        });
        Ok(())
    }

    // ---- private helpers ----

    /// Re-author the annotation + `/AP` of each named dimension from the
    /// model's CURRENT geometry, scale and format.
    ///
    /// The one regeneration path, shared by every operation that changes what
    /// a ce dimension should look like. A second copy of this would be a
    /// second place for "which keys does authoring own" to be answered, and
    /// the two answers would drift (R92).
    ///
    /// Dimensions not yet wired into a document (no `annot`/`ap` handle) are
    /// skipped rather than refused: they have no appearance to regenerate, and
    /// failing the whole operation because one member is unwired would make a
    /// group-wide format change fragile for no benefit.
    fn regenerate_dimension_writes(
        &mut self,
        model: &DimensionModel,
        ids: &[DimensionId],
    ) -> Result<Vec<ObjectWrite>, EditError> {
        let mut objects = Vec::new();
        for &id in ids {
            let Some(record) = model.dimension(id) else {
                return Err(EditError::DimensionNotFound { id: id.0 });
            };
            let (Some(annot_id), Some(ap_id)) = (record.annot, record.ap) else {
                continue; // never wired into a document; nothing to regenerate
            };
            let group = model
                .group(record.group)
                .ok_or(EditError::DimensionGroupNotFound { id: record.group.0 })?;
            let authored = author_dimension(&record.kind, DimensionStyle::from(group));

            let mut ap_dict = authored.ap_dict;
            ap_dict.insert(
                Name::from(b"Length"),
                Object::Integer(i64::try_from(authored.ap_content.len()).unwrap_or(i64::MAX)),
            );
            let ap_span = self.stage_bytes(&authored.ap_content);
            objects.push(ObjectWrite {
                id: ap_id,
                before: self.state.get(&ap_id).cloned(),
                after: Some(Object::Stream(Stream {
                    dict: ap_dict,
                    data_span: ap_span,
                })),
            });

            // Start from the EXISTING dictionary so `/P`, `/OC`, `/F` and any
            // foreign key survive, and overwrite only what authoring owns.
            let existing = self
                .value(annot_id)
                .and_then(Object::as_dict)
                .cloned()
                .unwrap_or_default();
            let mut annot = existing;
            for key in AUTHORED_ANNOT_KEYS
                .into_iter()
                .chain(std::iter::once(AUTHORED_MEASURE_KEY))
            {
                match authored.annot.get(key) {
                    Some(v) => {
                        annot.insert(Name::from(key), v.clone());
                    }
                    // Authoring did not produce this key for the new state —
                    // an uncalibrated group has no `/Measure` — so a stale one
                    // must GO, not linger and claim a scale that no longer
                    // applies.
                    None => {
                        annot.remove(key);
                    }
                }
            }
            objects.push(ObjectWrite {
                id: annot_id,
                before: self.state.get(&annot_id).cloned(),
                after: Some(Object::Dict(annot)),
            });
        }
        Ok(objects)
    }

    /// Read the authoritative model from the catalog `/PieceInfo /pdfce
    /// /Private` sidecar, or a fresh model if absent/unparseable (the
    /// disclose-and-start-fresh posture: a malformed sidecar never panics).
    fn read_dimension_model(&self) -> DimensionModel {
        self.try_read_dimension_model().unwrap_or_default()
    }

    /// Refuse a ce-dimension WRITE when the document's sidecar came from a
    /// newer pdfce than this build understands (Pass 27.2).
    ///
    /// Called by every ce-dimension mutation, in the same position as
    /// [`Self::check_certification`] — before any object is touched, so a
    /// refusal leaves the document exactly as it was (rule 4).
    ///
    /// A document with NO sidecar is not a newer document; it is a document
    /// that has never been dimensioned, and starting one is the whole point.
    /// Only a sidecar that declares a version this build cannot fully
    /// represent is refused.
    fn check_dimension_sidecar(&self) -> Result<(), EditError> {
        let Some(cid) = self.graph().catalog_id() else {
            return Ok(());
        };
        let found = self
            .value(cid)
            .and_then(Object::as_dict)
            .cloned()
            .and_then(|catalog| self.deref_dict(catalog.get(b"PieceInfo")))
            .and_then(|piece| self.deref_dict(piece.get(b"pdfce")))
            .and_then(|pdfce| self.deref_value(pdfce.get(b"Private")))
            .and_then(|private| crate::dimension::sidecar_version(&private));
        match found {
            Some(v) if v > crate::dimension::SIDECAR_VERSION => {
                Err(EditError::SidecarWrittenByNewerBuild {
                    found: v,
                    supported: crate::dimension::SIDECAR_VERSION,
                })
            }
            _ => Ok(()),
        }
    }

    fn try_read_dimension_model(&self) -> Option<DimensionModel> {
        let cid = self.graph().catalog_id()?;
        let catalog = self.value(cid)?.as_dict()?.clone();
        let piece = self.deref_dict(catalog.get(b"PieceInfo"))?;
        let pdfce = self.deref_dict(piece.get(b"pdfce"))?;
        let private = self.deref_value(pdfce.get(b"Private"))?;
        deserialize_model(&private)
    }

    /// Resolve an optional object (following one indirect reference) to an
    /// owned clone, overlay-aware.
    fn deref_value(&self, obj: Option<&Object>) -> Option<Object> {
        match obj? {
            Object::Reference(r) => self.value(*r).cloned(),
            other => Some(other.clone()),
        }
    }

    /// Resolve an optional object to an owned dictionary clone (a `/Stream`'s
    /// dict counts), overlay-aware.
    fn deref_dict(&self, obj: Option<&Object>) -> Option<Dict> {
        match self.deref_value(obj)? {
            Object::Dict(d) => Some(d),
            Object::Stream(s) => Some(s.dict),
            _ => None,
        }
    }

    /// Build the single catalog [`ObjectWrite`] that carries the updated
    /// `/PieceInfo` sidecar (§14.5) and the rebuilt `/OCProperties` (§8.11),
    /// preserving any foreign product `/PieceInfo` keys and foreign OCGs.
    fn catalog_dimension_write(&self, model: &DimensionModel) -> Result<ObjectWrite, EditError> {
        let catalog_id = self.graph().catalog_id().ok_or(EditError::NotADictionary {
            id: ObjId::new(0, 0),
            key: "Root",
        })?;
        let catalog = match self.value(catalog_id) {
            Some(Object::Dict(d)) => d.clone(),
            _ => {
                return Err(EditError::NotADictionary {
                    id: catalog_id,
                    key: "Root",
                });
            }
        };

        // /PieceInfo — preserve foreign product keys, replace /pdfce.
        let sidecar = serialize_model(model);
        let mut data_dict = Dict::new();
        data_dict.insert(
            Name::from(b"LastModified"),
            Object::String(SIDECAR_DATE.as_bytes().to_vec()),
        );
        data_dict.insert(Name::from(b"Private"), sidecar);
        let mut piece = self
            .deref_dict(catalog.get(b"PieceInfo"))
            .unwrap_or_default();
        piece.insert(Name::from(b"pdfce"), Object::Dict(data_dict));

        // /OCProperties — rebuild from pdfce's group OCGs, preserving foreign.
        let mine: Vec<(ObjId, bool)> = model
            .groups()
            .iter()
            .filter_map(|g| g.ocg.map(|o| (o, g.visible)))
            .collect();
        let mine_ids: std::collections::BTreeSet<ObjId> = mine.iter().map(|(id, _)| *id).collect();
        let foreign: Vec<ObjId> = self
            .deref_dict(catalog.get(b"OCProperties"))
            .and_then(|ocp| {
                ocp.get(b"OCGs").and_then(Object::as_array).map(|a| {
                    a.iter()
                        .filter_map(Object::as_reference)
                        .filter(|r| !mine_ids.contains(r))
                        .collect()
                })
            })
            .unwrap_or_default();

        let mut cat2 = catalog.clone();
        cat2.insert(Name::from(b"PieceInfo"), Object::Dict(piece));
        if !mine.is_empty() || !foreign.is_empty() {
            cat2.insert(
                Name::from(b"OCProperties"),
                build_ocproperties(&mine, &foreign),
            );
        }
        Ok(ObjectWrite {
            id: catalog_id,
            before: self.state.get(&catalog_id).cloned(),
            after: Some(Object::Dict(cat2)),
        })
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

    /// A small classic PDF with one page and an `/Info` dictionary.
    fn pdf_with_info() -> Vec<u8> {
        build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> >>",
                "<< /Title (Original) >>",
            ],
            "/Info 4 0 R /ID [<0102> <0304>] ",
        )
    }

    /// The same document with no `/Info` and no `/ID`.
    fn pdf_without_info() -> Vec<u8> {
        build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> >>",
            ],
            "",
        )
    }

    fn build(bodies: &[&str], trailer_extra: &str) -> Vec<u8> {
        let mut buf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in bodies.iter().enumerate() {
            offsets.push(buf.len());
            buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        let xref_at = buf.len();
        let size = bodies.len() + 1;
        buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for off in &offsets {
            buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        buf.extend_from_slice(
            format!(
                "trailer\n<< /Size {size} /Root 1 0 R {trailer_extra}>>\nstartxref\n{xref_at}\n%%EOF\n"
            )
            .as_bytes(),
        );
        buf
    }

    fn session(bytes: Vec<u8>) -> EditSession {
        EditSession::new(Document::from_bytes(bytes).unwrap())
    }

    #[test]
    fn a_fresh_session_is_unmodified_and_has_no_history() {
        let s = session(pdf_with_info());
        assert!(!s.is_modified());
        assert!(s.dirty_set().is_empty());
        assert!(!s.can_undo());
        assert!(!s.can_redo());
    }

    #[test]
    fn edit_then_undo_leaves_a_structurally_empty_dirty_set() {
        // THE Pass 3.1 contract, at the dirty-set level. "Empty-ish" is
        // not good enough: the writer branches on `is_empty()`, and a
        // set holding a net-zero entry would append a revision.
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("Changed")).unwrap();
        assert!(s.is_modified());
        assert_eq!(s.dirty_set().len(), 1);

        s.undo();
        let dirty = s.dirty_set();
        assert!(dirty.is_empty(), "dirty set must be structurally empty");
        assert_eq!(dirty.len(), 0);
        assert!(!dirty.changes_content());
        assert!(!s.is_modified());
    }

    #[test]
    fn redo_restores_the_edit() {
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("Changed")).unwrap();
        s.undo();
        assert_eq!(s.redo(), Some(CommandKind::SetInfoField(InfoField::Title)));
        assert!(s.is_modified());
        assert_eq!(s.info_text(InfoField::Title).unwrap().text, "Changed");
    }

    #[test]
    fn a_new_edit_after_an_undo_clears_the_redo_stack() {
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("A")).unwrap();
        s.undo();
        assert!(s.can_redo());
        s.set_info_field(InfoField::Author, Some("B")).unwrap();
        assert!(!s.can_redo(), "the redone future no longer exists");
    }

    #[test]
    fn repeated_edits_to_one_object_coalesce_into_one_dirty_entry() {
        // Three edits, three undo entries, but ONE object in the update
        // section — an update that restated the object three times would
        // be a minimal-diff violation.
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("One")).unwrap();
        s.set_info_field(InfoField::Author, Some("Two")).unwrap();
        s.set_info_field(InfoField::Subject, Some("Three")).unwrap();
        assert_eq!(s.undo_depth(), 3);
        assert_eq!(s.dirty_set().len(), 1);
    }

    #[test]
    fn partial_undo_of_several_edits_leaves_only_the_net_difference() {
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("One")).unwrap();
        s.set_info_field(InfoField::Author, Some("Two")).unwrap();
        s.undo(); // author gone; title still changed
        assert!(s.is_modified());
        assert_eq!(s.dirty_set().len(), 1);
        s.undo(); // back to base
        assert!(!s.is_modified());
    }

    #[test]
    fn setting_a_field_to_its_existing_value_is_a_no_op() {
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, Some("Original"))
            .unwrap();
        assert!(!s.can_undo(), "a no-op must not reach the undo stack");
        assert!(!s.is_modified());
    }

    #[test]
    fn clearing_a_field_removes_the_entry_rather_than_nulling_it() {
        let mut s = session(pdf_with_info());
        s.set_info_field(InfoField::Title, None).unwrap();
        let id = s.info_id().unwrap();
        let Some(Object::Dict(d)) = s.value(id) else {
            panic!("info is not a dict");
        };
        assert!(
            !d.0.iter().any(|(k, _)| k.as_bytes() == b"Title"),
            "the physical entry must be gone, not set to null"
        );
        assert!(s.is_modified());
    }

    #[test]
    fn creating_info_writes_both_the_object_and_the_trailer_reference() {
        let mut s = session(pdf_without_info());
        assert!(s.info_id().is_none());
        s.set_info_field(InfoField::Title, Some("Fresh")).unwrap();

        let dirty = s.dirty_set();
        assert_eq!(dirty.len(), 1, "one created object");
        assert!(dirty.changes_content());
        assert!(
            dirty.trailer_patch().contains_key(b"Info"),
            "the trailer must gain /Info"
        );
        // Table 15: /Info "shall be an indirect reference".
        assert!(
            dirty
                .trailer_patch()
                .get(b"Info")
                .and_then(Object::as_reference)
                .is_some()
        );
    }

    #[test]
    fn undoing_the_creation_of_info_removes_the_object_and_the_reference() {
        let mut s = session(pdf_without_info());
        s.set_info_field(InfoField::Title, Some("Fresh")).unwrap();
        s.undo();
        assert!(s.info_id().is_none(), "the trailer reference must be gone");
        assert!(s.dirty_set().is_empty());
        assert!(!s.is_modified());
    }

    #[test]
    fn created_objects_get_a_number_past_everything_the_base_uses() {
        let mut s = session(pdf_without_info());
        s.set_info_field(InfoField::Title, Some("Fresh")).unwrap();
        let id = s.info_id().unwrap();
        assert_eq!(id, ObjId::new(4, 0));
        assert!(s.document().get(id).is_none(), "must be a NEW number");
    }

    /// The bug the `writer_roundtrip` fuzz target found on its first
    /// 60-second run: a file whose `/Size` under-reports loads with real
    /// cross-reference entries invisible, and creating an object both
    /// (a) picked a number the file already used, and (b) raised
    /// `/Size` enough to resurrect the hidden — and unparseable —
    /// objects. The saved file then failed to reload.
    #[test]
    fn object_creation_is_refused_when_size_is_hiding_entries() {
        // Five real entries, `/Size 3`. Objects 3 and 4 exist in the
        // file and are invisible to a conforming reader.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [] /Count 0 >>",
                "<< /Hidden (three) >>",
                "<< /Hidden (four) >>",
            ],
            "",
        );
        let bytes = shrink_size(bytes, 3);
        let doc = Document::from_bytes(bytes).unwrap();
        assert_eq!(doc.suppressed_object_count(), 2);
        // Allocation is computed from the UNFILTERED maximum, so it can
        // never collide with object 3 or 4 even if creation were allowed.
        assert_eq!(doc.next_object_number(), Some(5));

        let mut s = EditSession::new(doc);
        let err = s.set_info_field(InfoField::Title, Some("New")).unwrap_err();
        assert!(matches!(
            err,
            EditError::ObjectCreationWouldExposeHiddenObjects { count: 2 }
        ));
        assert!(!s.can_undo(), "a refused edit must leave no history");
        assert!(!s.is_modified());
    }

    #[test]
    fn editing_an_existing_object_still_works_when_size_hides_entries() {
        // The refusal is scoped to *creation*: raising /Size is what
        // exposes hidden entries, and editing an existing object does
        // not raise it. Refusing more broadly would decline a safe
        // operation on a damaged-but-readable file.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>",
                "<< /Hidden (four) >>",
            ],
            "",
        );
        let bytes = shrink_size(bytes, 4);
        let doc = Document::from_bytes(bytes).unwrap();
        assert_eq!(doc.suppressed_object_count(), 1);
        let mut s = EditSession::new(doc);
        s.set_page_rotation(0, 90).unwrap();
        assert!(s.is_modified());
    }

    /// Rewrite a fixture's trailer `/Size` to `size`, leaving the
    /// cross-reference table's own entries alone — which is exactly the
    /// damaged shape real files exhibit.
    fn shrink_size(bytes: Vec<u8>, size: usize) -> Vec<u8> {
        // Byte-level, because the fixture's §7.5.2 binary-comment line
        // is deliberately not valid UTF-8.
        let needle = b"/Size ";
        let at = bytes
            .windows(needle.len())
            .rposition(|w| w == needle)
            .expect("fixture has no /Size");
        let value_start = at + needle.len();
        let digits = bytes
            .get(value_start..)
            .unwrap_or(&[])
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let mut out = bytes;
        out.splice(
            value_start..value_start + digits,
            size.to_string().into_bytes(),
        );
        out
    }

    #[test]
    fn rotation_must_be_a_multiple_of_ninety() {
        let mut s = session(pdf_with_info());
        let err = s.set_page_rotation(0, 45).unwrap_err();
        assert!(matches!(
            err,
            EditError::RotationNotMultipleOf90 { degrees: 45 }
        ));
        assert!(!s.can_undo());
    }

    #[test]
    fn rotation_normalizes_negative_and_overflowing_values() {
        // Table 30 says "a multiple of 90" and nothing about range, so
        // -90 and 450 are both conforming inputs.
        let mut s = session(pdf_with_info());
        s.set_page_rotation(0, -90).unwrap();
        assert_eq!(s.pages().unwrap()[0].rotate, 270);
        s.set_page_rotation(0, 450).unwrap();
        assert_eq!(s.pages().unwrap()[0].rotate, 90);
    }

    #[test]
    fn relative_rotation_accumulates_from_the_effective_value() {
        let mut s = session(pdf_with_info());
        s.rotate_page_by(0, 90).unwrap();
        s.rotate_page_by(0, 90).unwrap();
        assert_eq!(s.pages().unwrap()[0].rotate, 180);
        s.rotate_page_by(0, -90).unwrap();
        assert_eq!(s.pages().unwrap()[0].rotate, 90);
    }

    #[test]
    fn rotating_back_to_the_original_is_not_a_change() {
        // Four quarter turns land on the base value, so the diff is
        // empty even though commands are on the stack — the same
        // net-zero rule undo relies on, reached a different way. The
        // fourth turn must NOT leave an explicit `/Rotate 0` behind on
        // a page that never had the entry.
        let mut s = session(pdf_with_info());
        for _ in 0..4 {
            s.rotate_page_by(0, 90).unwrap();
        }
        assert!(s.can_undo());
        assert!(
            !s.is_modified(),
            "0 -> 90 -> 180 -> 270 -> 0 must net to nothing"
        );
        assert_eq!(s.pages().unwrap()[0].rotate, 0);
    }

    #[test]
    fn setting_an_inherited_rotation_writes_no_redundant_entry() {
        // The page inherits 90 from its Pages node. Setting 90 must not
        // stamp an explicit /Rotate 90 onto the page — that would be
        // modifying an object pdfce was not asked to modify (§5).
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /Rotate 90 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> >>",
            ],
            "",
        );
        let mut s = session(bytes);
        assert_eq!(s.pages().unwrap()[0].rotate, 90);
        s.set_page_rotation(0, 90).unwrap();
        assert!(!s.is_modified());
        assert!(!s.can_undo());

        // But overriding the inherited value DOES write the entry.
        s.set_page_rotation(0, 0).unwrap();
        assert!(s.is_modified());
        assert_eq!(s.pages().unwrap()[0].rotate, 0);
    }

    #[test]
    fn an_unusual_but_legal_rotate_spelling_is_not_normalized() {
        // R33: `/Rotate 450` means 90 (Table 30 constrains only "a
        // multiple of 90"). Setting 90 must leave the file's own
        // spelling alone rather than rewriting it to `90`.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Rotate 450 >>",
            ],
            "",
        );
        let mut s = session(bytes);
        assert_eq!(s.pages().unwrap()[0].rotate, 90);
        s.set_page_rotation(0, 90).unwrap();
        assert!(!s.is_modified(), "no normalization, no change");
    }

    #[test]
    fn page_index_out_of_range_is_a_named_refusal() {
        let mut s = session(pdf_with_info());
        let err = s.set_page_rotation(7, 90).unwrap_err();
        assert!(matches!(
            err,
            EditError::PageOutOfRange { index: 7, count: 1 }
        ));
    }

    #[test]
    fn pages_reflect_unsaved_rotation_edits() {
        let mut s = session(pdf_with_info());
        assert_eq!(s.pages().unwrap()[0].rotate, 0);
        s.set_page_rotation(0, 270).unwrap();
        assert_eq!(s.pages().unwrap()[0].rotate, 270);
        s.undo();
        assert_eq!(s.pages().unwrap()[0].rotate, 0);
    }

    #[test]
    fn the_base_document_is_never_mutated() {
        // The retained buffer and the parsed graph are what verbatim
        // re-emission depends on; an edit that touched them would break
        // §5 for every OTHER object in the file.
        let bytes = pdf_with_info();
        let mut s = session(bytes.clone());
        s.set_page_rotation(0, 90).unwrap();
        s.set_info_field(InfoField::Title, Some("Changed")).unwrap();
        assert_eq!(s.document().bytes(), bytes.as_slice());
        let page = s.document().get(ObjId::new(3, 0)).unwrap();
        assert!(page.value.as_dict().unwrap().get(b"Rotate").is_none());
    }

    #[test]
    fn undo_history_is_bounded_without_affecting_the_diff() {
        // Overflowing the stack must cost history, never correctness:
        // the dirty set is a diff, not a replay.
        let mut s = session(pdf_with_info());
        for i in 0..(MAX_UNDO_DEPTH + 10) {
            s.set_info_field(InfoField::Title, Some(&format!("v{i}")))
                .unwrap();
        }
        assert_eq!(s.undo_depth(), MAX_UNDO_DEPTH);
        assert_eq!(s.dirty_set().len(), 1);
        assert_eq!(
            s.info_text(InfoField::Title).unwrap().text,
            format!("v{}", MAX_UNDO_DEPTH + 9)
        );
    }

    #[test]
    fn undo_and_redo_return_what_they_moved() {
        let mut s = session(pdf_with_info());
        s.set_page_rotation(0, 90).unwrap();
        assert_eq!(
            s.undo_kind(),
            Some(CommandKind::SetPageRotation {
                page_index: 0,
                degrees: 90
            })
        );
        assert_eq!(s.undo(), s.redo_kind());
        assert!(s.redo().is_some());
        assert!(s.redo().is_none());
    }

    #[test]
    fn text_strings_round_trip_through_both_encodings() {
        for text in ["Plain ASCII", "Café", "日本語", ""] {
            let encoded = encode_text_string(text);
            let decoded = decode_text_string(&encoded);
            assert_eq!(decoded.text, text, "round trip failed for {text:?}");
            assert!(decoded.exact, "round trip was inexact for {text:?}");
        }
    }

    #[test]
    fn ascii_stays_readable_and_non_ascii_takes_the_bom_form() {
        assert_eq!(encode_text_string("Report 2026"), b"Report 2026".to_vec());
        assert_eq!(encode_text_string("é").first(), Some(&0xFE));
    }

    #[test]
    fn undecodable_bytes_are_flagged_rather_than_guessed() {
        // 0x91 is where PDFDocEncoding and Latin-1 disagree, and the
        // PDFDocEncoding table is a recorded spec-RAG gap. Guessing
        // would be silently wrong; U+FFFD plus `exact: false` is
        // visibly incomplete, which a front end can act on.
        let decoded = decode_text_string(&[b'A', 0x91, b'B']);
        assert!(!decoded.exact);
        assert!(decoded.text.contains('\u{FFFD}'));

        // An odd-length UTF-16BE body is malformed, and says so.
        assert!(!decode_text_string(&[0xFE, 0xFF, 0x00]).exact);
    }

    #[test]
    fn normalize_rotation_uses_a_positive_modulo() {
        // Rust's `%` keeps the dividend's sign, so a naive
        // implementation yields -90 here and every renderer downstream
        // sees a rotation it does not expect.
        assert_eq!(normalize_rotation(-90), 270);
        assert_eq!(normalize_rotation(-450), 270);
        assert_eq!(normalize_rotation(360), 0);
        assert_eq!(normalize_rotation(0), 0);
        // A malformed non-multiple floors to the enclosing quarter turn.
        assert_eq!(normalize_rotation(100), 90);
    }

    #[test]
    fn info_fields_all_have_distinct_keys() {
        let mut keys: Vec<&[u8]> = InfoField::all().iter().map(|f| f.key()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before);
        // R41: /Producer is deliberately NOT editable here.
        assert!(!keys.contains(&&b"Producer"[..]));
    }

    // -- Pass 6.1 annotation authoring ---------------------------------

    use crate::annot_author::{Color, MarkupSpec, TextMarkupKind};
    use crate::page_tree::Rect;

    /// A square spec on a fixed rect, the simplest authoring case.
    fn square_spec() -> MarkupSpec {
        MarkupSpec::Square {
            rect: Rect {
                llx: 20.0,
                lly: 20.0,
                urx: 120.0,
                ury: 70.0,
            },
            border: Some(Color::Rgb(1.0, 0.0, 0.0)),
            interior: None,
            border_width: 2.0,
        }
    }

    #[test]
    fn adding_a_markup_creates_appearance_annotation_and_patches_annots() {
        // One gesture ⇒ appearance stream + annotation dict + /Annots patch,
        // all in one command (§11.3, R49).
        let mut s = session(pdf_without_info());
        let annot_id = s.add_markup(0, &square_spec()).unwrap();
        assert!(s.is_modified());
        assert_eq!(s.undo_depth(), 1, "one undo entry for the whole gesture");

        // The annotation dict is present, geometry + AP wired.
        let Some(Object::Dict(annot)) = s.value(annot_id) else {
            panic!("annotation not created");
        };
        assert_eq!(
            annot.get(b"Subtype").unwrap().as_name().unwrap().as_bytes(),
            b"Square"
        );
        assert!(annot.get(b"AP").is_some(), "R44: a full /AP is baked");
        assert!(annot.get(b"P").is_some(), "back-reference to the page");

        // The page's /Annots now references it (page 3 in pdf_without_info).
        let Some(Object::Dict(page)) = s.value(ObjId::new(3, 0)) else {
            panic!("page missing");
        };
        let Some(Object::Array(annots)) = page.get(b"Annots") else {
            panic!("/Annots not created as an array");
        };
        assert_eq!(annots.len(), 1);
        assert_eq!(annots[0].as_reference(), Some(annot_id));
    }

    #[test]
    fn authored_appearance_survives_save_reload_byte_exact_r44() {
        // R44 round-trip: author → save → reload → the /AP /N stream's
        // decoded bytes equal what the generator emitted, and Pass 6.0
        // selects it as the normal appearance.
        let expected = {
            let a = crate::annot_author::build_appearance(&square_spec());
            a.ap_content
        };
        let mut s = session(pdf_without_info());
        let annot_id = s.add_markup(0, &square_spec()).unwrap();
        let (bytes, _report) = s
            .to_incremental_bytes(&crate::writer::SaveOptions::identity())
            .unwrap();

        let reloaded = Document::from_bytes(bytes).unwrap();
        // The reloaded annotation still resolves a usable normal appearance.
        let annots = crate::annot::page_annotations(&reloaded, ObjId::new(3, 0));
        assert_eq!(annots.len(), 1);
        let ap_id = match &annots[0].appearance {
            crate::annot::Appearance::Normal { stream_id } => stream_id.unwrap(),
            other => panic!("expected a normal appearance, got {other:?}"),
        };
        // Its content bytes survived the staging → save → reload path exact.
        let Some(io) = reloaded.get(ap_id) else {
            panic!("appearance stream missing after reload");
        };
        let Object::Stream(stream) = &io.value else {
            panic!("appearance is not a stream");
        };
        let raw = stream.data_span.slice(reloaded.bytes()).unwrap();
        let decoded = crate::filters::decode_stream(&stream.dict, raw).unwrap();
        assert_eq!(decoded, expected, "authored appearance bytes must survive");
        // The annotation id we created is the one referenced (basic sanity).
        assert!(reloaded.get(annot_id).is_some());
    }

    #[test]
    fn undo_of_authoring_removes_everything_and_empties_the_dirty_set() {
        let mut s = session(pdf_without_info());
        s.add_markup(0, &square_spec()).unwrap();
        assert!(!s.dirty_set().is_empty());
        s.undo();
        assert!(!s.is_modified(), "undo of authoring nets to nothing");
        assert!(s.dirty_set().is_empty());
        // The page's /Annots patch is gone too (the page reads through to
        // the base, which had no /Annots).
        let Some(Object::Dict(page)) = s.value(ObjId::new(3, 0)) else {
            panic!("page missing");
        };
        assert!(page.get(b"Annots").is_none());
    }

    fn freetext_spec() -> TextAnnotSpec {
        TextAnnotSpec::FreeText {
            rect: page_tree::Rect {
                llx: 20.0,
                lly: 40.0,
                urx: 180.0,
                ury: 70.0,
            },
            text: "Reviewed".to_owned(),
            font: crate::fontdata::Std14::Helvetica,
            font_size: 12.0,
            color: crate::vartext::TextColor::Gray(0.0),
            quadding: crate::vartext::Quadding::Center,
            multiline: false,
            border: None,
            border_width: 0.0,
        }
    }

    #[test]
    fn adding_freetext_bakes_da_contents_q_and_ap() {
        let mut s = session(pdf_without_info());
        let annot_id = s.add_text_annotation(0, &freetext_spec()).unwrap();
        assert_eq!(s.undo_depth(), 1, "one undo entry for the whole gesture");
        let Some(Object::Dict(annot)) = s.value(annot_id) else {
            panic!("annotation not created");
        };
        assert_eq!(
            annot.get(b"Subtype").unwrap().as_name().unwrap().as_bytes(),
            b"FreeText"
        );
        assert!(annot.get(b"DA").is_some(), "/DA is required on FreeText");
        assert!(annot.get(b"Contents").is_some(), "text goes in /Contents");
        assert_eq!(
            annot.get(b"Q").unwrap().as_number().unwrap() as i64,
            1,
            "centre quadding recorded"
        );
        assert!(annot.get(b"AP").is_some(), "R44: a baked /AP");
        // Print flag only (no NoZoom/NoRotate for FreeText).
        assert_eq!(annot.get(b"F").unwrap().as_number().unwrap() as u32, 1 << 2);
    }

    #[test]
    fn sticky_note_creates_popup_companion_and_is_nozoom_norotate() {
        let mut s = session(pdf_without_info());
        let annot_id = s
            .add_text_annotation(
                0,
                &TextAnnotSpec::Sticky {
                    rect: page_tree::Rect {
                        llx: 80.0,
                        lly: 80.0,
                        urx: 100.0,
                        ury: 100.0,
                    },
                    icon: crate::annot_author::StickyIcon::Note,
                    contents: "note body".to_owned(),
                    color: crate::annot_author::Color::Rgb(1.0, 0.9, 0.2),
                    open: false,
                },
            )
            .unwrap();
        let Some(Object::Dict(annot)) = s.value(annot_id) else {
            panic!("note not created");
        };
        // Always NoZoom + NoRotate + Print (§12.5.6.4).
        let f = annot.get(b"F").unwrap().as_number().unwrap() as u32;
        assert_eq!(f, (1 << 2) | (1 << 3) | (1 << 4));
        assert_eq!(
            annot.get(b"Name").unwrap().as_name().unwrap().as_bytes(),
            b"Note"
        );
        // The /Popup companion exists, points back at the note, and is a
        // Popup subtype (never painted as page content).
        let popup_id = annot.get(b"Popup").unwrap().as_reference().unwrap();
        let Some(Object::Dict(popup)) = s.value(popup_id) else {
            panic!("popup not created");
        };
        assert_eq!(
            popup.get(b"Subtype").unwrap().as_name().unwrap().as_bytes(),
            b"Popup"
        );
        assert_eq!(popup.get(b"Parent").unwrap().as_reference(), Some(annot_id));
        // The page's /Annots holds BOTH the note and its popup.
        let Some(Object::Dict(page)) = s.value(ObjId::new(3, 0)) else {
            panic!("page missing");
        };
        let Some(Object::Array(annots)) = page.get(b"Annots") else {
            panic!("/Annots missing");
        };
        assert_eq!(annots.len(), 2, "note + popup");
    }

    #[test]
    fn undo_of_text_authoring_nets_to_nothing() {
        // A sticky note authors THREE objects (appearance, note, popup) plus
        // the /Annots patch, all one command — undo removes them all.
        let mut s = session(pdf_without_info());
        s.add_text_annotation(
            0,
            &TextAnnotSpec::Sticky {
                rect: page_tree::Rect {
                    llx: 80.0,
                    lly: 80.0,
                    urx: 100.0,
                    ury: 100.0,
                },
                icon: crate::annot_author::StickyIcon::Note,
                contents: "x".to_owned(),
                color: crate::annot_author::Color::Rgb(1.0, 0.9, 0.2),
                open: false,
            },
        )
        .unwrap();
        assert!(!s.dirty_set().is_empty());
        s.undo();
        assert!(!s.is_modified(), "undo of authoring nets to nothing");
        assert!(s.dirty_set().is_empty());
        let Some(Object::Dict(page)) = s.value(ObjId::new(3, 0)) else {
            panic!("page missing");
        };
        assert!(page.get(b"Annots").is_none());
    }

    #[test]
    fn symbolic_font_freetext_is_a_named_refusal() {
        let mut s = session(pdf_without_info());
        let err = s
            .add_text_annotation(
                0,
                &TextAnnotSpec::FreeText {
                    rect: page_tree::Rect {
                        llx: 20.0,
                        lly: 40.0,
                        urx: 180.0,
                        ury: 70.0,
                    },
                    text: "x".to_owned(),
                    font: crate::fontdata::Std14::Symbol,
                    font_size: 12.0,
                    color: crate::vartext::TextColor::Gray(0.0),
                    quadding: crate::vartext::Quadding::Left,
                    multiline: false,
                    border: None,
                    border_width: 0.0,
                },
            )
            .unwrap_err();
        assert!(matches!(err, EditError::VariableText(_)), "{err:?}");
        assert!(!s.is_modified(), "a refused edit changes nothing");
    }

    #[test]
    fn appending_to_an_existing_direct_annots_array_preserves_it() {
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << >> \
                 /Annots [4 0 R] >>",
                "<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] >>",
            ],
            "",
        );
        let mut s = session(bytes);
        let new_id = s
            .add_markup(
                0,
                &MarkupSpec::TextMarkup {
                    kind: TextMarkupKind::Highlight,
                    quads: vec![crate::annot_author::Quad::from_rect(Rect {
                        llx: 10.0,
                        lly: 10.0,
                        urx: 90.0,
                        ury: 24.0,
                    })],
                    color: Color::Rgb(1.0, 1.0, 0.0),
                },
            )
            .unwrap();
        let Some(Object::Dict(page)) = s.value(ObjId::new(3, 0)) else {
            panic!("page missing");
        };
        let Some(Object::Array(annots)) = page.get(b"Annots") else {
            panic!("annots");
        };
        assert_eq!(annots.len(), 2, "the existing entry is preserved");
        assert_eq!(annots[0].as_reference(), Some(ObjId::new(4, 0)));
        assert_eq!(annots[1].as_reference(), Some(new_id));
    }

    #[test]
    fn shared_indirect_annots_array_is_copied_on_write_x7() {
        // Two pages reference the SAME indirect /Annots array (object 6).
        // Annotating page 0 must NOT annotate page 1.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 300 300] \
                 /Resources << >> >>",
                "<< /Type /Page /Parent 2 0 R /Annots 6 0 R >>",
                "<< /Type /Page /Parent 2 0 R /Annots 6 0 R >>",
                "<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] >>",
                "[5 0 R]",
            ],
            "",
        );
        let mut s = session(bytes);
        s.add_markup(0, &square_spec()).unwrap();

        // Page 0 (obj 3) now points at a NEW array with two entries.
        let Some(Object::Dict(p0)) = s.value(ObjId::new(3, 0)) else {
            panic!("p0");
        };
        let Some(Object::Reference(p0_arr)) = p0.get(b"Annots") else {
            panic!("p0 annots ref");
        };
        assert_ne!(*p0_arr, ObjId::new(6, 0), "page 0 was repointed (COW)");

        // Page 1 (obj 4) still points at the ORIGINAL array (obj 6), which
        // still has exactly its one original entry — unperturbed.
        let Some(Object::Dict(p1)) = s.value(ObjId::new(4, 0)) else {
            panic!("p1");
        };
        assert_eq!(
            p1.get(b"Annots").unwrap().as_reference(),
            Some(ObjId::new(6, 0))
        );
        let Some(Object::Array(orig)) = s.value(ObjId::new(6, 0)) else {
            panic!("orig array");
        };
        assert_eq!(orig.len(), 1, "the shared array must be untouched");
    }

    #[test]
    fn sole_owner_indirect_annots_array_is_edited_in_place() {
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 300] \
                 /Resources << >> >>",
                "<< /Type /Page /Parent 2 0 R /Annots 5 0 R >>",
                "[]",
            ],
            "",
        );
        let mut s = session(bytes);
        s.add_markup(0, &square_spec()).unwrap();
        // The array object (obj 5) gained the entry; the page dict is
        // unchanged (still references obj 5) — no COW.
        let Some(Object::Array(arr)) = s.value(ObjId::new(5, 0)) else {
            panic!("array");
        };
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn encrypted_documents_are_refused_before_authoring_can_be_reached_x10() {
        // Today pdfce refuses to LOAD an encrypted file at all
        // (`EncryptionUnsupported`), so the `DocumentEncrypted` guard in
        // `add_markup` is a forward-compatible R37 seam, not a path a
        // loadable file reaches. This test pins the load-time refusal so
        // the seam's rationale stays visible: when Pass 5 makes encrypted
        // files loadable, the guard is what keeps authoring from writing
        // plaintext strings into them (X10).
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 300] \
                 /Resources << >> >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "/Encrypt 9 0 R /ID [<01> <02>] ",
        );
        assert!(
            Document::from_bytes(bytes).is_err(),
            "an encrypted file is refused at load today; the add_markup guard is the Pass-5 seam"
        );
    }

    #[test]
    fn empty_geometry_is_refused() {
        let mut s = session(pdf_without_info());
        let empty_ink = MarkupSpec::Ink {
            strokes: vec![],
            color: Color::Gray(0.0),
            width: 1.0,
        };
        assert!(matches!(
            s.add_markup(0, &empty_ink),
            Err(EditError::EmptyGeometry)
        ));
    }

    #[test]
    fn out_of_range_page_is_a_named_refusal() {
        let mut s = session(pdf_without_info());
        assert!(matches!(
            s.add_markup(9, &square_spec()),
            Err(EditError::PageOutOfRange { index: 9, count: 1 })
        ));
    }

    #[test]
    fn authored_source_extends_the_base_only_when_something_is_staged() {
        let mut s = session(pdf_without_info());
        let base_len = s.document().bytes().len();
        assert_eq!(s.authored_source().len(), base_len, "no staging yet");
        s.add_markup(0, &square_spec()).unwrap();
        assert!(
            s.authored_source().len() > base_len,
            "staging appended past the base"
        );
    }

    // -- Pass 7 form fill ------------------------------------------------

    /// A one-page document with an `/AcroForm` carrying a merged (Shape A)
    /// text field `Name` (obj 4) and a merged checkbox `Agree` (obj 5) whose
    /// `/AP` `/N` subdictionary offers `/Yes` and `/Off` (obj 6). `trailer`
    /// may add `/Perms`/`/Info` etc.
    fn pdf_with_form(catalog_extra: &str, trailer_extra: &str) -> Vec<u8> {
        build(
            &[
                &format!(
                    "<< /Type /Catalog /Pages 2 0 R {catalog_extra} /AcroForm << /Fields [4 0 R 5 0 R] \
                     /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 \
                     /BaseFont /Helvetica >> >> >> >> >>"
                ),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> \
                 /Annots [4 0 R 5 0 R] >>",
                "<< /FT /Tx /T (Name) /Subtype /Widget /Rect [20 150 200 172] /P 3 0 R >>",
                "<< /FT /Btn /T (Agree) /V /Off /AS /Off /Subtype /Widget /Rect [20 100 32 112] \
                 /P 3 0 R /AP << /N << /Yes 6 0 R /Off 6 0 R >> >> >>",
                "<< /Type /XObject /Subtype /Form /BBox [0 0 12 12] /Length 0 >>\nstream\n\nendstream",
            ],
            trailer_extra,
        )
    }

    #[test]
    fn fill_text_field_sets_value_and_regenerates_appearance() {
        let mut s = session(pdf_with_form("", ""));
        let out = s.fill_text_field("Name", "Ada Lovelace").unwrap();
        assert_eq!(out.widgets_updated, 1);
        // Auto-size was requested (0 Tf) → disclosed.
        assert!(out.applied_autosize.is_some());
        assert_eq!(out.unencodable_chars, 0);

        let (bytes, _r) = s
            .to_incremental_bytes(&crate::writer::SaveOptions::identity())
            .unwrap();
        let reloaded = Document::from_bytes(bytes).unwrap();
        let form = forms::parse_acroform(&reloaded).unwrap();
        let f = form.field_by_name("Name").unwrap();
        assert_eq!(f.value, forms::FieldValue::Text(b"Ada Lovelace".to_vec()));
        // A regenerated /AP now paints through the Pass 6.0 read path.
        assert!(f.has_appearance(), "the field got a baked /AP");
        let annots = crate::annot::page_annotations(&reloaded, ObjId::new(3, 0));
        let name_widget = annots
            .iter()
            .find(|a| a.id == Some(ObjId::new(4, 0)))
            .unwrap();
        assert!(matches!(
            name_widget.appearance,
            crate::annot::Appearance::Normal { .. }
        ));
    }

    #[test]
    fn checkbox_state_selection_sets_v_and_as_without_new_appearance() {
        let mut s = session(pdf_with_form("", ""));
        s.set_button_state("Agree", "Yes").unwrap();
        // /V and /AS both become /Yes; no new object is created (state
        // selection, not generation).
        let Some(Object::Dict(d)) = s.value(ObjId::new(5, 0)) else {
            panic!("checkbox field missing");
        };
        assert_eq!(
            d.get(b"V").and_then(Object::as_name).unwrap().as_bytes(),
            b"Yes"
        );
        assert_eq!(
            d.get(b"AS").and_then(Object::as_name).unwrap().as_bytes(),
            b"Yes"
        );
        // Clearing goes back to Off.
        s.set_button_state("Agree", "Off").unwrap();
        let Some(Object::Dict(d)) = s.value(ObjId::new(5, 0)) else {
            panic!("checkbox field missing");
        };
        assert_eq!(
            d.get(b"V").and_then(Object::as_name).unwrap().as_bytes(),
            b"Off"
        );
    }

    #[test]
    fn fill_then_undo_is_byte_identical() {
        // The minimal-diff proof for fill: fill → undo → save re-emits the
        // input exactly (the §11.1 dirty-set-is-a-diff contract).
        let input = pdf_with_form("", "");
        let mut s = session(input.clone());
        s.fill_text_field("Name", "Grace Hopper").unwrap();
        assert!(s.is_modified());
        s.undo();
        assert!(!s.is_modified(), "fill undo nets to nothing");
        assert!(s.dirty_set().is_empty());
    }

    #[test]
    fn unknown_field_and_state_are_named_refusals() {
        let mut s = session(pdf_with_form("", ""));
        assert!(matches!(
            s.fill_text_field("Nonexistent", "x"),
            Err(EditError::FieldNotFound { .. })
        ));
        assert!(matches!(
            s.set_button_state("Agree", "Maybe"),
            Err(EditError::FieldStateUnknown { .. })
        ));
        // A text field is not a button.
        assert!(matches!(
            s.set_button_state("Name", "Yes"),
            Err(EditError::FieldNotFillable { .. })
        ));
    }

    #[test]
    fn read_only_field_is_refused() {
        // /Ff bit 1 (ReadOnly) on the text field.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/Helv 0 Tf 0 g) >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [4 0 R] >>",
                "<< /FT /Tx /Ff 1 /T (Locked) /Subtype /Widget /Rect [0 0 100 20] >>",
            ],
            "",
        );
        let mut s = session(bytes);
        assert!(matches!(
            s.fill_text_field("Locked", "x"),
            Err(EditError::FieldNotFillable { .. })
        ));
    }

    #[test]
    fn certification_p2_permits_fill_p1_refuses() {
        // A /P 2 certification (the form-filling tier) permits fill; a /P 1
        // (no changes) refuses by name. Perms enforced via catalog /Perms.
        let sig = "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 1 2 3] \
                   /Reference [ << /TransformMethod /DocMDP /TransformParams << /P 2 >> >> ] >>";
        // Field 4 text; field 7 a Sig field holding the certification; catalog
        // /Perms /DocMDP references the sig field's /V (obj 8).
        let p2 = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 8 0 R >> \
                 /AcroForm << /Fields [4 0 R 7 0 R] /DA (/Helv 0 Tf 0 g) >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [4 0 R] >>",
                "<< /FT /Tx /T (Name) /Subtype /Widget /Rect [0 0 100 20] >>",
                "<< /Length 0 >>\nstream\n\nendstream",
                "<< /Length 0 >>\nstream\n\nendstream",
                "<< /FT /Sig /T (sig) /V 8 0 R >>",
                sig,
            ],
            "",
        );
        let mut s = session(p2);
        assert!(
            s.fill_text_field("Name", "ok").is_ok(),
            "P=2 certification permits form fill"
        );

        let p1 = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 8 0 R >> \
                 /AcroForm << /Fields [4 0 R 7 0 R] /DA (/Helv 0 Tf 0 g) >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [4 0 R] >>",
                "<< /FT /Tx /T (Name) /Subtype /Widget /Rect [0 0 100 20] >>",
                "<< /Length 0 >>\nstream\n\nendstream",
                "<< /Length 0 >>\nstream\n\nendstream",
                "<< /FT /Sig /T (sig) /V 8 0 R >>",
                "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 1 2 3] \
                 /Reference [ << /TransformMethod /DocMDP /TransformParams << /P 1 >> >> ] >>",
            ],
            "",
        );
        let mut s = session(p1);
        assert!(matches!(
            s.fill_text_field("Name", "no"),
            Err(EditError::CertificationForbidsChange { permission: 1 })
        ));
    }

    // -- FF-D follow-up: add-text certification guard (§12.8.4 Table 258) --
    //
    // Adding NEW page text (`EditSession::add_text` and the free
    // `text_edit::add_text` engine) is a structural page-content change, so an
    // enforced-DocMDP certified document must refuse it — exactly as
    // `add_markup` does — rather than silently invalidate the signature. These
    // mirror the `fill_text_field` cert tests above, using the same `build`
    // helper and the same P=1 DocMDP signature.

    /// A one-page document (own `/Contents` + `/Resources /Font`, like the
    /// `plain.pdf` fixture) carrying an enforced-DocMDP certification with the
    /// most restrictive `/P 1` — `census().forbids_structural_change()` is
    /// true, so any add-text must refuse.
    fn pdf_certified_locked_page() -> Vec<u8> {
        build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 6 0 R >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
                "<< /Type /Page /Parent 2 0 R /Contents 4 0 R \
                 /Resources << /Font << /F1 5 0 R >> >> >>",
                "<< /Length 50 >>\nstream\nBT /F1 12 Tf 72 720 Td (Certified page text) Tj ET\nendstream",
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
                "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 1 2 3] \
                 /Reference [ << /TransformMethod /DocMDP /TransformParams << /P 1 >> >> ] >>",
            ],
            "",
        )
    }

    /// The same page WITHOUT the certification (no `/Perms`) — the common,
    /// permissive case that must still add text (regression guard).
    fn pdf_uncertified_page() -> Vec<u8> {
        build(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
                "<< /Type /Page /Parent 2 0 R /Contents 4 0 R \
                 /Resources << /Font << /F1 5 0 R >> >> >>",
                // Stream body is exactly 50 bytes (same as the certified helper
                // and the `plain.pdf` fixture) so `/Length` is correct.
                "<< /Length 50 >>\nstream\nBT /F1 12 Tf 72 720 Td (Uncertified doc run) Tj ET\nendstream",
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            ],
            "",
        )
    }

    #[test]
    fn session_add_text_point_is_refused_on_an_enforced_certified_document() {
        use crate::text_edit::AddTextRequest;
        let mut s = session(pdf_certified_locked_page());
        let req = AddTextRequest::new(0, (100.0, 650.0), "Blocked point run");
        assert!(matches!(
            s.add_text(&req),
            Err(crate::text_edit::AddTextError::CertificationForbidsChange { permission: 1 })
        ));
        // Refusal is before any mutation (rule 4): the session is untouched.
        assert!(!s.is_modified());
    }

    #[test]
    fn session_add_text_box_is_refused_on_an_enforced_certified_document() {
        use crate::text_edit::AddTextRequest;
        let mut s = session(pdf_certified_locked_page());
        // The boxed variant shares `EditSession::add_text` / the same planner,
        // so it is covered by the same guard.
        let req = AddTextRequest::new(0, (0.0, 0.0), "Blocked boxed run")
            .with_box(72.0, 600.0, 180.0, 120.0);
        assert!(matches!(
            s.add_text(&req),
            Err(crate::text_edit::AddTextError::CertificationForbidsChange { permission: 1 })
        ));
        assert!(!s.is_modified());
    }

    #[test]
    fn free_add_text_is_refused_on_an_enforced_certified_document() {
        use crate::text_edit::{AddTextRequest, add_text};
        let doc = Document::from_bytes(pdf_certified_locked_page()).unwrap();
        let req = AddTextRequest::new(0, (100.0, 650.0), "Blocked free run");
        assert!(matches!(
            add_text(&doc, &req),
            Err(crate::text_edit::AddTextError::CertificationForbidsChange { permission: 1 })
        ));
    }

    #[test]
    fn add_text_still_works_on_an_uncertified_document() {
        use crate::text_edit::{AddTextRequest, add_text};
        // Session path adds fine (no regression on the common case)…
        let mut s = session(pdf_uncertified_page());
        let req = AddTextRequest::new(0, (100.0, 650.0), "Allowed run");
        assert!(
            s.add_text(&req).is_ok(),
            "a non-certified doc still adds text"
        );
        assert!(s.is_modified());
        // …and so does the free engine.
        let doc = Document::from_bytes(pdf_uncertified_page()).unwrap();
        assert!(add_text(&doc, &req).is_ok());
    }

    #[test]
    fn add_text_certification_message_is_a_verbatim_mirror_of_edit_error() {
        // The add-text refusal must reuse `EditError::CertificationForbidsChange`'s
        // exact wording/citation, not a reinvented message. Assert the two
        // `Display` strings are byte-identical for the same `/P` value.
        for permission in [1_u8, 2, 3] {
            let at = crate::text_edit::AddTextError::CertificationForbidsChange { permission };
            let ee = EditError::CertificationForbidsChange { permission };
            assert_eq!(
                at.to_string(),
                ee.to_string(),
                "add-text cert message must mirror EditError's verbatim (P={permission})"
            );
        }
    }

    // -- Pass 7.1: choice fields, regenerate, flatten, FDF/XFDF ----------

    /// A one-page document with a single choice field `Color` (obj 4). `ff`
    /// is its `/Ff`; `opt` is the raw `/Opt` array text.
    fn pdf_with_choice(ff: u32, opt: &str) -> Vec<u8> {
        build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/Helv 0 Tf 0 g) \
                 /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Annots [4 0 R] >>",
                &format!(
                    "<< /FT /Ch /Ff {ff} /T (Color) /Opt {opt} /Subtype /Widget /Rect [20 50 200 72] /P 3 0 R >>"
                ),
            ],
            "",
        )
    }

    #[test]
    fn choice_single_select_stores_export_and_index() {
        // Combo (131072). Two-element opts: export != display.
        let mut s = session(pdf_with_choice(
            131072,
            "[ [(r)(Red)] [(g)(Green)] [(b)(Blue)] ]",
        ));
        // Match by display "Green" → export "g", index 1.
        let out = s.set_choice_value("Color", &["Green"]).unwrap();
        assert_eq!(out.widgets_updated, 1);
        let Some(Object::Dict(d)) = s.value(ObjId::new(4, 0)) else {
            panic!("choice field missing");
        };
        assert_eq!(
            str_bytes(d.get(b"V").unwrap()).unwrap(),
            b"g",
            "/V stores the EXPORT value"
        );
        let i = d.get(b"I").and_then(Object::as_array).unwrap();
        assert_eq!(i.len(), 1);
        assert_eq!(i[0].as_int(), Some(1));
    }

    #[test]
    fn choice_multi_select_stores_array_and_indices() {
        // List box + MultiSelect (2097152).
        let mut s = session(pdf_with_choice(2097152, "[ (Red) (Green) (Blue) ]"));
        s.set_choice_value("Color", &["Red", "Blue"]).unwrap();
        let Some(Object::Dict(d)) = s.value(ObjId::new(4, 0)) else {
            panic!("choice field missing");
        };
        let v = d.get(b"V").and_then(Object::as_array).unwrap();
        assert_eq!(v.len(), 2, "/V is an array under MultiSelect");
        assert_eq!(str_bytes(&v[0]), Some(&b"Red"[..]));
        let i = d.get(b"I").and_then(Object::as_array).unwrap();
        assert_eq!(
            i.iter().filter_map(Object::as_int).collect::<Vec<_>>(),
            vec![0, 2]
        );
    }

    #[test]
    fn choice_single_select_refuses_multiple_values() {
        let mut s = session(pdf_with_choice(131072, "[ (Red) (Green) ]"));
        assert!(matches!(
            s.set_choice_value("Color", &["Red", "Green"]),
            Err(EditError::ChoiceRequiresMultiSelect { count: 2, .. })
        ));
    }

    #[test]
    fn choice_unknown_value_refused_unless_editable_combo() {
        // Non-editable combo refuses an out-of-/Opt value.
        let mut s = session(pdf_with_choice(131072, "[ (Red) (Green) ]"));
        assert!(matches!(
            s.set_choice_value("Color", &["Purple"]),
            Err(EditError::ChoiceValueNotInOptions { .. })
        ));
        // Editable combo (Combo|Edit = 131072|262144) accepts free text.
        let mut s2 = session(pdf_with_choice(393216, "[ (Red) (Green) ]"));
        s2.set_choice_value("Color", &["Purple"]).unwrap();
        let Some(Object::Dict(d)) = s2.value(ObjId::new(4, 0)) else {
            panic!("choice field missing");
        };
        assert_eq!(str_bytes(d.get(b"V").unwrap()).unwrap(), b"Purple");
        assert!(d.get(b"I").is_none(), "free-text value has no /Opt index");
    }

    #[test]
    fn regenerate_appearances_clears_need_appearances() {
        // A form asserting /NeedAppearances, with a text field carrying a /V
        // but no /AP. Regenerate should build the /AP and clear the flag.
        let bytes = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true \
                 /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [4 0 R] >>",
                "<< /FT /Tx /T (Name) /V (Ada) /Subtype /Widget /Rect [20 150 200 172] /P 3 0 R >>",
            ],
            "",
        );
        let mut s = session(bytes);
        let out = s.regenerate_appearances().unwrap();
        assert_eq!(out.regenerated, 1);
        assert!(out.need_appearances_cleared);

        let (saved, _r) = s
            .to_incremental_bytes(&crate::writer::SaveOptions::identity())
            .unwrap();
        let reloaded = Document::from_bytes(saved).unwrap();
        let form = forms::parse_acroform(&reloaded).unwrap();
        assert!(!form.need_appearances, "the flag was cleared on output");
        assert!(
            form.field_by_name("Name").unwrap().has_appearance(),
            "the field now carries a baked /AP"
        );
    }

    #[test]
    fn flatten_burns_appearance_and_removes_field() {
        // Fill the text field (which authors its /AP), then flatten it.
        let mut s = session(pdf_with_form("", ""));
        s.fill_text_field("Name", "Ada").unwrap();
        let out = s.flatten_fields(Some(&["Name"])).unwrap();
        assert_eq!(out.fields_flattened, 1);
        assert_eq!(out.widgets_burned, 1);
        assert_eq!(out.pages_touched, 1);

        let (saved, _r) = s
            .to_incremental_bytes(&crate::writer::SaveOptions::identity())
            .unwrap();
        let reloaded = Document::from_bytes(saved.clone()).unwrap();

        // The field is GONE from /AcroForm.
        let form = forms::parse_acroform(&reloaded).unwrap();
        assert!(
            form.field_by_name("Name").is_none(),
            "the flattened field left /AcroForm /Fields"
        );
        // The widget is GONE from the page /Annots.
        let annots = crate::annot::page_annotations(&reloaded, ObjId::new(3, 0));
        assert!(
            !annots.iter().any(|a| a.id == Some(ObjId::new(4, 0))),
            "the flattened widget left /Annots"
        );
        // The page now invokes the burned appearance (byte-grep: the `Do`
        // and the value both live in the saved page content).
        assert!(
            saved.windows(3).any(|w| w == b"Do\n") || find(&saved, b"Do"),
            "the overlay content invokes the appearance XObject"
        );
        assert!(
            find(&saved, b"Ada"),
            "the flattened value is in page content"
        );
    }

    #[test]
    fn flatten_is_refused_on_a_certified_document() {
        // Even a /P 2 (fill-permitting) certification refuses the STRUCTURAL
        // flatten by name — flatten uses the strict gate, not the fill gate.
        let p2 = build(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 6 0 R >> \
                 /AcroForm << /Fields [4 0 R] /DA (/Helv 0 Tf 0 g) >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [4 0 R] >>",
                "<< /FT /Tx /T (Name) /V (Ada) /Subtype /Widget /Rect [0 0 100 20] /AP << /N 5 0 R >> >>",
                "<< /Type /XObject /Subtype /Form /BBox [0 0 100 20] /Length 0 >>\nstream\n\nendstream",
                "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 1 2 3] \
                 /Reference [ << /TransformMethod /DocMDP /TransformParams << /P 2 >> >> ] >>",
            ],
            "",
        );
        let mut s = session(p2);
        assert!(matches!(
            s.flatten_fields(None),
            Err(EditError::CertificationForbidsChange { .. })
        ));
    }

    #[test]
    fn export_import_form_data_round_trips_through_fdf_and_xfdf() {
        // Fill a form, export its data, import into a fresh copy, and confirm
        // the values match — for both FDF and XFDF.
        let input = pdf_with_form("", "");
        let mut src = session(input.clone());
        src.fill_text_field("Name", "Ada Lovelace").unwrap();
        src.set_button_state("Agree", "Yes").unwrap();
        let data = src.export_form_data().unwrap();
        assert_eq!(data.fields.len(), 2);

        for bytes in [data.to_fdf(None), data.to_xfdf(None)] {
            let parsed = crate::fdf::FormData::parse_fdf(&bytes)
                .or_else(|_| crate::fdf::FormData::parse_xfdf(&bytes))
                .unwrap();
            let mut dst = session(input.clone());
            let out = dst.import_form_data(&parsed).unwrap();
            assert_eq!(out.applied, 2);
            assert_eq!(out.skipped, 0);
            let form = forms::parse_acroform(&dst.graph()).unwrap();
            assert_eq!(
                form.field_by_name("Name").unwrap().value,
                forms::FieldValue::Text(b"Ada Lovelace".to_vec())
            );
            assert_eq!(
                form.field_by_name("Agree").unwrap().value,
                forms::FieldValue::Name(b"Yes".to_vec())
            );
        }
    }

    #[test]
    fn import_skips_fields_the_document_lacks() {
        let mut s = session(pdf_with_form("", ""));
        let data = crate::fdf::FormData {
            fields: vec![
                crate::fdf::FieldData {
                    name: "Name".to_owned(),
                    values: vec!["Grace".to_owned()],
                },
                crate::fdf::FieldData {
                    name: "Ghost".to_owned(),
                    values: vec!["x".to_owned()],
                },
            ],
        };
        let out = s.import_form_data(&data).unwrap();
        assert_eq!(out.applied, 1);
        assert_eq!(out.skipped, 1);
    }

    /// Substring search helper for byte-grep assertions.
    fn find(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    /// The bytes of a string object (tests only — `Object` has no public
    /// string accessor because the writer round-trips raw bytes).
    fn str_bytes(o: &Object) -> Option<&[u8]> {
        match o {
            Object::String(s) => Some(s.as_slice()),
            _ => None,
        }
    }
}

/// Pass 14.3 §0.2 — the session-integrated in-place text edit/format
/// commands: applied as ONE undo-able command each, undo/redo revert/reapply
/// cleanly, saved output is minimal-diff (incremental append, byte-identical
/// to the free function for a text-edit-only session), the free-function CLI
/// path stays unchanged (verified by the untouched 14.1/14.2 tests).
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod text_edit_session_tests {
    use super::*;
    use crate::text_edit::{
        EditError, EditOptions, EditRequest, FillModel, FontSelector, FormatOptions, FormatRequest,
        NewFill,
    };

    /// A minimal one-page PDF with a Helvetica (WinAnsi, non-embedded) run —
    /// the SAME synthetic shape the 14.1 free-function tests use, rebuilt here
    /// so the session tests own their fixture. `content` is the page content
    /// stream; the font is object 5.
    fn text_pdf(content: &str) -> Vec<u8> {
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();
        objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
        objects.push((
            2,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_vec(),
        ));
        objects.push((
            3,
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        ));
        let body = content.as_bytes();
        let mut s = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
        s.extend_from_slice(body);
        s.extend_from_slice(b"\nendstream");
        objects.push((4, s));
        objects.push((
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        ));

        let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = std::collections::BTreeMap::new();
        for (num, obj) in &objects {
            offsets.insert(*num, out.len());
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            out.extend_from_slice(obj);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = out.len();
        let highest = 5u32;
        out.extend_from_slice(format!("xref\n0 {}\n", highest + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for num in 1..=highest {
            match offsets.get(&num) {
                Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
                None => out.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
                highest + 1
            )
            .as_bytes(),
        );
        out
    }

    /// The sourced page-0 text of a saved PDF.
    fn page0_text(bytes: &[u8]) -> String {
        let doc = Document::from_bytes(bytes.to_vec()).unwrap();
        let pages = crate::page_tree::pages(&doc).unwrap();
        let page =
            crate::text_extract::extract_page(&doc, &pages[0], 0, &Default::default()).unwrap();
        page.sourced_text()
    }

    fn save(session: &EditSession) -> Vec<u8> {
        session
            .to_incremental_bytes(&SaveOptions::identity())
            .unwrap()
            .0
    }

    #[test]
    fn edit_text_is_one_undoable_command_that_reverts_and_reapplies() {
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (teh cat) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        assert!(!session.is_modified());

        let report = session
            .edit_text(
                &EditRequest::find_replace(0, "teh", "the"),
                &EditOptions::default(),
            )
            .unwrap();
        // Exactly ONE undo entry, labelled as a text edit.
        assert_eq!(session.undo_depth(), 1);
        assert_eq!(session.undo_kind(), Some(CommandKind::EditText));
        assert!(session.is_modified());
        assert_eq!(
            report.glyph_source,
            crate::text_edit::EditGlyphSource::NonEmbedded
        );
        // The edit round-trips through a save.
        assert!(page0_text(&save(&session)).contains("the cat"));

        // Undo reverts to byte-identical to the input (the §11.1 net-zero
        // rule — dirty set is a diff, not a replay).
        session.undo();
        assert!(!session.is_modified());
        assert_eq!(save(&session), src);

        // Redo reapplies the same one command.
        session.redo();
        assert!(session.is_modified());
        assert!(page0_text(&save(&session)).contains("the cat"));
    }

    #[test]
    fn reflow_block_is_one_undoable_command_and_undo_is_byte_identical() {
        // A three-line left paragraph; reflow to a wide width collapses it to
        // one line. The whole thing is ONE ReflowBlock command; undo restores
        // the byte-identical pre-reflow stream (decision 015 §3.4/R75).
        let src = text_pdf(
            "BT /F1 10 Tf 72 740 Td (alpha beta) Tj ET\n\
             BT /F1 10 Tf 72 726 Td (gamma delta) Tj ET\n\
             BT /F1 10 Tf 72 712 Td (epsilon) Tj ET\n",
        );
        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        assert!(!session.is_modified());

        let report = session
            .reflow_block(
                0,
                0,
                &crate::text_edit::ReflowRequest::new().with_wrap_width(400.0),
            )
            .unwrap();
        // Exactly ONE undo entry, labelled ReflowBlock with the line counts.
        assert_eq!(session.undo_depth(), 1);
        assert_eq!(
            session.undo_kind(),
            Some(CommandKind::ReflowBlock {
                lines_before: 3,
                lines_after: 1,
            })
        );
        assert!(session.is_modified());
        assert_eq!(report.lines_after, 1);
        // The re-wrap round-trips: the words survive on one line.
        assert!(page0_text(&save(&session)).contains("alpha beta gamma delta epsilon"));

        // Undo reverts to byte-identical to the input (§11.1 net-zero rule).
        session.undo();
        assert!(!session.is_modified());
        assert_eq!(save(&session), src);

        // Redo reapplies the same one command.
        session.redo();
        assert!(session.is_modified());
        assert!(page0_text(&save(&session)).contains("alpha beta gamma delta epsilon"));
    }

    #[test]
    fn reflow_after_an_in_session_edit_of_the_same_page_is_refused() {
        // Reflow is planned from base content; refuse if the page's content
        // was already rewritten this session (a clean named refusal, rule 4).
        let src = text_pdf("BT /F1 10 Tf 72 740 Td (teh cat) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src).unwrap());
        session
            .edit_text(
                &EditRequest::find_replace(0, "teh", "the"),
                &EditOptions::default(),
            )
            .unwrap();
        let err = session
            .reflow_block(0, 0, &crate::text_edit::ReflowRequest::new())
            .unwrap_err();
        assert!(
            matches!(err, crate::text_edit::ReflowApplyError::Unsupported(m)
                if m.contains("already edited this session")),
            "reflow after an in-session edit is refused by name"
        );
        // The failed reflow left the session at exactly the one prior edit.
        assert_eq!(session.undo_depth(), 1);
    }

    #[test]
    fn session_edit_output_matches_the_free_function_for_a_single_edit() {
        // The minimal-diff claim made concrete: for a text-edit-only session,
        // the incremental save is byte-identical to what the free function
        // `edit_text` produces — same object rewritten, everything else
        // verbatim (R32/R46).
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (teh cat) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let free = crate::text_edit::edit_text(
            &doc,
            &EditRequest::find_replace(0, "teh", "the"),
            &EditOptions::default(),
        )
        .unwrap();

        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        session
            .edit_text(
                &EditRequest::find_replace(0, "teh", "the"),
                &EditOptions::default(),
            )
            .unwrap();
        let sessioned = save(&session);

        // Incremental append: the original is a byte-prefix of both.
        assert_eq!(sessioned.get(..src.len()), Some(src.as_slice()));
        assert_eq!(sessioned, free.bytes);
    }

    #[test]
    fn two_edits_accumulate_and_undo_one_at_a_time() {
        // Two distinct runs; edit each. The second edit must compose on top of
        // the first (walk the CURRENT content), and the two must undo
        // independently.
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (teh cat) Tj 72 680 Td (sut) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        session
            .edit_text(
                &EditRequest::find_replace(0, "teh", "the"),
                &EditOptions::default(),
            )
            .unwrap();
        session
            .edit_text(
                &EditRequest::find_replace(0, "sut", "sat"),
                &EditOptions::default(),
            )
            .unwrap();
        assert_eq!(session.undo_depth(), 2);
        let both = page0_text(&save(&session));
        assert!(both.contains("the cat"), "got {both:?}");
        assert!(both.contains("sat"), "got {both:?}");

        // Undo the second edit only: "the cat" survives, "sat" reverts to "sut".
        session.undo();
        let after_one_undo = page0_text(&save(&session));
        assert!(after_one_undo.contains("the cat"));
        assert!(after_one_undo.contains("sut"));
        assert!(!after_one_undo.contains("sat"));

        // Undo the first edit: back to byte-identical input.
        session.undo();
        assert!(!session.is_modified());
        assert_eq!(save(&session), src);
    }

    #[test]
    fn a_refused_edit_leaves_the_session_untouched() {
        // WinAnsi Helvetica cannot encode an astral char ⇒ R-INV refusal, and
        // no command must reach the undo stack (rule 4 — refuse before mutate).
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (hi) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        let err = session
            .edit_text(
                &EditRequest::find_replace(0, "hi", "h\u{1D54F}"),
                &EditOptions::default(),
            )
            .unwrap_err();
        assert!(matches!(err, EditError::Refused(_)));
        assert_eq!(session.undo_depth(), 0);
        assert!(!session.is_modified());
        assert_eq!(save(&session), src);
    }

    #[test]
    fn format_text_is_one_undoable_command() {
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
        let report = session
            .format_text(
                &FormatRequest::new(0, "hello")
                    .fill(NewFill::new(FillModel::Cmyk, vec![0.0, 1.0, 1.0, 0.0]).unwrap()),
                &FormatOptions::default(),
            )
            .unwrap();
        assert_eq!(session.undo_depth(), 1);
        assert_eq!(session.undo_kind(), Some(CommandKind::FormatText));
        assert_eq!(report.fill_space, Some("k")); // parity-plus: CMYK stored as k
        // The stored device space appears in the appended revision.
        let saved = save(&session);
        assert!(String::from_utf8_lossy(&saved).contains("0 1 1 0 k"));

        session.undo();
        assert!(!session.is_modified());
        assert_eq!(save(&session), src);
    }

    #[test]
    fn format_text_no_op_and_missing_font_are_refused_without_mutating() {
        let src = text_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let mut session = EditSession::new(Document::from_bytes(src).unwrap());
        assert!(
            session
                .format_text(&FormatRequest::new(0, "hello"), &FormatOptions::default())
                .is_err()
        );
        assert!(
            session
                .format_text(
                    &FormatRequest::new(0, "hello").font(FontSelector::new("Nonexistent")),
                    &FormatOptions::default(),
                )
                .is_err()
        );
        assert_eq!(session.undo_depth(), 0);
        assert!(!session.is_modified());
    }

    /// Regression, Pass 19.1: the session's own emptiness check used to
    /// hand-list `set_size`/`set_fill`/`set_font`, so a request carrying
    /// ONLY one of the new spacing controls was rejected as a phantom
    /// `NoOp` here while succeeding through the free `set_format`. That is
    /// the worst shape of bug this project keeps re-learning — a predicate
    /// duplicated in two places, one of which is the path the GUI drives.
    ///
    /// Each of the three 19.1 controls is asserted to be a real operation
    /// on its own, so adding a fourth without updating
    /// [`FormatRequest::is_empty`](crate::text_edit::FormatRequest) fails
    /// here rather than in a GUI bug report.
    #[test]
    fn a_spacing_only_format_request_is_not_a_no_op_on_the_session_path() {
        use crate::text_edit::{MetricSpec, ScriptPosition};

        for req in [
            FormatRequest::new(0, "hello").char_spacing(MetricSpec::Absolute(0.5)),
            FormatRequest::new(0, "hello").h_scale(90.0),
            FormatRequest::new(0, "hello").script(ScriptPosition::Superscript),
        ] {
            let src = text_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
            let mut session = EditSession::new(Document::from_bytes(src.clone()).unwrap());
            session
                .format_text(&req, &FormatOptions::default())
                .expect("a 19.1 control alone is a real formatting operation");
            assert_eq!(session.undo_depth(), 1);
            assert_eq!(session.undo_kind(), Some(CommandKind::FormatText));

            // …and it rides the existing one-command-per-edit undo
            // semantics: no new CommandKind was needed (decision 019 §1.2).
            session.undo();
            assert!(!session.is_modified());
            assert_eq!(save(&session), src, "undo must be byte-identical");
        }
    }
}
