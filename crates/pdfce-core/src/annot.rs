//! # Annotation model — walk `/Annots`, model each annotation, select its
//! appearance (ISO 32000-1 §12.5)
//!
//! This is the **read/model half** of Pass 6.0 (docs/decisions/008,
//! `first_pass_scope`). It sits on decision 005's *"core decodes and
//! models, render paints"* axis (R26): this module walks a page's
//! `/Annots`, resolves each annotation dictionary, decodes its flags, and
//! **selects** the normal appearance stream a conforming reader would
//! paint — but it **never paints, never decides colour, and never
//! synthesises a look** (R43). `pdfce-render`'s `annot` module consumes
//! [`Annotation`] and performs the §12.5.5 placement + painting.
//!
//! ## Scope — deliberately read-only (R43, decision 008 non-goals)
//!
//! Pass 6.0 introduces **no authoring capability**. This module has no
//! writer, produces no `/AP`, and synthesises nothing from `/MK`, `/IC`,
//! `/C`, `/DA`, `/L`, `/QuadPoints`, `/InkList`, or icon names. An
//! annotation without a usable appearance stream is **classified and
//! counted, not drawn** — that counter is the measured demand signal for
//! the later appearance-*generation* Passes (6.1/6.2/7).
//!
//! **That scope is unchanged by `Pass 38.5`, which is worth saying
//! because the Pass added four things here.** This module still only
//! reads;
//! [`EditSession::delete_annotation`](crate::edit::EditSession::delete_annotation)
//! does the writing and lives in `edit`. What arrived here is the model
//! that verb needs, and each piece is a key this file previously skipped
//! for being display-irrelevant:
//!
//! | Added | Clause | Why a DELETION verb needs it |
//! |---|---|---|
//! | [`Annotation::popup`] | §12.5.6.14, Table 170 | A pop-up *"shall not appear alone"*; deleting a markup annotation must take its window, or §12.5.6.2 NOTE 2 makes the orphan start displaying the deleted comment's own text. |
//! | [`Annotation::in_reply_to`] | §12.5.6.2, Table 170 | Who refers to the annotation being removed. |
//! | [`Annotation::reply_type`] + [`ReplyType`] | §12.5.6.2, Table 170 | `/RT /R` and `/RT /Group` are the SAME key with different deletion consequences — and Table 170's default is `R`, so an absent key is a reply. |
//! | [`AnnotFlags::LOCKED`] | §12.5.3, Table 165 bit 8 | *"do not allow the annotation to be deleted"* — the only clause in ISO 32000-1 that gates this operation. Explicitly **not** bit 10 `LockedContents`, which *"does not restrict deletion"*. |
//!
//! `LOCKED` is a deliberate exception to [`AnnotFlags`]'s own
//! display-flags-only rule, stated at the type. It must never gate
//! rendering.
//!
//! ## Spec sources (PDF-spec RAG, ISO 32000-1:2008)
//!
//! - `iso32000__s__12.5.2.md` — §12.5.1–.2, Table 164 (entries common to
//!   all annotation dictionaries): `/Subtype` (Required), `/Rect`
//!   (Required), `/F` flags, `/AP`, `/AS`. **`/Annots` is Optional and
//!   NOT inheritable** (a flat per-page array; §7.7.3.4 lists exactly
//!   four inheritable attributes and this is not one). A given annotation
//!   dictionary *"shall be referenced from the `Annots` array of only one
//!   page"*.
//! - `iso32000__s__12.5.3.md` — §12.5.3, Table 165 (the 10 annotation
//!   flags). Bit *N* has integer value `2^(N-1)`. Hidden (bit 2) and
//!   NoView (bit 6) are the display-suppression flags; the rest have no
//!   Pass-6.0 display consequence.
//! - `iso32000__s__12.5.5.md` — §12.5.5, Table 168 (the appearance
//!   dictionary `/AP`: `/N` normal, `/R` rollover, `/D` down). `/N` may be
//!   a single stream **or** a subdictionary keyed by appearance state,
//!   with `/AS` selecting. The placement algorithm (BBox→Matrix→Rect)
//!   lives in `pdfce-render`; this module only *selects* the `/N` stream.
//! - `iso32000__s__12.5.6.md` — §12.5.6, per-subtype map. Every geometry
//!   subtype defines a fallback look AND says *"`/AP` takes precedence"*;
//!   R43 makes `/AP` the **only** thing pdfce draws. `/Popup`
//!   (§12.5.6.14) *"shall have no appearance stream"* and is **never**
//!   painted as page content — a structural rule, stronger than R43.
//!
//! ## What this module deliberately does NOT model yet
//!
//! - **`/OC` optional content (§8.11)** is a GAP: the clause is not in the
//!   RAG and pdfce implements no optional-content state anywhere (the
//!   content interpreter defers `BDC`/`EMC` marked content too). An
//!   annotation in an OFF optional-content group would be *"skipped as if
//!   not in the document"* by a full reader; pdfce paints it (consistent
//!   with the rest of the renderer ignoring OC). Recorded as a known,
//!   consistent deferral rather than a silent divergence.
//! - **`/R` and `/D`** (rollover/down) are recognised but never selected —
//!   they are interaction states no static display drives (§12.5.5); this
//!   module models only `/N`. **(`Pass 38.5` note: the deletion verb
//!   nevertheless collects `/R` and `/D` streams when it removes an
//!   annotation. "Never selected for painting" and "never cleaned up" are
//!   different claims, and leaving them would orphan a stream in every
//!   subsequent save.)**
//! - **The pop-up's own `/Parent` back-reference** (Table 183) is not
//!   modelled, and [`Annotation::popup`] is. Deliberate, not an
//!   asymmetry to be tidied: the authoritative direction is the parent's
//!   `/Popup`, and finding a parent by scanning for whoever names the
//!   pop-up also copes with the malformed-but-real case of a `/Parent`
//!   that disagrees with its claimed parent's `/Popup`. Modelling both
//!   would mean two answers to one question.
//! - **Reply-thread RESOLUTION.** [`Annotation::in_reply_to`] is a link,
//!   not a walk: this module never follows it, never assembles a thread,
//!   and never applies §12.5.6.2's group-attribute rule (a subordinate's
//!   `Contents`/`T`/`M`/`C` *"shall be ignored"* in favour of its
//!   primary's). Applying it here would make [`Annotation::contents`]
//!   disagree with the dictionary it came from — see that field's own
//!   note. A consumer that wants the resolved view builds it from the
//!   links.

use std::collections::BTreeSet;

use crate::graph::ObjectGraph;
use crate::object::{Dict, Name, ObjId, Object};
use crate::page_tree::Rect;
use crate::settings::MissingAppearanceState;

/// Maximum annotations modelled from one page's `/Annots` array
/// (pdfce policy, ARCHITECTURE.md §10.1 adversarial-input posture).
///
/// **No spec limit exists to inherit.** Annex C (informative) lists no
/// annotation-count bound, and PDF/A §6.1.12 positively forbids a reader
/// from imposing Annex C's implementation limits — so this is pure pdfce
/// policy and must clear any conformant corpus. It bounds only the linear
/// allocation a hostile `/Annots` array (millions of tiny dictionaries)
/// could pin; a page carrying more than this many real annotations is
/// beyond any measured document. Chosen far above the corpus maximum
/// (see `tools/annot-corpus-check.py`) so the veraPDF §6.1.12
/// implementation-limits suite reports comfortable headroom, in the same
/// spirit as [`crate::page_tree::MAX_PAGES`].
pub const MAX_ANNOTS_PER_PAGE: usize = 1_000_000;

/// Decoded `/F` annotation flags (ISO 32000-1 §12.5.3, Table 165).
///
/// Bit positions are numbered from the low-order bit as **bit 1**, so
/// bit *N* has integer value `2^(N-1)` (§12.5.3 verbatim). Getting this
/// off by one silently mis-reads every flag, so the bit constants below
/// are named against Table 165 and pinned by a test. Default `/F` is `0`
/// (no flags; Table 164).
///
/// Only the display-relevant flags get accessors here — Pass 6.0 is a
/// display Pass. ReadOnly/Locked/ToggleNoView/LockedContents have **no
/// display consequence** (they govern interaction/editing) and are
/// deliberately not surfaced, so they cannot accidentally gate rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnnotFlags(pub u32);

impl AnnotFlags {
    /// Bit 1 (value 1) — Invisible: suppress an *unknown* subtype that
    /// has no handler and no `/AP`. A near-noop for pdfce (R43 paints
    /// strictly from `/AP`), so an unknown subtype with an `/AP` is
    /// painted regardless.
    pub const INVISIBLE: u32 = 1 << 0;
    /// Bit 2 (value 2) — Hidden: *"do not display or print … regardless
    /// of annotation type or handler."* The strongest suppression —
    /// gone from both screen and print.
    pub const HIDDEN: u32 = 1 << 1;
    /// Bit 3 (value 4) — Print: print when the page is printed; clear
    /// means a screen-only annotation. No on-screen consequence.
    pub const PRINT: u32 = 1 << 2;
    /// Bit 4 (value 8) — NoZoom: do not scale the appearance to the page
    /// magnification. Feeds the §12.5.5 post-placement transform.
    pub const NO_ZOOM: u32 = 1 << 3;
    /// Bit 5 (value 16) — NoRotate: do not rotate the appearance to the
    /// page rotation. Feeds the §12.5.5 post-placement transform.
    pub const NO_ROTATE: u32 = 1 << 4;
    /// Bit 6 (value 32) — NoView: suppress on **screen** but allow
    /// **print** (if Print is set). The inverse of a screen-only
    /// annotation, and a document-forensics vector when paired with
    /// Print.
    pub const NO_VIEW: u32 = 1 << 5;
    /// Bit 8 (value 128) — **Locked**: *"If set, do not allow the
    /// annotation to be **deleted** or its properties (including position
    /// and size) to be modified by the user."*
    ///
    /// # The only editing gate the standard puts on an annotation
    ///
    /// Added by `Pass 38.5`, and it is the exception to this type's own
    /// "display flags only" scope note directly above — deliberately, and
    /// with the reason: `Locked` is the one Table 165 bit that constrains
    /// an operation pdfce performs. Every other non-display flag governs
    /// interaction pdfce has no surface for.
    ///
    /// **It has NO display consequence and must never gate rendering.**
    /// A locked annotation paints exactly like an unlocked one; the flag
    /// is about who may change it.
    ///
    /// **Not to be confused with bit 10, `LockedContents`**, whose own
    /// Table 165 text says it *"does not restrict deletion"* — it locks
    /// the *contents* of the annotation, not its existence. Treating the
    /// two as one would refuse deletions the standard permits; treating
    /// `Locked` as cosmetic would perform deletions it forbids. Hence a
    /// named constant for exactly one of them.
    pub const LOCKED: u32 = 1 << 7;

    /// Whether the Hidden flag (Table 165 bit 2) is set.
    #[must_use]
    pub const fn hidden(self) -> bool {
        self.0 & Self::HIDDEN != 0
    }

    /// Whether the NoView flag (Table 165 bit 6) is set.
    #[must_use]
    pub const fn no_view(self) -> bool {
        self.0 & Self::NO_VIEW != 0
    }

    /// Whether the Print flag (Table 165 bit 3) is set (for the future
    /// print path; no screen consequence).
    #[must_use]
    pub const fn print(self) -> bool {
        self.0 & Self::PRINT != 0
    }

    /// Whether the Invisible flag (Table 165 bit 1) is set.
    #[must_use]
    pub const fn invisible(self) -> bool {
        self.0 & Self::INVISIBLE != 0
    }

    /// Whether the NoZoom flag (Table 165 bit 4) is set.
    #[must_use]
    pub const fn no_zoom(self) -> bool {
        self.0 & Self::NO_ZOOM != 0
    }

    /// Whether the NoRotate flag (Table 165 bit 5) is set.
    #[must_use]
    pub const fn no_rotate(self) -> bool {
        self.0 & Self::NO_ROTATE != 0
    }

    /// Whether the **Locked** flag (Table 165 bit 8) is set — the
    /// annotation may not be deleted, moved or resized by the user.
    ///
    /// See [`Self::LOCKED`] for why this one non-display flag has an
    /// accessor when the other three do not, and for the
    /// `LockedContents` trap.
    #[must_use]
    pub const fn locked(self) -> bool {
        self.0 & Self::LOCKED != 0
    }

    /// Whether this annotation is suppressed from **on-screen** display,
    /// i.e. Hidden **or** NoView (§12.5.3, Table 165).
    ///
    /// This is the render path's screen-suppression predicate. Per R50 a
    /// suppressed annotation is *honoured AND counted* — never silently
    /// dropped — because *"a page carrying content the operator cannot
    /// see is a fact they are entitled to know"* (hidden annotations are
    /// a recognised document-forensics vector).
    #[must_use]
    pub const fn suppressed_on_screen(self) -> bool {
        self.hidden() || self.no_view()
    }
}

/// The outcome of §12.5.5 **normal-appearance** (`/AP` `/N`) selection for
/// one annotation.
///
/// Core *selects*; `pdfce-render` *places and paints*. The variants are
/// the full negative-result taxonomy the §12.5.5 RAG enumerates, because
/// under R43 *how* an annotation fails to yield an appearance is exactly
/// the diagnostic the operator is entitled to (R20/R27) and the demand
/// signal the later generation Passes measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Appearance {
    /// A normal appearance stream (a form XObject, §8.10) resolved and
    /// ready to place. `stream_id` is its object identity — present in
    /// every well-formed file because *"all streams shall be indirect
    /// objects"* (§7.3.8.1), and it is the §8.10 cycle-guard key the
    /// render path needs. `None` only in the pathological case of a
    /// stream reached without an indirect reference.
    Normal {
        /// Identity of the appearance form XObject (the cycle-guard key).
        stream_id: Option<ObjId>,
    },
    /// No usable normal appearance: `/AP` is absent, `/AP` is not a
    /// dictionary, `/N` is absent, `/N` resolves to null (dangling), or
    /// `/N` is neither a stream nor a subdictionary. Under R43 this is
    /// **named-not-painted, counted by subtype** — never synthesised.
    None,
    /// `/N` is an appearance-state **subdictionary** but the state could
    /// not be selected: `/AS` is missing against a multi-entry
    /// subdictionary, or `/AS` names a state the subdictionary does not
    /// define (§12.5.5 NOTE 3: *"reasonable behaviour such as displaying
    /// nothing"*). pdfce displays nothing and **does not guess** a
    /// first/`On`/`Off` key (the RAG's explicit negative result — real
    /// readers vary, so guessing would show a state no other reader
    /// picks). Counted separately from [`Appearance::None`] because the
    /// annotation *does* carry appearances; only selection failed.
    StateUnresolved,
}

/// One page annotation, modelled read-only (ISO 32000-1 §12.5, Table 164).
///
/// Carries exactly what the render path needs to place-and-paint plus what
/// the diagnostics need to count — no more. It is **not** a faithful echo
/// of the annotation dictionary (per-subtype geometry keys `/L`,
/// `/Vertices`, `/InkList`, `/QuadPoints`, `/IC`, `/MK`, icon `/Name` are
/// deliberately *not* modelled: under R43 they are neither painted nor,
/// in Pass 6.0, generated from).
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    /// The annotation object's identity, if it was reached by an indirect
    /// reference from `/Annots` (it always is in a well-formed file —
    /// Table 164's dictionaries are indirect objects). Used only for
    /// diagnostics/dedup; the render path does not need it.
    pub id: Option<ObjId>,
    /// The `/Subtype` name bytes (Table 164, Required). Empty when the
    /// entry is absent — a malformed annotation, surfaced not repaired.
    pub subtype: Vec<u8>,
    /// The `/Rect` in default user space, normalised per §7.9.5 (corners
    /// may be given in either order). `None` when `/Rect` is absent or
    /// malformed — the §12.5.5 placement target is then missing and the
    /// render path refuses placement by name.
    pub rect: Option<Rect>,
    /// Decoded `/F` flags (§12.5.3, Table 165). Default `0`.
    pub flags: AnnotFlags,
    /// The selected normal (`/N`) appearance, per §12.5.5.
    pub appearance: Appearance,
    /// Whether `/Subtype` is `Popup` (§12.5.6.14). A `/Popup` is a reader
    /// UI window, **never** page content — a structural non-paint rule
    /// stronger than R43, checked before flags or appearance (risk X4).
    pub is_popup: bool,
    /// `/Contents` — the annotation's text, decoded per §7.9.2 (Table 164,
    /// Optional, PDF 1.0). `None` when the key is absent.
    ///
    /// # It is DUAL-PURPOSE, and a consumer must not assume "comment"
    ///
    /// §12.5.2: this is *"text displayed for the annotation, **or** (if the
    /// type does not display text) an alternate human-readable description"*
    /// for accessibility (§14.9.3). Which one it is depends on the subtype
    /// (§12.5.6.2): a `FreeText` DISPLAYS it, most markup types put it in the
    /// pop-up, and `Link`/`Movie`/`Widget`/`PrinterMark`/`TrapNet` use it
    /// purely as an accessibility alternate. So a UI labelling this "comment"
    /// is right for markup and wrong for a Link — modelled here without that
    /// interpretation, which belongs to whoever displays it.
    ///
    /// **Not resolved here:** §12.5.6.2 NOTE 2 says a markup annotation with
    /// a parent (`/IRT` reply) has its own `Contents` "shall be ignored".
    /// The `/IRT` link IS now modelled ([`Self::in_reply_to`],
    /// [`Self::reply_type`], added by Pass 38.5 so annotation *deletion*
    /// could reason about reply chains) — but the rule is still deliberately
    /// **not applied here**: this field stays the raw value the dictionary
    /// carries, and any consumer that wants the group-attribute resolution
    /// walks the link itself. Silently substituting a primary's `/Contents`
    /// for a subordinate's would make the model disagree with the file,
    /// which is the one thing `pdfce-core`'s read half must never do.
    pub contents: Option<String>,
    /// `/T` — the annotation's title, conventionally the AUTHOR (Table 170,
    /// markup annotations only). `None` when absent.
    ///
    /// # Table 170, NOT Table 164 — this is not a common key
    ///
    /// `/T` is a **markup-annotation** key (§12.5.6), so it is legitimately
    /// absent on a `Link`, a `Widget` or a `PrinterMark`. Reading it here for
    /// every subtype is deliberate and harmless — an absent key is `None`,
    /// which is exactly the truth — but a consumer must not read `None` as
    /// "anonymous"; on a non-markup annotation it means "this subtype has no
    /// such concept".
    pub title: Option<String>,
    /// `/M` — the modification date, **raw and unparsed** (Table 164,
    /// Optional, PDF 1.1). `None` when absent.
    ///
    /// # Stored raw because the standard requires accepting anything
    ///
    /// §12.5.2 gives its type as "date **or text string**" and says a
    /// conforming reader *"shall accept and display a string in any format"*.
    /// Parsing to a date type would therefore have to either reject or
    /// silently mangle values the standard explicitly requires be accepted —
    /// so this is a `String`, and any future sort-by-date feature owns the
    /// decision about what to do with a value that is not a §7.9.4 date.
    pub mod_date: Option<String>,
    /// The `/OC` optional-content group/membership reference (§8.11.3.3), if
    /// the annotation carries one. Its default visibility is resolved against
    /// the catalog `/OCProperties /D` config: the annotation is visible only
    /// if the flags permit AND its OCG is ON (Pass 12.M2 authored-layer `/OC`
    /// honouring — decision 011 §2.4; full content-stream BDC/EMC `/OC` stays
    /// deferred). `None` when the annotation is on no layer.
    pub oc: Option<ObjId>,
    /// `/Popup` — this markup annotation's pop-up window companion
    /// (Table 170, Optional, PDF 1.3). `None` when the annotation has no
    /// pop-up, which includes every non-markup subtype.
    ///
    /// # Why the READ half models it: the popup cannot outlive its parent
    ///
    /// §12.5.6.14 is a `shall`: a pop-up annotation *"**shall not appear
    /// alone** but is associated with a markup annotation, its parent
    /// annotation"*. So this is not a decorative back-link — it is a
    /// **structural dependency**, and any operation that removes the
    /// parent must remove the pop-up in the same breath or leave the
    /// document violating that clause. Modelling it here rather than
    /// re-reading the dictionary at each call site is what lets
    /// [`crate::edit::EditSession::delete_annotation`] and any future
    /// mover/copier agree about the pair.
    ///
    /// The reference is surfaced **unresolved and unvalidated**: a
    /// `/Popup` pointing at a missing object, or at something that is not
    /// a `/Popup` annotation, is modelled as-is. Repairing it here would
    /// be the read half inventing structure (R27).
    pub popup: Option<ObjId>,
    /// `/IRT` — the annotation this one is *in reply to* (Table 170,
    /// Optional, PDF 1.5). `None` for an annotation that is not part of a
    /// thread or a group.
    ///
    /// # A reference with TWO meanings, disambiguated by [`Self::reply_type`]
    ///
    /// The same key builds two different structures (§12.5.6.2), and they
    /// behave differently under deletion, which is why both are modelled
    /// and neither is collapsed into the other:
    ///
    /// - **`/RT /R`** (a *reply*, and the default): an ordinary threaded
    ///   comment. It keeps its own author and text and is readable on its
    ///   own; losing its target costs the thread's shape, not its content.
    /// - **`/RT /Group`**: the target is the group **primary**, and
    ///   §12.5.6.2 says the subordinate's own `Contents`/`RC`+`DS`, `M`,
    ///   `C`, `T`, `Popup`, `CreationDate`, `Subj` and `Open` *"shall be
    ///   ignored"* in favour of the primary's. A subordinate whose primary
    ///   is gone therefore carries text a conforming reader is instructed
    ///   not to use.
    ///
    /// Surfaced unresolved, same as [`Self::popup`]: a dangling `/IRT` is
    /// modelled, not repaired.
    pub in_reply_to: Option<ObjId>,
    /// `/RT` — how [`Self::in_reply_to`] should be read (Table 170,
    /// Optional, PDF 1.6).
    ///
    /// `None` when the key is absent. **`None` is NOT the same as
    /// [`ReplyType::Reply`]** at the model level even though Table 170's
    /// default value is `R`: the absent case is a document fact this
    /// struct reports, and a consumer that wants the default applies it
    /// with [`Self::effective_reply_type`] rather than being unable to
    /// tell "the file said `R`" from "the file said nothing".
    pub reply_type: Option<ReplyType>,
}

/// `/RT` — the relationship [`Annotation::in_reply_to`] expresses
/// (ISO 32000-1 §12.5.6.2, Table 170, PDF 1.6).
///
/// Two values are defined by the standard; anything else is preserved
/// rather than coerced, because a name pdfce does not recognise is a
/// document fact and flattening it to the default would make the model
/// claim the file said something it did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyType {
    /// `/R` — a threaded **reply**. Table 170's default value.
    Reply,
    /// `/Group` — this annotation is a **subordinate** of the primary named
    /// by `/IRT`, and §12.5.6.2's group-attribute rule applies: its own
    /// `Contents` (or `RC`+`DS`), `M`, `C`, `T`, `Popup`, `CreationDate`,
    /// `Subj` and `Open` *"shall be ignored"* in favour of the primary's.
    Group,
    /// A `/RT` name that is neither `/R` nor `/Group`. Carried verbatim.
    Other(Vec<u8>),
}

impl Annotation {
    /// Whether this annotation is a form-field widget (`/Subtype`
    /// `Widget`, §12.5.6.19). A widget *is* an annotation first (R49);
    /// this is a census convenience — 87.8 % of organic annotations are
    /// widgets, so the count is a load-bearing demand signal.
    #[must_use]
    pub fn is_widget(&self) -> bool {
        self.subtype == b"Widget"
    }

    /// Whether this annotation is a `/RT /Group` **subordinate** — it has
    /// an `/IRT` target and that link is a grouping link, not a reply link
    /// (§12.5.6.2).
    ///
    /// The distinction matters to anything that removes or moves the
    /// primary: a subordinate's own `Contents`/`T`/`M`/`C`/`Popup`/
    /// `CreationDate`/`Subj`/`Open` *"shall be ignored"* in favour of the
    /// primary's, so losing the primary costs a subordinate its readable
    /// identity in a way losing a reply's target does not.
    ///
    /// Note the `&&`: an `/RT /Group` with **no** `/IRT` is not a
    /// subordinate of anything. Table 170 makes `/RT` meaningful only
    /// alongside `/IRT`, so the pair is checked, never `/RT` alone.
    #[must_use]
    pub fn is_group_subordinate(&self) -> bool {
        self.in_reply_to.is_some() && self.effective_reply_type() == Some(ReplyType::Group)
    }

    /// [`Self::reply_type`] with Table 170's **default value `R`** applied
    /// when the key is absent but `/IRT` is present.
    ///
    /// Returns `None` only when there is no `/IRT` at all — because `/RT`
    /// is meaningful only alongside `/IRT`, so "what relationship is this"
    /// has no answer for an annotation that is in reply to nothing.
    ///
    /// This exists so callers stop re-deriving the default. Table 170's
    /// defaults are permissive (§12.8.2.2's `/P` is the same trap), and a
    /// call site that treats an absent `/RT` as "not a reply" is wrong in
    /// the ordinary case: the ordinary case is a threaded comment that
    /// simply relies on the default.
    #[must_use]
    pub fn effective_reply_type(&self) -> Option<ReplyType> {
        self.in_reply_to?;
        Some(self.reply_type.clone().unwrap_or(ReplyType::Reply))
    }

    /// A stable, human/diagnostic label for the subtype: the `/Subtype`
    /// name bytes decoded lossily, or `"(no Subtype)"` when absent. Used
    /// as the by-subtype key of the `annotations_without_ap` counter, so
    /// it must be deterministic (it is — a pure function of the bytes).
    #[must_use]
    pub fn subtype_label(&self) -> String {
        if self.subtype.is_empty() {
            "(no Subtype)".to_owned()
        } else {
            String::from_utf8_lossy(&self.subtype).into_owned()
        }
    }
}

/// Walk one page's `/Annots` array and model every annotation on it
/// (ISO 32000-1 §12.5.2).
///
/// `page_id` is the page object's identity (from
/// [`crate::page_tree::Page::id`]). `/Annots` is read off the page
/// dictionary directly — it is **not inheritable** (§7.7.3.4), so there is
/// no page-tree walk: a page with no `/Annots` has no annotations, full
/// stop.
///
/// Generic over [`ObjectGraph`] so it works over both the loaded
/// [`Document`](crate::document::Document) and an
/// [`EditSession`](crate::edit::EditSession) overlay, exactly like
/// [`crate::page_tree::pages_in`]. Every malformed shape is tolerated by
/// skipping and modelling what is there — never a panic, never an abort
/// (the crate's adversarial-input policy):
///
/// - `/Annots` absent, null, or not an array → no annotations.
/// - An array entry that is not a dictionary (a null from a dangling
///   reference, a stray number) → skipped.
/// - `/Annots` may be a **shared indirect array** referenced by more than
///   one page (malformed per *"referenced from only one page"*, but seen
///   in the wild). This read-only walk simply reads it for each page; it
///   never mutates, so sharing is harmless here (the copy-on-write concern
///   is a Pass 6.1 authoring problem, risk X7).
///
/// The result is bounded by [`MAX_ANNOTS_PER_PAGE`].
#[must_use]
pub fn page_annotations<G: ObjectGraph + ?Sized>(graph: &G, page_id: ObjId) -> Vec<Annotation> {
    page_annotations_with(graph, page_id, MissingAppearanceState::default())
}

/// [`page_annotations`] with an explicit `AS-A1` policy (R169).
///
/// ## What `missing_as` decides, and what it does not
///
/// **Only** the malformed configuration §12.5.5 leaves undefined: an
/// `/AP` `/N` subdictionary of **two or more** entries with **no `/AS`**.
/// Table 164 makes `/AS` *required* there, and NOTE 3 covers only the
/// neighbouring case (`/AS` present, naming an absent state), so the
/// standard states no recovery at all. Every other path through
/// [`select_normal_appearance`] is spec-determined and this parameter
/// cannot reach it — a `/N` stream still wins outright, a present `/AS`
/// still selects, an absent named state is still
/// [`Appearance::StateUnresolved`], and a **single**-entry subdictionary
/// with no `/AS` is still painted (there are no alternatives to choose
/// between, so painting it is not a guess).
///
/// The default is [`MissingAppearanceState::PaintNothing`] — the shipped
/// behaviour, **evidence tier (d)**, a reasoned guess and deliberately the
/// conservative one. The spec RAG's row is explicit that "paint the first"
/// and "paint `/Off`" are *empirical* guesses belonging to
/// `C:\personal_rag\pdf\`, and installing one as the default would put a
/// plausible appearance on screen with nothing to say pdfce chose it.
///
/// ## A separate function rather than a changed signature
///
/// [`page_annotations`] has callers in `pdfce-gui`, `pdfce-cli` and four
/// test crates, none of which have an opinion about this. Following the
/// crate's existing `*_with` convention (`pageops::extract_with`,
/// `EditSession::delete_pages_with`) keeps the policy explicit at the one
/// call site that carries the operator's setting — the renderer — and
/// keeps it out of the way everywhere else.
#[must_use]
pub fn page_annotations_with<G: ObjectGraph + ?Sized>(
    graph: &G,
    page_id: ObjId,
    missing_as: MissingAppearanceState,
) -> Vec<Annotation> {
    let page = graph.resolved(page_id);
    let Some(page_dict) = page.as_dict() else {
        return Vec::new();
    };
    let Some(annots_obj) = page_dict.get(b"Annots") else {
        return Vec::new();
    };
    let Some(array) = graph.resolve(annots_obj).as_array() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in array {
        if out.len() >= MAX_ANNOTS_PER_PAGE {
            break;
        }
        let id = entry.as_reference();
        let Some(dict) = graph.resolve(entry).as_dict() else {
            // A null (dangling reference) or non-dictionary entry is not
            // an annotation. §7.3.10 makes a dangling reference null, not
            // an error; skip it.
            continue;
        };
        out.push(model_annotation(graph, id, dict, missing_as));
    }
    out
}

/// Model one annotation dictionary into an [`Annotation`] (Table 164 +
/// §12.5.5 appearance selection).
fn model_annotation<G: ObjectGraph + ?Sized>(
    graph: &G,
    id: Option<ObjId>,
    dict: &Dict,
    missing_as: MissingAppearanceState,
) -> Annotation {
    let subtype = graph
        .resolve(dict.get(b"Subtype").unwrap_or(&Object::Null))
        .as_name()
        .map(|n| n.as_bytes().to_vec())
        .unwrap_or_default();
    let is_popup = subtype == b"Popup";

    let rect = dict.get(b"Rect").and_then(|o| read_rect(graph, o));

    // §12.5.3: /F is an integer bitfield; default 0. A non-integer /F is
    // malformed — treated as 0 (no flags) rather than rejected.
    let flags = AnnotFlags(
        dict.get(b"F")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_int)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0),
    );

    let appearance = select_normal_appearance(graph, dict, missing_as);

    // §8.11.3.3 annotation /OC entry — an OCG or OCMD indirect reference. Only
    // the reference is modelled here; the render path resolves its default
    // visibility against /OCProperties /D (Pass 12.M2).
    let oc = dict.get(b"OC").and_then(Object::as_reference);

    // §7.9.2 text strings: `/Contents` and `/T` are text strings and may be
    // UTF-16BE with a BOM, so they go through the same decoder every other
    // text-string consumer in this crate uses rather than a second, private
    // lossy conversion that would disagree with it on non-Latin input.
    //
    // `/M` deliberately does NOT: it is "date or text string" and the
    // standard requires accepting any format, so it is surfaced verbatim.
    let text_of = |key: &[u8]| -> Option<String> {
        match graph.resolve(dict.get(key)?) {
            Object::String(bytes) => Some(crate::edit::decode_text_string(bytes).text),
            _ => None,
        }
    };
    let contents = text_of(b"Contents");
    let title = text_of(b"T");
    let mod_date = text_of(b"M");

    // §12.5.6.2 Table 170 reply/grouping structure. Read for EVERY subtype,
    // not only markup ones, for the same reason `/T` is: an absent key is
    // `None`, which is exactly the truth, and a type-gated reader would make
    // the model silently disagree with a file that carries the key anyway.
    //
    // All three are taken as REFERENCES rather than resolved dictionaries.
    // Deletion needs identity (is THIS the object my `/IRT` names?), and a
    // resolved copy cannot answer that; a `/Popup` written as a direct
    // dictionary — illegal, since Table 164 dictionaries are indirect
    // objects — yields `None` and is therefore reported as "no companion"
    // rather than as a companion nothing can address.
    let popup = dict.get(b"Popup").and_then(Object::as_reference);
    let in_reply_to = dict.get(b"IRT").and_then(Object::as_reference);
    let reply_type = graph
        .resolve(dict.get(b"RT").unwrap_or(&Object::Null))
        .as_name()
        .map(|n| match n.as_bytes() {
            b"R" => ReplyType::Reply,
            b"Group" => ReplyType::Group,
            other => ReplyType::Other(other.to_vec()),
        });

    Annotation {
        id,
        subtype,
        rect,
        flags,
        appearance,
        is_popup,
        oc,
        contents,
        title,
        mod_date,
        popup,
        in_reply_to,
        reply_type,
    }
}

/// The set of optional-content groups that are **OFF by default** per the
/// catalog `/OCProperties /D` configuration (ISO 32000-1 §8.11.4.3, Table
/// 101). Pass 12.M2 render-visibility input: an annotation whose `/OC`
/// resolves to (or through an OCMD to) an OFF group is hidden.
///
/// Follows the spec initialisation order: `/BaseState` (default `ON`) sets
/// all groups, then `/ON`/`/OFF` override. If `/OCProperties` or `/D` is
/// absent, the set is empty (nothing hidden by default). A missing
/// `/OCProperties` means optional content is ignored entirely (§8.11.4.2) —
/// returning an empty OFF set realises exactly that.
#[must_use]
pub fn optional_content_default_off<G: ObjectGraph + ?Sized>(graph: &G) -> BTreeSet<ObjId> {
    let mut off = BTreeSet::new();
    let Some(catalog) = graph.catalog_dict() else {
        return off;
    };
    let Some(ocp) = graph
        .resolve(catalog.get(b"OCProperties").unwrap_or(&Object::Null))
        .as_dict()
    else {
        return off;
    };
    let Some(d) = graph
        .resolve(ocp.get(b"D").unwrap_or(&Object::Null))
        .as_dict()
    else {
        return off;
    };
    let base_off = graph
        .resolve(d.get(b"BaseState").unwrap_or(&Object::Null))
        .as_name()
        .is_some_and(|n| n.as_bytes() == b"OFF");
    if base_off {
        // All OCGs start OFF; /ON re-enables.
        off.extend(oc_refs(graph, ocp.get(b"OCGs")));
        for on in oc_refs(graph, d.get(b"ON")) {
            off.remove(&on);
        }
    } else {
        // Default BaseState ON; /OFF disables specific groups.
        off.extend(oc_refs(graph, d.get(b"OFF")));
    }
    off
}

/// Whether an annotation's `/OC` reference resolves to a hidden state, given
/// the default-OFF set from [`optional_content_default_off`] (§8.11.3.3).
///
/// A direct `/OCG` is hidden iff it is in `off`. An `/OCMD` is evaluated with
/// its default `AnyOn` policy (§8.11.2.2): hidden iff **all** its member OCGs
/// are OFF (an empty/undetermined membership is visible — the spec's "no
/// effect" rule). An unresolvable or non-optional-content target is treated as
/// visible (never hide by guessing).
#[must_use]
pub fn oc_is_hidden<G: ObjectGraph + ?Sized>(graph: &G, oc: ObjId, off: &BTreeSet<ObjId>) -> bool {
    let Some(d) = graph.resolved(oc).as_dict() else {
        return false;
    };
    let is_ocmd = graph
        .resolve(d.get(b"Type").unwrap_or(&Object::Null))
        .as_name()
        .is_some_and(|n| n.as_bytes() == b"OCMD");
    if is_ocmd {
        let members = oc_refs(graph, d.get(b"OCGs"));
        if members.is_empty() {
            // Table 99: with no `/OCGs` there is nothing to test, and
            // §8.11.3.3 makes visibility the default. `/VE` could still
            // decide it, but pdfce does not evaluate visibility
            // expressions (see the fn docs), and refusing to guess shows
            // the content rather than hiding it.
            return false;
        }
        // Table 99 `/P` — the visibility POLICY, default `AnyOn`.
        //
        // ★ This was not read at all until 2026-08-10, so every
        // membership dictionary was evaluated as `AnyOn`. The divergence
        // is not a rounding error: under `/P /AllOff` with every member
        // group OFF the standard says the content is VISIBLE, and pdfce
        // hid it — the exact inverse of the answer, on the policy whose
        // whole purpose is "show this when the layers are off".
        //
        // Note that each arm is written as the VISIBILITY test from the
        // table and then negated once, rather than as four hand-derived
        // hidden-tests. Deriving them by hand is how `AnyOff` and
        // `AllOff` get swapped: the negation of "any member is off" is
        // "every member is on", which is not any of the other three
        // policies and is easy to write as one by mistake.
        let on = |g: &ObjId| !off.contains(g);
        let visible = match graph
            .resolve(d.get(b"P").unwrap_or(&Object::Null))
            .as_name()
            .map(|n| n.as_bytes().to_vec())
            .as_deref()
        {
            Some(b"AllOn") => members.iter().all(on),
            Some(b"AnyOff") => members.iter().any(|g| !on(g)),
            Some(b"AllOff") => members.iter().all(|g| !on(g)),
            // `AnyOn` explicitly, and every other value: Table 99 names
            // four policies and gives `AnyOn` as the default, so an
            // unrecognised name falls back to the default rather than
            // inventing a fifth behaviour.
            _ => members.iter().any(on),
        };
        !visible
    } else {
        // Treat the reference itself as the OCG (Type /OCG or an untyped
        // group-shaped dict — the authored-layer case, §8.11 NOTE 3).
        off.contains(&oc)
    }
}

/// Collect the OCG references an `/OCGs`/`/ON`/`/OFF` entry names — either a
/// single indirect reference or an array of them (§8.11 Table 99/100/101).
///
/// # `pub(crate)` on purpose, and it must stay that way
///
/// [`crate::layers`] calls this. It briefly carried a byte-for-byte copy,
/// because this was private — and its own doc comment said so and asked
/// for the copy to be deleted if this ever became reachable. It has, and
/// it was.
///
/// The duplication mattered for a specific reason worth keeping: the
/// layers panel and the renderer must agree about **which groups a `/D`
/// array names**, or `locked` and `off` end up computed over different
/// sets of the same document — and the visible symptom is a panel that
/// says "shown" about content the page hides. Two functions cannot
/// guarantee that; one can.
///
/// Note the tolerance being shared as well as the behaviour: a single
/// reference is accepted where Table 101 says array. Both callers must
/// tolerate exactly the same malformed shapes, not merely the same
/// well-formed ones.
pub(crate) fn oc_refs<G: ObjectGraph + ?Sized>(graph: &G, obj: Option<&Object>) -> Vec<ObjId> {
    match obj.map(|o| graph.resolve(o)) {
        Some(Object::Reference(r)) => vec![*r],
        Some(Object::Array(items)) => items.iter().filter_map(Object::as_reference).collect(),
        _ => obj.and_then(Object::as_reference).into_iter().collect(),
    }
}

/// Select the normal (`/N`) appearance per ISO 32000-1 §12.5.5 (Table 168
/// + the `/AS` state-selection rule).
///
/// Returns the full negative-result taxonomy ([`Appearance`]); it never
/// guesses and never synthesises (R43).
fn select_normal_appearance<G: ObjectGraph + ?Sized>(
    graph: &G,
    annot: &Dict,
    missing_as: MissingAppearanceState,
) -> Appearance {
    // /AP (Table 164) — a dictionary. Absent or non-dictionary ⇒ nothing
    // to paint.
    let Some(ap) = annot
        .get(b"AP")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
    else {
        return Appearance::None;
    };
    // /N (Table 168, Required). Absent ⇒ no normal appearance (R43
    // named-not-painted).
    let Some(n) = ap.get(b"N") else {
        return Appearance::None;
    };

    match graph.resolve(n) {
        // Form 1 — /N is a stream: that stream IS the normal appearance;
        // /AS is ignored (§12.5.5). Streams are indirect (§7.3.8.1), so
        // `n` is a reference and carries the cycle-guard identity.
        Object::Stream(_) => Appearance::Normal {
            stream_id: n.as_reference(),
        },
        // Form 2 — /N is a subdictionary keyed by appearance state; /AS
        // selects.
        Object::Dict(subdict) => {
            let state = annot
                .get(b"AS")
                .map(|o| graph.resolve(o))
                .and_then(Object::as_name);
            select_state(graph, subdict, state, missing_as)
        }
        // /N present but neither stream nor dictionary (malformed). Under
        // R43 there is no usable appearance; named-not-painted.
        _ => Appearance::None,
    }
}

/// Select one stream from a `/N` appearance-state subdictionary using
/// `/AS` (§12.5.5, Table 164 `/AS` + NOTE 3).
fn select_state<G: ObjectGraph + ?Sized>(
    graph: &G,
    subdict: &Dict,
    state: Option<&Name>,
    missing_as: MissingAppearanceState,
) -> Appearance {
    match state {
        // /AS present: paint the sub-entry it names, or display nothing if
        // that state is absent (§12.5.5 NOTE 3).
        Some(state) => match subdict.get(state.as_bytes()) {
            Some(entry) => classify_state_entry(graph, entry),
            None => Appearance::StateUnresolved,
        },
        // /AS absent. §12.5.5: /AS is Required when /AP holds
        // subdictionaries, so this is malformed. The RAG's negative
        // result: the spec gives NO rule for choosing among entries, so
        // "display nothing" is the conservative extension of NOTE 3 —
        // pdfce must NOT guess a first/On/Off key *by default*. Under
        // R169 the guesses are available, named, and opt-in (`AS-A1`).
        None => {
            let mut present = subdict.iter().filter(|(_, v)| !matches!(v, Object::Null));
            match (present.next(), present.next()) {
                // Empty subdictionary ⇒ nothing to paint.
                (None, _) => Appearance::None,
                // Exactly one entry ⇒ unambiguous: there is only one
                // possible appearance, so painting it is not "guessing
                // among alternatives" that the RAG forbids — there are no
                // alternatives. (The forbidden case is a *multi-entry*
                // subdictionary with no /AS.) The setting does NOT reach
                // this arm: there is nothing here to have a policy about.
                (Some((_, only)), None) => classify_state_entry(graph, only),
                // Two or more entries, no /AS ⇒ the one genuinely
                // undefined case, and the only one `missing_as` governs.
                // Whichever way it goes the annotation is still surfaced
                // as state-unresolved when nothing is painted, so the
                // count never depends on the setting.
                (Some((_, first)), Some(_)) => match missing_as {
                    MissingAppearanceState::PaintNothing => Appearance::StateUnresolved,
                    // "First" is the dictionary's own iteration order,
                    // which `Dict` preserves from the file — so this is
                    // the PRODUCER's first entry, not an alphabetical
                    // invention of pdfce's.
                    MissingAppearanceState::FirstEntry => classify_state_entry(graph, first),
                    // The checkbox-shaped guess. `/Off` is Table 164's own
                    // conventional name for an unset widget state, and it
                    // is the state that misleads least if the guess is
                    // wrong. Absent ⇒ back to painting nothing rather than
                    // falling through to a second guess.
                    MissingAppearanceState::OffElseNothing => subdict
                        .get(b"Off")
                        .filter(|v| !matches!(v, Object::Null))
                        .map_or(Appearance::StateUnresolved, |entry| {
                            classify_state_entry(graph, entry)
                        }),
                },
            }
        }
    }
}

/// Classify one appearance-subdictionary entry: a stream is paintable, a
/// dangling/non-stream entry is not (R43 named-not-painted).
fn classify_state_entry<G: ObjectGraph + ?Sized>(graph: &G, entry: &Object) -> Appearance {
    match graph.resolve(entry) {
        Object::Stream(_) => Appearance::Normal {
            stream_id: entry.as_reference(),
        },
        _ => Appearance::None,
    }
}

/// Read a `/Rect`-shaped array (four numbers, each possibly an indirect
/// reference per §7.3.10) and normalise it per §7.9.5.
///
/// Returns `None` when the value is not an array of four resolvable
/// numbers — a malformed `/Rect`, surfaced by the caller as a missing
/// placement target rather than repaired.
fn read_rect<G: ObjectGraph + ?Sized>(graph: &G, obj: &Object) -> Option<Rect> {
    let array = graph.resolve(obj).as_array()?;
    let nums: Vec<f64> = array
        .iter()
        .filter_map(|o| graph.resolve(o).as_number())
        .collect();
    match nums.as_slice() {
        &[x1, y1, x2, y2] => Some(Rect::from_corners(x1, y1, x2, y2)),
        _ => None,
    }
}

/// Whether the document's interactive form asserts its field appearances
/// are stale (`/AcroForm` `/NeedAppearances` true, ISO 32000-1 §12.7.2).
///
/// This is **document-scoped**, not per-page or per-annotation, so it is a
/// separate query rather than a field of [`Annotation`]. Pass 6.0 only
/// **counts** the documents that set it (R51): a document setting
/// `/NeedAppearances` true is asserting its widget appearances need
/// regenerating, and pdfce reports that condition but **never** silently
/// regenerates on load — doing so would rewrite objects the operator
/// never touched (a §5 minimal-diff violation dressed as helpfulness) and
/// pick appearances for them (fuzzy-never-sneaky). Regeneration is a
/// Pass 7 operator-requested action.
///
/// A widget whose `/AP` `/N` is present is still painted from it at
/// display time regardless of `/NeedAppearances`; this flag only governs
/// the stale-appearance *disclosure*, not per-widget painting.
#[must_use]
pub fn need_appearances<G: ObjectGraph + ?Sized>(graph: &G) -> bool {
    let Some(catalog) = graph.catalog_dict() else {
        return false;
    };
    matches!(
        catalog
            .get(b"AcroForm")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|af| af.get(b"NeedAppearances"))
            .map(|o| graph.resolve(o)),
        Some(Object::Boolean(true))
    )
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

    /// Assemble a classic-xref PDF from numbered object bodies (raw bytes,
    /// so stream objects can be built by the same helper). Object 1 is the
    /// catalog; the xref is generated from contiguous numbering.
    fn build_pdf(objects: &[(u32, Vec<u8>)]) -> Document {
        let mut buf = b"%PDF-1.7\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        for (num, body) in objects {
            offsets.push((*num, buf.len()));
            buf.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            buf.extend_from_slice(body);
            buf.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = buf.len();
        // Object numbers may be non-contiguous (annotation fixtures skip
        // ids for readability), so the xref spans 0..=max and any gap is a
        // free entry — mirroring `page_tree::tests::build_pdf`.
        let max_num = objects.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let size = max_num + 1;
        buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f\r\n").as_bytes());
        for num in 1..=max_num {
            match offsets.iter().find(|(n, _)| *n == num) {
                Some((_, off)) => {
                    buf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
                }
                None => buf.extend_from_slice(b"0000000000 65535 f\r\n"),
            }
        }
        buf.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
                .as_bytes(),
        );
        Document::from_bytes(buf).unwrap()
    }

    /// A stream object body.
    fn stream_object(dict_extra: &str, data: &[u8]) -> Vec<u8> {
        let mut out = format!("<< {dict_extra} /Length {} >>\nstream\n", data.len()).into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\nendstream");
        out
    }

    /// A one-page document whose single page carries the given raw
    /// `/Annots` array text and the given extra objects (numbered from 5).
    /// The page is object 3; its id is `ObjId::new(3, 0)`.
    fn doc_with_annots(annots: &str, extra: &[(u32, Vec<u8>)]) -> Document {
        let mut objects: Vec<(u32, Vec<u8>)> = vec![
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] \
                  /Resources << >> >>"
                    .to_vec(),
            ),
            (
                3,
                format!("<< /Type /Page /Parent 2 0 R /Annots {annots} >>").into_bytes(),
            ),
        ];
        objects.extend_from_slice(extra);
        build_pdf(&objects)
    }

    const PAGE_ID: ObjId = ObjId::new(3, 0);

    /// A form-XObject appearance stream body (a valid `/N` target).
    fn ap_stream(extra: &str) -> Vec<u8> {
        stream_object(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 20 20] {extra}"),
            b"0 0 0 rg 0 0 20 20 re f",
        )
    }

    #[test]
    fn flag_bit_values_match_table_165() {
        // Off-by-one here silently mis-reads every flag: Hidden is bit 2 =
        // value 2, NOT 1<<2.
        assert_eq!(AnnotFlags::INVISIBLE, 1);
        assert_eq!(AnnotFlags::HIDDEN, 2);
        assert_eq!(AnnotFlags::PRINT, 4);
        assert_eq!(AnnotFlags::NO_ZOOM, 8);
        assert_eq!(AnnotFlags::NO_ROTATE, 16);
        assert_eq!(AnnotFlags::NO_VIEW, 32);
        assert!(AnnotFlags(2).hidden() && AnnotFlags(2).suppressed_on_screen());
        assert!(AnnotFlags(32).no_view() && AnnotFlags(32).suppressed_on_screen());
        assert!(
            !AnnotFlags(4).suppressed_on_screen(),
            "Print is screen-neutral"
        );
    }

    #[test]
    fn absent_annots_yields_nothing() {
        let doc = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 10 10] \
                  /Resources << >> >>"
                    .to_vec(),
            ),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
        ]);
        assert!(page_annotations(&doc, PAGE_ID).is_empty());
    }

    #[test]
    fn stream_n_is_selected_and_rect_normalized() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Type /Annot /Subtype /Square /Rect [30 40 10 20] /AP << /N 6 0 R >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
            ],
        );
        let annots = page_annotations(&doc, PAGE_ID);
        assert_eq!(annots.len(), 1);
        let a = &annots[0];
        assert_eq!(a.subtype, b"Square");
        // §7.9.5: corners normalised min→max.
        let r = a.rect.unwrap();
        assert_eq!((r.llx, r.lly, r.urx, r.ury), (10.0, 20.0, 30.0, 40.0));
        assert_eq!(
            a.appearance,
            Appearance::Normal {
                stream_id: Some(ObjId::new(6, 0))
            }
        );
    }

    #[test]
    fn no_ap_is_none_by_subtype() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[(
                5,
                b"<< /Subtype /Circle /Rect [0 0 10 10] /IC [1 0 0] >>".to_vec(),
            )],
        );
        let a = &page_annotations(&doc, PAGE_ID)[0];
        // R43: an /IC-only Circle synthesises nothing — named-not-painted.
        assert_eq!(a.appearance, Appearance::None);
        assert_eq!(a.subtype_label(), "Circle");
    }

    #[test]
    fn as_selects_from_state_subdictionary() {
        // Checkbox: /N subdictionary keyed On/Off, /AS picks On.
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] /AS /On \
                      /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        let a = &page_annotations(&doc, PAGE_ID)[0];
        assert!(a.is_widget());
        assert_eq!(
            a.appearance,
            Appearance::Normal {
                stream_id: Some(ObjId::new(6, 0))
            }
        );
    }

    #[test]
    fn as_naming_absent_state_displays_nothing() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] /AS /Maybe \
                      /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        // §12.5.5 NOTE 3: state not found ⇒ display nothing.
        assert_eq!(
            page_annotations(&doc, PAGE_ID)[0].appearance,
            Appearance::StateUnresolved
        );
    }

    #[test]
    fn missing_as_multi_entry_displays_nothing_not_a_guess() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] \
                      /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        // No /AS against a multi-entry subdictionary: the RAG's negative
        // result — display nothing, never guess On/Off.
        assert_eq!(
            page_annotations(&doc, PAGE_ID)[0].appearance,
            Appearance::StateUnresolved
        );
    }

    #[test]
    fn missing_as_policy_offers_the_two_empirical_guesses_as_opt_ins() {
        // `AS-A1` (R169). The default above stays "paint nothing"; these
        // are the guesses the spec RAG explicitly forbids INSTALLING but
        // does not forbid OFFERING. `/On` is written first, so the
        // producer's first entry is object 6.
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] \
                      /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        assert_eq!(
            page_annotations_with(&doc, PAGE_ID, MissingAppearanceState::FirstEntry)[0].appearance,
            Appearance::Normal {
                stream_id: Some(ObjId::new(6, 0))
            },
            "`first_entry` must take the FILE's first key, not an \
             alphabetical one"
        );
        assert_eq!(
            page_annotations_with(&doc, PAGE_ID, MissingAppearanceState::OffElseNothing)[0]
                .appearance,
            Appearance::Normal {
                stream_id: Some(ObjId::new(7, 0))
            }
        );
        assert_eq!(
            page_annotations_with(&doc, PAGE_ID, MissingAppearanceState::PaintNothing)[0]
                .appearance,
            Appearance::StateUnresolved,
            "the default must be unchanged by the setting existing"
        );
        assert_eq!(
            page_annotations(&doc, PAGE_ID)[0].appearance,
            page_annotations_with(&doc, PAGE_ID, MissingAppearanceState::default())[0].appearance,
            "the convenience wrapper must be the default policy"
        );
    }

    #[test]
    fn off_else_nothing_falls_back_rather_than_guessing_twice() {
        // The guess is specifically "/Off", not "some entry". A
        // subdictionary with no /Off must go back to painting nothing —
        // falling through to the first entry would be a second, unnamed
        // guess stacked on the operator's chosen one.
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] \
                      /AP << /N << /Yes 6 0 R /No 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        assert_eq!(
            page_annotations_with(&doc, PAGE_ID, MissingAppearanceState::OffElseNothing)[0]
                .appearance,
            Appearance::StateUnresolved
        );
    }

    #[test]
    fn the_missing_as_policy_cannot_reach_a_well_formed_annotation() {
        // Blast-radius containment. The setting governs ONE malformed
        // configuration; a present /AS and a single-entry subdictionary
        // are both spec-determined and must be identical under all three
        // values, or the knob is wider than its documentation claims.
        let with_as = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] /AS /On \
                      /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
                (7, ap_stream("")),
            ],
        );
        let single = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] \
                      /AP << /N << /Only 6 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
            ],
        );
        for policy in [
            MissingAppearanceState::PaintNothing,
            MissingAppearanceState::FirstEntry,
            MissingAppearanceState::OffElseNothing,
        ] {
            assert_eq!(
                page_annotations_with(&with_as, PAGE_ID, policy)[0].appearance,
                Appearance::Normal {
                    stream_id: Some(ObjId::new(6, 0))
                },
                "{policy:?} disturbed a present /AS"
            );
            assert_eq!(
                page_annotations_with(&single, PAGE_ID, policy)[0].appearance,
                Appearance::Normal {
                    stream_id: Some(ObjId::new(6, 0))
                },
                "{policy:?} disturbed a single-entry subdictionary"
            );
        }
    }

    #[test]
    fn missing_as_single_entry_is_unambiguous() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Widget /Rect [0 0 10 10] \
                      /AP << /N << /Only 6 0 R >> >> >>"
                        .to_vec(),
                ),
                (6, ap_stream("")),
            ],
        );
        // One entry, no /AS: there are no alternatives to guess among, so
        // painting the sole appearance is unambiguous (not the forbidden
        // multi-entry guess).
        assert_eq!(
            page_annotations(&doc, PAGE_ID)[0].appearance,
            Appearance::Normal {
                stream_id: Some(ObjId::new(6, 0))
            }
        );
    }

    #[test]
    fn popup_is_flagged_structurally() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[(
                5,
                b"<< /Subtype /Popup /Rect [0 0 10 10] /Open true >>".to_vec(),
            )],
        );
        let a = &page_annotations(&doc, PAGE_ID)[0];
        assert!(
            a.is_popup,
            "/Popup must be flagged for the never-paint rule"
        );
    }

    #[test]
    fn non_dictionary_annots_entries_are_skipped() {
        // A dangling reference (null) and a bare number are not
        // annotations; the real one survives.
        let doc = doc_with_annots(
            "[99 0 R 42 5 0 R]",
            &[(5, b"<< /Subtype /Link /Rect [0 0 10 10] >>".to_vec())],
        );
        let annots = page_annotations(&doc, PAGE_ID);
        assert_eq!(annots.len(), 1);
        assert_eq!(annots[0].subtype, b"Link");
    }

    #[test]
    fn flags_decoded_from_f_integer() {
        let doc = doc_with_annots(
            "[5 0 R]",
            &[(
                5,
                // Hidden|Print = 2|4 = 6.
                b"<< /Subtype /Text /Rect [0 0 10 10] /F 6 >>".to_vec(),
            )],
        );
        let a = &page_annotations(&doc, PAGE_ID)[0];
        assert!(a.flags.hidden());
        assert!(a.flags.print());
        assert!(a.flags.suppressed_on_screen());
    }

    #[test]
    fn need_appearances_reads_acroform() {
        let doc = build_pdf(&[
            (
                1,
                b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /NeedAppearances true >> >>".to_vec(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 10 10] \
                  /Resources << >> >>"
                    .to_vec(),
            ),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
        ]);
        assert!(need_appearances(&doc));
    }

    /// `/Contents`, `/T` and `/M` are modelled, and each is `None` when absent
    /// rather than an empty string — "no note" and "an empty note" are
    /// different facts and a UI captions them differently.
    #[test]
    fn contents_title_and_mod_date_are_modelled_and_absent_means_none() {
        let doc = doc_with_annots(
            "[4 0 R 5 0 R]",
            &[
                (
                    4,
                    b"<< /Type /Annot /Subtype /Square /Rect [0 0 10 10]                        /Contents (Check this dimension) /T (Ken) /M (D:20260806120000Z) >>"
                        .to_vec(),
                ),
                // No /Contents, no /T, no /M — the common case for a shape
                // pdfce itself authored (Pass 6.1 sets none of them).
                (
                    5,
                    b"<< /Type /Annot /Subtype /Circle /Rect [0 0 10 10] >>".to_vec(),
                ),
            ],
        );
        let annots = page_annotations(&doc, PAGE_ID);
        assert_eq!(annots.len(), 2);

        assert_eq!(annots[0].contents.as_deref(), Some("Check this dimension"));
        assert_eq!(annots[0].title.as_deref(), Some("Ken"));
        assert_eq!(annots[0].mod_date.as_deref(), Some("D:20260806120000Z"));

        assert_eq!(annots[1].contents, None, "absent /Contents is None");
        assert_eq!(annots[1].title, None, "absent /T is None");
        assert_eq!(annots[1].mod_date, None, "absent /M is None");
    }

    /// A UTF-16BE `/Contents` decodes through the SAME §7.9.2 decoder every
    /// other text-string consumer uses.
    ///
    /// Non-vacuous by construction: the assertion is on a non-ASCII character
    /// that a naive byte-to-char conversion would mangle, so a second private
    /// lossy decoder could not pass this.
    #[test]
    fn a_utf16_contents_decodes_rather_than_mojibake() {
        // UTF-16BE BOM + "Ré" — 0xFEFF, 'R', 0x00E9.
        let doc = doc_with_annots(
            "[4 0 R]",
            &[(
                4,
                b"<< /Type /Annot /Subtype /Text /Rect [0 0 10 10]                    /Contents <FEFF005200E9> >>"
                    .to_vec(),
            )],
        );
        let annots = page_annotations(&doc, PAGE_ID);
        assert_eq!(annots[0].contents.as_deref(), Some("Ré"));
    }

    /// `/M` is stored VERBATIM, including a value that is not a §7.9.4 date.
    ///
    /// §12.5.2 gives its type as "date or text string" and requires a reader
    /// to "accept and display a string in any format" — so a parser that
    /// rejected or normalised this would violate the standard, and this test
    /// is what stops one being added later.
    #[test]
    fn a_non_date_mod_date_is_kept_verbatim_because_the_standard_demands_it() {
        let doc = doc_with_annots(
            "[4 0 R]",
            &[(
                4,
                b"<< /Type /Annot /Subtype /Square /Rect [0 0 10 10]                    /M (last Tuesday) >>"
                    .to_vec(),
            )],
        );
        let annots = page_annotations(&doc, PAGE_ID);
        assert_eq!(annots[0].mod_date.as_deref(), Some("last Tuesday"));
    }
    // ---- Table 99 `/P`, the OCMD visibility policy ----

    /// A document with one OCMD over two OCGs, `off` naming which of the
    /// two the default configuration hides, and `policy` the `/P` value
    /// (empty for "absent", which must behave as `AnyOn`).
    fn ocmd_is_hidden(policy: &str, first_off: bool, second_off: bool) -> bool {
        let p = if policy.is_empty() {
            String::new()
        } else {
            format!("/P /{policy}")
        };
        let mut off = String::from("/OFF [");
        if first_off {
            off.push_str("5 0 R ");
        }
        if second_off {
            off.push_str("6 0 R");
        }
        off.push(']');
        let objects: Vec<(u32, Vec<u8>)> = vec![
            (
                1,
                format!(
                    "<< /Type /Catalog /Pages 2 0 R /OCProperties \
                     << /OCGs [5 0 R 6 0 R] /D << {off} >> >> >>"
                )
                .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 10 10] >>".to_vec(),
            ),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
            (
                4,
                format!("<< /Type /OCMD /OCGs [5 0 R 6 0 R] {p} >>").into_bytes(),
            ),
            (5, b"<< /Type /OCG /Name (A) >>".to_vec()),
            (6, b"<< /Type /OCG /Name (B) >>".to_vec()),
        ];
        let doc = build_pdf(&objects);
        let graph = doc.view();
        let off_set = optional_content_default_off(&graph);
        oc_is_hidden(&graph, ObjId::new(4, 0), &off_set)
    }

    /// ★ **`/P /AllOff` with every member off is VISIBLE.**
    ///
    /// The case that was inverted. Until `/P` was read, every OCMD was
    /// evaluated as `AnyOn`, so "show this when the layers are off" —
    /// which is the entire purpose of `AllOff` — produced exactly the
    /// opposite answer. A "no layers shown" placeholder would have been
    /// hidden precisely when it was meant to appear.
    #[test]
    fn an_all_off_membership_is_visible_when_every_member_is_off() {
        assert!(
            !ocmd_is_hidden("AllOff", true, true),
            "Table 99 /P /AllOff: visible iff all members are OFF"
        );
        assert!(
            ocmd_is_hidden("AllOff", true, false),
            "one member still on means not all off, so hidden"
        );
    }

    /// The default, and an absent `/P`, are both `AnyOn` — the behaviour
    /// every OCMD had before `/P` was read, pinned so implementing the
    /// other three policies cannot have moved it.
    #[test]
    fn an_absent_p_behaves_as_any_on() {
        for policy in ["", "AnyOn"] {
            assert!(ocmd_is_hidden(policy, true, true), "{policy:?}: none on");
            assert!(!ocmd_is_hidden(policy, true, false), "{policy:?}: one on");
            assert!(!ocmd_is_hidden(policy, false, false), "{policy:?}: both on");
        }
    }

    /// `AllOn` and `AnyOff` are each other's inverse over the same two
    /// groups, which is the property that catches the likeliest mistake:
    /// deriving the hidden-test by hand and swapping them.
    #[test]
    fn all_on_and_any_off_are_complementary() {
        for (a, b) in [(false, false), (true, false), (false, true), (true, true)] {
            assert_ne!(
                ocmd_is_hidden("AllOn", a, b),
                ocmd_is_hidden("AnyOff", a, b),
                "AllOn and AnyOff must disagree for off=({a}, {b})"
            );
        }
    }

    /// An unrecognised `/P` falls back to the default rather than
    /// inventing a fifth policy — Table 99 names exactly four.
    #[test]
    fn an_unknown_p_falls_back_to_the_default() {
        assert_eq!(
            ocmd_is_hidden("Sometimes", true, false),
            ocmd_is_hidden("AnyOn", true, false)
        );
    }

    /// An OCMD with no `/OCGs` is visible: there is nothing to test, and
    /// hiding content because a membership dictionary was empty would
    /// remove marks no clause asks to remove.
    #[test]
    fn an_empty_membership_is_visible() {
        let objects: Vec<(u32, Vec<u8>)> = vec![
            (
                1,
                b"<< /Type /Catalog /Pages 2 0 R /OCProperties \
                  << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >> >>"
                    .to_vec(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 10 10] >>".to_vec(),
            ),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
            (4, b"<< /Type /OCMD /P /AllOn >>".to_vec()),
            (5, b"<< /Type /OCG /Name (A) >>".to_vec()),
        ];
        let doc = build_pdf(&objects);
        let graph = doc.view();
        let off_set = optional_content_default_off(&graph);
        assert!(!oc_is_hidden(&graph, ObjId::new(4, 0), &off_set));
    }
}
