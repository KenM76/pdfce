//! # Structural page operations — the "Core document ops" bucket
//!
//! The seven operations Pass 3.2 exists to deliver: **delete, reorder,
//! rotate, insert, extract, merge, split**. They divide cleanly in two,
//! and the division is not an implementation detail — it is what decides
//! whether an operation belongs on the undo stack.
//!
//! | Kind | Operations | Where they live | Undo |
//! |---|---|---|---|
//! | **In-place edits** — mutate the open document's page tree | delete, reorder, rotate, | [`crate::edit::EditSession`] | one command each |
//! | **Document producers** — read documents, write a *new* file | extract, merge, split, insert | this module | none — nothing changed |
//!
//! The second row has no undo story and must not be given one. The Pass
//! 3.2 UI spec puts it bluntly: *"Do NOT wire Extract/Merge/Split into
//! the undo stack 'for consistency' — they genuinely have no undo story;
//! forcing one would be ceremony without meaning."* The source document
//! is not touched by any of them.
//!
//! ## Where `insert` sits, and the deviation that put it there
//!
//! The UI spec places Insert on the undo stack as *"One EditSession
//! command for the whole insert"*. **pdfce ships it here instead**, as a
//! document producer, and the reason is structural rather than a matter
//! of taste:
//!
//! An `EditSession` overlay stores [`Object`] values whose
//! [`Stream`](crate::object::Stream) data are `ByteSpan`s into **the base
//! document's retained buffer**. That is the mechanism §5's verbatim
//! re-emission rests on. Pages imported from *another* file bring stream
//! bytes that buffer does not contain, so an overlay-based insert needs
//! two things this Pass does not build: a per-object source buffer
//! threaded through the writer, and an **overlay-aware renderer** (the
//! GUI would otherwise show a blank page for every inserted page, since
//! `pdfce-render` resolves content streams against the base document).
//!
//! Shipping insert as a document producer delivers the capability in
//! `pdfce-core` and `pdfce-cli` now, with no undo stack to mislead anyone
//! and no half-rendered pages. The GUI's in-place Insert waits for the
//! overlay-aware render path. Recorded as a deviation with its reason, as
//! the spec's own preamble requires.
//!
//! ## What every operation in this module has in common
//!
//! All four go through [`assemble`], which owns the deep-copy closure,
//! the barrier that stops it dragging in the whole source document, the
//! attribute-materialization rule, and the carryover policy. Read
//! [`assemble`]'s module docs before changing any of the four — the
//! interesting decisions are all there, not here.
//!
//! ## Signature and permission gating
//!
//! Deliberately **not** enforced in this module, and that is not an
//! oversight. These four operations produce a *new file*; they do not
//! modify a signed one, so no signature is invalidated by running them
//! (`core_ops__extract_pages.md`: plain extraction *"reads the source and
//! produces an independent new file"*). The gate belongs on the in-place
//! edits, and lives in [`crate::edit`] where the mutation happens.
//!
//! The one Acrobat behaviour deliberately **not** reproduced is its
//! page-extraction permission bit, and only because pdfce cannot yet read
//! it: §7.6 encryption is Pass 5, so the `/P` bitmask is not parsed. The
//! RAG asks pdfce to *exceed* Acrobat here — Acrobat's own enforcement of
//! that bit is reported inconsistent across versions — by enforcing
//! strictly and failing closed. That is recorded as owed work in
//! [`PermissionGate`], which exists now so the enforcement point is
//! named before it is reachable rather than discovered later.
//!
//! ## Spec sources
//!
//! - `iso32000__s__7.7.3.md` — page tree, Tables 29/30, inheritance
//! - `iso32000__s__12.3.md` — destinations and outlines
//! - `iso32000__s__7.5.4.md` — cross-reference tables (the output form)
//! - `iso32000__s__14.4.md` — file identifiers on a new document
//!
//! Acrobat-parity sources: `core_ops__*.md` in the Acrobat_Features RAG,
//! cited inline at each decision they informed.

pub mod assemble;
pub mod outline;
pub mod references;
pub mod split;

pub use assemble::{
    AssembleOptions, AssembleReport, DocumentView, OutlinePolicy, PageRef, assemble,
};
pub use references::{DanglingReport, DestinationResolver, census_dangling};
pub use split::{SplitCriterion, SplitPart, plan_split, render_name_template, split};

use crate::object::ObjId;
use crate::page_tree::PageTreeError;

/// Why a structural page operation could not be performed.
///
/// Every variant is a **named refusal** the operator or a calling script
/// can act on — the R27 fail-clean posture, applied to page operations.
/// There is deliberately no catch-all: an operation that cannot say why
/// it declined is indistinguishable from one that is broken.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PageOpError {
    /// A source document's page tree could not be walked.
    #[error("the page tree could not be resolved: {0}")]
    PageTree(#[from] PageTreeError),
    /// A page index is past the end of its document.
    #[error("page index {index} is out of range (the document has {count} page(s))")]
    PageOutOfRange {
        /// The 0-based index that was asked for.
        index: usize,
        /// How many pages the document actually has.
        count: usize,
    },
    /// A source index names a document that was not supplied.
    #[error("source index {index} was not supplied")]
    SourceOutOfRange {
        /// The index that was asked for.
        index: usize,
    },
    /// The same page was named twice in one output.
    ///
    /// Refused rather than deduplicated or duplicated: a page object may
    /// appear exactly once in a page tree (§7.7.3.2 gives each page one
    /// `/Parent`), so honouring the request would produce a file whose
    /// two "copies" of a page share one object and one parent — visibly
    /// wrong in some readers and silently wrong in others.
    #[error("page index {index} was named more than once in a single output")]
    DuplicatePage {
        /// The offending index.
        index: usize,
    },
    /// The operation would produce a document with no pages.
    ///
    /// §7.7.3.3 requires a page tree to contain at least one page, and
    /// Acrobat likewise refuses to delete the last one
    /// (`core_ops__delete_pages.md`: *"Cannot delete the only remaining
    /// page"*). Refused rather than written, because a zero-page PDF is
    /// a file most readers decline to open at all.
    #[error("the result would have no pages, and a PDF must have at least one")]
    NoPages,
    /// A page-tree leaf resolved to something that is not a dictionary.
    #[error("page object {id} is not a dictionary")]
    PageNotADictionary {
        /// The offending object.
        id: ObjId,
    },
    /// The copy exceeded [`assemble::MAX_COPIED_OBJECTS`].
    #[error("the operation would copy more objects than pdfce's limit allows")]
    ObjectLimit,
    /// A split criterion produced no output parts.
    #[error("the split criterion selected no break points, so nothing would be written")]
    NoSplitPoints,
    /// A naming template produced the same file name for two outputs.
    #[error(
        "the naming template produces the same name for parts {first} and {second}; \
         add {{n}}, {{start}} or {{end}} to make each output distinct"
    )]
    AmbiguousNames {
        /// The 1-based index of the first colliding part.
        first: usize,
        /// The 1-based index of the second.
        second: usize,
    },
}

/// The point at which §7.6 document permissions will be enforced — named
/// now, reachable later.
///
/// `core_ops__permissions_and_signature_interaction.md` makes this
/// `must_have` for the bucket: *"every core_ops CLI subcommand and GUI
/// action must consult the `/P` permission bitmask AND
/// certification-permission-level BEFORE attempting a structural
/// mutation, and refuse with a named, reported reason"*. Half of that
/// ships in this Pass — the certification half, in
/// [`crate::signature`]. The other half **cannot**: pdfce refuses to open
/// encrypted documents at all right now
/// (`XrefErrorKind::EncryptionUnsupported`), so no document that reaches
/// a page operation carries a `/P` bitmask to consult.
///
/// This type records the obligation and the fail-closed posture the RAG
/// asks for, so that Pass 5 wires an existing gate rather than
/// discovering that page operations never had one. It is deliberately
/// not a no-op function that "returns allowed": a function that always
/// says yes reads, at a call site, exactly like a check that passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PermissionGate {
    /// No encryption dictionary is reachable, because pdfce declines to
    /// open encrypted documents at all until Pass 5. Every document that
    /// reaches a page operation today is in this state.
    NotApplicableYet,
}

/// Extract a subset of pages into a new standalone document.
///
/// Returns the complete bytes of a freestanding PDF — own header, own
/// cross-reference table, own trailer — not a fragment.
///
/// Pages appear in the order given, so this is simultaneously "extract
/// pages 3, 7 and 4" and "extract them in that order"; Acrobat preserves
/// only *"their relative order from the source"*, so ordering here is a
/// deliberate superset with no parity cost.
///
/// # Errors
///
/// [`PageOpError`] — see [`assemble`].
///
/// # Examples
///
/// ```
/// use pdfce_core::document::Document;
/// use pdfce_core::pageops::{extract, DocumentView};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = include_bytes!("../../../../fixtures/synthetic/hello.pdf").to_vec();
/// let doc = Document::from_bytes(bytes)?;
/// let view = DocumentView::new(&doc, doc.bytes(), doc.version());
///
/// let (out, report) = extract(&view, &[0])?;
/// assert_eq!(report.pages, 1);
/// // The result is a real, loadable document, not a fragment.
/// assert!(Document::from_bytes(out).is_ok());
/// # Ok(())
/// # }
/// ```
pub fn extract(
    source: &DocumentView<'_>,
    pages: &[usize],
) -> Result<(Vec<u8>, AssembleReport), PageOpError> {
    let order: Vec<PageRef> = pages.iter().map(|index| (0, *index)).collect();
    assemble(
        std::slice::from_ref(source),
        &order,
        &AssembleOptions::default(),
    )
}

/// Concatenate several documents into one, in the order given.
///
/// `titles` supplies the top-level bookmark text for each source, already
/// encoded as a PDF text string (§7.9.2) — `pdfce-core` does not turn
/// file paths into display text (decision 002 R1). Pass an empty slice to
/// suppress per-source bookmarks entirely.
///
/// Reproduces the two behaviours Acrobat's Combine Files actually
/// documents (`core_ops__merge_combine_files.md`): per-source bookmark
/// generation, default on; and duplicate form-field auto-rename with the
/// `Doc0_`/`Doc1_` prefix pattern, which prevents same-named fields
/// across sources being treated as one logical field.
///
/// **Non-PDF inputs are out of scope**, per the RAG's own explicit
/// scope boundary for this Pass: converting Word/Excel/images to PDF as
/// part of a merge is *"a whole document-conversion engine"* and a
/// separate feature.
///
/// # Errors
///
/// [`PageOpError`] — see [`assemble`]. In particular
/// [`PageOpError::NoPages`] when every source is empty.
pub fn merge(
    sources: &[DocumentView<'_>],
    titles: &[Vec<u8>],
) -> Result<(Vec<u8>, AssembleReport), PageOpError> {
    let mut order: Vec<PageRef> = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let count = crate::page_tree::page_slots(source.graph())?.len();
        order.extend((0..count).map(|page| (index, page)));
    }
    let options = AssembleOptions {
        // A merged document is a new one: taking any single source's
        // catalog would silently privilege it. Acrobat's own metadata
        // behaviour here is an unverified GAP, so pdfce carries the FIRST
        // source's /Info (a documented pdfce policy) and nothing else.
        catalog_from: None,
        info_from: Some(0),
        outline: if titles.is_empty() {
            OutlinePolicy::Drop
        } else {
            OutlinePolicy::PerSource
        },
        source_titles: titles.to_vec(),
        // No single source to inherit a numbering scheme from, and
        // Acrobat does not generate one either — it tells the operator to
        // apply a fresh scheme (or Bates numbering) after combining.
        carry_page_labels: false,
        rename_duplicate_fields: true,
    };
    assemble(sources, &order, &options)
}

/// Where inserted pages land relative to the target's existing pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InsertPosition {
    /// Before the target page at this 0-based index.
    Before(usize),
    /// After the target page at this 0-based index.
    After(usize),
    /// Before every existing page.
    Start,
    /// After every existing page.
    End,
}

impl InsertPosition {
    /// The 0-based slot in the target's page sequence that inserted pages
    /// occupy, clamped to the document's actual length.
    ///
    /// Clamping rather than refusing: `After(last)` and `End` mean the
    /// same thing, and an off-by-one at the end of a document is a
    /// request pdfce can satisfy exactly as asked.
    #[must_use]
    pub const fn slot(self, page_count: usize) -> usize {
        match self {
            Self::Start => 0,
            Self::End => page_count,
            Self::Before(index) => {
                if index > page_count {
                    page_count
                } else {
                    index
                }
            }
            Self::After(index) => {
                let after = index.saturating_add(1);
                if after > page_count {
                    page_count
                } else {
                    after
                }
            }
        }
    }
}

/// Splice pages from `source` into `target` at `position`, producing a
/// new document.
///
/// `source_pages` are 0-based indices into the source's document order;
/// pass every index to insert the whole file. The target's catalog,
/// outline and form are carried; the source contributes pages only —
/// which is `core_ops__insert_pages.md`'s recommended pdfce default
/// (*"No bookmark carryover on plain Insert"*), adopted explicitly
/// because Adobe's own behaviour here is an unresolved **GAP** and the
/// RAG asks pdfce to *"decide independently and document the choice"*.
///
/// Duplicate form-field names **are** auto-renamed, because two documents
/// are involved and §12.7.3.1's whole-document name identity makes the
/// collision real here even though Acrobat's behaviour for plain Insert
/// is unconfirmed. The RAG recommends exactly this, as *"mirrors Combine
/// Files' confirmed default, the one documented precedent in this
/// cluster"*.
///
/// # Errors
///
/// [`PageOpError`] — see [`assemble`].
pub fn insert(
    target: &DocumentView<'_>,
    source: &DocumentView<'_>,
    source_pages: &[usize],
    position: InsertPosition,
) -> Result<(Vec<u8>, AssembleReport), PageOpError> {
    let target_count = crate::page_tree::page_slots(target.graph())?.len();
    let at = position.slot(target_count);

    let mut order: Vec<PageRef> = Vec::with_capacity(target_count + source_pages.len());
    order.extend((0..at).map(|page| (0, page)));
    order.extend(source_pages.iter().map(|page| (1, *page)));
    order.extend((at..target_count).map(|page| (0, page)));

    let options = AssembleOptions {
        catalog_from: Some(0),
        info_from: Some(0),
        outline: OutlinePolicy::Subset,
        source_titles: Vec::new(),
        // The target keeps every one of its own pages, so its label tree
        // still describes real pages — just with the wrong numbers after
        // the insertion point. Acrobat leaves exactly this stale; pdfce
        // leaves it stale and reports it.
        carry_page_labels: true,
        rename_duplicate_fields: true,
    };
    assemble(
        &[target.clone_view(), source.clone_view()],
        &order,
        &options,
    )
}

impl DocumentView<'_> {
    /// A second handle on the same borrowed document.
    ///
    /// `DocumentView` is a pair of shared borrows and is therefore
    /// trivially copyable in principle; it is not `Clone`-derived because
    /// `&dyn ObjectGraph` blocks the derive's bounds. This is the manual
    /// equivalent, and exists so [`insert`] can put the same view into a
    /// slice twice-shaped API without the caller pre-building one.
    #[must_use]
    pub const fn clone_view(&self) -> DocumentView<'_> {
        DocumentView::new(self.graph(), self.bytes(), self.version())
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    //! Fixture builders shared by this module's test suites.
    //!
    //! Deliberately `pub(crate)` and `#[cfg(test)]`: a builder that
    //! produces deliberately-minimal PDFs is a testing tool, not part of
    //! the engine's API, and exposing it would invite production code to
    //! construct documents outside the one object model.

    use crate::document::Document;

    /// Build an offset-consistent classic PDF from `(number, body)`
    /// pairs, all at generation 0.
    #[allow(clippy::expect_used)]
    pub fn build_pdf(objects: &[(u32, &str)]) -> Document {
        Document::from_bytes(build_pdf_bytes(objects)).expect("fixture must load")
    }

    /// As [`build_pdf`], but returning the raw bytes.
    pub fn build_pdf_bytes(objects: &[(u32, &str)]) -> Vec<u8> {
        let mut buf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        for (num, body) in objects {
            offsets.push((*num, buf.len()));
            buf.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref_at = buf.len();
        let max_num = objects.iter().map(|(n, _)| *n).max().unwrap_or(0);
        buf.extend_from_slice(format!("xref\n0 {}\n", max_num + 1).as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for num in 1..=max_num {
            match offsets.iter().find(|(n, _)| *n == num) {
                Some((_, off)) => {
                    buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
                }
                None => buf.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }
        buf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
                max_num + 1
            )
            .as_bytes(),
        );
        buf
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
    use crate::document::Document;
    use crate::graph::ObjectGraph;
    use crate::object::Object;
    use tests_support::build_pdf;

    /// Three pages, inheriting `MediaBox` and `Resources` from the root
    /// node — the shape that catches a missing attribute-materialization
    /// step, because a naive extract produces pages with no MediaBox.
    fn three_pages() -> Document {
        build_pdf(&[
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (
                2,
                "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 \
                 /MediaBox [0 0 200 100] /Resources << >> >>",
            ),
            (3, "<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>"),
            (4, "<< /Type /Page /Parent 2 0 R /Contents 7 0 R >>"),
            (5, "<< /Type /Page /Parent 2 0 R >>"),
            (6, "<< /Length 5 >>\nstream\npage1\nendstream"),
            (7, "<< /Length 5 >>\nstream\npage2\nendstream"),
        ])
    }

    fn view(doc: &Document) -> DocumentView<'_> {
        DocumentView::new(doc, doc.bytes(), doc.version())
    }

    #[test]
    fn extract_produces_a_loadable_standalone_document() {
        let doc = three_pages();
        let (bytes, report) = extract(&view(&doc), &[1]).unwrap();
        assert_eq!(report.pages, 1);

        let out = Document::from_bytes(bytes).unwrap();
        let pages = crate::page_tree::pages(&out).unwrap();
        assert_eq!(pages.len(), 1);
        // The one that matters: the source page inherited its MediaBox
        // from a node that was NOT copied, so without materialization
        // this page would have no MediaBox at all and the walk would fail
        // with MissingRequired.
        assert_eq!(pages[0].media_box.width(), 200.0);
    }

    #[test]
    fn extracted_content_streams_are_copied_byte_for_byte() {
        // Filters are never run, so a copied stream is byte-exact rather
        // than merely equivalent (assemble's module docs).
        let doc = three_pages();
        let (bytes, _) = extract(&view(&doc), &[1]).unwrap();
        let out = Document::from_bytes(bytes).unwrap();
        let pages = crate::page_tree::pages(&out).unwrap();
        let stream_id = pages[0].contents[0];
        let Some(Object::Stream(stream)) = out.value(stream_id) else {
            panic!("content stream missing from the extracted document");
        };
        assert_eq!(stream.data_span.slice(out.bytes()).unwrap(), b"page2");
    }

    #[test]
    fn extract_preserves_the_order_the_caller_asked_for() {
        let doc = three_pages();
        let (bytes, _) = extract(&view(&doc), &[2, 0]).unwrap();
        let out = Document::from_bytes(bytes).unwrap();
        let pages = crate::page_tree::pages(&out).unwrap();
        assert_eq!(pages.len(), 2);
        // Page 3 has no /Contents; page 1 has one. Order preserved means
        // the empty one comes first.
        assert!(pages[0].contents.is_empty());
        assert_eq!(pages[1].contents.len(), 1);
    }

    #[test]
    fn extract_is_deterministic() {
        // No clock, no host, no counter: the same extraction twice must
        // produce identical bytes, or none of this is testable and R41's
        // no-fingerprint rule is violated by the /ID alone.
        let doc = three_pages();
        let (a, _) = extract(&view(&doc), &[0, 1]).unwrap();
        let (b, _) = extract(&view(&doc), &[0, 1]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn extracting_no_pages_is_a_named_refusal() {
        let doc = three_pages();
        assert_eq!(extract(&view(&doc), &[]).unwrap_err(), PageOpError::NoPages);
    }

    #[test]
    fn naming_a_page_twice_is_refused_rather_than_duplicated() {
        // §7.7.3.2 gives a page one /Parent, so two "copies" sharing one
        // object would be visibly wrong in some readers and silently
        // wrong in others.
        let doc = three_pages();
        assert_eq!(
            extract(&view(&doc), &[0, 0]).unwrap_err(),
            PageOpError::DuplicatePage { index: 0 }
        );
    }

    #[test]
    fn an_out_of_range_page_names_the_real_count() {
        let doc = three_pages();
        assert_eq!(
            extract(&view(&doc), &[9]).unwrap_err(),
            PageOpError::PageOutOfRange { index: 9, count: 3 }
        );
    }

    #[test]
    fn merge_concatenates_every_source_in_order() {
        let a = three_pages();
        let b = three_pages();
        let (bytes, report) = merge(&[view(&a), view(&b)], &[]).unwrap();
        assert_eq!(report.pages, 6);
        let out = Document::from_bytes(bytes).unwrap();
        assert_eq!(crate::page_tree::pages(&out).unwrap().len(), 6);
    }

    #[test]
    fn merge_generates_one_top_level_bookmark_per_source() {
        // Acrobat's confirmed Combine-Files default.
        let a = three_pages();
        let b = three_pages();
        let (bytes, _) = merge(
            &[view(&a), view(&b)],
            &[b"First".to_vec(), b"Second".to_vec()],
        )
        .unwrap();
        let out = Document::from_bytes(bytes).unwrap();
        let outlines = out
            .catalog()
            .unwrap()
            .get(b"Outlines")
            .map(|o| out.resolve(o))
            .and_then(Object::as_dict)
            .expect("merged document must carry an outline");
        assert_eq!(outlines.get(b"Count").unwrap().as_int(), Some(2));
    }

    #[test]
    fn insert_places_source_pages_at_the_requested_slot() {
        let target = three_pages();
        let source = three_pages();
        let (bytes, report) = insert(
            &view(&target),
            &view(&source),
            &[0],
            InsertPosition::After(0),
        )
        .unwrap();
        assert_eq!(report.pages, 4);
        let out = Document::from_bytes(bytes).unwrap();
        let pages = crate::page_tree::pages(&out).unwrap();
        assert_eq!(pages.len(), 4);
        // Target page 1, then the inserted page, then target pages 2-3.
        assert_eq!(pages[1].contents.len(), 1);
    }

    #[test]
    fn insert_position_clamps_rather_than_refusing_past_the_end() {
        assert_eq!(InsertPosition::After(99).slot(3), 3);
        assert_eq!(InsertPosition::Before(99).slot(3), 3);
        assert_eq!(InsertPosition::Start.slot(3), 0);
        assert_eq!(InsertPosition::End.slot(3), 3);
        assert_eq!(InsertPosition::After(0).slot(3), 1);
    }

    #[test]
    fn extract_from_a_session_with_an_authored_annotation_survives_byte_exact_x5() {
        // The X5 discharge: a DocumentView over an editing session that has
        // authored an appearance must read that appearance's staged span
        // through `authored_source()` (base ++ staging), not the base bytes
        // alone. If the view were built over the base bytes, the authored
        // span would read off the end and stage as EMPTY — the silent
        // failure the DocumentView assertion was written to catch.
        use crate::annot_author::{Color, MarkupSpec};
        use crate::edit::EditSession;
        use crate::page_tree::Rect;

        let doc = build_pdf(&[
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (
                2,
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 300] \
                 /Resources << >> >>",
            ),
            (3, "<< /Type /Page /Parent 2 0 R >>"),
        ]);
        let mut session = EditSession::new(doc);
        let spec = MarkupSpec::Square {
            rect: Rect {
                llx: 20.0,
                lly: 20.0,
                urx: 120.0,
                ury: 70.0,
            },
            border: Some(Color::Rgb(1.0, 0.0, 0.0)),
            interior: None,
            border_width: 2.0,
        };
        let expected = crate::annot_author::build_appearance(&spec).ap_content;
        session.add_markup(0, &spec).unwrap();

        // Build the view over the SESSION (its overlay graph + combined
        // authored source), then extract page 0.
        let source = session.authored_source();
        let version = session.document().version();
        let graph = session.graph();
        let view = DocumentView::new(&graph, &source, version);
        let (bytes, report) = extract(&view, &[0]).unwrap();
        assert_eq!(report.pages, 1);

        // The extracted document must carry the annotation AND its
        // appearance content byte-for-byte.
        let out = Document::from_bytes(bytes).unwrap();
        let out_pages = crate::page_tree::pages(&out).unwrap();
        let annots = crate::annot::page_annotations(&out, out_pages[0].id);
        assert_eq!(annots.len(), 1, "the authored annotation was copied");
        let crate::annot::Appearance::Normal { stream_id } = &annots[0].appearance else {
            panic!(
                "expected a usable normal appearance, got {:?}",
                annots[0].appearance
            );
        };
        let Some(Object::Stream(stream)) = out.value(stream_id.unwrap()) else {
            panic!("appearance stream missing after extract");
        };
        let raw = stream.data_span.slice(out.bytes()).unwrap();
        let decoded = crate::filters::decode_stream(&stream.dict, raw).unwrap();
        assert_eq!(
            decoded, expected,
            "the authored appearance must survive extract byte-exact (X5)"
        );
    }
}
