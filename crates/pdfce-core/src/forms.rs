//! # Interactive-form (AcroForm) field model (ISO 32000-1 §12.7)
//!
//! The **read/model half** of Pass 7 (docs/decisions/008, candidate B). It
//! sits on the same *"core decodes and models, render paints / edit
//! mutates"* axis (R26) as [`crate::annot`]: this module walks a
//! document's `/AcroForm` field tree, resolves each terminal field's
//! inherited attributes, constructs its fully-qualified name, decodes its
//! `/Ff` flags and per-type `/V`, and models the field-vs-widget MERGE —
//! but it **never** paints, mutates, or generates an appearance. Filling
//! and flattening are [`crate::edit`] commands that build on this model;
//! painting a widget's appearance is [`crate::annot`] + `pdfce-render`
//! (R49 — *a widget is an annotation first, and there is exactly one
//! appearance pipeline*).
//!
//! ## Scope — read-only recognition (Pass 7 P0)
//!
//! This module produces no bytes. It recognises and round-trips embedded
//! form JavaScript (`/AA`, `/CO`) as **presence flags** and never executes
//! it (decision 008 §5.1, spec NF4 in `iso32000__s__12.7.2.md`: ISO
//! 32000-1 defines no JavaScript semantics, only the carrier — so a reader
//! that recognises-and-discloses without executing is making a deliberate,
//! spec-grounded scope decision, not skipping a `shall`). XFA is
//! **detected and counted, never parsed** (0.08 % of the corpus; Backlog).
//!
//! ## Spec sources (PDF-spec RAG, ISO 32000-1:2008)
//!
//! - `iso32000__s__12.7.2.md` — Table 218 (the `/AcroForm` dict: `/Fields`
//!   roots, `/NeedAppearances`, `/SigFlags` Table 219, `/CO`, `/DR`, `/DA`,
//!   `/Q`, `/XFA`), the XFA two-shape detection, and the document-level
//!   behaviour negatives (NF1 `/NeedAppearances` is a *may*, NF4 JS
//!   never-execute).
//! - `iso32000__s__12.7.3.md` — Table 220 (entries common to all field
//!   dictionaries: `/FT`, `/Parent`, `/Kids`, `/T`, `/TU`, `/TM`, `/Ff`,
//!   `/V`, `/DV`, `/AA`), Table 221 (the three universal flags:
//!   `ReadOnly` 1, `Required` 2, `NoExport` 4), the §12.7.3.2 fully-
//!   qualified-name construction (dotted `/T` path), and **the field-vs-
//!   widget MERGE** (Shape A single merged dict / Shape B field + `/Kids`
//!   widget array) that is R49's structural basis.
//! - `iso32000__s__12.7.4.md` — Tables 226/228/230 (the per-type `/Ff`
//!   flag bits, VERBATIM values), per-type `/V` forms (button = name,
//!   text = string, choice = string-or-array, signature = dict, pushbutton
//!   = none), Table 227 (`/Opt` for buttons), Tables 229/231 (`/MaxLen`,
//!   choice `/Opt`/`/TI`/`/I`).
//!
//! ## The MERGE, in one sentence (the load-bearing fact)
//!
//! A terminal field's on-page appearance is a **widget annotation**
//! (`/Subtype /Widget`). When the field has exactly one widget, the field
//! dictionary and the widget dictionary are **one indirect object** with
//! `/Kids` omitted (Shape A — ~88 % of widgets); when it has several
//! widgets (a radio set, or a field repeated across pages), the field
//! dictionary is separate and its `/Kids` array holds the widget
//! annotations, each with its own `/Rect`/`/AP`/`/AS` and a `/Parent` back
//! at the field (Shape B). Field attributes (`/FT`, `/V`, `/DV`, `/Ff`,
//! `/DA`, `/Q`) inherit **down the field tree via `/Parent`** — never the
//! page tree.

use std::collections::HashSet;

use crate::graph::ObjectGraph;
use crate::object::{Dict, ObjId, Object};
use crate::page_tree::Rect;
use crate::vartext::Quadding;

/// Maximum terminal fields modelled from one document's field tree
/// (pdfce policy, ARCHITECTURE.md §10.1 adversarial-input posture).
///
/// **No spec limit exists to inherit** — Annex C (informative) lists no
/// form-field bound and PDF/A §6.1.12 forbids a reader imposing Annex C's
/// limits, so this is pure pdfce policy and must clear any conformant
/// corpus. The organic census (decision 008 §1.2) put the busiest
/// form-bearing file near ~63 fields on average; a document past this
/// bound is beyond any measured document and the excess is dropped rather
/// than allowed to pin unbounded allocation. Sized far above the corpus
/// maximum so the veraPDF §6.1.12 suite reports comfortable headroom, in
/// the same spirit as [`crate::annot::MAX_ANNOTS_PER_PAGE`].
pub const MAX_FORM_FIELDS: usize = 500_000;

/// Maximum depth the `/Kids` field-tree walk descends before refusing to
/// recurse further (pdfce policy). Bounds a hostile deeply-nested — or
/// `/Kids`-cyclic — field tree. A cycle is *also* caught by the visited-id
/// set, but a depth cap is the cheaper first guard and mirrors
/// [`crate::document::MAX_RESOLVE_DEPTH`]'s role for reference chains.
pub const MAX_FIELD_TREE_DEPTH: usize = 64;

// ---------------------------------------------------------------------------
// /Ff flag bits — VERBATIM from Tables 221/226/228/230 (bit N = 2^(N-1))
// ---------------------------------------------------------------------------

/// Decoded `/Ff` field flags (ISO 32000-1 §12.7.3.1, Tables 221/226/228/230).
///
/// Bit positions are numbered from the low-order bit as **bit 1**, so bit
/// *N* has integer value `2^(N-1)` (§12.7.3.1 verbatim). Getting this off
/// by one silently mis-reads every flag, so the constants are named against
/// their tables and pinned by a test. Default `/Ff` is `0` (Table 220).
///
/// The type-specific bits (button/text/choice) share the same word and are
/// only meaningful for the matching `/FT`; a decoder consults them **after**
/// resolving the field type (e.g. bit 23 is `DoNotSpellCheck` for both text
/// and choice, and bit 26 is `RichText` for text but `RadiosInUnison` for a
/// button). Accessors here are therefore raw bit tests; the type dispatch
/// lives in [`Field`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldFlags(pub u32);

impl FieldFlags {
    // -- Table 221: common to all field types ---------------------------
    /// Bit 1 (value 1) — `ReadOnly`: the user may not change the value; the
    /// widgets do not interact. The one universal flag with an edit-time
    /// consequence (pdfce refuses to fill a read-only field).
    pub const READ_ONLY: u32 = 1 << 0;
    /// Bit 2 (value 2) — `Required`: the field shall have a value when a
    /// submit-form action exports it. Submit is out of scope; modelled.
    pub const REQUIRED: u32 = 1 << 1;
    /// Bit 3 (value 4) — `NoExport`: the field shall not be exported by a
    /// submit-form action. Submit is out of scope; modelled.
    pub const NO_EXPORT: u32 = 1 << 2;

    // -- Table 226: button fields ---------------------------------------
    /// Bit 15 (value 16384) — `NoToggleToOff` (radio only): exactly one
    /// button is always selected; clicking the selected one is inert.
    pub const NO_TOGGLE_TO_OFF: u32 = 1 << 14;
    /// Bit 16 (value 32768) — `Radio`: the field is a set of radio buttons
    /// (else a check box). Valid only when `Pushbutton` is clear.
    pub const RADIO: u32 = 1 << 15;
    /// Bit 17 (value 65536) — `Pushbutton`: a button that retains no value.
    pub const PUSHBUTTON: u32 = 1 << 16;
    /// Bit 26 (value 33554432) — `RadiosInUnison`: radio kids that share an
    /// on-state name toggle together (else mutually exclusive).
    pub const RADIOS_IN_UNISON: u32 = 1 << 25;

    // -- Table 228: text fields -----------------------------------------
    /// Bit 13 (value 4096) — `Multiline`: the field may hold multiple lines.
    pub const MULTILINE: u32 = 1 << 12;
    /// Bit 14 (value 8192) — `Password`: entry is echoed unreadably, and a
    /// reader *should never store* the value (Table 228 NOTE).
    pub const PASSWORD: u32 = 1 << 13;
    /// Bit 21 (value 1048576) — `FileSelect`: the text is a file pathname.
    pub const FILE_SELECT: u32 = 1 << 20;
    /// Bit 23 (value 4194304) — `DoNotSpellCheck`, on text **and** choice
    /// fields, with the **same meaning but a different precondition** on
    /// each. That asymmetry is the whole reason this note exists.
    ///
    /// Table 228 (`/Tx`) states it unconditionally. Table 230 (`/Ch`) gates
    /// it on `Combo` **and** `Edit` — spell-checking is only a question for
    /// a choice field the operator can type into, and a set bit on a
    /// list-box or a non-editable combo is meaningless rather than
    /// meaningful.
    ///
    /// So the bit position and the meaning are shared, and the
    /// **validation rule is not**. A decoder that resolves only the type
    /// gets the right answer for "what does this bit mean"; one that also
    /// wants "is this bit legitimate here" must additionally check
    /// `COMBO | EDIT` on `/Ch`. Unlike bit 26 — the one genuinely
    /// overloaded position, which `Field::is_rich_text` exists to make
    /// undecodable-wrong (`587e520`) — this cannot produce a wrong meaning,
    /// only a missed validation, which is why it is documented here rather
    /// than wrapped in an accessor.
    ///
    /// Nothing in pdfce consumes this flag yet. The note is placed at the
    /// definition so the precondition is present at the moment something
    /// first does, instead of being re-derived from the spec.
    pub const DO_NOT_SPELL_CHECK: u32 = 1 << 22;
    /// Bit 24 (value 8388608) — `DoNotScroll`: the field does not scroll.
    pub const DO_NOT_SCROLL: u32 = 1 << 23;
    /// Bit 25 (value 16777216) — `Comb`: the field is divided into `MaxLen`
    /// equally-spaced combs. Valid only with `/MaxLen` present and
    /// `Multiline`/`Password`/`FileSelect` all clear.
    pub const COMB: u32 = 1 << 24;
    /// Bit 26 (value 33554432) — `RichText` (text): the value is a rich-text
    /// string. **Shares its value with `RadiosInUnison`** (button) — decode
    /// against the resolved `/FT`.
    pub const RICH_TEXT: u32 = 1 << 25;

    // -- Table 230: choice fields ---------------------------------------
    /// Bit 18 (value 131072) — `Combo`: a combo box (else a list box).
    pub const COMBO: u32 = 1 << 17;
    /// Bit 19 (value 262144) — `Edit`: the combo box has an editable text
    /// box. Used only with `Combo`.
    pub const EDIT: u32 = 1 << 18;
    /// Bit 20 (value 524288) — `Sort`: option items are sorted (a **writer**
    /// flag; readers display `/Opt` order as-is).
    pub const SORT: u32 = 1 << 19;
    /// Bit 22 (value 2097152) — `MultiSelect`: more than one option may be
    /// selected. Governs whether `/V` is a string or an array.
    pub const MULTI_SELECT: u32 = 1 << 21;
    /// Bit 27 (value 67108864) — `CommitOnSelChange`: commit a selection
    /// immediately rather than on leave-field.
    pub const COMMIT_ON_SEL_CHANGE: u32 = 1 << 26;

    /// Whether `ReadOnly` (bit 1) is set — the only common flag that gates
    /// editing.
    #[must_use]
    pub const fn read_only(self) -> bool {
        self.0 & Self::READ_ONLY != 0
    }

    /// Whether `Required` (bit 2) is set.
    #[must_use]
    pub const fn required(self) -> bool {
        self.0 & Self::REQUIRED != 0
    }

    /// Whether `NoExport` (bit 3) is set.
    #[must_use]
    pub const fn no_export(self) -> bool {
        self.0 & Self::NO_EXPORT != 0
    }

    /// Test an arbitrary flag bit (a type-specific bit consulted after the
    /// `/FT` is known).
    #[must_use]
    pub const fn has(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
}

/// The four field types (ISO 32000-1 §12.7.4.1, `/FT`).
///
/// A **non-terminal** field may carry `/FT` purely to provide an
/// inheritable value to descendant terminals, but "does not logically have
/// a type of its own" (§12.7.3.1) — so this type is attached only to
/// terminal [`Field`]s, resolved from the field's own `/FT` or the nearest
/// ancestor's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// `/Btn` — pushbutton, check box, or radio button (§12.7.4.2). The
    /// specific control is refined by [`ButtonKind`] from `/Ff` bits 16/17.
    Button,
    /// `/Tx` — a text field (§12.7.4.3). Variable-text appearance.
    Text,
    /// `/Ch` — a choice field: list box or combo box (§12.7.4.4).
    Choice,
    /// `/Sig` — a signature field (§12.7.4.5). Recognition only in Pass 7.
    Signature,
}

impl FieldType {
    /// The `/FT` name bytes.
    #[must_use]
    pub const fn as_ft_name(self) -> &'static [u8] {
        match self {
            Self::Button => b"Btn",
            Self::Text => b"Tx",
            Self::Choice => b"Ch",
            Self::Signature => b"Sig",
        }
    }

    /// Parse an `/FT` name.
    ///
    /// `pub(crate)` rather than private so the write-side resolver
    /// (`forms_author`) classifies a type with the SAME function the reader
    /// does. Two classifiers that could drift would mean a node the
    /// projection lists as a text field and the authoring path is willing to
    /// merge a check box into. Not `pub`: the public surface stays as
    /// shipped (rule 10) — an outside caller has `Field::field_type` already.
    #[must_use]
    pub(crate) fn from_name(name: &[u8]) -> Option<Self> {
        match name {
            b"Btn" => Some(Self::Button),
            b"Tx" => Some(Self::Text),
            b"Ch" => Some(Self::Choice),
            b"Sig" => Some(Self::Signature),
            _ => None,
        }
    }
}

/// Which kind of button a `/Btn` field is (ISO 32000-1 §12.7.4.2.1, `/Ff`
/// bits 16 `Radio` and 17 `Pushbutton`).
///
/// Pushbutton set ⇒ [`ButtonKind::Push`]; Radio set (Pushbutton clear) ⇒
/// [`ButtonKind::Radio`]; both clear ⇒ [`ButtonKind::Check`]. The
/// combination Radio+Pushbutton is malformed (Table 226: `Radio` "may be
/// set only if the `Pushbutton` flag is clear") — pdfce resolves it to
/// `Push`, matching the "Pushbutton wins" precedence, and does not repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// A pushbutton — retains no permanent value (no `/V`).
    Push,
    /// A check box — two states, `/V` a name (`/Yes`/`/Off`, or an
    /// `/Opt`-positional name), selected by `/AS`.
    Check,
    /// A radio-button set — kids share the field; `/V` is the on-state name
    /// of the selected kid (default `/Off`).
    Radio,
}

impl ButtonKind {
    /// Classify a button from its resolved `/Ff`.
    ///
    /// `pub(crate)` for the same reason as [`FieldType::from_name`]: the
    /// resolver must decide "is this the same kind of button?" exactly as the
    /// reader decides "what kind of button is this?".
    #[must_use]
    pub(crate) fn from_flags(flags: FieldFlags) -> Self {
        if flags.has(FieldFlags::PUSHBUTTON) {
            Self::Push
        } else if flags.has(FieldFlags::RADIO) {
            Self::Radio
        } else {
            Self::Check
        }
    }
}

/// A field's value (`/V`) or default value (`/DV`), typed per `/FT`
/// (ISO 32000-1 §12.7.4).
///
/// The value's COS type varies by field type, so a single stringly-typed
/// model would lose the distinction between a checkbox's `/Yes` **name**
/// and a text field's `(Yes)` **string** — which fill and export both
/// depend on. Raw bytes are kept (never re-encoded here); text decoding is
/// offered on demand via [`FieldValue::display_text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// No `/V` (absent, or present-as-null which §7.3.7 collapses to
    /// absent). A choice field's documented default is `null` ⇒ this.
    Absent,
    /// A button/checkbox/radio on-state **name** (`/Yes`, `/Off`, `/1`, …).
    /// Raw name bytes (no leading `/`).
    Name(Vec<u8>),
    /// A text field value (§12.7.4.3) — a raw PDF text string's decoded
    /// bytes (§7.9.2 interpretation is deferred to [`FieldValue::display_text`]).
    Text(Vec<u8>),
    /// A choice selection (§12.7.4.4): one display string (single-select) or
    /// several (`MultiSelect`). Each is a raw text-string's bytes.
    Choice(Vec<Vec<u8>>),
    /// A signature dictionary is present as the field's `/V` (§12.7.4.5).
    /// Recognition only — the signature is validated by [`crate::signature`],
    /// not created or verified here.
    Signature,
}

impl FieldValue {
    /// Whether a value is present (anything but [`FieldValue::Absent`]).
    #[must_use]
    pub const fn is_present(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// A best-effort human-readable rendering of the value for display and
    /// diagnostics.
    ///
    /// Names render as their raw bytes lossily; text and choice values are
    /// decoded through the §7.9.2 / Annex D.3 text-string decoder
    /// ([`crate::edit::decode_text_string`]); a signature renders as
    /// `<signature>`. This is a *display* helper — round-trip and export use
    /// the raw bytes, never this string.
    #[must_use]
    pub fn display_text(&self) -> String {
        match self {
            Self::Absent => String::new(),
            Self::Name(b) => String::from_utf8_lossy(b).into_owned(),
            Self::Text(b) => crate::edit::decode_text_string(b).text,
            Self::Choice(items) => items
                .iter()
                .map(|b| crate::edit::decode_text_string(b).text)
                .collect::<Vec<_>>()
                .join(", "),
            Self::Signature => "<signature>".to_owned(),
        }
    }
}

/// One `/Opt` choice option (ISO 32000-1 §12.7.4.4, Table 231): the export
/// value a submit sends and the display value the user sees.
///
/// A single-string `/Opt` element sets both to the same bytes; a
/// two-element `[export display]` element sets them separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    /// The export value (raw text-string bytes).
    pub export: Vec<u8>,
    /// The display value shown in the list/combo (raw text-string bytes).
    pub display: Vec<u8>,
}

/// One widget annotation of a field (ISO 32000-1 §12.5.6.19), modelled for
/// the forms layer.
///
/// For a **merged** (Shape A) field this widget's [`Widget::id`] equals the
/// field object's id — the field dict *is* the widget. For a **Shape B**
/// field each kid widget is a separate object with its own `/Rect`, `/AP`,
/// `/AS`, and a `/Parent` back at the field.
#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    /// The widget object's identity.
    pub id: ObjId,
    /// `/Rect` in default user space, normalised (§7.9.5). `None` when
    /// absent/malformed — for a signature field a **zero-area** `/Rect` is
    /// intentional invisibility (§12.7.4.5), which normalises to a `Rect`
    /// of zero width/height rather than `None`.
    pub rect: Option<Rect>,
    /// `/AS` — the selected appearance state (a name), for buttons. `None`
    /// for text/choice widgets (no state selection).
    pub appearance_state: Option<Vec<u8>>,
    /// The button on-state names this widget's `/AP` `/N` subdictionary
    /// defines, excluding `Off` (§12.7.4.2.3). Empty for a non-button widget
    /// or one whose `/AP` `/N` is a single stream. Used by checkbox/radio
    /// fill to know which state to select.
    pub on_states: Vec<Vec<u8>>,
    /// `/P` — the page this widget appears on, if present (§12.5.6.19).
    pub page: Option<ObjId>,
    /// `/MK` `/CA` — the widget's **normal caption** (Table 189), as raw
    /// §7.9.2 text-string bytes. `None` when absent.
    ///
    /// # Why only this one key out of `/MK`
    ///
    /// `/MK` (the appearance-characteristics dictionary, Table 189) also
    /// carries `/BC`, `/BG`, `/R`, `/RC`, `/AC`, `/I`, `/RI`, `/IX`, `/IF`
    /// and `/TP`. Every one of those is **cosmetic**, and R43 is the standing
    /// rule that pdfce does not synthesise appearance from `/MK` at display
    /// time — it paints the baked `/AP`. Modelling them would add read-path
    /// surface that nothing consumes.
    ///
    /// `/CA` is different because on a **push button** it is not cosmetic: it
    /// is the button's only human-readable identity. A push button has no
    /// `/V` at all (§12.7.4.2.2), so `/CA` is the sole thing distinguishing
    /// *Submit* from *Reset* to anyone reading the field list — and the only
    /// property `--defaults-from` could copy from a push-button template.
    /// Modelling it is what lets a caption be listed, copied and compared
    /// rather than being visible only as pixels inside an appearance stream.
    ///
    /// Read on every widget rather than only on buttons: `/MK` is legal on
    /// any widget annotation, and a type-gated reader would mean the model
    /// silently disagreeing with the file for the non-button case.
    pub caption: Option<Vec<u8>>,
    /// Whether the widget carries a usable normal appearance (`/AP` `/N`
    /// resolving to a stream, or a state subdictionary). The measured demand
    /// signal for regeneration.
    pub has_normal_appearance: bool,
    /// Whether this widget IS the field dictionary (Shape A merge).
    pub merged: bool,
}

/// One terminal form field, fully resolved (ISO 32000-1 §12.7.3).
///
/// A *terminal* field is one with no field children — its `/Kids` (if any)
/// are widget annotations, not fields. Every attribute here is resolved:
/// `/FT`, `/Ff`, `/V`, `/DV`, `/DA`, `/Q` are taken from the field's own
/// dictionary or inherited from the nearest ancestor that sets them (§12.7.3.1),
/// and the [`Field::fully_qualified_name`] is the dotted `/T` path (§12.7.3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The field dictionary's object identity — the addressable object a
    /// fill edit writes `/V` onto. For a merged (Shape A) field this is also
    /// the sole widget's id.
    pub id: ObjId,
    /// The fully-qualified field name: the field's `/T` appended to its
    /// ancestors' names, separated by `.` (§12.7.3.2), each segment decoded
    /// as a §7.9.2 text string. Empty when no `/T` appears anywhere on the
    /// path (a `/T`-less terminal shares its parent's FQN — see
    /// [`Field::shares_parent_name`]).
    pub fully_qualified_name: String,
    /// This field's own partial name `/T` (raw bytes), or `None`.
    pub partial_name: Option<Vec<u8>>,
    /// `/TU` — the alternate (UI/accessibility) field name (raw bytes).
    pub alternate_name: Option<Vec<u8>>,
    /// `/TM` — the export mapping name (raw bytes), distinct from `/T`.
    pub mapping_name: Option<Vec<u8>>,
    /// `/RV` — the **rich text value** (§12.7.3.4, PDF 1.5): an XHTML/CSS2
    /// subset document carrying the field's formatting, raw bytes.
    ///
    /// # Why this is modelled even though pdfce cannot yet author it
    ///
    /// Reading it is what makes an EXPORT non-destructive. FDF Table 246 and
    /// XFDF's `<value-richtext>` both carry `/RV` precisely so formatting
    /// travels beside the plain value; pdfce's exporter dropped it, so a
    /// styled field round-tripped out and came back unstyled.
    ///
    /// # The `/V` relationship, which is not symmetrical
    ///
    /// §12.7.3.4 says the flat text **should** also be preserved in `/V` — a
    /// `should`, so `/RV` without `/V` is conforming but under-serving. The
    /// reverse violates a `shall` (Table 228 bit 26). And §12.7.3.3 makes
    /// `/DS` + `/RV` the inputs to appearance generation, so a fresh `/V`
    /// beside a stale `/RV` renders the OLD text in any conforming reader —
    /// which is why `fill_text_field` refuses a rich-text field outright
    /// rather than writing half the pair.
    ///
    /// `None` on any field without the entry, which is nearly all of them.
    pub rich_value: Option<Vec<u8>>,
    /// The resolved field type, or `None` for a terminal field with no
    /// resolvable `/FT` (a malformed field — surfaced, not repaired).
    pub field_type: Option<FieldType>,
    /// For a `/Btn` field, which kind of button (from `/Ff` bits 16/17).
    pub button_kind: Option<ButtonKind>,
    /// The resolved `/Ff` flags (own or inherited; default 0).
    pub flags: FieldFlags,
    /// The resolved field value `/V`, typed per [`Field::field_type`].
    pub value: FieldValue,
    /// The resolved default value `/DV` (the reset-form target).
    pub default_value: FieldValue,
    /// The resolved `/DA` default-appearance string (raw bytes), or `None`
    /// (a variable-text field then falls back to the AcroForm `/DA`).
    pub default_appearance: Option<Vec<u8>>,
    /// The resolved `/Q` quadding for variable text (default left).
    pub quadding: Quadding,
    /// `/MaxLen` — the maximum text length / comb count (text fields).
    pub max_len: Option<i64>,
    /// `/Opt` — the choice options (export + display), in `/Opt` order
    /// (§12.7.4.4; readers never re-sort even under the `Sort` flag).
    pub options: Vec<ChoiceOption>,
    /// `/TI` — a list box's top (first-visible) index. Default 0.
    pub top_index: i64,
    /// `/I` — the selected indices for a multi-select choice (`/V` wins on
    /// conflict, §12.7.4.4).
    pub selected_indices: Vec<i64>,
    /// The field's widget annotations: `[self]` for a merged (Shape A) field,
    /// the `/Kids` widgets for a Shape B field, or empty for a value-only
    /// terminal field with no on-page presence.
    pub widgets: Vec<Widget>,
    /// Whether the field is a single merged field+widget object (Shape A).
    pub merged: bool,
    /// Whether the field carries an `/AA` additional-actions dictionary
    /// (§12.6.3) — a form-JavaScript hook point. **Recognition + disclosure
    /// only: pdfce never executes it** (decision 008 §5.1, NF4). Surfaced so
    /// the operator knows a field is script-driven (e.g. a `/CO` calculated
    /// value pdfce shows as-stored but does not recompute).
    pub has_additional_actions: bool,
    /// Whether the field has no `/T` of its own and therefore shares its
    /// parent's fully-qualified name — meaning it is one representation of a
    /// field that has others (they share `/FT`/`/V`/`/DV`, §12.7.3.2). A
    /// value edit must apply to every same-FQN representation.
    pub shares_parent_name: bool,
    /// The object id of this field's `/Parent` node in the field tree
    /// (§12.7.3.1), or `None` for a root field in `/AcroForm /Fields`.
    ///
    /// # Why the read projection carries a pointer back INTO the tree
    ///
    /// The projection is deliberately flat — it lists terminal fields and
    /// discards the non-terminal grouping nodes above them, because that is
    /// the right shape for "show me the fields / fill this one / flatten
    /// these". But two operations genuinely need the ancestor and cannot
    /// recover it from a flat list:
    ///
    /// * **Writing an inherited `/V`.** §12.7.3.1 lets `/V` be declared on an
    ///   ancestor and inherited by every terminal beneath it. The projection
    ///   RESOLVES that inheritance, so a resolved value looks identical
    ///   whether it was declared here or three levels up — and a fill that
    ///   writes to the terminal in the second case leaves the ancestor's
    ///   declaration untouched, so the old value is still inherited by every
    ///   SIBLING. The write went to the wrong dictionary and nothing said so.
    /// * **Subtree rename.** Renaming `Personal` changes the FQN of every
    ///   terminal beneath it; the terminals must be able to name the node
    ///   whose `/T` actually moves.
    ///
    /// Populated by the tree walk, which is the only place the relationship
    /// is known for certain. It is deliberately NOT read from the node's own
    /// `/Parent` key: that key is a back-reference a producer may omit or get
    /// wrong, whereas the walk arrived here FROM the parent and cannot be
    /// mistaken about which node that was.
    ///
    /// # Honest limit
    ///
    /// Adding this field does not by itself fix the inherited-`/V` write —
    /// the three setters still write to the terminal (decision 020 §8.4,
    /// named there as a limit rather than a defect). This is the substrate
    /// that fix needs, added where the walk can populate it correctly.
    pub parent: Option<ObjId>,
}

impl Field {
    /// Whether this field's value can be changed by a fill edit: not
    /// `ReadOnly`, and not a signature/pushbutton (which hold no fillable
    /// value).
    #[must_use]
    pub fn is_fillable(&self) -> bool {
        if self.flags.read_only() {
            return false;
        }
        match self.field_type {
            Some(FieldType::Text | FieldType::Choice) => true,
            Some(FieldType::Button) => matches!(
                self.button_kind,
                Some(ButtonKind::Check | ButtonKind::Radio)
            ),
            Some(FieldType::Signature) | None => false,
        }
    }

    /// Whether this is a **rich-text** field — `/Ff` bit 26 on a `/Tx` field
    /// (§12.7.4.3 Table 228).
    ///
    /// # Why this exists as a method rather than a flag test at the call site
    ///
    /// **Bit 26 is the only overloaded bit position in the whole `/Ff` family**
    /// (confirmed by exhaustive enumeration of Tables 221/226/228/230): it is
    /// `RichText` on a text field and [`RadiosInUnison`](Self::radios_in_unison)
    /// on a button field, same value `33554432`. So
    /// `field.flags.has(FieldFlags::RICH_TEXT)` compiles perfectly and is
    /// WRONG on a radio group — it reports every one of them as rich text.
    ///
    /// The hazard is the CONJUNCTION of two facts, and neither alone is
    /// dangerous: bit positions are reused across field types, **and** `/FT`
    /// is inheritable through `/Parent` (§12.7.3.1), so a widget-shaped
    /// dictionary routinely carries `/Ff` and no `/FT` of its own. A caller
    /// holding only the flag word cannot decode it correctly even in
    /// principle.
    ///
    /// Putting the question on [`Field`] — which has already resolved the type
    /// through the parent walk — makes the mistake unavailable rather than
    /// merely documented. That is the difference between a warning and a
    /// signature.
    ///
    /// Returns `false` for every non-text field, including a `/Sig` field with
    /// bit 26 set: signature fields have **no type-specific flag table at
    /// all** (Table 232 adds only `/Lock` and `/SV`), so bits 4–32 there are
    /// reserved and a set bit is malformed, never a meaning.
    #[must_use]
    pub fn is_rich_text(&self) -> bool {
        matches!(self.field_type, Some(FieldType::Text)) && self.flags.has(FieldFlags::RICH_TEXT)
    }

    /// Whether this button field's radio kids toggle **in unison** — `/Ff`
    /// bit 26 on a `/Btn` field (§12.7.4.2 Table 226).
    ///
    /// The other half of bit 26's overload; see [`Self::is_rich_text`] for the
    /// full argument. Gated on the button kind as well as the type, because
    /// the flag is meaningful only for a radio set — Table 226 defines it
    /// against radio kids sharing an on-state name.
    #[must_use]
    pub fn radios_in_unison(&self) -> bool {
        matches!(self.field_type, Some(FieldType::Button))
            && matches!(self.button_kind, Some(ButtonKind::Radio))
            && self.flags.has(FieldFlags::RADIOS_IN_UNISON)
    }

    /// Whether any of this field's widgets carries a usable normal
    /// appearance (`/AP` `/N`). The per-field demand signal for
    /// regeneration and `/NeedAppearances` disclosure.
    #[must_use]
    pub fn has_appearance(&self) -> bool {
        self.widgets.iter().any(|w| w.has_normal_appearance)
    }
}

/// Whether a document's `/AcroForm` carries an XFA layer, and in which of
/// the two Table-218 / §12.7.8 shapes.
///
/// pdfce **detects and discloses** XFA; it never parses it (0.08 % of the
/// corpus; Backlog, deprecation status open). The AcroForm `/Fields` side
/// — the part ISO 32000-1 fully specifies — is modelled and rendered as
/// usual; this only records that a dynamic XFA layer also exists that pdfce
/// does not render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfaPresence {
    /// No `/XFA` entry.
    None,
    /// `/XFA` is a single stream (the whole XML Data Package).
    Stream,
    /// `/XFA` is an array of alternating `(packet-name) <stream>` pairs;
    /// `packets` counts the name/stream pairs.
    PacketArray {
        /// Number of `(name, stream)` packet pairs.
        packets: usize,
    },
}

impl XfaPresence {
    /// Whether any XFA layer is present.
    #[must_use]
    pub const fn is_present(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// A document's interactive form, fully modelled (ISO 32000-1 §12.7.2).
///
/// The list of terminal [`Field`]s plus the document-level attributes from
/// Table 218. Obtained from [`parse_acroform`]; `None` when the document has
/// no `/AcroForm`.
/// A **pure grouping node** in the §12.7.3.2 field-name tree: a field
/// dictionary with child fields and no widgets of its own.
///
/// # Why this is surfaced separately rather than in [`AcroForm::fields`]
///
/// Table 220 gives such a node no presence, no type and no value, so it
/// contributes nothing to a projection of *terminal* fields and
/// `walk_field` deliberately stops at one. That projection is right for
/// filling, flattening and appearance work, which is everything
/// [`AcroForm::fields`] feeds.
///
/// It is wrong for exactly one thing: **the node still owns a `/T`, and
/// renaming it re-derives the fully-qualified name of every field beneath
/// it.** `EditSession::rename_field` accepts a grouping node's FQN and
/// handles it (`FieldPath::Grouping`) — so the capability existed while the
/// name of the thing to address was unreachable from a reader.
///
/// # ★ Why a shell must NOT derive this by splitting a terminal's FQN
///
/// It is tempting: `Personal.Address.Zip` looks like it yields `Personal`
/// and `Personal.Address` for free. It does not, and the failure is silent.
/// [`Field::fully_qualified_name`] is built by joining **decoded `/T` text
/// strings** (§7.9.2), and nothing prevents a `/T` from containing a
/// literal period — `rename_field` refuses a period in a NEW name, which
/// says nothing about names already in a file. On such a document a split
/// misattributes every segment after the first, and reports ancestors that
/// do not exist.
///
/// So the node's own partial name is carried here, read from the object
/// rather than reconstructed from a string (project rule 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldGroupNode {
    /// The grouping node's field dictionary.
    pub id: ObjId,
    /// Its fully-qualified name — the string
    /// [`EditSession::rename_field`](crate::edit::EditSession::rename_field)
    /// takes to address it.
    pub fully_qualified_name: String,
    /// Its own `/T` (raw bytes). `None` for a `/T`-less intermediate, which
    /// contributes no segment and therefore cannot be renamed.
    pub partial_name: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcroForm {
    /// The terminal fields, in field-tree DFS order (the order `/Fields` and
    /// each `/Kids` present them).
    pub fields: Vec<Field>,
    /// The **pure grouping nodes** — the field-name tree's interior, which
    /// [`Self::fields`] deliberately omits.
    ///
    /// **Deepest-first (post-order): a child appears before its parent.**
    /// Not a choice — the node is recorded at the early return that stops the
    /// walk, which is reached *after* recursing into its children. Stated
    /// because it is the opposite of what "DFS order" suggests and a consumer
    /// that assumed parents-first would render a breadcrumb backwards. It is
    /// also the useful order for a caller renaming several: rename the
    /// deepest first and the shallower paths stay valid.
    ///
    /// Empty for a flat form, which is every file in the Pass 7.0 census;
    /// non-empty is produced mainly by pdfce's own same-name merge. A
    /// consumer that renders these must therefore render *nothing* when the
    /// list is empty rather than an empty section (R124).
    pub groups: Vec<FieldGroupNode>,
    /// `/NeedAppearances` (Table 218; default false) — the producer's
    /// assertion that widget appearances are stale. **Disclosed, never a
    /// silent on-load regenerate** (R51, NF1): a *may*, not a *shall*.
    pub need_appearances: bool,
    /// `/SigFlags` (Table 219; default 0) — the raw document-level signature
    /// flags word.
    pub sig_flags: u32,
    /// `SigFlags` bit 1 (`SignaturesExist`): the document contains ≥1
    /// signature field.
    pub signatures_exist: bool,
    /// `SigFlags` bit 2 (`AppendOnly`): signatures may be invalidated by a
    /// non-incremental save — the document-level echo of pdfce's default
    /// incremental-save discipline (R36).
    pub append_only: bool,
    /// The number of fields in `/CO` (the calculation order, §12.6.3).
    /// **Recognition only** — pdfce round-trips `/CO` and never executes the
    /// calculation scripts it orders (NF4). Non-zero means the form has
    /// calculated fields whose stored `/V` pdfce shows but does not recompute.
    pub calc_order_count: usize,
    /// Whether the AcroForm has a `/DR` default-resources dictionary (the
    /// fonts a widget `/DA` resolves against, §12.7.3.3).
    pub has_default_resources: bool,
    /// The document-wide default `/DA` (Table 218), the fallback for a field
    /// with none.
    pub default_appearance: Option<Vec<u8>>,
    /// The document-wide default `/Q` quadding (Table 218; default left).
    pub quadding: Quadding,
    /// Whether — and how — the form carries an XFA layer (detect only).
    pub xfa: XfaPresence,
    /// How many `/AcroForm /Fields` entries were **direct dictionaries**
    /// rather than indirect references, and are therefore absent from
    /// [`AcroForm::fields`].
    ///
    /// §12.7.3.1 requires every field to be an indirect object. A direct dict
    /// in `/Fields` is malformed input — but a reader that skips it silently
    /// reports a field count the file does not have, which is the difference
    /// between tolerating damage and hiding it. pdfce cannot descend into
    /// such an entry (every downstream operation addresses a field by object
    /// id, and a direct dict has none), so it counts it and says so.
    ///
    /// Normally `0`. A non-zero value means `fields.len()` understates the
    /// file's field count by exactly this much.
    pub inline_field_roots: usize,
}

impl AcroForm {
    /// The terminal fields that can be filled (see [`Field::is_fillable`]).
    pub fn fillable_fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| f.is_fillable())
    }

    /// The first field with the given fully-qualified name.
    ///
    /// Several fields may share an FQN (same-FQN representations of one
    /// logical field, §12.7.3.2); this returns the first in DFS order. A
    /// fill edit must apply to *all* of them —
    /// [`AcroForm::fields_named`] enumerates them.
    #[must_use]
    pub fn field_by_name(&self, fqn: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.fully_qualified_name == fqn)
    }

    /// Every field sharing the given fully-qualified name (§12.7.3.2's
    /// same-FQN representations), in DFS order.
    pub fn fields_named<'a>(&'a self, fqn: &'a str) -> impl Iterator<Item = &'a Field> {
        self.fields
            .iter()
            .filter(move |f| f.fully_qualified_name == fqn)
    }

    /// Every field lying **under** `fqn` in the §12.7.3.2 name tree — the
    /// fields whose fully-qualified name would change if `fqn`'s partial
    /// name changed, **without any of their own objects being written**.
    ///
    /// # Why this is a shared function and not a one-line filter
    ///
    /// It is a one-line filter, and that is exactly the problem: the line
    /// contains a subtlety that is invisible once written and wrong once
    /// forgotten. **The prefix carries the separator.** `Address.` matches
    /// `Address.City` and does *not* match `Addressed` — and a caller who
    /// writes `starts_with(fqn)` instead of `starts_with(&format!("{fqn}."))`
    /// gets a count that is right on every form anyone tests with and wrong
    /// on the first form that happens to contain two fields whose names
    /// share a prefix.
    ///
    /// It had one caller ([`EditSession::rename_field`](crate::edit::EditSession::rename_field),
    /// for [`FieldRename::descendants_renamed`](crate::edit::FieldRename::descendants_renamed))
    /// and now has more: a shell renaming a field must re-key any in-flight
    /// per-field state it holds under the old names, which needs the same
    /// notion of descendant and must not re-derive it. Project rule 2 —
    /// a shell reconstructing core's own definition is how the two drift.
    ///
    /// # The projection this inherits, stated because it affects the count
    ///
    /// [`Self::fields`] is a projection of **terminal** fields: `walk_field`
    /// stops at a pure grouping node, which has no presence and no type of
    /// its own (Table 220). So renaming a group containing three
    /// intermediate nodes and five terminals reports **five**, not eight.
    ///
    /// That is the right number for what the count is FOR. The disclosure it
    /// feeds is about breakage outside the document — FDF entries, JavaScript
    /// references, submit mappings — and every one of those names a terminal
    /// field. An intermediate node's name changing breaks nothing on its own;
    /// it breaks things by changing the terminals beneath it, which are
    /// already counted.
    ///
    /// Excludes `fqn` itself: a rename writes that node's dictionary, so it
    /// is the subject of the operation rather than a consequence of it.
    pub fn descendants_of<'a>(&'a self, fqn: &'a str) -> impl Iterator<Item = &'a Field> {
        // Built once, outside the closure, rather than per element — and
        // bound to a name so the trailing separator is visible at a glance
        // instead of buried in a `format!` inside a filter.
        let prefix = format!("{fqn}.");
        self.fields
            .iter()
            .filter(move |f| f.fully_qualified_name.starts_with(&prefix))
    }
}

/// The inheritable attributes carried down the field tree during the walk
/// (§12.7.3.1 — inheritance follows `/Parent`, not the page tree).
///
/// Each is the *raw* nearest-ancestor value, resolved lazily by the caller.
/// Kept as owned `Object`s (cloned) because the walk clones each field dict
/// anyway and a borrow would tangle the recursion's lifetimes.
#[derive(Debug, Clone, Default)]
struct Inherited {
    field_type: Option<FieldType>,
    flags: Option<u32>,
    value: Option<Object>,
    default_value: Option<Object>,
    default_appearance: Option<Vec<u8>>,
    quadding: Option<i64>,
}

/// Parse a document's interactive form (ISO 32000-1 §12.7.2), or `None`
/// when the catalog has no `/AcroForm`.
///
/// Generic over [`ObjectGraph`] so it runs over both a loaded
/// [`Document`](crate::document::Document) and an
/// [`EditSession`](crate::edit::EditSession) overlay — exactly like
/// [`crate::annot::page_annotations`] and [`crate::page_tree::pages_in`].
/// Every malformed shape is tolerated (skip and model what is there, never
/// a panic): a non-array `/Fields`, a `/Kids` cycle (caught by a visited-id
/// set), a dangling field reference, a `/T` that is not a string.
///
/// # Examples
///
/// ```
/// use pdfce_core::document::Document;
/// use pdfce_core::forms::parse_acroform;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Document::from_bytes(
///     include_bytes!("../../../fixtures/synthetic/hello.pdf").to_vec(),
/// )?;
/// // hello.pdf has no form.
/// assert!(parse_acroform(&doc).is_none());
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn parse_acroform<G: ObjectGraph + ?Sized>(graph: &G) -> Option<AcroForm> {
    let catalog = graph.catalog_dict()?;
    let acro = graph.resolve(catalog.get(b"AcroForm")?).as_dict()?.clone();

    // Document-level Table 218 attributes.
    let need_appearances = matches!(
        acro.get(b"NeedAppearances").map(|o| graph.resolve(o)),
        Some(Object::Boolean(true))
    );
    let sig_flags = acro
        .get(b"SigFlags")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_int)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let calc_order_count = acro
        .get(b"CO")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
        .map_or(0, <[Object]>::len);
    let has_default_resources = acro
        .get(b"DR")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
        .is_some();
    let default_appearance = acro
        .get(b"DA")
        .map(|o| graph.resolve(o))
        .and_then(string_bytes);
    let quadding = Quadding::from_code(
        acro.get(b"Q")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_int)
            .unwrap_or(0),
    );
    let xfa = detect_xfa(graph, &acro);

    // Walk the root fields depth-first, resolving inheritance down /Kids.
    let mut fields = Vec::new();
    let mut groups: Vec<FieldGroupNode> = Vec::new();
    let mut visited = HashSet::new();
    let mut inline_field_roots = 0usize;
    if let Some(roots) = acro
        .get(b"Fields")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
    {
        // Collect the root field ids up front so `roots` (borrowed from the
        // cloned `acro`) does not tangle with the recursive borrow of `graph`.
        //
        // NON-REFERENCE entries are COUNTED, not silently skipped. §12.7.3.1
        // requires every field to be an indirect object, so a direct dict
        // sitting in `/Fields` is malformed — but "malformed" is not the same
        // as "absent", and a reader that drops it without saying so reports a
        // field count the file does not have. The count is the disclosure;
        // the walk still cannot descend into it, because every operation
        // downstream (fill, flatten, appearance regeneration) addresses a
        // field by OBJECT ID and a direct dict has none to address.
        //
        // This matters to authoring rather than to reading: pdfce must never
        // WRITE such an entry, which is why F1's acceptance asserts the
        // `/Fields` entry it appends is an indirect reference.
        let mut root_ids: Vec<ObjId> = Vec::with_capacity(roots.len());
        for entry in roots {
            match entry.as_reference() {
                Some(id) => root_ids.push(id),
                None => inline_field_roots += 1,
            }
        }
        for id in root_ids {
            walk_field(
                graph,
                id,
                &Inherited::default(),
                String::new(),
                None,
                0,
                &mut visited,
                &mut fields,
                &mut groups,
            );
        }
    }

    Some(AcroForm {
        fields,
        groups,
        inline_field_roots,
        need_appearances,
        sig_flags,
        signatures_exist: sig_flags & 1 != 0,
        append_only: sig_flags & 2 != 0,
        calc_order_count,
        has_default_resources,
        default_appearance,
        quadding,
        xfa,
    })
}

/// Detect an `/XFA` layer's shape without parsing it (§12.7.8).
fn detect_xfa<G: ObjectGraph + ?Sized>(graph: &G, acro: &Dict) -> XfaPresence {
    let Some(xfa) = acro.get(b"XFA").map(|o| graph.resolve(o)) else {
        return XfaPresence::None;
    };
    match xfa {
        Object::Stream(_) => XfaPresence::Stream,
        // An array of alternating (name-string) <stream-ref> pairs; count
        // the pairs (two array entries each).
        Object::Array(a) => XfaPresence::PacketArray {
            packets: a.len() / 2,
        },
        // Present but neither shape (malformed) — record it as present in
        // the closest shape rather than dropping the signal.
        _ => XfaPresence::Stream,
    }
}

/// One node of the field-tree DFS (§12.7.3.1).
///
/// `inherited` is the nearest-ancestor context; `parent_fqn` is the
/// ancestors' fully-qualified name so far; `parent_id` is the node this walk
/// descended FROM (`None` at an `/AcroForm /Fields` root); `depth`/`visited`
/// bound cycles and hostile trees; resolved terminal fields are pushed onto
/// `out`.
#[allow(clippy::too_many_arguments)]
fn walk_field<G: ObjectGraph + ?Sized>(
    graph: &G,
    id: ObjId,
    inherited: &Inherited,
    parent_fqn: String,
    parent_id: Option<ObjId>,
    depth: usize,
    visited: &mut HashSet<ObjId>,
    out: &mut Vec<Field>,
    // The field-name tree's INTERIOR. Collected in the same walk rather than
    // by a second traversal, so the two projections cannot disagree about
    // what the tree is — and captured at the early return below, which is the
    // only place a pure grouping node is known and then discarded.
    groups: &mut Vec<FieldGroupNode>,
) {
    if out.len() >= MAX_FORM_FIELDS || depth >= MAX_FIELD_TREE_DEPTH {
        return;
    }
    // Cycle guard: a /Kids or /Parent loop must terminate (§7.3.10 posture).
    if !visited.insert(id) {
        return;
    }
    let Some(dict) = graph.resolved(id).as_dict().cloned() else {
        return;
    };

    // Resolve this node's own vs inherited attributes.
    let own_ft = dict
        .get(b"FT")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_name)
        .and_then(|n| FieldType::from_name(n.as_bytes()));
    let field_type = own_ft.or(inherited.field_type);

    let own_flags = dict
        .get(b"Ff")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_int)
        .and_then(|v| u32::try_from(v).ok());
    let flags_word = own_flags.or(inherited.flags).unwrap_or(0);

    let own_value = dict.get(b"V").map(|o| graph.resolve(o).clone());
    let value_obj = own_value.or_else(|| inherited.value.clone());
    let own_dv = dict.get(b"DV").map(|o| graph.resolve(o).clone());
    let dv_obj = own_dv.or_else(|| inherited.default_value.clone());

    let own_da = dict
        .get(b"DA")
        .map(|o| graph.resolve(o))
        .and_then(string_bytes);
    let da = own_da.or_else(|| inherited.default_appearance.clone());

    let own_q = dict
        .get(b"Q")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_int);
    let q_code = own_q.or(inherited.quadding);

    // This node's partial name and the fully-qualified name it contributes.
    let partial_name = dict
        .get(b"T")
        .map(|o| graph.resolve(o))
        .and_then(string_bytes);
    let has_own_name = partial_name.is_some();
    let this_fqn = match &partial_name {
        Some(t) => {
            let seg = crate::edit::decode_text_string(t).text;
            if parent_fqn.is_empty() {
                seg
            } else {
                format!("{parent_fqn}.{seg}")
            }
        }
        None => parent_fqn.clone(),
    };

    // Classify /Kids: field children (each with its own /T) mean a
    // non-terminal node; /T-less widget children (or none) mean a terminal
    // field (§12.7.3.1 + the merge rule).
    let kid_ids: Vec<ObjId> = dict
        .get(b"Kids")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
        .map(|a| a.iter().filter_map(Object::as_reference).collect())
        .unwrap_or_default();

    let child_fields: Vec<ObjId> = kid_ids
        .iter()
        .copied()
        .filter(|kid| kid_is_field(graph, *kid))
        .collect();

    let ctx = Inherited {
        field_type,
        flags: Some(flags_word),
        value: value_obj.clone(),
        default_value: dv_obj.clone(),
        default_appearance: da.clone(),
        quadding: q_code,
    };

    // MIXED `/Kids` — a node may hold BOTH child fields and bare widgets.
    //
    // §12.7.3.1's merge rule classifies each kid INDIVIDUALLY: a kid with its
    // own `/T` is a child field, a `/T`-less widget kid is one of this node's
    // own appearances. Nothing in the spec says a node must pick one KIND of
    // kid, and a node that mixes them is both a non-terminal (for its child
    // fields) and a terminal with on-page presence (for its widget kids).
    //
    // This code used to pick one: if ANY kid was a field it recursed and
    // RETURNED, so a mixed node's own widgets were never modelled and the
    // node itself never reached `out`. Its `/V` and its rectangle simply
    // vanished from `list-fields`, from `regenerate-appearances`, from
    // `export-data` and from `flatten` — while the page's `/Annots` still
    // referenced the widget, so a viewer painted a field the form did not
    // contain. Measured on `mixed-kids-form.pdf` before the fix:
    // `list-fields` reported `Order.Qty` alone and `Order` not at all.
    //
    // No corpus file has the shape (Pass 7.0's census: no file nests fields
    // at all), which is why it survived — and F1's merge can GENERATE it, by
    // attaching a widget to a node that already has a child field. So the two
    // classifications now both happen: recurse into the child fields, and
    // fall through to model this node's own widget kids.
    let widget_kids: Vec<ObjId> = kid_ids
        .iter()
        .copied()
        .filter(|kid| !child_fields.contains(kid))
        .collect();

    for kid in &child_fields {
        walk_field(
            graph,
            *kid,
            &ctx,
            this_fqn.clone(),
            Some(id),
            depth + 1,
            visited,
            out,
            groups,
        );
    }

    // A PURE non-terminal — child fields and no widgets of its own — has no
    // presence and no type of its own (Table 220), so it contributes nothing
    // to a projection of terminal fields. Stop here.
    if !child_fields.is_empty() && widget_kids.is_empty() {
        // ★ Recorded on the way past. Everything needed is in hand here and
        // nowhere else: `this_fqn` is the joined, decoded path and
        // `partial_name` is the node's own `/T` read from the object. A
        // caller that wanted these later would have to rebuild them by
        // splitting a descendant's FQN, which is wrong on any file whose
        // `/T` contains a period — see `FieldGroupNode`.
        //
        // Bounded by the same `MAX_FORM_FIELDS` ceiling the terminal list is,
        // checked at the top of this function, so a pathological tree cannot
        // grow this list without bound either.
        groups.push(FieldGroupNode {
            id,
            fully_qualified_name: this_fqn.clone(),
            partial_name: partial_name.clone(),
        });
        visited.remove(&id);
        return;
    }

    // Terminal field (possibly ALSO a non-terminal, per the mixed case above).
    // Its widgets are either the /T-less /Kids (Shape B) or, when there are
    // none, the field dict itself if it looks like a widget (Shape A merge) —
    // otherwise a value-only terminal with no presence.
    let flags = FieldFlags(flags_word);
    let button_kind =
        (field_type == Some(FieldType::Button)).then(|| ButtonKind::from_flags(flags));

    let widgets = if widget_kids.is_empty() {
        // Shape A (merged) or value-only.
        if dict_is_widget(&dict) {
            vec![model_widget(graph, id, &dict, true)]
        } else {
            Vec::new()
        }
    } else {
        // Shape B: each /T-less kid is a widget of this field.
        widget_kids
            .iter()
            .filter_map(|kid| {
                graph
                    .resolved(*kid)
                    .as_dict()
                    .cloned()
                    .map(|kd| model_widget(graph, *kid, &kd, false))
            })
            .collect()
    };

    let value = decode_value(field_type, button_kind, value_obj.as_ref());
    let default_value = decode_value(field_type, button_kind, dv_obj.as_ref());

    out.push(Field {
        id,
        fully_qualified_name: this_fqn,
        partial_name,
        alternate_name: dict
            .get(b"TU")
            .map(|o| graph.resolve(o))
            .and_then(string_bytes),
        // Read unconditionally, NOT gated on the RichText flag. A file may
        // carry `/RV` with bit 26 clear — malformed under Table 228, and
        // exactly the case where silently dropping the entry on export would
        // destroy the only copy of the formatting.
        rich_value: dict
            .get(b"RV")
            .map(|o| graph.resolve(o))
            .and_then(string_bytes),
        mapping_name: dict
            .get(b"TM")
            .map(|o| graph.resolve(o))
            .and_then(string_bytes),
        field_type,
        button_kind,
        flags,
        value,
        default_value,
        default_appearance: da,
        quadding: Quadding::from_code(q_code.unwrap_or(0)),
        max_len: dict
            .get(b"MaxLen")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_int),
        options: read_options(graph, &dict),
        top_index: dict
            .get(b"TI")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_int)
            .unwrap_or(0),
        selected_indices: read_indices(graph, &dict),
        widgets,
        merged: widget_kids.is_empty() && dict_is_widget(&dict),
        has_additional_actions: dict.contains_key(b"AA"),
        shares_parent_name: !has_own_name,
        parent: parent_id,
    });

    visited.remove(&id);
}

/// Whether a `/Kids` entry is a **child field** (its own `/T`, `/FT`, or
/// `/Kids`) as opposed to a bare widget of the parent field.
///
/// The spec's merge rule keys on `/Kids`: a kid with its own `/T` is a
/// distinct child field (even if it is *also* a merged widget); a `/T`-less
/// widget kid is one of the parent field's appearances. `/FT` or `/Kids`
/// on the kid likewise mark it as a field node.
fn kid_is_field<G: ObjectGraph + ?Sized>(graph: &G, id: ObjId) -> bool {
    let Some(d) = graph.resolved(id).as_dict() else {
        return false;
    };
    d.contains_key(b"T") || d.contains_key(b"FT") || d.contains_key(b"Kids")
}

/// Whether a dictionary looks like a widget annotation — `/Subtype /Widget`,
/// or (defensively) an `/AP`/`/Rect` presence. A merged Shape-A field always
/// carries `/Subtype /Widget`; the fallback catches producers that omit it.
fn dict_is_widget(dict: &Dict) -> bool {
    if let Some(Object::Name(n)) = dict.get(b"Subtype")
        && n.as_bytes() == b"Widget"
    {
        return true;
    }
    dict.contains_key(b"Rect") || dict.contains_key(b"AP")
}

/// Model one widget of a field.
fn model_widget<G: ObjectGraph + ?Sized>(
    graph: &G,
    id: ObjId,
    dict: &Dict,
    merged: bool,
) -> Widget {
    let rect = dict.get(b"Rect").and_then(|o| read_rect(graph, o));
    let appearance_state = dict
        .get(b"AS")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_name)
        .map(|n| n.as_bytes().to_vec());
    let page = dict.get(b"P").and_then(Object::as_reference);
    // `/MK` `/CA` (Table 189). Resolved through the graph like every other
    // key here, because a producer is free to make `/MK` an indirect object
    // and a direct-only read would report "no caption" for a file that has
    // one.
    let caption = dict
        .get(b"MK")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
        .and_then(|mk| mk.get(b"CA").map(|o| graph.resolve(o)))
        .and_then(string_bytes);
    let (has_normal_appearance, on_states) = appearance_of(graph, dict);
    Widget {
        id,
        rect,
        appearance_state,
        on_states,
        page,
        caption,
        has_normal_appearance,
        merged,
    }
}

/// Read a widget's `/AP` `/N`: whether it is usable, and (for a state
/// subdictionary) the non-`Off` on-state names.
fn appearance_of<G: ObjectGraph + ?Sized>(graph: &G, dict: &Dict) -> (bool, Vec<Vec<u8>>) {
    let Some(ap) = dict
        .get(b"AP")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
    else {
        return (false, Vec::new());
    };
    let Some(n) = ap.get(b"N").map(|o| graph.resolve(o)) else {
        return (false, Vec::new());
    };
    match n {
        Object::Stream(_) => (true, Vec::new()),
        Object::Dict(sub) => {
            let states: Vec<Vec<u8>> = sub
                .iter()
                .filter(|(_, v)| !matches!(v, Object::Null))
                .map(|(k, _)| k.as_bytes().to_vec())
                .filter(|k| k.as_slice() != b"Off")
                .collect();
            (!sub.is_empty(), states)
        }
        _ => (false, Vec::new()),
    }
}

/// Read a `/Rect`-shaped array (four resolvable numbers) and normalise it
/// (§7.9.5). `None` when not four numbers.
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

/// Decode a `/V` or `/DV` object into a typed [`FieldValue`] (§12.7.4).
fn decode_value(
    field_type: Option<FieldType>,
    button_kind: Option<ButtonKind>,
    obj: Option<&Object>,
) -> FieldValue {
    let Some(obj) = obj else {
        return FieldValue::Absent;
    };
    if matches!(obj, Object::Null) {
        return FieldValue::Absent;
    }
    match field_type {
        Some(FieldType::Button) => {
            // Pushbutton retains no value (§12.7.4.2.2); checkbox/radio hold
            // a name.
            if matches!(button_kind, Some(ButtonKind::Push)) {
                return FieldValue::Absent;
            }
            match obj {
                Object::Name(n) => FieldValue::Name(n.as_bytes().to_vec()),
                _ => FieldValue::Absent,
            }
        }
        Some(FieldType::Text) => match obj {
            Object::String(s) => FieldValue::Text(s.clone()),
            // PDF 1.5: a text field's /V may be a stream. Its decoded bytes
            // need the filter chain (not available in this read model);
            // recognised as text, body left empty rather than guessed.
            Object::Stream(_) => FieldValue::Text(Vec::new()),
            _ => FieldValue::Absent,
        },
        Some(FieldType::Choice) => match obj {
            Object::String(s) => FieldValue::Choice(vec![s.clone()]),
            Object::Array(items) => FieldValue::Choice(
                items
                    .iter()
                    .filter_map(|o| match o {
                        Object::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => FieldValue::Absent,
        },
        Some(FieldType::Signature) => match obj {
            Object::Dict(_) => FieldValue::Signature,
            _ => FieldValue::Absent,
        },
        // Untyped terminal (malformed): keep a name/string if that is what
        // is there, so the value is not silently lost.
        None => match obj {
            Object::Name(n) => FieldValue::Name(n.as_bytes().to_vec()),
            Object::String(s) => FieldValue::Text(s.clone()),
            _ => FieldValue::Absent,
        },
    }
}

/// Read a choice field's `/Opt` array (§12.7.4.4, Table 231).
fn read_options<G: ObjectGraph + ?Sized>(graph: &G, dict: &Dict) -> Vec<ChoiceOption> {
    let Some(arr) = dict
        .get(b"Opt")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
    else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|el| match graph.resolve(el) {
            // A single string: export == display.
            Object::String(s) => Some(ChoiceOption {
                export: s.clone(),
                display: s.clone(),
            }),
            // A two-element [export display] array.
            Object::Array(pair) => {
                let export = pair.first().and_then(|o| string_bytes(graph.resolve(o)))?;
                let display = pair
                    .get(1)
                    .and_then(|o| string_bytes(graph.resolve(o)))
                    .unwrap_or_else(|| export.clone());
                Some(ChoiceOption { export, display })
            }
            _ => None,
        })
        .collect()
}

/// Read a choice field's `/I` selected-indices array (§12.7.4.4).
fn read_indices<G: ObjectGraph + ?Sized>(graph: &G, dict: &Dict) -> Vec<i64> {
    dict.get(b"I")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
        .map(|a| a.iter().filter_map(|o| graph.resolve(o).as_int()).collect())
        .unwrap_or_default()
}

/// The raw bytes of a string object, or `None`.
fn string_bytes(obj: &Object) -> Option<Vec<u8>> {
    match obj {
        Object::String(s) => Some(s.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Embedded-JavaScript disclosure histogram (decision 009, posture A)
// ---------------------------------------------------------------------------

/// A recognition-only inventory of a document's embedded form/document
/// JavaScript and its action triggers (decision 009, posture A).
///
/// **pdfce NEVER executes any of this** (R53/R54): §12.6.4.16 is a "hollow
/// shall" — ISO 32000-1 defines the JavaScript *carrier* and *hook points*
/// but no JavaScript semantics, API, DOM, or security model, so
/// non-execution is fully ISO-conformant. This struct exists to **disclose**
/// what a document *would* run in Acrobat/Reader, so the operator knows a
/// field is script-driven (its stored `/V` is shown as-last-saved, never
/// recomputed) and knows a document runs scripts on open.
///
/// The `/AA` action-type counts flag the R12 (no-network) and R13
/// (no-process-launch) hazards loudly: a trigger action can be `/URI`,
/// `/SubmitForm`, or `/ImportData` (network) or `/Launch` (process); pdfce
/// recognizes and counts them but has **no JS/action dispatcher** to run them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormJavaScript {
    /// Fields with a calculate (`/AA` `/C`) JavaScript action.
    pub fields_with_calculate_script: usize,
    /// Fields with a format (`/AA` `/F`) JavaScript action.
    pub fields_with_format_script: usize,
    /// Fields with a validate (`/AA` `/V`) JavaScript action.
    pub fields_with_validate_script: usize,
    /// Fields with a keystroke (`/AA` `/K`) JavaScript action.
    pub fields_with_keystroke_script: usize,
    /// Total field-level JavaScript actions seen (all classified `Custom`
    /// in posture A — the whitelist recompute of posture B is Pass 7.x).
    pub custom_scripts: usize,
    /// Document-level scripts: entries in the catalog `/Names /JavaScript`
    /// name tree (the scripts Acrobat runs on document open).
    pub doc_level_scripts: usize,
    /// Whether the catalog `/OpenAction` is (or contains) a JavaScript
    /// action — a document that runs a script the instant it opens.
    pub open_action_is_javascript: bool,
    /// `/AA` actions referencing the **network** — `/URI`, `/SubmitForm`,
    /// or `/ImportData`. A **blocked capability** under R12; counted and
    /// flagged loudly, never dispatched.
    pub network_action_count: usize,
    /// `/AA` actions referencing a **process launch** (`/Launch`). A blocked
    /// capability under R13; counted, never dispatched.
    pub launch_action_count: usize,
}

impl FormJavaScript {
    /// Whether the document carries any embedded script or auto-run trigger
    /// at all — the single "this document is script-driven" disclosure bit.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.custom_scripts > 0
            || self.doc_level_scripts > 0
            || self.open_action_is_javascript
            || self.network_action_count > 0
            || self.launch_action_count > 0
    }
}

/// Scan a document for embedded JavaScript and action triggers, producing the
/// recognition-only [`FormJavaScript`] histogram (decision 009, posture A).
///
/// Covers **field-level `/AA`** (`/C`/`/F`/`/V`/`/K` triggers on every form
/// field), **document-level `/AA`** (catalog `WC`/`WS`/`DS`/`WP`/`DP`), the
/// catalog **`/OpenAction`**, and the **`/Names /JavaScript`** document-level
/// name tree. Every script and action is **recognized and counted, never
/// executed** (R53/R54). Bounded and cycle-guarded like [`parse_acroform`].
#[must_use]
pub fn scan_javascript<G: ObjectGraph + ?Sized>(graph: &G) -> FormJavaScript {
    let mut js = FormJavaScript::default();
    let Some(catalog) = graph.catalog_dict() else {
        return js;
    };
    let catalog = catalog.clone();

    // Field-level /AA across the whole /Fields tree.
    if let Some(acro) = catalog
        .get(b"AcroForm")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
        && let Some(roots) = acro
            .get(b"Fields")
            .map(|o| graph.resolve(o))
            .and_then(Object::as_array)
    {
        let root_ids: Vec<ObjId> = roots.iter().filter_map(Object::as_reference).collect();
        let mut visited = HashSet::new();
        for id in root_ids {
            scan_field_js(graph, id, 0, &mut visited, &mut js);
        }
    }

    // Document-level /AA (WC/WS/DS/WP/DP): count only the network/launch
    // hazard (these are not field calc/format/validate/keystroke hooks).
    if let Some(aa) = catalog
        .get(b"AA")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
    {
        for (_, action) in aa.iter() {
            classify_action_hazard(graph, action, &mut js);
        }
    }

    // /OpenAction: a JavaScript action means a script runs on open.
    if let Some(oa) = catalog.get(b"OpenAction").map(|o| graph.resolve(o)) {
        if action_is_javascript(oa) {
            js.open_action_is_javascript = true;
        }
        classify_action_hazard(graph, oa, &mut js);
    }

    // /Names /JavaScript document-level name tree.
    js.doc_level_scripts = count_name_tree_scripts(graph, &catalog);

    js
}

/// Whether an object is (resolves to) an `/S /JavaScript` action dictionary.
fn action_is_javascript(action: &Object) -> bool {
    action
        .as_dict()
        .and_then(|d| d.get(b"S"))
        .and_then(Object::as_name)
        .is_some_and(|n| n.as_bytes() == b"JavaScript")
}

/// Flag an action's R12 (network) / R13 (launch) hazard by its `/S` type.
fn classify_action_hazard<G: ObjectGraph + ?Sized>(
    graph: &G,
    action: &Object,
    js: &mut FormJavaScript,
) {
    let Some(s) = graph
        .resolve(action)
        .as_dict()
        .and_then(|d| d.get(b"S"))
        .and_then(Object::as_name)
    else {
        return;
    };
    match s.as_bytes() {
        b"URI" | b"SubmitForm" | b"ImportData" => js.network_action_count += 1,
        b"Launch" => js.launch_action_count += 1,
        _ => {}
    }
}

/// DFS one field, counting its `/AA` JavaScript hooks and action hazards, then
/// recursing into `/Kids` that are themselves fields.
fn scan_field_js<G: ObjectGraph + ?Sized>(
    graph: &G,
    id: ObjId,
    depth: usize,
    visited: &mut HashSet<ObjId>,
    js: &mut FormJavaScript,
) {
    if depth >= MAX_FIELD_TREE_DEPTH || !visited.insert(id) {
        return;
    }
    let Some(dict) = graph.resolved(id).as_dict().cloned() else {
        return;
    };
    if let Some(aa) = dict
        .get(b"AA")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
    {
        // Each trigger's action: count the JS hook and the network/launch
        // hazard. /C calculate, /F format, /V validate, /K keystroke.
        for trigger in [&b"C"[..], b"F", b"V", b"K"] {
            let Some(action) = aa.get(trigger) else {
                continue;
            };
            if action_is_javascript(graph.resolve(action)) {
                match trigger {
                    b"C" => js.fields_with_calculate_script += 1,
                    b"F" => js.fields_with_format_script += 1,
                    b"V" => js.fields_with_validate_script += 1,
                    b"K" => js.fields_with_keystroke_script += 1,
                    _ => {}
                }
                js.custom_scripts += 1;
            }
            classify_action_hazard(graph, action, js);
        }
    }
    if let Some(kids) = dict
        .get(b"Kids")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
    {
        let kid_ids: Vec<ObjId> = kids.iter().filter_map(Object::as_reference).collect();
        for kid in kid_ids {
            scan_field_js(graph, kid, depth + 1, visited, js);
        }
    }
    visited.remove(&id);
}

/// Count the entries in the catalog `/Names /JavaScript` name tree — the
/// document-level scripts Acrobat runs on open. Bounded traversal.
fn count_name_tree_scripts<G: ObjectGraph + ?Sized>(graph: &G, catalog: &Dict) -> usize {
    let Some(js_tree) = catalog
        .get(b"Names")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_dict)
        .and_then(|names| names.get(b"JavaScript").map(|o| graph.resolve(o)))
        .and_then(Object::as_dict)
        .cloned()
    else {
        return 0;
    };
    let mut count = 0usize;
    let mut visited = HashSet::new();
    count_name_tree_node(graph, &js_tree, 0, &mut visited, &mut count);
    count
}

/// Recurse a name-tree node: a `/Names` leaf holds `[key value key value …]`
/// (count the value half); a `/Kids` intermediate holds child nodes.
fn count_name_tree_node<G: ObjectGraph + ?Sized>(
    graph: &G,
    node: &Dict,
    depth: usize,
    visited: &mut HashSet<ObjId>,
    count: &mut usize,
) {
    if depth >= MAX_FIELD_TREE_DEPTH {
        return;
    }
    if let Some(names) = node
        .get(b"Names")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
    {
        // [name1 action1 name2 action2 …] — half are actions.
        *count += names.len() / 2;
    }
    if let Some(kids) = node
        .get(b"Kids")
        .map(|o| graph.resolve(o))
        .and_then(Object::as_array)
    {
        let kid_ids: Vec<ObjId> = kids.iter().filter_map(Object::as_reference).collect();
        for kid in kid_ids {
            if !visited.insert(kid) {
                continue;
            }
            if let Some(child) = graph.resolved(kid).as_dict().cloned() {
                count_name_tree_node(graph, &child, depth + 1, visited, count);
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
    use crate::document::Document;

    /// Assemble a classic-xref PDF from numbered object bodies. Object 1 is
    /// the catalog; the xref is generated from contiguous numbering (gaps
    /// become free entries), mirroring `annot::tests::build_pdf`.
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

    /// A minimal document whose catalog carries the given raw `/AcroForm`
    /// dictionary text plus the given extra objects. Catalog=1, Pages=2,
    /// Page=3.
    fn doc_with_acroform(acroform: &str, extra: &[(u32, Vec<u8>)]) -> Document {
        let mut objects: Vec<(u32, Vec<u8>)> = vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R /AcroForm {acroform} >>").into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>".to_vec(),
            ),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
        ];
        objects.extend_from_slice(extra);
        build_pdf(&objects)
    }

    #[test]
    fn flag_bit_values_match_the_tables() {
        // Off-by-one silently mis-reads every flag. Verbatim Table values.
        assert_eq!(FieldFlags::READ_ONLY, 1);
        assert_eq!(FieldFlags::REQUIRED, 2);
        assert_eq!(FieldFlags::NO_EXPORT, 4);
        assert_eq!(FieldFlags::NO_TOGGLE_TO_OFF, 16384);
        assert_eq!(FieldFlags::RADIO, 32768);
        assert_eq!(FieldFlags::PUSHBUTTON, 65536);
        assert_eq!(FieldFlags::MULTILINE, 4096);
        assert_eq!(FieldFlags::PASSWORD, 8192);
        assert_eq!(FieldFlags::FILE_SELECT, 1048576);
        assert_eq!(FieldFlags::DO_NOT_SPELL_CHECK, 4194304);
        assert_eq!(FieldFlags::DO_NOT_SCROLL, 8388608);
        assert_eq!(FieldFlags::COMB, 16777216);
        assert_eq!(FieldFlags::RICH_TEXT, 33554432);
        assert_eq!(FieldFlags::RADIOS_IN_UNISON, 33554432);
        assert_eq!(FieldFlags::COMBO, 131072);
        assert_eq!(FieldFlags::EDIT, 262144);
        assert_eq!(FieldFlags::SORT, 524288);
        assert_eq!(FieldFlags::MULTI_SELECT, 2097152);
        assert_eq!(FieldFlags::COMMIT_ON_SEL_CHANGE, 67108864);
    }

    #[test]
    fn no_acroform_is_none() {
        let doc = build_pdf(&[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
            (3, b"<< /Type /Page /Parent 2 0 R >>".to_vec()),
        ]);
        assert!(parse_acroform(&doc).is_none());
    }

    #[test]
    fn javascript_histogram_counts_hooks_and_flags_hazards() {
        // A field with a calculate JS hook (/C) and a keystroke action that
        // is a SubmitForm (network, R12) — recognized, counted, never run.
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[
                (
                    5,
                    b"<< /FT /Tx /T (Total) /Subtype /Widget /Rect [0 0 10 10] \
                      /AA << /C 6 0 R /K 7 0 R >> >>"
                        .to_vec(),
                ),
                (6, b"<< /S /JavaScript /JS (x=1;) >>".to_vec()),
                (7, b"<< /S /SubmitForm /F (http://example.test) >>".to_vec()),
            ],
        );
        let js = scan_javascript(&doc);
        assert_eq!(js.fields_with_calculate_script, 1);
        assert_eq!(js.custom_scripts, 1, "the /C JavaScript action is counted");
        assert_eq!(
            js.fields_with_keystroke_script, 0,
            "the /K action is a SubmitForm, not JavaScript"
        );
        assert_eq!(js.network_action_count, 1, "SubmitForm is an R12 hazard");
        assert_eq!(js.launch_action_count, 0);
        assert!(js.any());
    }

    #[test]
    fn no_javascript_is_all_zero() {
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[(
                5,
                b"<< /FT /Tx /T (Name) /Subtype /Widget /Rect [0 0 10 10] >>".to_vec(),
            )],
        );
        assert!(!scan_javascript(&doc).any());
    }

    #[test]
    fn merged_shape_a_text_field_is_one_object() {
        // Shape A: field + widget merged, /Kids omitted (~88% case).
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] /DA (/Helv 0 Tf 0 g) >>",
            &[(
                5,
                b"<< /FT /Tx /T (Name) /V (Ada) /Subtype /Widget /Rect [10 10 200 30] \
                  /AP << /N 6 0 R >> >>"
                    .to_vec(),
            ),
            (6, b"<< /Type /XObject /Subtype /Form /BBox [0 0 190 20] /Length 0 >>\nstream\n\nendstream".to_vec())],
        );
        let form = parse_acroform(&doc).unwrap();
        assert_eq!(form.fields.len(), 1);
        let f = &form.fields[0];
        assert_eq!(f.fully_qualified_name, "Name");
        assert_eq!(f.field_type, Some(FieldType::Text));
        assert!(f.merged);
        assert_eq!(f.widgets.len(), 1);
        assert_eq!(f.widgets[0].id, ObjId::new(5, 0));
        assert!(f.widgets[0].merged);
        assert!(f.has_appearance());
        assert_eq!(f.value, FieldValue::Text(b"Ada".to_vec()));
        assert!(f.is_fillable());
    }

    #[test]
    fn shape_b_radio_set_has_kid_widgets_and_inherits_ft() {
        // Shape B: separate field dict + /Kids widget array; kids inherit
        // /FT /Btn and the field carries /V naming the selected on-state.
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[
                (
                    5,
                    b"<< /FT /Btn /Ff 32768 /T (Color) /V /Red /Kids [6 0 R 7 0 R] >>".to_vec(),
                ),
                (
                    6,
                    b"<< /Subtype /Widget /Parent 5 0 R /Rect [10 10 30 30] /AS /Red \
                      /AP << /N << /Red 8 0 R /Off 8 0 R >> >> >>"
                        .to_vec(),
                ),
                (
                    7,
                    b"<< /Subtype /Widget /Parent 5 0 R /Rect [40 10 60 30] /AS /Off \
                      /AP << /N << /Blue 8 0 R /Off 8 0 R >> >> >>"
                        .to_vec(),
                ),
                (
                    8,
                    b"<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"
                        .to_vec(),
                ),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        assert_eq!(form.fields.len(), 1);
        let f = &form.fields[0];
        assert_eq!(f.field_type, Some(FieldType::Button));
        assert_eq!(f.button_kind, Some(ButtonKind::Radio));
        assert!(!f.merged);
        assert_eq!(f.widgets.len(), 2);
        assert_eq!(f.value, FieldValue::Name(b"Red".to_vec()));
        // The first widget offers on-state "Red" (Off excluded).
        assert_eq!(f.widgets[0].on_states, vec![b"Red".to_vec()]);
        assert_eq!(f.widgets[1].on_states, vec![b"Blue".to_vec()]);
        assert!(f.is_fillable());
    }

    #[test]
    fn fully_qualified_name_is_the_dotted_parent_path() {
        // §12.7.3.2 example: PersonalData.Address.ZipCode.
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[
                (5, b"<< /T (PersonalData) /Kids [6 0 R] >>".to_vec()),
                (6, b"<< /T (Address) /Kids [7 0 R] >>".to_vec()),
                (
                    7,
                    b"<< /FT /Tx /T (ZipCode) /Subtype /Widget /Rect [0 0 10 10] >>".to_vec(),
                ),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        assert_eq!(form.fields.len(), 1);
        assert_eq!(
            form.fields[0].fully_qualified_name,
            "PersonalData.Address.ZipCode"
        );
        // /FT inherited by neither ancestor here; ZipCode carries its own.
        assert_eq!(form.fields[0].field_type, Some(FieldType::Text));
    }

    #[test]
    fn inheritance_flows_down_kids_via_parent() {
        // A non-terminal carries /FT and /DA for a subtree of terminals with
        // no /T-less-merge; the terminals inherit both.
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] /DA (/Helv 0 Tf 0 g) >>",
            &[
                (
                    5,
                    b"<< /FT /Tx /DA (/Cour 10 Tf) /T (group) /Kids [6 0 R 7 0 R] >>".to_vec(),
                ),
                (
                    6,
                    b"<< /T (a) /Subtype /Widget /Rect [0 0 10 10] >>".to_vec(),
                ),
                (
                    7,
                    b"<< /T (b) /Subtype /Widget /Rect [0 20 10 30] >>".to_vec(),
                ),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        assert_eq!(form.fields.len(), 2);
        for f in &form.fields {
            assert_eq!(f.field_type, Some(FieldType::Text));
            assert_eq!(f.default_appearance.as_deref(), Some(&b"/Cour 10 Tf"[..]));
        }
        assert_eq!(form.fields[0].fully_qualified_name, "group.a");
        assert_eq!(form.fields[1].fully_qualified_name, "group.b");
    }

    #[test]
    fn checkbox_value_and_on_states() {
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[
                (
                    5,
                    b"<< /FT /Btn /T (Urgent) /V /Yes /AS /Yes /Subtype /Widget /Rect [0 0 12 12] \
                      /AP << /N << /Yes 6 0 R /Off 6 0 R >> >> >>"
                        .to_vec(),
                ),
                (
                    6,
                    b"<< /Type /XObject /Subtype /Form /BBox [0 0 12 12] /Length 0 >>\nstream\n\nendstream"
                        .to_vec(),
                ),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        let f = &form.fields[0];
        assert_eq!(f.button_kind, Some(ButtonKind::Check));
        assert_eq!(f.value, FieldValue::Name(b"Yes".to_vec()));
        assert_eq!(f.widgets[0].on_states, vec![b"Yes".to_vec()]);
        assert_eq!(f.widgets[0].appearance_state.as_deref(), Some(&b"Yes"[..]));
    }

    #[test]
    fn pushbutton_has_no_value() {
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[(
                5,
                b"<< /FT /Btn /Ff 65536 /T (Submit) /Subtype /Widget /Rect [0 0 40 20] >>".to_vec(),
            )],
        );
        let form = parse_acroform(&doc).unwrap();
        let f = &form.fields[0];
        assert_eq!(f.button_kind, Some(ButtonKind::Push));
        assert_eq!(f.value, FieldValue::Absent);
        assert!(!f.is_fillable());
    }

    #[test]
    fn choice_options_and_multiselect_value() {
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[(
                5,
                b"<< /FT /Ch /Ff 2097152 /T (Cities) /V [(NYC) (LA)] \
                  /Opt [(NYC) [(la_export) (LA)] (SF)] /Subtype /Widget /Rect [0 0 100 60] >>"
                    .to_vec(),
            )],
        );
        let form = parse_acroform(&doc).unwrap();
        let f = &form.fields[0];
        assert_eq!(f.field_type, Some(FieldType::Choice));
        assert!(f.flags.has(FieldFlags::MULTI_SELECT));
        assert_eq!(
            f.value,
            FieldValue::Choice(vec![b"NYC".to_vec(), b"LA".to_vec()])
        );
        assert_eq!(f.options.len(), 3);
        assert_eq!(f.options[1].export, b"la_export");
        assert_eq!(f.options[1].display, b"LA");
    }

    #[test]
    fn signature_field_value_is_recognized() {
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] /SigFlags 3 >>",
            &[
                (
                    5,
                    b"<< /FT /Sig /T (sig1) /V 6 0 R /Subtype /Widget /Rect [0 0 0 0] >>".to_vec(),
                ),
                (6, b"<< /Type /Sig /Filter /Adobe.PPKLite >>".to_vec()),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        let f = &form.fields[0];
        assert_eq!(f.field_type, Some(FieldType::Signature));
        assert_eq!(f.value, FieldValue::Signature);
        // Zero-area /Rect is intentional invisibility, not a None.
        assert!(f.widgets[0].rect.is_some());
        assert_eq!(f.widgets[0].rect.unwrap().width(), 0.0);
        assert!(!f.is_fillable());
        assert!(form.signatures_exist);
        assert!(form.append_only);
    }

    #[test]
    fn need_appearances_and_xfa_and_co_are_detected() {
        let doc = doc_with_acroform(
            "<< /Fields [] /NeedAppearances true /CO [5 0 R] \
              /XFA [(template) 7 0 R (datasets) 8 0 R] /DR << /Font << >> >> >>",
            &[
                (5, b"<< /FT /Tx /T (calc) >>".to_vec()),
                (7, b"<< /Length 0 >>\nstream\n\nendstream".to_vec()),
                (8, b"<< /Length 0 >>\nstream\n\nendstream".to_vec()),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        assert!(form.need_appearances);
        assert_eq!(form.calc_order_count, 1);
        assert!(form.has_default_resources);
        assert_eq!(form.xfa, XfaPresence::PacketArray { packets: 2 });
        assert!(form.xfa.is_present());
    }

    #[test]
    fn cyclic_kids_terminates() {
        // A /Kids self-cycle must not loop (visited-id guard).
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[(5, b"<< /T (loop) /Kids [5 0 R] >>".to_vec())],
        );
        // Node 5 references itself as a child field; the visited set breaks
        // the cycle and no terminal is produced (it is non-terminal with a
        // single child that is itself, already visited).
        let form = parse_acroform(&doc).unwrap();
        assert!(form.fields.is_empty());
    }

    #[test]
    fn t_less_widget_kids_are_not_child_fields() {
        // Two /T-less widget kids of a named checkbox field are widgets, not
        // separate fields — one logical field, two representations.
        let doc = doc_with_acroform(
            "<< /Fields [5 0 R] >>",
            &[
                (
                    5,
                    b"<< /FT /Btn /T (Agree) /V /Off /Kids [6 0 R 7 0 R] >>".to_vec(),
                ),
                (
                    6,
                    b"<< /Subtype /Widget /Parent 5 0 R /Rect [0 0 12 12] /AS /Off \
                      /AP << /N << /Yes 8 0 R /Off 8 0 R >> >> >>"
                        .to_vec(),
                ),
                (
                    7,
                    b"<< /Subtype /Widget /Parent 5 0 R /Rect [0 20 12 32] /AS /Off \
                      /AP << /N << /Yes 8 0 R /Off 8 0 R >> >> >>"
                        .to_vec(),
                ),
                (
                    8,
                    b"<< /Type /XObject /Subtype /Form /BBox [0 0 12 12] /Length 0 >>\nstream\n\nendstream"
                        .to_vec(),
                ),
            ],
        );
        let form = parse_acroform(&doc).unwrap();
        assert_eq!(
            form.fields.len(),
            1,
            "one field, two widgets — not two fields"
        );
        assert_eq!(form.fields[0].widgets.len(), 2);
    }

    // -----------------------------------------------------------------
    // `/Ff` bit 26 — the ONLY overloaded bit position in the whole family.
    // -----------------------------------------------------------------

    /// A radio group with bit 26 set is `RadiosInUnison`, **not** rich text.
    ///
    /// This is the test the whole `is_rich_text`/`radios_in_unison` pair
    /// exists for. `flags.has(FieldFlags::RICH_TEXT)` on this field returns
    /// TRUE — the bit really is set — and every consumer that asks the flag
    /// word directly gets a wrong answer that compiles. Only a question that
    /// has the resolved `/FT` in hand can tell the two apart.
    ///
    /// Non-vacuous by construction: it asserts the raw bit IS set, so it
    /// cannot pass by the flag accidentally being absent.
    #[test]
    fn bit_26_on_a_radio_group_is_radios_in_unison_not_rich_text() {
        let radio = Field {
            id: ObjId::new(1, 0),
            fully_qualified_name: "Choice".to_owned(),
            partial_name: None,
            alternate_name: None,
            rich_value: None,
            mapping_name: None,
            field_type: Some(FieldType::Button),
            button_kind: Some(ButtonKind::Radio),
            flags: FieldFlags(FieldFlags::RADIO | FieldFlags::RADIOS_IN_UNISON),
            value: FieldValue::Absent,
            default_value: FieldValue::Absent,
            default_appearance: None,
            quadding: Quadding::Left,
            max_len: None,
            options: Vec::new(),
            top_index: 0,
            selected_indices: Vec::new(),
            widgets: Vec::new(),
            merged: false,
            has_additional_actions: false,
            shares_parent_name: false,
            parent: None,
        };
        assert!(
            radio.flags.has(FieldFlags::RICH_TEXT),
            "precondition: the RAW bit is set — this is exactly why the bare              flag test is unsafe, and asserting it keeps this test honest"
        );
        assert!(
            !radio.is_rich_text(),
            "a RADIO GROUP must never be reported as a rich-text field"
        );
        assert!(
            radio.radios_in_unison(),
            "bit 26 on a /Btn IS RadiosInUnison"
        );
    }

    /// The mirror: bit 26 on a text field is rich text, and is NOT reported
    /// as `RadiosInUnison`.
    #[test]
    fn bit_26_on_a_text_field_is_rich_text_not_radios_in_unison() {
        let text = Field {
            id: ObjId::new(2, 0),
            fully_qualified_name: "Notes".to_owned(),
            partial_name: None,
            alternate_name: None,
            rich_value: None,
            mapping_name: None,
            field_type: Some(FieldType::Text),
            button_kind: None,
            flags: FieldFlags(FieldFlags::RICH_TEXT),
            value: FieldValue::Absent,
            default_value: FieldValue::Absent,
            default_appearance: None,
            quadding: Quadding::Left,
            max_len: None,
            options: Vec::new(),
            top_index: 0,
            selected_indices: Vec::new(),
            widgets: Vec::new(),
            merged: false,
            has_additional_actions: false,
            shares_parent_name: false,
            parent: None,
        };
        assert!(text.is_rich_text());
        assert!(!text.radios_in_unison());
    }

    /// A SIGNATURE field with bit 26 set is neither.
    ///
    /// `/Sig` has **no type-specific flag table at all** (Table 232 adds only
    /// `/Lock` and `/SV`), so bits 4–32 there are reserved and a set bit is
    /// malformed rather than meaningful. Surfaced as "neither", never
    /// silently decoded against some other type's table.
    #[test]
    fn bit_26_on_a_signature_field_is_neither_meaning() {
        let sig = Field {
            id: ObjId::new(3, 0),
            fully_qualified_name: "Sig1".to_owned(),
            partial_name: None,
            alternate_name: None,
            rich_value: None,
            mapping_name: None,
            field_type: Some(FieldType::Signature),
            button_kind: None,
            flags: FieldFlags(FieldFlags::RICH_TEXT),
            value: FieldValue::Absent,
            default_value: FieldValue::Absent,
            default_appearance: None,
            quadding: Quadding::Left,
            max_len: None,
            options: Vec::new(),
            top_index: 0,
            selected_indices: Vec::new(),
            widgets: Vec::new(),
            merged: false,
            has_additional_actions: false,
            shares_parent_name: false,
            parent: None,
        };
        assert!(!sig.is_rich_text());
        assert!(!sig.radios_in_unison());
    }
}
