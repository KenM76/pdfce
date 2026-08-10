//! # The document outline — bookmarks, read (ISO 32000-1 §12.3.3, §12.3.2)
//!
//! A PDF's *document outline* is the tree a viewer shows in its
//! bookmarks panel. This module turns the raw, pointer-linked structure
//! the file stores — `/Root /Outlines`, then `/First` / `/Next` /
//! `/Parent` chains of indirect references — into an owned
//! [`Vec<OutlineItem>`] with resolved titles, resolved destinations, and
//! a resolved **0-based page index** wherever the file makes one
//! reachable.
//!
//! It is a **reader**. Nothing here mutates a document, and nothing here
//! is on the round-trip path (`CLAUDE.md` rule 3): the outline is parsed
//! into a parallel value tree and the file's own objects are untouched.
//! Outline *authoring* and outline *carryover across page operations*
//! live elsewhere — see [`crate::pageops::outline`], which rebuilds an
//! outline for an assembled document and is a deliberately different job
//! with different simplifications.
//!
//! ## What the file actually stores, and why reading it is not trivial
//!
//! §12.3.3 stores the outline as a doubly-linked tree of indirect
//! references. Table 152 gives the **root** dictionary:
//!
//! | Key | Meaning |
//! |---|---|
//! | `/Type` | `/Outlines` (optional, but conventional) |
//! | `/First` / `/Last` | the first and last **top-level** items |
//! | `/Count` | the number of *visible* items at all levels |
//!
//! Table 153 gives each **item** dictionary: `/Title`, `/Parent`,
//! `/Prev`, `/Next`, `/First`, `/Last`, `/Count`, `/Dest`, `/A`, `/SE`,
//! `/C`, `/F`.
//!
//! Three properties of that encoding drive nearly every design decision
//! in this file:
//!
//! **1. There is no array anywhere.** A sibling list is a `/Next` chain
//! and a child list is `/First` plus a `/Next` chain. Nothing bounds
//! either. A `/Next` that points back at an earlier sibling is a
//! perfectly well-formed *file* — the syntax is valid, the references
//! resolve — and it describes an infinite list. A reader that follows
//! the chain until it ends never returns. See
//! [`MAX_OUTLINE_DEPTH`] and the cycle guard below; this is requirement
//! (4) of the module's brief and is treated as a correctness property,
//! not a hardening nicety. **A PDF reader that hangs on a bad outline is
//! worse than one that reports a truncated tree.**
//!
//! **2. `/Count` is not a child count.** This is the single easiest
//! thing to get wrong in §12.3.3, because the key is named `Count` and
//! sits next to `/First` and `/Last`. Its **sign** carries the item's
//! open/closed state and its **magnitude** counts *visible descendants
//! at all levels* — not immediate children. An item with two children,
//! each with three children of their own, all expanded, has `/Count 8`.
//! The same item collapsed has `/Count -2` (§12.3.3: for a closed item
//! the magnitude is the number of descendants that *would* become
//! visible if it were reopened, i.e. its immediate children). pdfce
//! therefore reads **only the sign** for structure, records the declared
//! magnitude verbatim in [`OutlineItem::declared_count`] for
//! diagnostics, and derives the real child count from the traversal. The
//! fixture `basic-tree.pdf` exists to pin exactly this: it declares
//! `/Count 9` on an item with two children, so a reader that trusts the
//! magnitude fails visibly.
//!
//! **3. A destination is four different things.** §12.3.2 lets an item
//! reach a page by an explicit array, by a name resolved through either
//! of two catalog namespaces, or by an action dictionary that itself
//! carries any of those. All four are handled here; see
//! [`Destination`].
//!
//! ## Contract
//!
//! - **Infallible.** [`read_outline`] and [`parse_outline`] return a
//!   tree, never a `Result`. Malformed input yields a *partial* tree
//!   plus a populated [`OutlineDiagnostics`]. There is no input that
//!   makes them panic, abort, recurse without bound, or loop — that is
//!   the crate-wide panic-free policy (`lib.rs`'s `deny(unwrap_used,
//!   expect_used, panic, indexing_slicing)`) applied to a structure that
//!   is unusually easy to weaponise.
//! - **Bounded.** At most [`MAX_OUTLINE_ITEMS`] items are read, at most
//!   [`MAX_OUTLINE_DEPTH`] levels deep, and no object is visited twice.
//!   Every one of those three limits, when it bites, sets a flag on
//!   [`OutlineDiagnostics`]. A truncated tree is never silently
//!   presented as a complete one — that is `CLAUDE.md` rule 4
//!   (*fuzzy, never sneaky*) applied to a structural inference.
//! - **Nothing is dropped silently.** An item whose destination cannot
//!   be resolved keeps its place in the tree with the most specific
//!   [`Destination`] variant the file supports — an unresolvable name
//!   stays a [`Destination::Named`], a page object that is not in the
//!   page tree becomes a [`Destination::UnmappedPage`] carrying the
//!   object id that failed. Requirement (1) of the brief: *a destination
//!   that names a page object not in the tree is a real corruption case
//!   — surface it, do not silently drop the bookmark.*
//!
//! ## Relationship to [`crate::pageops::references`]
//!
//! `pageops::references::DestinationResolver` already resolves a
//! destination to a **page object id**, for the delete/extract dangling
//! census. It is deliberately *not* reused here, and that is a known,
//! recorded duplication rather than an oversight:
//!
//! `DestinationResolver` answers "*which page?*" and discards the view
//! parameters on the way. This module needs "*which page, and looking at
//! it how?*" — `/XYZ`'s left/top/zoom, `/FitR`'s rectangle — because a
//! bookmarks panel that navigates to the right page at the wrong zoom is
//! a visible defect. For a destination reached **by name** the view
//! parameters live in the name tree's value, which `DestinationResolver`
//! flattens into a private map with no accessor. Getting at them means
//! either flattening the name tree again (what this module does) or
//! adding a lookup accessor to `references.rs`.
//!
//! **The accessor is the better end state** — one flatten, one set of
//! semantics — and the refactor is small: expose
//! `DestinationResolver::lookup(&self, key: &[u8]) -> Option<&Object>`
//! and have [`NamedDestinations`] become a thin wrapper over it. It is
//! not done here only because this module was written under a file-
//! ownership constraint that put `references.rs` off limits. Until then,
//! the two flatteners are kept **behaviourally identical on purpose**,
//! including the collision rule (see [`NamedDestinations::new`]), so
//! that a document cannot resolve one way for the bookmarks panel and a
//! different way for the delete census.
//!
//! ## Spec sources
//!
//! - §12.3.3 (Tables 152, 153) — document outline, `/Count` semantics,
//!   the `/Dest`-vs-`/A` exclusivity.
//! - §12.3.2 (Table 151) — destination syntax: `/XYZ`, `/Fit`, `/FitH`,
//!   `/FitV`, `/FitR`, `/FitB`, `/FitBH`, `/FitBV`.
//! - §12.3.2.2 — *"the page shall be specified by an indirect
//!   reference"* for a destination inside the document.
//! - §12.3.2.3 — named destinations: the PDF 1.1 catalog `/Dests`
//!   **dictionary** and the PDF 1.2 `/Names → /Dests` **name tree**.
//! - §7.9.6 — name trees: `/Names` as an alternating key/value array,
//!   `/Kids` for interior nodes, and the `<< /D … >>` wrapper a
//!   destination value may take.
//! - §12.6.4.2 / §12.6.4.3 — the `/GoTo` and `/GoToR` actions.
//! - §7.9.2 — `/Title` is a *text string*: UTF-16BE when it starts
//!   `FE FF`, PDFDocEncoding (Annex D.3) otherwise.
//! - §7.3.10 — a dangling reference resolves to `null` and *"shall not
//!   be considered an error"*, which is why an unreadable link truncates
//!   a chain rather than failing the parse.
//!
//! ## Behavioural reference
//!
//! `D:\Dev\Rag-Specialized\Acrobat_Features\bookmarks__destinations_and_navigation.md`
//! records what Acrobat Reader honours, and pins three expectations this
//! module is built to meet: the `must_have` view types are `/XYZ`,
//! `/Fit`, `/FitH`, `/FitV` and `/FitR` in **both** direct and named
//! form; open/closed state comes from the `/Count` **sign**; and a
//! bookmark may carry any action type, not just navigation — for which
//! that RAG recommends the *recognize-and-disclose-never-execute*
//! posture already established for form-field JavaScript. This module
//! implements the "recognize and disclose" half:
//! [`Destination::NonNavigation`] names the action's `/S` and evaluates
//! nothing.

use std::collections::{HashMap, HashSet};

use crate::graph::ObjectGraph;
use crate::object::{Dict, Name, ObjId, Object};
use crate::page_tree::{PageTreeError, page_slots};
use crate::pageops::references::{MAX_NAME_TREE_DEPTH, MAX_NAME_TREE_NODES, MAX_OUTLINE_ITEMS};
use crate::textstring::decode_text_string;

/// Maximum outline nesting read (pdfce policy, `ARCHITECTURE.md` §10).
///
/// §12.3.3 imposes no nesting limit, and Annex C's implementation limits
/// do not mention outlines either, so this is **pdfce policy on
/// untrusted input** rather than a spec constant. The value matches
/// [`crate::pageops::outline`]'s own `MAX_OUTLINE_DEPTH` deliberately:
/// a tree the reader can display but the assembler would silently
/// flatten is a worse failure than either limit alone.
///
/// Thirty-two levels is far past anything a document produces on
/// purpose — a technical manual with parts, chapters, sections,
/// subsections and figures reaches five. Exceeding it sets
/// [`OutlineDiagnostics::depth_truncations`]; it never panics and never
/// drops the ancestors that were read.
pub const MAX_OUTLINE_DEPTH: usize = 32;

/// Maximum hops followed while chasing a destination through names and
/// `/D` wrappers before giving up.
///
/// §12.3.2.3 does not forbid a named destination whose value is another
/// name, so the resolution is a small graph walk and needs its own
/// bound. Matches the bound
/// [`crate::pageops::references::DestinationResolver`] uses for the same
/// walk, for the consistency reason given in the module docs.
const MAX_DEST_HOPS: usize = 8;

// ---------------------------------------------------------------------
// Public data model
// ---------------------------------------------------------------------

/// One entry in the document outline, with its subtree.
///
/// Owned rather than borrowed from the graph: a bookmarks panel outlives
/// any single borrow of the document, and an outline is small enough
/// (thousands of short strings) that copying it is cheaper than
/// threading a lifetime through the GUI.
///
/// `#[non_exhaustive]` because Table 153 has entries this Pass does not
/// read yet — `/SE` (the structure element a bookmark corresponds to,
/// needed for tagged-PDF navigation) is the obvious next one — and
/// adding a field must not be a breaking change.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OutlineItem {
    /// The object id of this item's own dictionary.
    ///
    /// Carried because identity is what a GUI needs and the tree cannot
    /// otherwise supply: selecting a bookmark, scrolling back to it
    /// after a reload, or (later) editing it all key off the object, not
    /// off a path through the tree that any edit invalidates.
    pub id: ObjId,
    /// `/Title` decoded as a §7.9.2 text string.
    ///
    /// Empty when `/Title` is absent or is not a string — both are
    /// malformed (Table 153 marks `/Title` **required**) and both are
    /// counted in [`OutlineDiagnostics::titles_unreadable`]. An empty
    /// title is not itself an error: a file may legitimately carry one.
    pub title: String,
    /// `false` when at least one byte of `/Title` could not be decoded
    /// and U+FFFD was substituted.
    ///
    /// Surfaced rather than folded into `title` because of `CLAUDE.md`
    /// rule 4: a title pdfce partly guessed at must be visibly distinct
    /// from one it read exactly. See
    /// [`crate::textstring::DecodedText::exact`] for the three ways this
    /// goes false.
    pub title_exact: bool,
    /// Where this bookmark navigates, as far as the file makes knowable.
    ///
    /// `None` means the item carries neither `/Dest` nor `/A` — a legal
    /// and common shape for a pure grouping entry ("Part II") that only
    /// exists to hold children.
    pub destination: Option<Destination>,
    /// This item's children, in document order.
    pub children: Vec<OutlineItem>,
    /// Whether this item's children are shown expanded by default.
    ///
    /// **Derived from the sign of `/Count` and nothing else** (§12.3.3;
    /// see the module docs). Defaults to `false` when `/Count` is absent
    /// — see [`OutlineDiagnostics::open_state_defaulted`] for why closed
    /// is the safe default.
    pub open: bool,
    /// Nesting depth: `0` for a top-level item, `1` for its children,
    /// and so on.
    ///
    /// Redundant with the tree's shape, and carried anyway because every
    /// consumer that renders a flat list with indentation would
    /// otherwise recompute it, and one of them would eventually
    /// recompute it wrong.
    pub level: usize,
    /// The raw `/Count` integer as the file declared it, un-interpreted.
    ///
    /// Kept **verbatim** so a diagnostic can compare the file's claim
    /// against the traversal's finding without re-reading the document.
    /// `None` when `/Count` is absent or is not an integer. Do not use
    /// this to size anything — see the module docs on why the magnitude
    /// is not a child count.
    pub declared_count: Option<i64>,
    /// `/C` — the item's display colour in DeviceRGB, components in
    /// `0.0..=1.0` (Table 153, PDF 1.4).
    ///
    /// `None` when absent or not an array of three numbers. Not clamped:
    /// an out-of-range component is a defect the caller should see
    /// rather than one this module launders.
    pub color: Option<[f64; 3]>,
    /// `/F` — the item's display style flags as the file declared them
    /// (Table 153, PDF 1.4).
    ///
    /// Stored raw rather than as booleans because the field is a bit
    /// field with room to grow and pdfce should not silently discard
    /// bits it does not recognise. Use [`OutlineItem::is_italic`] and
    /// [`OutlineItem::is_bold`] for the two defined bits.
    pub style_flags: Option<i64>,
}

impl OutlineItem {
    /// `/F` bit position 1 — render the title in italic (Table 153).
    const FLAG_ITALIC: i64 = 1;
    /// `/F` bit position 2 — render the title in bold (Table 153).
    const FLAG_BOLD: i64 = 2;

    /// Whether `/F` asks for an italic title.
    ///
    /// A named accessor rather than exposing the constant, so that a
    /// caller cannot accidentally test bit *value* 1 against bit
    /// *position* 1 — the classic off-by-one in PDF flag words, where
    /// the spec numbers positions from 1 and the arithmetic needs
    /// `1 << (position - 1)`.
    #[must_use]
    pub const fn is_italic(&self) -> bool {
        match self.style_flags {
            Some(flags) => flags & Self::FLAG_ITALIC != 0,
            None => false,
        }
    }

    /// Whether `/F` asks for a bold title.
    #[must_use]
    pub const fn is_bold(&self) -> bool {
        match self.style_flags {
            Some(flags) => flags & Self::FLAG_BOLD != 0,
            None => false,
        }
    }

    /// The 0-based page index this item navigates to within *this*
    /// document, if it reaches one.
    ///
    /// The convenience every caller wants and none should write twice:
    /// "jump to this bookmark" is the whole feature, and it needs
    /// exactly this number. Deliberately `None` for a remote
    /// (`/GoToR`) destination — that page index belongs to a different
    /// file and returning it here would let a caller scroll this
    /// document to a page the bookmark never meant.
    #[must_use]
    pub const fn page_index(&self) -> Option<usize> {
        match self.destination {
            Some(Destination::Page { page_index, .. }) => Some(page_index),
            _ => None,
        }
    }
}

/// Where a bookmark points (§12.3.2, §12.6.4.2, §12.6.4.3).
///
/// The variants are ordered by how much pdfce could determine, from
/// "fully resolved" to "recognised but not a navigation at all". Every
/// variant except [`Destination::Page`] represents something the
/// operator may need told about, which is why none of them is folded
/// into a `None`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Destination {
    /// Fully resolved: a 0-based page index in **this** document, plus
    /// the view to establish on arrival.
    Page {
        /// 0-based index into the page tree's document-order page list,
        /// as produced by [`crate::page_tree::page_slots`].
        page_index: usize,
        /// The Table 151 fit style and its parameters.
        view: DestView,
    },
    /// An explicit destination array that pdfce could **not** map to a
    /// page index.
    ///
    /// This is requirement (1) of this module's brief made visible. It
    /// covers three genuinely different corruptions, which are
    /// distinguished by `page` and by
    /// [`OutlineDiagnostics::page_tree_error`]:
    ///
    /// - `page: Some(id)` and no page-tree error — the array named a
    ///   real object that is **not a page in the page tree**. A
    ///   destination left behind by a page delete looks exactly like
    ///   this.
    /// - `page: Some(id)` with a page-tree error — the object might well
    ///   be a page; pdfce could not walk the tree to find out.
    /// - `page: None` — element 0 was absent, `null`, or not an indirect
    ///   reference, contrary to §12.3.2.2's *"the page shall be
    ///   specified by an indirect reference"*.
    UnmappedPage {
        /// The object the array named, when it named one at all.
        page: Option<ObjId>,
        /// The fit style, which is readable even when the page is not.
        view: DestView,
    },
    /// A named destination (§12.3.2.3) that neither catalog namespace
    /// defines.
    ///
    /// Kept rather than discarded — brief requirement (2) — because the
    /// name is the only evidence of what the bookmark was for, and
    /// because a name that resolves in the producing workflow but not in
    /// this file is a repair case, not a non-entry.
    Named {
        /// The raw key bytes, exactly as the file spelled them.
        ///
        /// Bytes rather than a `String` because §7.9.6 name-tree keys
        /// are *strings* with no declared encoding — they are compared
        /// byte-wise, not textually, and round-tripping them through
        /// UTF-8 would corrupt the ones that are not text. Use
        /// [`Destination::name_lossy`] for display.
        name: Vec<u8>,
    },
    /// A `/GoToR` action (§12.6.4.3): a destination in **another** file.
    ///
    /// Never resolved to a page index of this document, by design — see
    /// [`OutlineItem::page_index`].
    Remote {
        /// The `/F` file specification, reduced to bytes for display.
        /// `None` when absent or in a shape this module does not read
        /// (see [`file_spec_bytes`]).
        file: Option<Vec<u8>>,
        /// How the remote destination names its page.
        target: RemoteTarget,
        /// The fit style to establish in the remote file.
        view: DestView,
        /// `/NewWindow`, when stated. `None` is meaningful: §12.6.4.3
        /// makes the entry optional and leaves the choice to the viewer,
        /// so absent is *not* the same as `false`.
        new_window: Option<bool>,
    },
    /// An action that is not a page navigation: `/URI`, `/Launch`,
    /// `/JavaScript`, `/Named`, `/Thread`, and anything else §12.6 or a
    /// later extension defines.
    ///
    /// **Recognised and disclosed, never executed.** That posture is the
    /// one `Acrobat_Features/bookmarks__destinations_and_navigation.md`
    /// recommends for bookmark actions by analogy with pdfce's existing
    /// form-field JavaScript handling, and this variant is what makes
    /// disclosure possible: a UI can say *"this bookmark runs a script"*
    /// instead of appearing to be a broken bookmark.
    NonNavigation {
        /// The action's `/S` subtype. `None` when the `/A` value was not
        /// a dictionary, or carried no readable `/S` — malformed either
        /// way, and counted in
        /// [`OutlineDiagnostics::unreadable_actions`].
        action: Option<Name>,
    },
}

impl Destination {
    /// A named destination's key rendered for display, with invalid
    /// UTF-8 replaced.
    ///
    /// Lossy on purpose and named so: the exact bytes stay available in
    /// [`Destination::Named::name`] for anything that must match or
    /// rewrite them, and a panel that cannot show a slightly-mangled
    /// name is worse than one that shows it with U+FFFD in it — the same
    /// judgement [`crate::textstring::decode_text_string`] makes for
    /// titles.
    #[must_use]
    pub fn name_lossy(&self) -> Option<String> {
        match self {
            Self::Named { name } => Some(String::from_utf8_lossy(name).into_owned()),
            _ => None,
        }
    }
}

/// How a `/GoToR` destination names its page in the remote file
/// (§12.6.4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemoteTarget {
    /// An integer page number, carried **verbatim** from the file.
    ///
    /// Not converted to a `usize` index, and that is deliberate. A
    /// remote destination cannot use an indirect reference — the target
    /// file's objects are not available to the producer — so §12.3.2
    /// permits an integer here instead. What this module will **not**
    /// do is assert whether that integer is 0-based or 1-based: the
    /// PDF_Spec RAG had no §12.3.2 entry when this module was written
    /// (see the module-level gap note in the Pass report), and
    /// `CLAUDE.md` rule 1 forbids implementing spec-governed behaviour
    /// from memory. Storing the raw `i64` means a later confirmation is
    /// a one-line change at the *consumer*, and means no caller is
    /// silently handed an off-by-one today.
    PageNumber(i64),
    /// A named destination in the remote file.
    ///
    /// Unresolvable from here by construction: the names live in the
    /// other file's catalog.
    Named(Vec<u8>),
    /// `/D` was absent, or in no shape this module recognises.
    Unknown,
}

/// A Table 151 destination fit style and its parameters (§12.3.2).
///
/// Coordinates are in the target page's **user space**, unmodified.
/// pdfce does not apply `/CropBox`, `/Rotate` or any viewer-side
/// clamping here — that is the display layer's job, and doing it during
/// parsing would make the parsed value disagree with the file.
///
/// Every numeric parameter is an `Option<f64>`, including `/FitR`'s
/// four. For `/XYZ` that models the spec directly: a `null` parameter
/// means *retain the current value*, which is a real, distinct state
/// from "zero". For the others it models **malformation** — §12.3.2
/// requires their parameters, so a `None` there means the array was
/// short or carried a non-number, and
/// [`OutlineDiagnostics::malformed_views`] counts it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DestView {
    /// `[page /XYZ left top zoom]` — position the given point at the
    /// upper-left of the window at the given zoom. Any parameter may be
    /// `null`, meaning "leave that aspect of the current view alone".
    Xyz {
        /// Horizontal coordinate of the point to place at the window's
        /// left edge.
        left: Option<f64>,
        /// Vertical coordinate of the point to place at the window's
        /// top edge.
        top: Option<f64>,
        /// Magnification factor. See the gap note on
        /// [`DestView::zoom_is_retain`] for the zero case.
        zoom: Option<f64>,
    },
    /// `[page /Fit]` — fit the whole page in the window.
    Fit,
    /// `[page /FitH top]` — fit the page **width**, with `top` at the
    /// window's top edge.
    FitH {
        /// Vertical coordinate of the window's top edge.
        top: Option<f64>,
    },
    /// `[page /FitV left]` — fit the page **height**, with `left` at the
    /// window's left edge.
    FitV {
        /// Horizontal coordinate of the window's left edge.
        left: Option<f64>,
    },
    /// `[page /FitR left bottom right top]` — fit the given rectangle
    /// entirely in the window.
    ///
    /// The four parameters are read **positionally in the order the
    /// array gives them**, which is the order named here. See
    /// [`DestView::rect`] for the assembled form.
    FitR {
        /// Left edge of the rectangle to fit.
        left: Option<f64>,
        /// Bottom edge.
        bottom: Option<f64>,
        /// Right edge.
        right: Option<f64>,
        /// Top edge.
        top: Option<f64>,
    },
    /// `[page /FitB]` — fit the page's **bounding box** in the window.
    FitB,
    /// `[page /FitBH top]` — fit the bounding box's width.
    FitBH {
        /// Vertical coordinate of the window's top edge.
        top: Option<f64>,
    },
    /// `[page /FitBV left]` — fit the bounding box's height.
    FitBV {
        /// Horizontal coordinate of the window's left edge.
        left: Option<f64>,
    },
    /// The array named a fit style pdfce does not know.
    ///
    /// Preserved by name rather than collapsed to [`DestView::Absent`]
    /// so that an extension's destination type shows up as *"pdfce does
    /// not implement `/FitSomething`"* rather than as damage. Counted in
    /// [`OutlineDiagnostics::unknown_views`].
    Unknown {
        /// The unrecognised fit name, verbatim.
        fit: Name,
    },
    /// The array carried no fit-style name at all — it was empty, held
    /// only a page, or its second element was not a name.
    ///
    /// Malformed: §12.3.2 requires the style. Counted in
    /// [`OutlineDiagnostics::malformed_views`].
    Absent,
}

impl DestView {
    /// `/FitR`'s rectangle as `[left, bottom, right, top]`, when all four
    /// parameters were present and numeric.
    ///
    /// Returns `None` for every other variant *and* for a `/FitR` whose
    /// array was short — the caller that wants to draw or scroll to the
    /// rectangle needs all four or none of them, and forcing that
    /// through one accessor is what stops a partial rectangle being
    /// completed with plausible defaults.
    #[must_use]
    pub const fn rect(&self) -> Option<[f64; 4]> {
        match *self {
            Self::FitR {
                left: Some(l),
                bottom: Some(b),
                right: Some(r),
                top: Some(t),
            } => Some([l, b, r, t]),
            _ => None,
        }
    }

    /// Whether an `/XYZ` destination asks the viewer to **retain** the
    /// current zoom.
    ///
    /// Unambiguously true when `zoom` is `null` (absent from the
    /// `Option`). **A zoom of literal `0` is widely treated the same
    /// way** by viewers, and this accessor does so too — but that is a
    /// *stated reading*, not a verified clause: the PDF_Spec RAG carried
    /// no §12.3.2 entry when this module was written, so the "zoom 0
    /// means null" rule could not be sourced to ISO 32000-1 text. It is
    /// recorded here as a pdfce reading precisely so it can be checked
    /// and, if wrong, changed in one place. Returns `false` for every
    /// non-`/XYZ` variant, which have no zoom to retain.
    #[must_use]
    pub fn zoom_is_retain(&self) -> bool {
        match *self {
            Self::Xyz { zoom, .. } => match zoom {
                None => true,
                Some(value) => value == 0.0,
            },
            _ => false,
        }
    }
}

/// What the outline read could not do, counted.
///
/// Every field is something a front end can put in a sentence, and every
/// non-zero field means the returned tree is a *reading* of the document
/// rather than a transcription of it. That distinction is `CLAUDE.md`
/// rule 4 applied to structure: a truncated or partly-guessed outline
/// must be visibly so.
///
/// Counted rather than itemised, following
/// [`crate::pageops::references::DanglingReport`]'s precedent: an
/// outline with 300 broken destinations should say "300", and the list
/// is what a future repair flow would need rather than something to
/// carry unused now.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct OutlineDiagnostics {
    /// Total items read, at every level. The tree's real size, as
    /// opposed to anything `/Count` claimed.
    pub items: usize,
    /// Deepest level reached, `0` for a flat outline.
    pub max_depth: usize,
    /// Whether [`MAX_OUTLINE_ITEMS`] was hit and the read stopped early.
    ///
    /// When true the tree is **incomplete** and must be presented as
    /// such.
    pub item_budget_exhausted: bool,
    /// How many subtrees were cut off at [`MAX_OUTLINE_DEPTH`].
    ///
    /// Each one is an item that has children in the file and none in the
    /// returned tree.
    pub depth_truncations: usize,
    /// How many links were refused because they pointed at an object
    /// already visited — the cycle guard firing.
    ///
    /// Non-zero means the file's outline contains a loop. Requirement
    /// (4) of the brief: bounded traversal, **reported**.
    pub cycles_broken: usize,
    /// How many `/First` or `/Next` values were present but not indirect
    /// references, truncating a chain.
    pub non_reference_links: usize,
    /// How many outline objects did not resolve to a dictionary — a
    /// dangling reference (§7.3.10) or an object of the wrong type.
    pub unreadable_items: usize,
    /// How many items had an absent or non-string `/Title`, which Table
    /// 153 marks required.
    pub titles_unreadable: usize,
    /// How many titles decoded with at least one U+FFFD substitution.
    pub titles_inexact: usize,
    /// How many items with children carried no usable `/Count`, so their
    /// open/closed state was defaulted.
    ///
    /// **The default is closed.** A wrongly-closed node still shows that
    /// children exist — the twisty is drawn either way — and costs the
    /// operator one click. A wrongly-*open* node on a large damaged
    /// outline floods the panel with entries the author meant to hide,
    /// and there is no equally cheap recovery from that.
    pub open_state_defaulted: usize,
    /// How many items carried **both** `/Dest` and `/A`, which §12.3.3
    /// forbids.
    ///
    /// See [`resolve_item_destination`] for which one wins and why.
    pub dest_and_action_both_present: usize,
    /// How many explicit destinations could not be mapped to a page
    /// index — the [`Destination::UnmappedPage`] count.
    pub unmapped_pages: usize,
    /// How many named destinations neither namespace defined — the
    /// [`Destination::Named`] count.
    pub unresolved_names: usize,
    /// How many destination arrays named a fit style pdfce does not
    /// implement.
    pub unknown_views: usize,
    /// How many destination arrays were missing a required fit-style
    /// name or a required numeric parameter.
    pub malformed_views: usize,
    /// How many `/A` values were not a dictionary, or carried no
    /// readable `/S`.
    pub unreadable_actions: usize,
    /// How many named destinations the document defines across both
    /// namespaces.
    ///
    /// Context rather than a defect: "4 of this file's 900 bookmarks did
    /// not resolve" reads very differently from "4 did not resolve and
    /// the file defines no names at all", and the second case points at
    /// a lost `/Names` tree rather than at four bad bookmarks.
    pub named_destinations_defined: usize,
    /// Why the page tree could not be walked, when it could not be.
    ///
    /// When this is `Some`, **no** explicit destination could be mapped
    /// to an index, and every one of them is a
    /// [`Destination::UnmappedPage`] for that reason alone rather than
    /// because it was broken. A UI must say so — reporting "900 broken
    /// bookmarks" when the real fault is one unreadable page tree sends
    /// the operator after the wrong problem.
    pub page_tree_error: Option<PageTreeError>,
}

impl OutlineDiagnostics {
    /// Whether the returned tree is a complete transcription of the
    /// file's outline, with nothing truncated, defaulted or guessed.
    ///
    /// Deliberately strict: a single U+FFFD in one title makes this
    /// `false`. The point is to give a UI one cheap test for *"can I
    /// present this as simply the document's bookmarks?"*, and anything
    /// looser would let a partly-inferred tree pass as a faithful one.
    #[must_use]
    pub const fn is_faithful(&self) -> bool {
        !self.item_budget_exhausted
            && self.depth_truncations == 0
            && self.cycles_broken == 0
            && self.non_reference_links == 0
            && self.unreadable_items == 0
            && self.titles_unreadable == 0
            && self.titles_inexact == 0
            && self.open_state_defaulted == 0
            && self.dest_and_action_both_present == 0
            && self.unmapped_pages == 0
            && self.unresolved_names == 0
            && self.unknown_views == 0
            && self.malformed_views == 0
            && self.unreadable_actions == 0
            && self.page_tree_error.is_none()
    }
}

/// A document's outline plus the record of what reading it cost.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Outline {
    /// Top-level items in document order. Empty when the document has
    /// no outline, which is the common case and not an error.
    pub items: Vec<OutlineItem>,
    /// What could not be read exactly. See [`OutlineDiagnostics`].
    pub diagnostics: OutlineDiagnostics,
}

impl Outline {
    /// Every item, at every level, in **document order** — the order a
    /// bookmarks panel lists them with everything expanded.
    ///
    /// Iterative with an explicit stack rather than recursive. Depth is
    /// already bounded by [`MAX_OUTLINE_DEPTH`] so recursion would in
    /// fact be safe here, but this method is the one a caller is most
    /// likely to reach for on a tree it built itself rather than one
    /// this module produced — and that tree carries no such bound.
    #[must_use]
    pub fn flatten(&self) -> Vec<&OutlineItem> {
        let mut out = Vec::with_capacity(self.diagnostics.items);
        let mut stack: Vec<&OutlineItem> = self.items.iter().rev().collect();
        while let Some(item) = stack.pop() {
            out.push(item);
            stack.extend(item.children.iter().rev());
        }
        out
    }
}

// ---------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------

/// Read `graph`'s document outline (§12.3.3) into a tree, with
/// diagnostics.
///
/// Infallible: a document with no `/Outlines`, an `/Outlines` that is
/// not a dictionary, a looping `/Next` chain, or a page tree that cannot
/// be walked all produce a value, never an error. See the module docs'
/// **Contract** section for exactly what "partial" is allowed to mean.
///
/// Works over any [`ObjectGraph`], so it reads the **edited** state when
/// handed an [`EditSession`](crate::edit::EditSession)'s overlay and the
/// **base** file when handed a [`Document`](crate::document::Document) —
/// the reason that trait exists at all (see [`crate::graph`]'s module
/// docs, which name the outline walk as one of its intended consumers).
///
/// # Examples
///
/// ```
/// use pdfce_core::document::Document;
/// use pdfce_core::outline::read_outline;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Document::from_bytes(
///     include_bytes!("../../../fixtures/synthetic/outline/basic-tree.pdf").to_vec(),
/// )?;
/// let outline = read_outline(&doc);
///
/// // Two top-level chapters; the first has two children.
/// assert_eq!(outline.items.len(), 2);
/// assert_eq!(outline.diagnostics.items, 5);
///
/// // /Count's SIGN is the open/closed state. Chapter 1 declares +9 —
/// // wrong in magnitude, right in sign — and is open with 2 children.
/// let chapter1 = outline.items.first().ok_or("no first item")?;
/// assert_eq!(chapter1.title, "Chapter 1");
/// assert!(chapter1.open);
/// assert_eq!(chapter1.children.len(), 2);
/// assert_eq!(chapter1.declared_count, Some(9));
///
/// // Chapter 2 declares -1: closed, and it really does have one child.
/// let chapter2 = outline.items.get(1).ok_or("no second item")?;
/// assert!(!chapter2.open);
/// assert_eq!(chapter2.children.len(), 1);
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn read_outline<G: ObjectGraph + ?Sized>(graph: &G) -> Outline {
    let mut diagnostics = OutlineDiagnostics::default();

    // The page-object -> 0-based-index map, built once. `page_slots` is
    // used rather than `pages` deliberately: `pages` also resolves
    // `/Resources` and `/MediaBox` and can fail with `MissingRequired`
    // for a damaged file, and a bookmark should still know which page it
    // points at when that page has no MediaBox. `page_slots`'s own docs
    // make the same argument for structural operations.
    let page_index: HashMap<ObjId, usize> = match page_slots(graph) {
        Ok(slots) => slots
            .iter()
            .enumerate()
            .map(|(index, slot)| (slot.id, index))
            .collect(),
        Err(error) => {
            diagnostics.page_tree_error = Some(error);
            HashMap::new()
        }
    };

    let named = NamedDestinations::new(graph);
    diagnostics.named_destinations_defined = named.len();

    // §12.3.3: the outline is reached from the catalog's `/Outlines`.
    // Absent is the common case, and `Outline::default()` — an empty
    // tree with clean diagnostics — is the honest answer for it.
    let Some(root) = graph
        .catalog_dict()
        .and_then(|catalog| catalog.get(b"Outlines").map(|value| graph.resolve(value)))
        .and_then(Object::as_dict)
    else {
        return Outline {
            items: Vec::new(),
            diagnostics,
        };
    };

    let mut context = ReadContext {
        budget: MAX_OUTLINE_ITEMS,
        visited: HashSet::new(),
        page_index: &page_index,
        named: &named,
        diagnostics,
    };

    // The root's own `/Count` is the visible-item total (Table 152) and
    // carries no open/closed state, so it is read only as a chain start.
    let first = link(graph, root, b"First", &mut context);
    let items = read_siblings(graph, first, 0, &mut context);

    Outline {
        items,
        diagnostics: context.diagnostics,
    }
}

/// Read `graph`'s document outline, discarding the diagnostics.
///
/// The shape named in this module's brief, and a genuine convenience for
/// the many callers that only need the tree — but note what it throws
/// away: [`read_outline`] is the entry point that can tell you the tree
/// was **truncated**, and any UI that presents an outline to an operator
/// should use that one instead. This is for tests, for scripted dumps,
/// and for callers that have already checked.
#[must_use]
pub fn parse_outline<G: ObjectGraph + ?Sized>(graph: &G) -> Vec<OutlineItem> {
    read_outline(graph).items
}

// ---------------------------------------------------------------------
// Traversal
// ---------------------------------------------------------------------

/// Mutable state threaded through the whole read.
///
/// A struct rather than eight arguments because every one of these
/// crosses every frame of the traversal, and because the budget and the
/// visited set are only correct if they are *shared* — a per-level copy
/// of either would bound each branch separately and leave the total
/// unbounded, which is the exact bug the guards exist to prevent.
struct ReadContext<'a> {
    /// Items still permitted. Decremented once per item accepted.
    budget: usize,
    /// Every outline object already visited, anywhere in the tree.
    ///
    /// Global, not per-branch. An item reachable twice is malformed
    /// however it is reached, and a per-branch set would let a diamond
    /// expand exponentially while every individual path stayed short.
    visited: HashSet<ObjId>,
    /// Page object id to 0-based document-order index.
    page_index: &'a HashMap<ObjId, usize>,
    /// Both catalog named-destination namespaces, pre-flattened.
    named: &'a NamedDestinations,
    /// Accumulating record of what could not be read exactly.
    diagnostics: OutlineDiagnostics,
}

/// Read a `/Next` chain and everything below it.
///
/// **Iterative across siblings, recursive across levels**, which is the
/// same split [`crate::pageops::references::walk_outline`] makes and for
/// the same reason: the sibling chain is the unbounded direction in real
/// files — a flat 10,000-entry outline is ordinary — while nesting is
/// shallow and hard-capped at [`MAX_OUTLINE_DEPTH`]. Recursing on
/// siblings would overflow the stack on exactly the documents this needs
/// to work for, and no `#[deny(panic)]` catches a stack overflow.
///
/// Every early exit records *why*. A chain that stops because the budget
/// ran out and a chain that stops because it genuinely ended must not be
/// indistinguishable in the output — see [`OutlineDiagnostics`].
fn read_siblings<G: ObjectGraph + ?Sized>(
    graph: &G,
    first: Option<ObjId>,
    level: usize,
    context: &mut ReadContext<'_>,
) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let mut current = first;

    while let Some(id) = current {
        if context.budget == 0 {
            context.diagnostics.item_budget_exhausted = true;
            break;
        }
        // The cycle guard. Note it fires on the *link*, before any work:
        // a `/Next` pointing back at an earlier sibling ends the chain
        // here rather than re-reading a subtree that is already in the
        // output.
        if !context.visited.insert(id) {
            context.diagnostics.cycles_broken += 1;
            break;
        }
        context.budget -= 1;

        // §7.3.10: a dangling reference is `null`, not an error. There
        // is no way to continue a chain through an object that is not a
        // dictionary — the `/Next` would have to come from it — so the
        // chain ends, and the truncation is recorded.
        let Some(dict) = graph.resolved(id).as_dict() else {
            context.diagnostics.unreadable_items += 1;
            break;
        };

        let item = read_item(graph, id, dict, level, context);
        items.push(item);

        current = link(graph, dict, b"Next", context);
    }

    items
}

/// Build one [`OutlineItem`] from its dictionary, then descend.
///
/// Split out from [`read_siblings`] so the sibling loop stays readable
/// as a loop: the per-item work is a dozen independent field reads and
/// interleaving them with the chain-walking control flow is how a
/// `continue` eventually skips the wrong thing.
fn read_item<G: ObjectGraph + ?Sized>(
    graph: &G,
    id: ObjId,
    dict: &Dict,
    level: usize,
    context: &mut ReadContext<'_>,
) -> OutlineItem {
    context.diagnostics.items += 1;
    context.diagnostics.max_depth = context.diagnostics.max_depth.max(level);

    // --- /Title (Table 153, required; §7.9.2 text string) ------------
    let (title, title_exact) = match graph.resolve(dict.get(b"Title").unwrap_or(&Object::Null)) {
        Object::String(bytes) => {
            let decoded = decode_text_string(bytes);
            if !decoded.exact {
                context.diagnostics.titles_inexact += 1;
            }
            (decoded.text, decoded.exact)
        }
        _ => {
            context.diagnostics.titles_unreadable += 1;
            (String::new(), true)
        }
    };

    // --- /Count (Table 153) — SIGN only; see the module docs ---------
    let declared_count = graph
        .resolve(dict.get(b"Count").unwrap_or(&Object::Null))
        .as_int();
    let has_children_link = dict.contains_key(b"First");
    let open = match declared_count {
        Some(count) => count > 0,
        None => {
            // Only a defect when there is something to expand. A leaf
            // with no `/Count` is entirely ordinary and must not inflate
            // the diagnostic.
            if has_children_link {
                context.diagnostics.open_state_defaulted += 1;
            }
            false
        }
    };

    // --- /C and /F (Table 153, PDF 1.4) ------------------------------
    let color = read_color(graph, dict);
    let style_flags = graph
        .resolve(dict.get(b"F").unwrap_or(&Object::Null))
        .as_int();

    // --- /Dest and /A (Table 153; §12.3.2, §12.6) --------------------
    let destination = resolve_item_destination(graph, dict, context);

    // --- children ----------------------------------------------------
    // The depth cap is checked *before* descending, so an item at the
    // limit keeps everything already read about itself and loses only
    // its subtree.
    let children = if level + 1 >= MAX_OUTLINE_DEPTH {
        if has_children_link {
            context.diagnostics.depth_truncations += 1;
        }
        Vec::new()
    } else {
        let first = link(graph, dict, b"First", context);
        read_siblings(graph, first, level + 1, context)
    };

    OutlineItem {
        id,
        title,
        title_exact,
        destination,
        children,
        open,
        level,
        declared_count,
        color,
        style_flags,
    }
}

/// Read a structural link (`/First` or `/Next`) as an object id.
///
/// Absent is normal — it is how both chains terminate — but *present and
/// not a reference* is a defect worth counting: §12.3.3's links are
/// indirect references, and a direct dictionary there means a producer
/// inlined a node that the `/Prev`/`/Parent` back-links can no longer
/// name. The chain has to stop either way; the counter is what stops
/// that stop from looking like a normal end.
fn link<G: ObjectGraph + ?Sized>(
    graph: &G,
    dict: &Dict,
    key: &[u8],
    context: &mut ReadContext<'_>,
) -> Option<ObjId> {
    let value = dict.get(key)?;
    match value.as_reference() {
        Some(id) => Some(id),
        None => {
            // A reference that resolves to null is a *dangling* link,
            // already covered by `as_reference` returning the id and the
            // caller finding no dictionary. Reaching here means the
            // value was never a reference at all.
            if !matches!(graph.resolve(value), Object::Null) {
                context.diagnostics.non_reference_links += 1;
            }
            None
        }
    }
}

/// `/C` — a DeviceRGB triple (Table 153, PDF 1.4).
///
/// Returns `None` for anything that is not exactly three numbers.
/// Silently tolerating a two- or four-element array would mean guessing
/// which component was missing, and a bookmark drawn in the wrong colour
/// is a defect that never announces itself.
fn read_color<G: ObjectGraph + ?Sized>(graph: &G, dict: &Dict) -> Option<[f64; 3]> {
    let array = graph
        .resolve(dict.get(b"C")?)
        .as_array()
        .filter(|items| items.len() == 3)?;
    let mut out = [0.0f64; 3];
    for (slot, value) in out.iter_mut().zip(array.iter()) {
        *slot = graph.resolve(value).as_number()?;
    }
    Some(out)
}

// ---------------------------------------------------------------------
// Destination resolution
// ---------------------------------------------------------------------

/// Resolve an item's `/Dest` and/or `/A` to a [`Destination`].
///
/// ## Precedence, and why it is `/Dest`
///
/// §12.3.3 makes `/Dest` and `/A` **mutually exclusive** — an item
/// carrying both is malformed, and the spec gives no rule for reading
/// one. pdfce prefers `/Dest`, for two reasons:
///
/// 1. It is the cheaper and more direct statement of intent. `/A` wraps
///    the same destination in an action dictionary that a producer
///    emitting both has, by construction, already contradicted.
/// 2. **[`crate::pageops::references::DestinationResolver::resolve_target`]
///    already does exactly this**, and it is the function that decides
///    whether deleting a page reports this bookmark as broken. If the
///    bookmarks panel and the delete census disagreed about where a
///    bookmark points, the operator would be told a bookmark is fine and
///    then watch it break. Crate-internal consistency outranks any
///    independent judgement about which key is "better" here.
///
/// The fall-through matches that function too: `/A` is consulted when
/// `/Dest` is **absent or wholly unreadable**, not when `/Dest` merely
/// failed to reach a live page. A `/Dest` naming a missing page is an
/// *answer* — the bookmark is broken, and saying so is the point — and
/// quietly substituting the `/A` would hide the corruption behind a
/// working link.
fn resolve_item_destination<G: ObjectGraph + ?Sized>(
    graph: &G,
    dict: &Dict,
    context: &mut ReadContext<'_>,
) -> Option<Destination> {
    let has_dest = dict.contains_key(b"Dest");
    let has_action = dict.contains_key(b"A");
    if has_dest && has_action {
        context.diagnostics.dest_and_action_both_present += 1;
    }

    if let Some(dest) = dict.get(b"Dest")
        && let Some(resolved) = resolve_destination_value(graph, dest, context)
    {
        return Some(resolved);
    }

    if has_action {
        return Some(read_action(graph, dict, context));
    }
    None
}

/// Read an item's `/A` action dictionary (§12.6) as a [`Destination`].
///
/// Only `/GoTo` and `/GoToR` are navigations. Everything else — `/URI`,
/// `/Launch`, `/JavaScript`, `/Named`, `/Thread`, `/GoToE`, and any
/// future extension — becomes [`Destination::NonNavigation`] carrying
/// its `/S`, which is the *recognise and disclose, never execute*
/// posture the Acrobat-parity RAG recommends for bookmark actions.
///
/// `/Next` on an action dictionary (§12.6.1's action chaining) is
/// deliberately **not** followed: a bookmark whose first action is a
/// navigation navigates, and one whose first action is a script is
/// disclosed as a script regardless of what it chains to. Following the
/// chain would let a `/JavaScript` action be reported as a page jump
/// because something further down the chain was a `/GoTo`.
fn read_action<G: ObjectGraph + ?Sized>(
    graph: &G,
    dict: &Dict,
    context: &mut ReadContext<'_>,
) -> Destination {
    let Some(action) = dict.get(b"A").map(|value| graph.resolve(value)).and_then(Object::as_dict)
    else {
        context.diagnostics.unreadable_actions += 1;
        return Destination::NonNavigation { action: None };
    };
    let subtype = graph
        .resolve(action.get(b"S").unwrap_or(&Object::Null))
        .as_name()
        .cloned();
    let Some(subtype) = subtype else {
        context.diagnostics.unreadable_actions += 1;
        return Destination::NonNavigation { action: None };
    };

    match subtype.as_bytes() {
        // §12.6.4.2 — same document. `/D` is a destination in any of
        // §12.3.2's forms, so it goes through the same resolver the
        // item-level `/Dest` uses.
        b"GoTo" => action
            .get(b"D")
            .and_then(|dest| resolve_destination_value(graph, dest, context))
            .unwrap_or(Destination::NonNavigation {
                action: Some(subtype),
            }),
        // §12.6.4.3 — another file.
        b"GoToR" => read_remote(graph, action, context),
        _ => Destination::NonNavigation {
            action: Some(subtype),
        },
    }
}

/// Read a `/GoToR` action (§12.6.4.3) as [`Destination::Remote`].
///
/// The remote destination's `/D` is resolved **without** consulting this
/// document's name trees, which is the whole reason `/GoToR` cannot
/// share [`resolve_destination_value`]. A name in a remote destination
/// belongs to the *target* file's namespace; looking it up here would,
/// on a document that happens to define the same name, silently
/// navigate to a page of the wrong file — a wrong answer that looks
/// entirely convincing.
fn read_remote<G: ObjectGraph + ?Sized>(
    graph: &G,
    action: &Dict,
    context: &mut ReadContext<'_>,
) -> Destination {
    let file = action.get(b"F").and_then(|spec| file_spec_bytes(graph, spec));
    let new_window = match graph.resolve(action.get(b"NewWindow").unwrap_or(&Object::Null)) {
        Object::Boolean(value) => Some(*value),
        _ => None,
    };

    // Chase `/D` through any `<< /D … >>` wrappers, bounded, but never
    // through this document's named-destination map.
    let mut current = action
        .get(b"D")
        .map_or(Object::Null, |value| graph.resolve(value).clone());
    let mut target = RemoteTarget::Unknown;
    let mut view = DestView::Absent;
    for _ in 0..MAX_DEST_HOPS {
        match current {
            Object::Array(ref items) => {
                view = read_view(graph, items, context);
                target = match items.first().map(|first| graph.resolve(first)) {
                    Some(Object::Integer(number)) => RemoteTarget::PageNumber(*number),
                    _ => RemoteTarget::Unknown,
                };
                break;
            }
            Object::String(ref bytes) => {
                target = RemoteTarget::Named(bytes.clone());
                break;
            }
            Object::Name(ref name) => {
                target = RemoteTarget::Named(name.as_bytes().to_vec());
                break;
            }
            Object::Dict(ref dict) => match dict.get(b"D") {
                Some(inner) => current = graph.resolve(inner).clone(),
                None => break,
            },
            _ => break,
        }
    }

    Destination::Remote {
        file,
        target,
        view,
        new_window,
    }
}

/// Reduce a file specification (§7.11) to display bytes.
///
/// Handles the two shapes that actually occur on `/GoToR`: a bare
/// string, and a file-specification dictionary. From the dictionary,
/// `/UF` is preferred over `/F` because it is the Unicode form and `/F`
/// is a platform-encoded legacy string — where both exist, `/UF` is the
/// one that will render correctly.
///
/// **Stated gap.** §7.11's full model — `/DOS`, `/Mac`, `/Unix`,
/// relative-path resolution against `/Root /URI`, embedded-file streams
/// — is not implemented, and the PDF_Spec RAG had no §7.11 entry when
/// this module was written. Anything not covered returns `None` rather
/// than a guess, so a caller sees "pdfce could not read this file
/// reference" instead of a plausible wrong path.
fn file_spec_bytes<G: ObjectGraph + ?Sized>(graph: &G, spec: &Object) -> Option<Vec<u8>> {
    match graph.resolve(spec) {
        Object::String(bytes) => Some(bytes.clone()),
        Object::Dict(dict) => {
            for key in [b"UF".as_slice(), b"F".as_slice()] {
                if let Some(Object::String(bytes)) = dict.get(key).map(|v| graph.resolve(v)) {
                    return Some(bytes.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Resolve a **same-document** destination value (§12.3.2) — any of its
/// four shapes — to a [`Destination`].
///
/// Returns `None` only when the value is nothing a destination can be
/// (a number, a boolean, `null`, a dangling reference). Every other
/// outcome, including complete failure to reach a page, is a `Some` with
/// a variant that says what went wrong — brief requirements (1) and (2).
///
/// The loop is bounded by [`MAX_DEST_HOPS`] because §12.3.2.3 does not
/// forbid a named destination whose value is another name, and a
/// two-name cycle is as easy to author as a `/Next` cycle.
fn resolve_destination_value<G: ObjectGraph + ?Sized>(
    graph: &G,
    dest: &Object,
    context: &mut ReadContext<'_>,
) -> Option<Destination> {
    // Owned, because a hop through the named-destination map lands on a
    // value the map owns and a borrow would tie the loop's lifetime to
    // the first iteration.
    let mut current = graph.resolve(dest).clone();
    // Names already followed, so `/A -> /B -> /A` terminates at the
    // cycle rather than at the hop budget. Almost always empty.
    let mut seen: Vec<Vec<u8>> = Vec::new();

    for _ in 0..MAX_DEST_HOPS {
        match current {
            // Shape 1: the explicit array. §12.3.2.2 requires element 0
            // to be an indirect reference to a page object.
            Object::Array(ref items) => {
                let view = read_view(graph, items, context);
                let page = items.first().and_then(Object::as_reference);
                return Some(match page.and_then(|id| context.page_index.get(&id)) {
                    Some(&page_index) => Destination::Page { page_index, view },
                    None => {
                        context.diagnostics.unmapped_pages += 1;
                        Destination::UnmappedPage { page, view }
                    }
                });
            }
            // Shapes 3 and 4: a name (the PDF 1.1 `/Dests` dictionary)
            // or a string (the PDF 1.2 `/Names -> /Dests` tree). Both
            // namespaces are searched for both spellings — see
            // `NamedDestinations::new` on why they are merged.
            Object::Name(ref name) => {
                let key = name.as_bytes().to_vec();
                current = match next_named(context.named, &key, &mut seen) {
                    Step::Value(value) => value,
                    Step::Unresolved => {
                        context.diagnostics.unresolved_names += 1;
                        return Some(Destination::Named { name: key });
                    }
                };
            }
            Object::String(ref bytes) => {
                let key = bytes.clone();
                current = match next_named(context.named, &key, &mut seen) {
                    Step::Value(value) => value,
                    Step::Unresolved => {
                        context.diagnostics.unresolved_names += 1;
                        return Some(Destination::Named { name: key });
                    }
                };
            }
            // §7.9.6: a name-tree value may be `<< /D [...] >>` rather
            // than the array itself.
            Object::Dict(ref dict) => {
                current = graph.resolve(dict.get(b"D")?).clone();
            }
            _ => return None,
        }
    }
    None
}

/// One step of the named-destination walk.
enum Step {
    /// The name resolved; here is what it resolved to.
    Value(Object),
    /// Neither namespace defines it, or following it would loop.
    Unresolved,
}

/// Look up one destination name, refusing to revisit one already
/// followed.
///
/// The `seen` list is a `Vec` rather than a `HashSet` on purpose: it is
/// empty for every destination in every well-formed document and holds
/// one entry for almost every remaining one, so a linear scan over at
/// most [`MAX_DEST_HOPS`] entries beats hashing a byte string.
fn next_named(named: &NamedDestinations, key: &[u8], seen: &mut Vec<Vec<u8>>) -> Step {
    if seen.iter().any(|previous| previous == key) {
        return Step::Unresolved;
    }
    seen.push(key.to_vec());
    match named.lookup(key) {
        Some(value) => Step::Value(value.clone()),
        None => Step::Unresolved,
    }
}

/// Read the fit style and parameters from a destination array
/// (§12.3.2, Table 151).
///
/// Element 0 is the page (handled by the caller, which needs it in a
/// different form for the local and remote cases); element 1 is the fit
/// name; elements 2 onward are its parameters, **positional**.
///
/// A `null` parameter and a *missing* parameter both become `None`, and
/// that conflation is deliberate: §12.3.2 gives `null` the meaning
/// "retain the current value" for `/XYZ`, and a viewer handed a short
/// `/XYZ` array has no better option than the same behaviour. For the
/// styles whose parameters are required, `None` is malformation and is
/// counted — the difference between the two readings is recorded in
/// [`OutlineDiagnostics::malformed_views`] rather than in the value.
fn read_view<G: ObjectGraph + ?Sized>(
    graph: &G,
    items: &[Object],
    context: &mut ReadContext<'_>,
) -> DestView {
    /// Positional parameter `index` (0-based *after* the fit name), as a
    /// number, or `None` for absent / `null` / non-numeric.
    fn param<G: ObjectGraph + ?Sized>(graph: &G, items: &[Object], index: usize) -> Option<f64> {
        graph.resolve(items.get(index + 2)?).as_number()
    }

    let Some(fit) = items.get(1).map(|value| graph.resolve(value)).and_then(Object::as_name) else {
        context.diagnostics.malformed_views += 1;
        return DestView::Absent;
    };

    let mut required_missing = 0usize;
    /// Count a required parameter that was absent, so the caller can
    /// report the array as malformed without duplicating the test.
    macro_rules! required {
        ($value:expr) => {{
            let value = $value;
            if value.is_none() {
                required_missing += 1;
            }
            value
        }};
    }

    let view = match fit.as_bytes() {
        // `/XYZ`'s three parameters are the ONLY ones the spec gives a
        // null meaning to, so they are read without the `required!`
        // wrapper — an absent one is a documented state, not damage.
        b"XYZ" => DestView::Xyz {
            left: param(graph, items, 0),
            top: param(graph, items, 1),
            zoom: param(graph, items, 2),
        },
        b"Fit" => DestView::Fit,
        b"FitH" => DestView::FitH {
            top: required!(param(graph, items, 0)),
        },
        b"FitV" => DestView::FitV {
            left: required!(param(graph, items, 0)),
        },
        b"FitR" => DestView::FitR {
            left: required!(param(graph, items, 0)),
            bottom: required!(param(graph, items, 1)),
            right: required!(param(graph, items, 2)),
            top: required!(param(graph, items, 3)),
        },
        b"FitB" => DestView::FitB,
        b"FitBH" => DestView::FitBH {
            top: required!(param(graph, items, 0)),
        },
        b"FitBV" => DestView::FitBV {
            left: required!(param(graph, items, 0)),
        },
        _ => {
            context.diagnostics.unknown_views += 1;
            DestView::Unknown { fit: fit.clone() }
        }
    };

    if required_missing > 0 {
        context.diagnostics.malformed_views += 1;
    }
    view
}

// ---------------------------------------------------------------------
// Named destinations (§12.3.2.3, §7.9.6)
// ---------------------------------------------------------------------

/// Both catalog named-destination namespaces, flattened once.
///
/// Flattening eagerly is what keeps the outline read linear: resolving a
/// name means walking a name tree, and doing that per bookmark on a
/// 5,000-entry outline is quadratic. Same argument, same shape, as
/// [`crate::pageops::references::DestinationResolver`] — see this
/// module's docs for why the two exist separately and what should be
/// done about it.
#[derive(Debug, Default)]
struct NamedDestinations {
    /// Name bytes to destination value (an array, or a `<< /D … >>`
    /// dictionary, or — in a malformed file — another name).
    map: HashMap<Vec<u8>, Object>,
}

impl NamedDestinations {
    /// Flatten `graph`'s catalog `/Dests` dictionary and
    /// `/Names → /Dests` name tree into one lookup table.
    ///
    /// ## The two namespaces
    ///
    /// §12.3.2.3 defines two, from different PDF versions:
    ///
    /// - **PDF 1.1** — catalog `/Dests`, a plain dictionary whose keys
    ///   are *name objects*, referenced as `/Dest /SomeName`.
    /// - **PDF 1.2** — catalog `/Names → /Dests`, a §7.9.6 *name tree*
    ///   whose keys are *strings*, referenced as `/Dest (SomeName)`.
    ///
    /// ## Why they are merged, and who wins a collision
    ///
    /// They are merged into one table, keyed by raw bytes, because a
    /// `/Dest` value gives no hint which namespace its author meant and
    /// real files are not consistent about the name-vs-string spelling.
    /// A resolver that searched only the matching namespace would fail
    /// on documents that other readers open fine.
    ///
    /// The legacy dictionary is loaded **first** and the name tree
    /// second, so **the name tree wins** a colliding key. That ordering
    /// is not a considered ruling on which namespace is more
    /// authoritative — the spec says nothing about collisions, and no
    /// observed file populates both with the same key. It is copied
    /// verbatim from
    /// [`crate::pageops::references::DestinationResolver::new`] so the
    /// bookmarks panel and the page-delete census cannot resolve the
    /// same name two different ways. If that ordering is ever revisited,
    /// **both** must change together.
    fn new<G: ObjectGraph + ?Sized>(graph: &G) -> Self {
        let mut map = HashMap::new();
        let Some(catalog) = graph.catalog_dict() else {
            return Self { map };
        };

        // §12.3.2.3, PDF 1.1: a plain dictionary keyed by name objects.
        if let Some(dests) = catalog
            .get(b"Dests")
            .map(|value| graph.resolve(value))
            .and_then(Object::as_dict)
        {
            for (key, value) in dests.iter() {
                map.insert(key.as_bytes().to_vec(), value.clone());
            }
        }

        // §12.3.2.3 + §7.9.6, PDF 1.2: a name tree.
        if let Some(tree) = catalog
            .get(b"Names")
            .map(|value| graph.resolve(value))
            .and_then(Object::as_dict)
            .and_then(|names| names.get(b"Dests").map(|value| graph.resolve(value)))
            .and_then(Object::as_dict)
        {
            let mut budget = MAX_NAME_TREE_NODES;
            let mut visited = HashSet::new();
            flatten_name_tree(graph, tree, 0, &mut budget, &mut visited, &mut map);
        }

        Self { map }
    }

    /// How many names are defined across both namespaces.
    fn len(&self) -> usize {
        self.map.len()
    }

    /// The value `key` names, if either namespace defines it.
    fn lookup(&self, key: &[u8]) -> Option<&Object> {
        self.map.get(key)
    }
}

/// Flatten a §7.9.6 name tree into `out`.
///
/// `/Names` alternates key, value, key, value — *"an array of the form
/// `[key₁ value₁ key₂ value₂ … keyₙ valueₙ]`"* — and `/Kids` holds
/// interior nodes. A malformed file can carry both at one node, so both
/// are read wherever present rather than dispatched on; `/Limits` is
/// deliberately **ignored**, because this is a full flatten rather than
/// a binary search and trusting a `/Limits` range that disagrees with
/// its node's contents would drop real entries.
///
/// Bounded three ways — depth, node budget, and a visited set — for the
/// same reason the outline walk is: a `/Kids` array pointing back at an
/// ancestor is trivial to author, and without the visited set every
/// branch would re-walk the whole subtree until the depth guard fired,
/// which is exponential rather than merely bounded.
fn flatten_name_tree<G: ObjectGraph + ?Sized>(
    graph: &G,
    node: &Dict,
    depth: usize,
    budget: &mut usize,
    visited: &mut HashSet<ObjId>,
    out: &mut HashMap<Vec<u8>, Object>,
) {
    if depth > MAX_NAME_TREE_DEPTH || *budget == 0 {
        return;
    }
    *budget -= 1;

    if let Some(pairs) = node
        .get(b"Names")
        .map(|value| graph.resolve(value))
        .and_then(Object::as_array)
    {
        for pair in pairs.chunks_exact(2) {
            let (Some(key), Some(value)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            // §7.9.6 says keys "shall be strings". A file using names is
            // malformed but readable, and both are accepted for the same
            // reason the two namespaces are merged at all.
            let key_bytes = match graph.resolve(key) {
                Object::String(bytes) => bytes.clone(),
                Object::Name(name) => name.as_bytes().to_vec(),
                _ => continue,
            };
            out.insert(key_bytes, value.clone());
        }
    }

    if let Some(kids) = node
        .get(b"Kids")
        .map(|value| graph.resolve(value))
        .and_then(Object::as_array)
    {
        for kid in kids {
            if let Some(id) = kid.as_reference()
                && !visited.insert(id)
            {
                continue;
            }
            if let Some(dict) = graph.resolve(kid).as_dict() {
                flatten_name_tree(graph, dict, depth + 1, budget, visited, out);
            }
        }
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
    use std::collections::BTreeMap;

    /// A hand-built graph, so the traversal's guards can be tested on
    /// shapes no fixture generator should have to express — a `/First`
    /// pointing at an object of the wrong type, for instance.
    struct TestGraph {
        objects: BTreeMap<ObjId, Object>,
        trailer: Dict,
    }

    impl ObjectGraph for TestGraph {
        fn value(&self, id: ObjId) -> Option<&Object> {
            self.objects.get(&id)
        }
        fn trailer_entry(&self, key: &[u8]) -> Option<&Object> {
            self.trailer.get(key)
        }
    }

    /// Build a dictionary from `(key, value)` pairs.
    fn dict(entries: Vec<(&[u8], Object)>) -> Dict {
        let mut d = Dict::new();
        for (key, value) in entries {
            d.insert(Name::from(key), value);
        }
        d
    }

    fn reference(num: u32) -> Object {
        Object::Reference(ObjId::new(num, 0))
    }

    /// A one-page document whose catalog points at outline object 10,
    /// with `extra` objects laid on top.
    fn graph_with(extra: Vec<(u32, Object)>) -> TestGraph {
        let mut objects = BTreeMap::new();
        objects.insert(
            ObjId::new(1, 0),
            Object::Dict(dict(vec![
                (b"Type", Object::Name(Name::from(b"Catalog"))),
                (b"Pages", reference(2)),
                (b"Outlines", reference(10)),
            ])),
        );
        objects.insert(
            ObjId::new(2, 0),
            Object::Dict(dict(vec![
                (b"Type", Object::Name(Name::from(b"Pages"))),
                (b"Kids", Object::Array(vec![reference(3), reference(4)])),
                (b"Count", Object::Integer(2)),
            ])),
        );
        for num in [3u32, 4] {
            objects.insert(
                ObjId::new(num, 0),
                Object::Dict(dict(vec![
                    (b"Type", Object::Name(Name::from(b"Page"))),
                    (b"Parent", reference(2)),
                ])),
            );
        }
        for (num, value) in extra {
            objects.insert(ObjId::new(num, 0), value);
        }
        let mut trailer = Dict::new();
        trailer.insert(Name::from(b"Root"), Object::Reference(ObjId::new(1, 0)));
        TestGraph { objects, trailer }
    }

    /// An outline root dictionary pointing at `first`.
    fn outline_root(first: u32) -> Object {
        Object::Dict(dict(vec![
            (b"Type", Object::Name(Name::from(b"Outlines"))),
            (b"First", reference(first)),
            (b"Last", reference(first)),
        ]))
    }

    /// Would catch: a document with no `/Outlines` being treated as an
    /// error, or as a reason to return a non-empty tree.
    #[test]
    fn a_document_without_an_outline_reads_as_empty_and_faithful() {
        let mut graph = graph_with(vec![]);
        // Remove the /Outlines entry entirely.
        graph.objects.insert(
            ObjId::new(1, 0),
            Object::Dict(dict(vec![
                (b"Type", Object::Name(Name::from(b"Catalog"))),
                (b"Pages", reference(2)),
            ])),
        );
        let outline = read_outline(&graph);
        assert!(outline.items.is_empty());
        assert_eq!(outline.diagnostics.items, 0);
        assert!(outline.diagnostics.is_faithful());
    }

    /// Would catch: `/Count`'s magnitude being mistaken for a child
    /// count, and a positive/negative sign being read backwards. The
    /// table pins BOTH mistakes at once — a reader that returns
    /// `count.abs()` children fails row 1, and one that inverts the sign
    /// fails every row.
    #[test]
    fn count_sign_alone_decides_open_state() {
        // (declared /Count, expected `open`)
        let cases: &[(Option<i64>, bool)] = &[
            (Some(9), true),   // magnitude lies; sign says open
            (Some(1), true),   // the ordinary open case
            (Some(-1), false), // the ordinary closed case
            (Some(-7), false), // magnitude lies; sign says closed
            (Some(0), false),  // zero is not positive => closed
            (None, false),     // absent => defaulted closed
        ];
        for &(count, expected_open) in cases {
            let mut item = vec![
                (b"Title".as_slice(), Object::String(b"Parent".to_vec())),
                (b"First".as_slice(), reference(12)),
                (b"Last".as_slice(), reference(12)),
            ];
            if let Some(value) = count {
                item.push((b"Count".as_slice(), Object::Integer(value)));
            }
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (11, Object::Dict(dict(item))),
                (
                    12,
                    Object::Dict(dict(vec![(
                        b"Title".as_slice(),
                        Object::String(b"Child".to_vec()),
                    )])),
                ),
            ]);
            let outline = read_outline(&graph);
            let parent = &outline.items[0];
            assert_eq!(parent.open, expected_open, "for /Count {count:?}");
            // The real child count comes from the traversal, never from
            // the declared magnitude.
            assert_eq!(parent.children.len(), 1, "for /Count {count:?}");
            assert_eq!(parent.declared_count, count);
        }
        // Only the absent case counts as a defaulted open state.
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Parent".to_vec())),
                    (b"First", reference(12)),
                ])),
            ),
            (12, Object::Dict(dict(vec![]))),
        ]);
        assert_eq!(read_outline(&graph).diagnostics.open_state_defaulted, 1);
    }

    /// Would catch: a leaf with no `/Count` being reported as a
    /// defaulted open state, which would make
    /// `OutlineDiagnostics::is_faithful` false for almost every real
    /// document and train callers to ignore it.
    #[test]
    fn a_childless_item_without_a_count_is_not_a_defect() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Leaf".to_vec())),
                    (b"Dest", Object::Array(vec![reference(3), Object::Name(Name::from(b"Fit"))])),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.diagnostics.open_state_defaulted, 0);
        assert!(outline.diagnostics.is_faithful());
    }

    /// Would catch: the sibling cycle guard being absent — this test
    /// does not fail, it **hangs**, which is exactly the failure mode a
    /// reader must not have. Also catches a guard that silently breaks
    /// the loop without recording it.
    #[test]
    fn a_next_cycle_terminates_and_is_reported() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Ping".to_vec())),
                    (b"Next", reference(12)),
                ])),
            ),
            (
                12,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Pong".to_vec())),
                    (b"Next", reference(11)),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.items.len(), 2);
        assert_eq!(outline.diagnostics.cycles_broken, 1);
        assert!(!outline.diagnostics.is_faithful());
    }

    /// Would catch: a `/First` pointing at its own item recursing until
    /// the stack overflows. The depth guard alone would *bound* this at
    /// 32 frames; only the visited set stops it at one.
    #[test]
    fn a_self_parenting_item_terminates_at_one_level() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Ouroboros".to_vec())),
                    (b"First", reference(11)),
                    (b"Count", Object::Integer(1)),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.items.len(), 1);
        assert!(outline.items[0].children.is_empty());
        assert_eq!(outline.diagnostics.cycles_broken, 1);
    }

    /// Would catch: the depth cap silently dropping a subtree without
    /// saying so, or the cap being applied one level early/late.
    #[test]
    fn nesting_past_the_cap_is_truncated_and_reported() {
        // A chain of MAX_OUTLINE_DEPTH + 5 nested items, objects 11..
        let mut extra = vec![(10u32, outline_root(11))];
        let levels = MAX_OUTLINE_DEPTH + 5;
        for depth in 0..levels {
            let num = 11 + u32::try_from(depth).unwrap();
            let mut entries = vec![(
                b"Title".as_slice(),
                Object::String(format!("L{depth}").into_bytes()),
            )];
            if depth + 1 < levels {
                entries.push((b"First".as_slice(), reference(num + 1)));
                entries.push((b"Count".as_slice(), Object::Integer(1)));
            }
            extra.push((num, Object::Dict(dict(entries))));
        }
        let outline = read_outline(&graph_with(extra));

        // Walk down and confirm exactly MAX_OUTLINE_DEPTH levels exist.
        let mut node = &outline.items[0];
        let mut seen = 1usize;
        while let Some(child) = node.children.first() {
            node = child;
            seen += 1;
        }
        assert_eq!(seen, MAX_OUTLINE_DEPTH);
        assert_eq!(node.level, MAX_OUTLINE_DEPTH - 1);
        assert_eq!(outline.diagnostics.depth_truncations, 1);
        assert_eq!(outline.diagnostics.max_depth, MAX_OUTLINE_DEPTH - 1);
        assert!(!outline.diagnostics.is_faithful());
    }

    /// Would catch: an explicit destination's page reference not being
    /// mapped to a 0-based index, or being mapped to the object NUMBER
    /// (3 and 4 here) instead of the index (0 and 1) — a confusion the
    /// fixture's object numbering is chosen to expose.
    #[test]
    fn explicit_destinations_map_to_zero_based_page_indices() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"First page".to_vec())),
                    (
                        b"Dest",
                        Object::Array(vec![reference(3), Object::Name(Name::from(b"Fit"))]),
                    ),
                    (b"Next", reference(12)),
                ])),
            ),
            (
                12,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Second page".to_vec())),
                    (
                        b"Dest",
                        Object::Array(vec![reference(4), Object::Name(Name::from(b"Fit"))]),
                    ),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.items[0].page_index(), Some(0));
        assert_eq!(outline.items[1].page_index(), Some(1));
        assert!(outline.diagnostics.is_faithful());
    }

    /// Would catch: a bookmark whose destination names a missing or
    /// non-page object being silently dropped from the tree, or being
    /// reported as pointing at page 0. Brief requirement (1).
    #[test]
    fn a_destination_naming_no_page_survives_as_unmapped() {
        // (destination array, expected page object recorded)
        let cases: Vec<(Vec<Object>, Option<ObjId>)> = vec![
            // Object 99 does not exist at all.
            (
                vec![reference(99), Object::Name(Name::from(b"Fit"))],
                Some(ObjId::new(99, 0)),
            ),
            // Object 1 exists but is the catalog, not a page.
            (
                vec![reference(1), Object::Name(Name::from(b"Fit"))],
                Some(ObjId::new(1, 0)),
            ),
            // Element 0 is not a reference at all (§12.3.2.2 violation).
            (
                vec![Object::Integer(0), Object::Name(Name::from(b"Fit"))],
                None,
            ),
            // An empty array: no page, and no fit style either.
            (vec![], None),
        ];
        for (array, expected_page) in cases {
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (
                    11,
                    Object::Dict(dict(vec![
                        (b"Title", Object::String(b"Broken".to_vec())),
                        (b"Dest", Object::Array(array.clone())),
                    ])),
                ),
            ]);
            let outline = read_outline(&graph);
            // The bookmark is still there.
            assert_eq!(outline.items.len(), 1, "for {array:?}");
            assert_eq!(outline.items[0].title, "Broken");
            match &outline.items[0].destination {
                Some(Destination::UnmappedPage { page, .. }) => {
                    assert_eq!(*page, expected_page, "for {array:?}");
                }
                other => panic!("expected UnmappedPage for {array:?}, got {other:?}"),
            }
            assert_eq!(outline.diagnostics.unmapped_pages, 1);
        }
    }

    /// Would catch: a named destination that neither namespace defines
    /// being discarded instead of preserved. Brief requirement (2).
    #[test]
    fn an_unresolvable_name_is_kept_not_dropped() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Ghost".to_vec())),
                    (b"Dest", Object::String(b"nowhere".to_vec())),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(
            outline.items[0].destination,
            Some(Destination::Named {
                name: b"nowhere".to_vec()
            })
        );
        assert_eq!(outline.diagnostics.unresolved_names, 1);
        assert_eq!(
            outline.items[0].destination.as_ref().unwrap().name_lossy(),
            Some("nowhere".to_string())
        );
    }

    /// Would catch: a two-name destination cycle looping, or exhausting
    /// the hop budget instead of terminating at the repeat.
    #[test]
    fn a_named_destination_cycle_terminates() {
        let mut graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Loop".to_vec())),
                    (b"Dest", Object::String(b"a".to_vec())),
                ])),
            ),
            (
                20,
                Object::Dict(dict(vec![
                    (b"a", Object::String(b"b".to_vec())),
                    (b"b", Object::String(b"a".to_vec())),
                ])),
            ),
        ]);
        graph.objects.insert(
            ObjId::new(1, 0),
            Object::Dict(dict(vec![
                (b"Type", Object::Name(Name::from(b"Catalog"))),
                (b"Pages", reference(2)),
                (b"Outlines", reference(10)),
                (b"Dests", reference(20)),
            ])),
        );
        let outline = read_outline(&graph);
        // Terminates at the repeat, reporting the LAST name tried.
        assert!(matches!(
            outline.items[0].destination,
            Some(Destination::Named { .. })
        ));
        assert_eq!(outline.diagnostics.named_destinations_defined, 2);
    }

    /// Would catch: `/GoToR`'s `/D` being resolved against THIS
    /// document's name table — the silent-wrongness case where a remote
    /// bookmark navigates the wrong file convincingly.
    #[test]
    fn a_remote_destination_never_resolves_against_this_documents_names() {
        let mut graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Elsewhere".to_vec())),
                    (
                        b"A",
                        Object::Dict(dict(vec![
                            (b"S", Object::Name(Name::from(b"GoToR"))),
                            (b"F", Object::String(b"other.pdf".to_vec())),
                            // A name this document DOES define.
                            (b"D", Object::String(b"shared".to_vec())),
                            (b"NewWindow", Object::Boolean(true)),
                        ])),
                    ),
                ])),
            ),
            (
                20,
                Object::Dict(dict(vec![(
                    b"shared",
                    Object::Array(vec![reference(4), Object::Name(Name::from(b"Fit"))]),
                )])),
            ),
        ]);
        graph.objects.insert(
            ObjId::new(1, 0),
            Object::Dict(dict(vec![
                (b"Type", Object::Name(Name::from(b"Catalog"))),
                (b"Pages", reference(2)),
                (b"Outlines", reference(10)),
                (b"Dests", reference(20)),
            ])),
        );
        let outline = read_outline(&graph);
        match &outline.items[0].destination {
            Some(Destination::Remote {
                file,
                target,
                new_window,
                ..
            }) => {
                assert_eq!(file.as_deref(), Some(b"other.pdf".as_slice()));
                assert_eq!(*target, RemoteTarget::Named(b"shared".to_vec()));
                assert_eq!(*new_window, Some(true));
            }
            other => panic!("expected Remote, got {other:?}"),
        }
        // And crucially: NOT resolved to this document's page 1.
        assert_eq!(outline.items[0].page_index(), None);
    }

    /// Would catch: a non-navigation action being reported as a broken
    /// bookmark (destination `None`) rather than as a disclosed action,
    /// or — far worse — being treated as a navigation.
    #[test]
    fn non_navigation_actions_are_named_not_executed() {
        for subtype in [b"URI".as_slice(), b"JavaScript", b"Launch", b"Named"] {
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (
                    11,
                    Object::Dict(dict(vec![
                        (b"Title", Object::String(b"Action".to_vec())),
                        (
                            b"A",
                            Object::Dict(dict(vec![(b"S", Object::Name(Name::from(subtype)))])),
                        ),
                    ])),
                ),
            ]);
            let outline = read_outline(&graph);
            assert_eq!(
                outline.items[0].destination,
                Some(Destination::NonNavigation {
                    action: Some(Name::from(subtype))
                }),
                "for /S /{}",
                String::from_utf8_lossy(subtype)
            );
            assert_eq!(outline.items[0].page_index(), None);
        }
    }

    /// Would catch: the `/Dest`-over-`/A` precedence drifting away from
    /// `pageops::references::resolve_target`, which would let the
    /// bookmarks panel and the page-delete census disagree about where
    /// one bookmark points.
    #[test]
    fn dest_wins_over_a_and_the_conflict_is_reported() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Contested".to_vec())),
                    // /Dest -> page index 0
                    (
                        b"Dest",
                        Object::Array(vec![reference(3), Object::Name(Name::from(b"Fit"))]),
                    ),
                    // /A -> page index 1
                    (
                        b"A",
                        Object::Dict(dict(vec![
                            (b"S", Object::Name(Name::from(b"GoTo"))),
                            (
                                b"D",
                                Object::Array(vec![reference(4), Object::Name(Name::from(b"Fit"))]),
                            ),
                        ])),
                    ),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.items[0].page_index(), Some(0));
        assert_eq!(outline.diagnostics.dest_and_action_both_present, 1);
    }

    /// Would catch: destination view parameters being read at the wrong
    /// array offsets — the classic off-by-one where `/FitH`'s `top` is
    /// taken from the fit-name slot — and `/XYZ`'s `null` being turned
    /// into `0.0` instead of "retain".
    #[test]
    fn view_parameters_are_read_positionally() {
        let name = |bytes: &[u8]| Object::Name(Name::from(bytes));
        let num = |value: f64| Object::Real(value);
        let cases: Vec<(Vec<Object>, DestView)> = vec![
            (vec![reference(3), name(b"Fit")], DestView::Fit),
            (vec![reference(3), name(b"FitB")], DestView::FitB),
            (
                vec![reference(3), name(b"FitH"), num(700.0)],
                DestView::FitH { top: Some(700.0) },
            ),
            (
                vec![reference(3), name(b"FitV"), num(40.0)],
                DestView::FitV { left: Some(40.0) },
            ),
            (
                vec![reference(3), name(b"FitBH"), num(12.0)],
                DestView::FitBH { top: Some(12.0) },
            ),
            (
                vec![reference(3), name(b"FitBV"), num(13.0)],
                DestView::FitBV { left: Some(13.0) },
            ),
            (
                vec![reference(3), name(b"XYZ"), num(72.0), num(720.0), Object::Null],
                DestView::Xyz {
                    left: Some(72.0),
                    top: Some(720.0),
                    zoom: None,
                },
            ),
            (
                vec![
                    reference(3),
                    name(b"FitR"),
                    num(10.0),
                    num(20.0),
                    num(300.0),
                    num(400.0),
                ],
                DestView::FitR {
                    left: Some(10.0),
                    bottom: Some(20.0),
                    right: Some(300.0),
                    top: Some(400.0),
                },
            ),
            (
                vec![reference(3), name(b"FitSideways")],
                DestView::Unknown {
                    fit: Name::from(b"FitSideways"),
                },
            ),
            (vec![reference(3)], DestView::Absent),
        ];
        for (array, expected) in cases {
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (
                    11,
                    Object::Dict(dict(vec![
                        (b"Title", Object::String(b"V".to_vec())),
                        (b"Dest", Object::Array(array.clone())),
                    ])),
                ),
            ]);
            let outline = read_outline(&graph);
            let view = match &outline.items[0].destination {
                Some(Destination::Page { view, .. }) => view.clone(),
                Some(Destination::UnmappedPage { view, .. }) => view.clone(),
                other => panic!("expected a page destination for {array:?}, got {other:?}"),
            };
            assert_eq!(view, expected, "for {array:?}");
        }
    }

    /// Would catch: `DestView::rect` inventing a default for a `/FitR`
    /// whose array was short, which would scroll the viewer to a
    /// plausible but wrong rectangle.
    #[test]
    fn a_short_fitr_yields_no_rectangle_and_is_reported_malformed() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Short".to_vec())),
                    (
                        b"Dest",
                        Object::Array(vec![
                            reference(3),
                            Object::Name(Name::from(b"FitR")),
                            Object::Real(10.0),
                            Object::Real(20.0),
                        ]),
                    ),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        let Some(Destination::Page { view, .. }) = &outline.items[0].destination else {
            panic!("expected a resolved page destination");
        };
        assert_eq!(view.rect(), None);
        assert_eq!(outline.diagnostics.malformed_views, 1);
    }

    /// Would catch: `/Title` being taken as raw bytes rather than a
    /// §7.9.2 text string — which shows up as mojibake for UTF-16BE
    /// titles and as the wrong character for PDFDocEncoding's
    /// non-Latin-1 codes — and an undecodable byte being hidden.
    #[test]
    fn titles_decode_as_text_strings_and_disclose_inexactness() {
        // (raw /Title bytes, expected text, expected `exact`)
        let cases: &[(&[u8], &str, bool)] = &[
            (b"Plain", "Plain", true),
            // UTF-16BE, discriminated by the FE FF BOM and nothing else.
            (b"\xfe\xff\x00H\x00i", "Hi", true),
            // 0xA0 is EURO in PDFDocEncoding (Annex D.3), NOT a no-break
            // space — a Latin-1 reader gets this wrong and looks fine.
            (b"\xa05", "\u{20AC}5", true),
            // 0xAD is an UNDEFINED PDFDocEncoding code.
            (b"bad\xadbyte", "bad\u{FFFD}byte", false),
        ];
        for &(raw, expected, exact) in cases {
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (
                    11,
                    Object::Dict(dict(vec![(b"Title", Object::String(raw.to_vec()))])),
                ),
            ]);
            let outline = read_outline(&graph);
            assert_eq!(outline.items[0].title, expected, "for {raw:?}");
            assert_eq!(outline.items[0].title_exact, exact, "for {raw:?}");
        }
    }

    /// Would catch: `/F`'s bit POSITIONS (numbered from 1 in Table 153)
    /// being confused with bit VALUES, which would report italic for a
    /// bold-only bookmark.
    #[test]
    fn style_flags_map_to_italic_and_bold() {
        // (/F value, italic, bold)
        let cases: &[(i64, bool, bool)] = &[
            (0, false, false),
            (1, true, false),
            (2, false, true),
            (3, true, true),
            // An unknown high bit must not disturb the two defined ones.
            (0b1001, true, false),
        ];
        for &(flags, italic, bold) in cases {
            let graph = graph_with(vec![
                (10, outline_root(11)),
                (
                    11,
                    Object::Dict(dict(vec![
                        (b"Title", Object::String(b"Styled".to_vec())),
                        (b"F", Object::Integer(flags)),
                        (
                            b"C",
                            Object::Array(vec![
                                Object::Real(1.0),
                                Object::Real(0.5),
                                Object::Integer(0),
                            ]),
                        ),
                    ])),
                ),
            ]);
            let outline = read_outline(&graph);
            assert_eq!(outline.items[0].is_italic(), italic, "for /F {flags}");
            assert_eq!(outline.items[0].is_bold(), bold, "for /F {flags}");
            // An integer component widens to f64 (§7.3.3 NOTE 2).
            assert_eq!(outline.items[0].color, Some([1.0, 0.5, 0.0]));
        }
    }

    /// Would catch: `flatten` visiting the tree in the wrong order, or
    /// `level` not matching the depth the tree actually places an item
    /// at — the two things a flat, indented bookmark list depends on.
    #[test]
    fn flatten_walks_in_document_order_with_correct_levels() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"A".to_vec())),
                    (b"First", reference(12)),
                    (b"Count", Object::Integer(2)),
                    (b"Next", reference(14)),
                ])),
            ),
            (
                12,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"A.1".to_vec())),
                    (b"Next", reference(13)),
                ])),
            ),
            (
                13,
                Object::Dict(dict(vec![(b"Title", Object::String(b"A.2".to_vec()))])),
            ),
            (
                14,
                Object::Dict(dict(vec![(b"Title", Object::String(b"B".to_vec()))])),
            ),
        ]);
        let outline = read_outline(&graph);
        let flat = outline.flatten();
        let seen: Vec<(&str, usize)> = flat
            .iter()
            .map(|item| (item.title.as_str(), item.level))
            .collect();
        assert_eq!(
            seen,
            vec![("A", 0), ("A.1", 1), ("A.2", 1), ("B", 0)],
            "document order, with children between their parent and its next sibling"
        );
        assert_eq!(outline.diagnostics.items, 4);
        assert_eq!(outline.diagnostics.max_depth, 1);
    }

    /// Would catch: an outline chain that runs through a dangling
    /// reference aborting the whole read, or silently ending without
    /// distinguishing itself from a chain that genuinely finished.
    #[test]
    fn a_dangling_link_truncates_the_chain_and_is_reported() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![
                    (b"Title", Object::String(b"Real".to_vec())),
                    // Object 77 is never defined.
                    (b"Next", reference(77)),
                ])),
            ),
        ]);
        let outline = read_outline(&graph);
        assert_eq!(outline.items.len(), 1);
        assert_eq!(outline.diagnostics.unreadable_items, 1);
        assert!(!outline.diagnostics.is_faithful());
    }

    /// Would catch: an `/Outlines` that is not a dictionary (a stray
    /// array, a number) producing anything other than an empty tree.
    #[test]
    fn a_non_dictionary_outlines_entry_reads_as_empty() {
        let graph = graph_with(vec![(10, Object::Integer(5))]);
        let outline = read_outline(&graph);
        assert!(outline.items.is_empty());
    }

    /// Would catch: `parse_outline` diverging from `read_outline`'s
    /// tree, which would let a caller that used the convenience get a
    /// different answer from one that read the diagnostics.
    #[test]
    fn parse_outline_returns_read_outlines_items() {
        let graph = graph_with(vec![
            (10, outline_root(11)),
            (
                11,
                Object::Dict(dict(vec![(b"Title", Object::String(b"Only".to_vec()))])),
            ),
        ]);
        assert_eq!(parse_outline(&graph), read_outline(&graph).items);
    }
}
