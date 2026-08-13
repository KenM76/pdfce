//! Persisted operator settings — the R15 user-state partition.
//!
//! # Why this module exists, and why it exists *now*
//!
//! Two standing obligations converge here, one old and one new.
//!
//! **R15 (decision 003 §5.6, §6.1)** says the distribution folder is
//! partitioned from the start: *"Replaceable payload and user state are
//! separate; user state never sits loose among the binaries; the
//! documented update procedure names exactly which files to keep. Binding
//! from the first Pass that persists anything."* pdfce's update story is
//! **manual replace-the-folder**, and replacing a folder destroys
//! everything in it — so the moment pdfce writes a settings file into the
//! program directory, a routine update silently wipes the operator's
//! configuration. Decision 003 required this decided *before* the first
//! Pass that persists anything, precisely so it would never need
//! retrofitting onto existing users' state.
//!
//! **The 2026-08-08 operator directive** is what finally made something
//! need persisting: *"where standards are ambiguous those should become
//! settings that the user can choose direction one, with the initial
//! installed default as the best guess of what is usually followed."* The
//! spec RAG's ambiguity register triages **18** such settings out of 155
//! recorded findings, and **10 of the 18 are already hard-coded in shipped
//! source** — pdfce has silently picked a side ten times. A setting that
//! forgets itself at restart is worse than no setting, so the store comes
//! first.
//!
//! It is also the shared prerequisite for three *other* operator asks that
//! have nothing to do with the spec: a fully customizable ribbon with
//! saveable layout configurations, mouse/keyboard bindings with saveable
//! configurations, and dock-layout persistence (`crate::` has no view of
//! that last one — see `pdfce-gui`'s `dock.rs`, which states outright that
//! nothing is written to disk and that serializing the dock tree is the
//! natural mechanism *"when R15 lands"*). Four asks, one missing
//! component.
//!
//! # Where the file lives
//!
//! `<directory of the running executable>/userdata/settings.txt`.
//!
//! Decision 003 wrote the folder as a literal `<user-state>` placeholder
//! and never named it — the README sentence it drafted reads *"replace the
//! program files (keep your `<user-state>` folder)"*. **`userdata` is that
//! name**, chosen here because it reads correctly in exactly that
//! sentence, is self-describing to someone who has never read the docs,
//! and is the convention portable Windows applications already use.
//!
//! ## The read-only-install fallback, and why it is disclosed
//!
//! `ARCHITECTURE.md` §6 requires pdfce to *"run read-only-folder-clean"* —
//! an operator may put the program on a read-only share or in
//! `Program Files`. So when `userdata/` cannot be created or written,
//! [`resolve_store`] falls back to the platform configuration directory
//! and **says which one it used** ([`StoreLocation::kind`]).
//!
//! The disclosure is not decoration. The two locations behave differently
//! on update — the portable one is the operator's to preserve, the
//! platform one survives a folder replace by itself — so an operator who
//! does not know which one is live cannot follow the update instructions
//! correctly. This is the fuzzy-never-sneaky rule applied to a decision
//! pdfce made on the operator's behalf: pdfce inferred a location, so the
//! inference is visible.
//!
//! # The format, and why it is not TOML or JSON
//!
//! A flat, line-oriented `key = value` text file with `#` comments.
//!
//! The obvious move is `serde` plus `toml`. It was rejected, and the
//! reason is a requirement rather than a preference: **§7's fail-soft
//! contract is per-key, and derived deserialization is per-document.** A
//! `serde` derive presented with one unknown key, one misspelled enum
//! variant, or one out-of-range number fails the *whole* file, which would
//! discard every setting the operator got right because of the one they
//! got wrong — on a file they are explicitly invited to hand-edit. Writing
//! per-field recovery on top of `serde` means fighting it with
//! `#[serde(default)]` on every field plus a custom deserializer per
//! enum, which is more code than the twenty-line grammar below and still
//! cannot report *which line* was wrong.
//!
//! The grammar is small enough to state completely:
//!
//! ```text
//! line    := comment | blank | entry
//! comment := ws* '#' .*
//! blank   := ws*
//! entry   := ws* key ws* '=' ws* value ws*
//! key     := [A-Za-z0-9_.]+
//! value   := .*            (trailing whitespace trimmed; not unquoted)
//! ```
//!
//! No sections, no nesting, no escapes, no quoting. Values that would need
//! any of those do not belong in this file — a ribbon layout or a keymap
//! is a *document*, not a setting, and gets its own file under the same
//! `userdata/` roof rather than being crammed into this grammar.
//!
//! # The fail-soft contract (§7, A.6, R82)
//!
//! Nothing in this module returns an error to a caller who merely wants to
//! know what the settings are. [`Settings::load`] **always** produces
//! usable settings. Every departure from the file's literal content is
//! recorded as a [`SettingNote`] the front end can show:
//!
//! | Situation | Result |
//! |---|---|
//! | No file at all | Every default. **No note** — a first run is not a fault. |
//! | Directory unreachable / unreadable file | Every default, [`SettingNote::Unreadable`]. |
//! | Unknown key | Other keys still applied, [`SettingNote::UnknownKey`]. |
//! | Unparseable value | That key defaults, [`SettingNote::BadValue`] naming the value used. |
//! | Value out of range | Clamped, [`SettingNote::Clamped`]. |
//! | Malformed line (no `=`) | Skipped, [`SettingNote::Malformed`]. |
//! | Duplicate key | **Last wins**, [`SettingNote::Duplicate`]. |
//!
//! A missing file is deliberately silent and everything else is not. The
//! distinction is whether the operator did something: a first run is the
//! expected state, whereas a typo in a file they edited is a thing they
//! want told about *at the line number*, not discovered later by noticing
//! pdfce behaves oddly.
//!
//! **Never an error dialog, never a lost document session** — `dock.rs`'s
//! wording, and it binds here because this store is what `dock.rs` was
//! waiting for. A configuration problem must not be able to stop pdfce
//! opening a file.
//!
//! # What this module does not do
//!
//! - **It does not decide defaults.** Each default lives with the type it
//!   belongs to ([`Default`] impls elsewhere in the crate), so there is
//!   exactly one answer to "what does pdfce do by default?" and it is not
//!   in a settings file. This module reads and writes; it does not define.
//! - **It does not watch the file.** Settings are read when asked. A file
//!   watcher would make the live configuration depend on when an editor
//!   happened to flush, which is a source of irreproducible behaviour, not
//!   a feature.
//! - **It does not write on exit.** [`Settings::save`] is called
//!   deliberately, so a crash cannot persist half a session's accidental
//!   state, and an operator's hand-edited file is never rewritten behind
//!   their back with pdfce's own formatting.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::pageops::separation::SeparationPolicy;

/// File name of the settings file inside the user-state directory.
pub const SETTINGS_FILE: &str = "settings.txt";

/// Name of the user-state directory beside the executable.
///
/// The name decision 003 left as a `<user-state>` placeholder. It appears
/// verbatim in the update instructions, so changing it is a documentation
/// change and a migration, not a rename.
pub const USER_STATE_DIR: &str = "userdata";

/// Which of the two possible homes the settings file is actually using.
///
/// Surfaced rather than hidden because the operator's update procedure
/// differs between them — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StoreKind {
    /// `<exe dir>/userdata/` — the intended, portable location. The
    /// operator keeps this folder across an update.
    Portable,
    /// The platform configuration directory, used because the portable
    /// location was not writable (a read-only share, `Program Files`
    /// without elevation). Survives a folder replace on its own, and is
    /// **not** portable — it does not travel with the program folder.
    PlatformFallback,
    /// No writable location was found at all. Settings still load from
    /// defaults and the session works; saving will report why it cannot.
    None,
}

/// Where the settings file is, and how that was decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLocation {
    /// The settings file's full path, absent only for [`StoreKind::None`].
    pub path: Option<PathBuf>,
    /// Which home this is.
    pub kind: StoreKind,
}

impl StoreLocation {
    /// The directory holding the settings file, if there is one.
    #[must_use]
    pub fn directory(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }
}

/// One thing that happened while loading which the operator may want to
/// know about.
///
/// Every variant names the **line** where possible, because the whole
/// point of a hand-editable file is that a mistake in it is findable. A
/// note that says "a value was wrong" without saying which line is a note
/// that makes the operator re-read the entire file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SettingNote {
    /// The file exists but could not be read.
    Unreadable {
        /// The path that failed.
        path: PathBuf,
        /// The operating system's reason, already rendered.
        reason: String,
    },
    /// A key pdfce does not recognise. Left alone, never deleted — it may
    /// belong to a newer version the operator also runs from the same
    /// folder.
    UnknownKey {
        /// The key as written.
        key: String,
        /// 1-based line number.
        line: usize,
    },
    /// A known key whose value could not be interpreted.
    BadValue {
        /// The key.
        key: String,
        /// The value as written.
        value: String,
        /// 1-based line number.
        line: usize,
        /// What pdfce used instead, already rendered.
        using: String,
    },
    /// A numeric value outside the range the setting accepts.
    Clamped {
        /// The key.
        key: String,
        /// The value as written.
        value: String,
        /// 1-based line number.
        line: usize,
        /// The value actually used, already rendered.
        using: String,
    },
    /// A line that is neither blank, a comment, nor `key = value`.
    Malformed {
        /// 1-based line number.
        line: usize,
    },
    /// The same key set more than once. The last occurrence wins, which
    /// is the behaviour that makes appending to the file work.
    Duplicate {
        /// The key.
        key: String,
        /// 1-based line number of the occurrence that won.
        line: usize,
    },
}

/// Everything [`Settings::load`] wants to tell the caller.
///
/// Separate from [`Settings`] so that the settings themselves stay a plain
/// value with no diagnostic baggage: a caller that only wants to know what
/// the operator chose does not carry the story of how it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// Where the file was looked for, and which home that is.
    pub location: StoreLocation,
    /// Whether a file was actually found.
    ///
    /// `false` on a first run, which is **not** a fault and produces no
    /// note.
    pub existed: bool,
    /// Everything worth telling the operator, in file order.
    pub notes: Vec<SettingNote>,
}

impl LoadReport {
    /// Whether anything at all needs saying.
    #[must_use]
    pub fn is_quiet(&self) -> bool {
        self.notes.is_empty()
    }
}

/// How `DeviceCMYK` is converted for display (ISO 32000-1 §8.6.4.4).
///
/// §8.6.4.4 mandates **no** conversion at all — it is device-dependent by
/// definition — so there is no correct answer to appeal to, and Acrobat's
/// own answer is a user-configurable working-space profile. That makes
/// this the textbook case for R169: the standard is silent, so the choice
/// is the operator's.
///
/// # The default is an OPERATOR RULING, and knowingly diverges
///
/// R169 says a shipped default should be "the best guess of what is
/// usually followed", and by that rule the default would be
/// [`Self::Calibrated`] — it is what Acrobat's shipped profile and pdfium
/// both produce, which is tier-(a)/(c) evidence, the strongest in the
/// whole ambiguity register.
///
/// **The default is [`Self::NeutralBlack`] anyway**, by Ken's explicit
/// ruling of 2026-08-08 ("flip it") once he saw what the calibrated
/// answer does to pure-K line art. This is recorded as a *divergence*
/// rather than quietly relabelled as the consensus, because the two are
/// different claims and a future session must not read this default as
/// evidence of what other readers do. It is the reference-exceeding case
/// from the other side: matching the reference is a floor, not an
/// obligation, and here the operator judged the floor wrong for the
/// documents he actually opens.
///
/// The divergence is also **narrow by construction** — only the pure-K
/// axis moves; every mixed colour still uses the calibrated table — so
/// what is given up is agreement on black line art specifically, not
/// colour fidelity generally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CmykIntent {
    /// The calibrated table in [`crate::color`] — agreement with the
    /// SWOP-family rendering that Acrobat's default profile and pdfium
    /// both produce.
    ///
    /// **Not the shipped default, despite being the best-evidenced
    /// answer** — see the type docs. Its visible consequence is that
    /// solid black ink (`0 0 0 1 k`) renders `#231F20` rather than
    /// `#000000`, and mid greys are slightly cool. Choose this when the
    /// question is *"what will this look like in Acrobat?"* — proofing a
    /// document for someone else's screen, or checking a render-parity
    /// difference.
    Calibrated,
    /// As [`Self::Calibrated`], except that pure black — `C = M = Y = 0`
    /// with any `K` — is forced to a neutral grey of `1 − K`, so pure-K
    /// line art renders `#000000`.
    ///
    /// **The shipped default, by operator ruling** (see the type docs).
    /// For CAD and engineering drawings, where every line is stroked in
    /// pure K and true black on white is the expectation — which is the
    /// document population this project's operator actually works with.
    #[default]
    NeutralBlack,
    /// The naive additive formula pdfce used before the calibration —
    /// `1 − min(1, x + k)` per channel.
    ///
    /// Kept because it is what every pdfce-rendered image before this
    /// change looked like, so an operator comparing against an old export
    /// has a way to reproduce it. Not recommended: it misses the reference
    /// by up to 37/255 per channel.
    Naive,
}

/// Which filter resamples a `/SMask` or explicit `/Mask` whose pixel grid
/// differs from its base image's (spec ambiguity `SM-A1`).
///
/// # The silence being filled
///
/// ISO 32000-1 fixes the **geometry** and says nothing about the
/// **filter**. Table 145's `Width` row, verbatim: *"Both images shall be
/// mapped to the unit square in user space (as are all images),
/// **regardless of whether the samples coincide individually**."* §8.9.6.3
/// says the same for an explicit mask (*"need not have the same
/// resolution … their boundaries on the page will coincide"*).
///
/// The spec RAG records the sourced negatives that establish the silence
/// is real rather than merely unfound (`iso32000__s__11.6.5.md` § SM-A1):
/// over the whole 756-page source, `resample*` **0 hits**,
/// `nearest neigh*` **0 hits**, `bilinear` **3 hits, none image-related**.
/// §8.9.5.3's NOTE then grants a conforming reader *"any specific
/// implementation of interpolation that it wishes"*.
///
/// # Default: [`Self::Nearest`] — **EVIDENCE TIER (d)**
///
/// Tier (d) is the register's vocabulary for **reasoned inference only —
/// this is a guess and is written as one**. No tier-(a)/(b)/(c) evidence
/// exists: `Acrobat_Features` does not cover mask resampling, no census
/// has been run, and no other implementation's documented behaviour was
/// located. The reasoning (good, but still reasoning) is that
/// nearest-neighbour is the only filter that cannot invent an alpha value
/// appearing nowhere in the mask — decisive for a 1-bit stencil supplied
/// as an `/SMask`, where a blend across a 0/1 edge fabricates
/// half-transparent texels the document never asked for.
///
/// Do not read this default as evidence of what other readers do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MaskResample {
    /// Take the single mask sample containing the base texel's centre.
    ///
    /// **The shipped default.** Never invents an alpha; preserves a
    /// stencil's hard edges exactly. Aliases (staircases) when a small
    /// mask is stretched over a large base image.
    #[default]
    Nearest,
    /// Average every mask sample the base texel's footprint covers.
    ///
    /// The right answer when the mask is *higher* resolution than the base
    /// image, where nearest-neighbour throws away most of the mask: a
    /// 4× mask read one-sample-per-texel discards fifteen sixteenths of
    /// what the producer supplied. Degenerates to [`Self::Nearest`] when
    /// the footprint covers one sample.
    BoxAverage,
    /// Interpolate linearly between the four mask samples nearest the base
    /// texel's centre.
    ///
    /// Smooth on magnification, which is what makes it the wrong default:
    /// across a stencil's 0↔255 boundary it manufactures intermediate
    /// alphas. Offered for a continuous-tone `/SMask` (a photographic
    /// vignette) supplied at lower resolution than its base image, which
    /// is the case it is actually good at.
    Bilinear,
}

/// How an image XObject is sampled when it is drawn **smaller** than its
/// own pixel grid (spec ambiguity `IM-A1`).
///
/// # The silence being filled
///
/// §8.9.5.3 (*Image Interpolation*) defines interpolation **only for
/// magnification** — *"When the resolution of a source image is
/// significantly **lower** than that of the output device …"* — and its
/// NOTE grants a reader leave to *"not implement this feature"* or to
/// *"use any specific implementation of interpolation that it wishes"*.
///
/// It says nothing at all about minification. Term-frequency evidence over
/// the source (`iso32000__ref__ambiguity_settings_register.md` §5.5):
/// `minif` **0 hits**, `mipmap` **0**, `decimat` **0**, `down-sampl` **0**,
/// `downsampl` **2 hits, both unrelated** (multimedia rate conversion in
/// clause 13; the thumbnail note in §8.9.5.4). So `/Interpolate false`
/// does **not** mandate point-sampling on the way *down* — it switches off
/// the *up*-scaling smoothing the clause actually defines, and a reader
/// minifying an image is unconstrained.
///
/// # Default: [`Self::PointSample`] — **EVIDENCE TIER (d)**
///
/// Tier (d): reasoned inference only, i.e. **a guess**. The register
/// deliberately declines to recommend the flip despite pdfce's own
/// `interpret.rs` asserting *"Most production viewers smooth on
/// minification regardless of `/Interpolate`"* — **that assertion is
/// unverified**, it is exactly the shape of claim the claim-bearing-copy
/// rule targets, and moving a default onto it would be churn dressed as
/// research. A viewer-behaviour check filed to `C:\personal_rag\pdf\`
/// would raise this to tier (c) and, if it confirms, flip the default.
/// Until then the status quo stands and is labelled a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MinifyFilter {
    /// Take one texel per output pixel, in both directions — treat
    /// `/Interpolate` as the only switch there is.
    ///
    /// **The shipped default.** Spec-literal, and it is what makes a 2×2
    /// test image's pixels exactly assertable. Its cost is aliasing
    /// (shimmer, dropped hairlines) on a heavily downscaled image.
    #[default]
    PointSample,
    /// Smooth when the image is drawn smaller than its pixel grid, while
    /// still honouring `/Interpolate` on the way up.
    ///
    /// Removes the aliasing at the price of a departure from the clause's
    /// stated switch — which is legitimate precisely because the clause
    /// never legislated this direction.
    Smooth,
}

/// How to read a four-component `DCTDecode` image that declares no
/// `/Decode` array (spec ambiguity `DCT-A1`).
///
/// # The question
///
/// A CMYK JPEG with **effective `ColorTransform` 0** and **no `/Decode`**:
/// are the stored samples direct CMYK, or Adobe-complemented CMYK? Nothing
/// in the codestream or the image dictionary disambiguates it — the
/// undocumented 1990s Photoshop convention stores complemented values, and
/// there is no marker bit that says so.
///
/// # Default: [`Self::NeverInvert`] — **EVIDENCE TIER (c)**
///
/// Tier (c) means *what other major implementations do, as documented* —
/// and this is the **strongest-sourced default in the whole ambiguity
/// register**, the one place it is not a guess:
///
/// - the word `"invert"` occurs **zero times** in Adobe TN #5116, the
///   document ISO 32000-1 §7.4.8 footnote *a* makes normative by
///   reference (verified 2026-07-31);
/// - **APP14 carries no polarity flag** — there is no bit to test, so
///   "invert when the marker is present" keys off mere presence;
/// - `filter__dct.md` records that all four reference engines accept the
///   ambiguity rather than inverting on APP14 presence.
///
/// This is also pdfce's standing rule **R29** (decision 006), and the
/// residual risk is already disclosed rather than repaired by
/// [`crate::image_codec::CodecNotes::cmyk_polarity_unverifiable`] (R30).
/// The setting adds the operator's escape hatch; it does not weaken R29,
/// which remains what pdfce does unless the operator says otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CmykJpegPolarity {
    /// Take the samples as stored. `/Decode` is the sole polarity control
    /// (`/Decode [1 0 1 0 1 0 1 0]` *is* the sanctioned way for a producer
    /// to declare inverted storage).
    ///
    /// **The shipped default**, and the standing rule.
    #[default]
    NeverInvert,
    /// Complement all four components (`255 − x`) when the codestream
    /// carries an Adobe APP14 marker, the effective transform is 0, and
    /// the image dictionary declares no `/Decode`.
    ///
    /// For a library of old Photoshop-authored CMYK JPEGs that genuinely
    /// do store complemented ink and say so nowhere. Getting this wrong in
    /// either direction renders a photographic negative — which is at
    /// least an obvious failure, not a subtle one.
    InvertOnApp14,
}

/// What character extraction emits for a code no rung of the §9.10.2
/// ladder could map (spec ambiguity `TX-A1`).
///
/// # The silence being filled
///
/// §9.10.2's failure clause is *grammatically broken* — it says a
/// conforming reader *"may choose a character code of their choosing"*
/// where a **Unicode value** is what is being produced — and **no
/// sentinel is specified anywhere in the standard**: not U+FFFD, not
/// omission, not a placeholder.
///
/// # Default: [`Self::ReplacementChar`] — **EVIDENCE TIER (d)**
///
/// Tier (d): reasoned inference only — **a guess**. The reasoning is that
/// U+FFFD is the only option that is simultaneously length-preserving
/// *and* visibly wrong, which is what rule 4 wants; omission silently
/// shortens the text and makes the failure invisible. No census, no
/// Acrobat citation, no documented third-party behaviour backs it.
///
/// # This is an EXTRACT-radius setting, which makes it a correctness knob
///
/// Downstream of extraction sit search, clipboard copy, **and
/// redaction-by-text**. Changing the sentinel changes character offsets,
/// therefore changes which runs a redaction pattern matches (**R35**). A
/// redaction built under one value is not equivalent under another.
/// Whatever is chosen, the rung-4 counter keeps counting — that counter is
/// documented as *"the headline honesty metric"* and the setting must not
/// be able to switch it off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum UnmappableCode {
    /// U+FFFD REPLACEMENT CHARACTER, one per unmappable code.
    ///
    /// **The shipped default.** Length-preserving and visibly wrong.
    #[default]
    ReplacementChar,
    /// `?`, one per unmappable code.
    ///
    /// Also length-preserving, but it survives being pasted into tools
    /// that mangle U+FFFD, and it reads as a question rather than as a
    /// font problem. It is *less* honest than U+FFFD in one specific way:
    /// a genuine `?` in the document is indistinguishable from a failure.
    QuestionMark,
    /// Nothing at all — the code contributes no characters.
    ///
    /// The failure is still counted (`ladder_failures`), so it is never
    /// hidden from the operator; only the text is shorter, and the
    /// shortening is invisible **in the text itself**. Choose this when
    /// the extracted text is being fed to something that chokes on
    /// sentinels.
    ///
    /// **Two consequences worth knowing before choosing it**, both
    /// measured rather than assumed:
    ///
    /// 1. **Character offsets move**, so a search hit and a
    ///    redaction-by-text match land in different places than they do
    ///    under the other two values (R35). That is true of any change
    ///    here, but `omit` is the one that changes them the most.
    /// 2. **A run whose codes are ALL unmappable disappears entirely** —
    ///    glyph records included. The layout pass drops a run with no
    ///    characters (it has nothing a caller can index into), so under
    ///    `omit` a page of `Identity-H` text with no `/ToUnicode` yields
    ///    zero runs rather than runs of sentinels. A caller that needs
    ///    per-glyph positions for unmappable codes must not choose this.
    ///    Pinned by
    ///    `the_unmappable_sentinel_changes_the_characters_but_never_the_count`.
    ///
    /// **Scope: extraction output only.** Three internal paths pin the
    /// sentinel to [`Self::ReplacementChar`] regardless, because in each
    /// of them a zero-length character would break something structural
    /// rather than merely look different — the text-editing slot table
    /// (a zero-length span is a glyph the operator can see and cannot
    /// address), the redaction audit record (which must not report a
    /// removal as nothing), and the vector-object text preview (which must
    /// not make an undecodable run look empty). Each site says so at the
    /// call.
    Omit,
}

/// Whether `/ActualText` replaces the glyph-derived characters
/// (spec ambiguity `AT-A1`).
///
/// # The disagreement being resolved
///
/// Three statements in ISO 32000-1 do not agree, and none dislodges the
/// others:
///
/// - **§14.9.4**: `/ActualText` *"shall be used as a replacement"* — the
///   only **`shall`** in the set.
/// - **§14.8.2.4.2 NOTE 2**: readers *"may choose to use"* it, and *"some
///   conforming readers"* do — a `may`, inside an **informative NOTE**.
/// - **§9.10.1**: it *"may be used"*.
///
/// The only sentence that addresses precedence is the `may`, and it sits
/// in a NOTE, so neither reading can be eliminated from the standard.
///
/// # Default: [`Self::Always`] — **EVIDENCE TIER (d)**
///
/// Tier (d) — **a guess**, though the best-supported guess available:
/// §14.9.4's is the only `shall`, and its competitors are a NOTE and a
/// `may`. Per the standing normative-vs-informative rule, the NOTE is
/// **not** cited alone as authority anywhere in the code.
///
/// # A bound that is NOT a setting
///
/// **No length correspondence exists** between `/ActualText` and the
/// content it replaces — the standard's own example maps two shown
/// characters to one. Character-level mapping back to glyph positions is
/// therefore *impossible* across an `/ActualText` run, which bounds
/// search-highlight, selection and redaction-by-text to **sequence**
/// granularity whichever value is chosen. That is a fact to disclose, not
/// a direction to pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ActualTextPrecedence {
    /// `/ActualText` replaces the glyphs it covers, wherever it appears.
    ///
    /// **The shipped default.** §14.9.4's `shall`, applied literally.
    #[default]
    Always,
    /// `/ActualText` replaces the glyphs only when the marked-content
    /// sequence carrying it is part of the structure tree.
    ///
    /// "Part of the structure tree" is tested as **an `/MCID` in scope** —
    /// on the sequence itself or on an enclosing one. That is the only
    /// test available inside a content stream: `/MCID` is precisely what
    /// §14.7.4.2 uses to join a marked-content sequence to a structure
    /// element, so a sequence without one in scope is not tagged content
    /// in any sense the page itself can express. Elsewhere the glyphs win.
    ///
    /// Choose this when a producer sprinkles `/ActualText` outside its
    /// tagged content and the replacements are worse than the glyphs.
    TaggedOnly,
    /// The glyphs always win; `/ActualText` is counted and reported but
    /// never substituted.
    ///
    /// The forensic setting: what is extracted is what the page draws.
    /// Note that this **loses** genuinely unrecoverable text — a ligature
    /// whose only Unicode identity was in its `/ActualText` extracts as
    /// whatever the ladder makes of the glyph, which may be U+FFFD.
    Glyphs,
}

/// What to paint for an annotation whose `/AP` `/N` is a subdictionary of
/// two or more entries and which carries **no `/AS`**
/// (spec ambiguity `AS-A1`).
///
/// # The gap being filled
///
/// Table 164 makes `/AS` *required* in exactly that configuration, so such
/// a file is **malformed**. §12.5.5 NOTE 3 covers only the neighbouring
/// case — `/AS` present but naming an absent state — and states no
/// recovery for `/AS` being absent altogether.
///
/// A single-entry subdictionary is **not** covered by this setting and
/// never was: with one entry there are no alternatives to choose between,
/// so painting it is not a guess. The forbidden case is specifically the
/// multi-entry one.
///
/// # Default: [`Self::PaintNothing`] — **EVIDENCE TIER (d)**
///
/// Tier (d) — **a guess**, and deliberately the conservative one. The spec
/// RAG's row is explicit that the other two options are *empirical*
/// guesses belonging to `C:\personal_rag\pdf\`: *"do NOT silently pick
/// first/`Off`/`On`."* Offering them as opt-ins is legitimate; making one
/// the installed default would be exactly the "sneaky" failure rule 4
/// forbids, because the operator would see a plausible appearance with no
/// indication that pdfce chose it.
///
/// Whatever is chosen, the case stays **counted** — pdfce never repairs
/// the file by writing an `/AS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MissingAppearanceState {
    /// Paint nothing, and count the annotation as state-unresolved.
    ///
    /// **The shipped default.**
    #[default]
    PaintNothing,
    /// Paint the subdictionary's first entry in key order.
    ///
    /// "First" is the dictionary's own iteration order, which pdfce
    /// preserves from the file, so this is *the producer's* first entry
    /// and not an alphabetical invention.
    FirstEntry,
    /// Paint the `/Off` entry if there is one, otherwise nothing.
    ///
    /// The checkbox-shaped guess: for a widget the unchecked state is the
    /// one that misleads least if it is wrong.
    OffElseNothing,
}

/// Which of §7.5.4's three permitted two-byte terminators ends a classic
/// cross-reference **entry** (spec ambiguity `EOL-A1`).
///
/// # The choice being made
///
/// §7.5.4 fixes the entry at exactly 20 bytes and permits three, and only
/// three, forms for bytes 18–19. `LF CR`, a bare `LF`, a bare `CR`,
/// `SP SP` and `SP CR LF` are **not** legal and are deliberately not
/// offered here — a settings file is not a licence to emit a
/// non-conforming file.
///
/// # Default: [`Self::MatchSource`] — the register's own recommendation
///
/// **Changed on the operator's ruling of 2026-08-08** ("change the shipped
/// default so that we match the file's existing 2-byte EOL"), replacing a
/// fixed `SP LF`.
///
/// `iso32000__ref__ambiguity_settings_register.md` §5.11 recommended
/// exactly this and pdfce shipped the fixed form anyway, because
/// implementing "match the source" needed an observation of the base
/// file's bytes that no channel carried. The register said plainly that
/// the shipped default was *"arguably wrong on pdfce's own invariant"*,
/// and it was right: **rule 3 says objects pdfce did not logically touch
/// are re-emitted byte-identical, and a full rewrite of a `CR LF` file
/// under a fixed `SP LF` changes two bytes in every entry of the table.**
/// On a 5,000-object file that is a 10,000-byte diff in a document nobody
/// edited — the exact diff minimal-diff editing exists to prevent.
///
/// The channel now exists: [`crate::xref::observed_entry_eol`] reads the
/// form back out of the base file, and the writer resolves
/// [`Self::MatchSource`] against it. This is the same idea
/// `Document::section_shape` already served at a coarser grain — *the base
/// file's own form* (R33) — one level finer.
///
/// **Evidence tier is no longer (d)-shaped at all**, which is the quiet
/// win here. The old default rested on the RAG's uncited claim that
/// `SP LF` is *"the common choice"* — flagged in the register's §11.3 as
/// carrying no source and pending a downgrade. The new default rests on
/// nothing external: it derives the answer from the file in front of it,
/// so there is no guess left to grade. The uncited claim now governs only
/// the fallback, where there is genuinely nothing to match.
///
/// **BYTES blast radius, zero render effect.** Every value is conforming,
/// so no operator disclosure is needed when one is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum XrefEntryEol {
    /// Write whichever of the three forms the file being saved already
    /// used, falling back to `SP LF` when there is nothing to match.
    ///
    /// **The shipped default.** "Nothing to match" means a
    /// cross-reference *stream* file (§7.5.8 is binary and has no entry
    /// EOL), a file whose table is non-conforming at that position, or a
    /// document pdfce assembled from nothing — see
    /// [`crate::xref::observed_entry_eol`].
    #[default]
    MatchSource,
    /// Always `SP LF` (`20 0A`), whatever the source used.
    SpaceLf,
    /// Always `SP CR` (`20 0D`).
    SpaceCr,
    /// Always `CR LF` (`0D 0A`).
    CrLf,
}

impl XrefEntryEol {
    /// The concrete two bytes to emit, resolving [`Self::MatchSource`]
    /// against the file being saved.
    ///
    /// `base` is the bytes of the document this save is derived from —
    /// empty for a document assembled from nothing. Kept as a method on
    /// the setting rather than a branch in the writer so that every
    /// caller resolves it the same way; a second resolution site is how
    /// an incremental save and a full rewrite would come to disagree
    /// about the same file.
    #[must_use]
    pub fn resolve(self, base: &[u8]) -> Self {
        match self {
            Self::MatchSource => crate::xref::observed_entry_eol(base).unwrap_or(Self::SpaceLf),
            other => other,
        }
    }

    /// The two bytes themselves. [`Self::MatchSource`] resolves to the
    /// fallback here, so callers that have a base file must call
    /// [`Self::resolve`] first.
    #[must_use]
    pub const fn bytes(self) -> [u8; 2] {
        match self {
            Self::SpaceLf | Self::MatchSource => *b" \n",
            Self::SpaceCr => *b" \r",
            Self::CrLf => *b"\r\n",
        }
    }
}

/// Whether the writer puts an end-of-line byte after the final `%%EOF`
/// (spec ambiguity `EOL-A2`).
///
/// # The disagreement being resolved
///
/// §7.5.1 requires every line to be EOL-terminated; §7.5.5 says the last
/// line *"contains only"* `%%EOF`. **Both readings are self-consistent and
/// the standard does not choose between them.**
///
/// # Default: [`Self::Lf`] — **EVIDENCE TIER (d)**
///
/// Tier (d) — **a guess**, and the safe side of one: §7.2.3 requires the
/// incremental-append path to have an EOL before a following `12 0 obj`
/// anyway, and a trailing EOL never breaks a reader's backward `%%EOF`
/// scan. Low value as a knob; it exists because the choice is currently
/// hard-coded, is labelled in the source as a recorded spec ambiguity, and
/// an engineer who finds that label will ask where the switch is.
///
/// **BYTES blast radius — one byte.** No disclosure needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TrailingEol {
    /// Terminate the `%%EOF` line with `LF`. **The shipped default.**
    #[default]
    Lf,
    /// End the file at the final `F` of `%%EOF`.
    None,
}

/// The operator's persisted choices.
///
/// Deliberately a flat struct of plain values. Grouping into
/// sub-structures would make the file format hierarchical, and the format
/// is flat on purpose (see the module docs).
///
/// # Adding a setting
///
/// Four edits, all in this file, and the compiler finds three of them:
/// a field here, a line in [`Settings::apply`], a line in
/// [`Settings::write_to_string`], and a row in the round-trip test. The
/// default belongs on the *type*, not here.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Settings {
    /// What happens when a page operation splits a preseparated page set
    /// (§14.11.4).
    ///
    /// Note this is a **product policy**, not a spec ambiguity — §14.11.4
    /// is perfectly clear about the invariant and simply does not say what
    /// an editor should do when an edit breaks it. It is a setting because
    /// all three answers are defensible for different workflows, not
    /// because the standard is unclear.
    pub separations: SeparationPolicy,
    /// How `DeviceCMYK` is converted for display.
    pub cmyk_intent: CmykIntent,
    /// Which visual theme the GUI uses, as an opaque token.
    ///
    /// # A `String`, deliberately, and core does not validate it
    ///
    /// The set of themes is a **shell** concern — `pdfce-gui` owns the
    /// palettes, and `pdfce-core` must never gain a GUI dependency
    /// (`ARCHITECTURE.md` §3, the invariant that keeps a future WASM
    /// fork a shell swap rather than a rewrite). An enum here would put
    /// the shell's vocabulary in the core crate for no benefit, and
    /// would have to be extended in core every time the shell added a
    /// look.
    ///
    /// So core stores and round-trips the token and takes no view on
    /// what it means. The shell resolves it, and is responsible for
    /// saying so when it cannot — an unknown token is a note the
    /// operator sees, not a silent reset, because silently discarding a
    /// preference is indistinguishable from losing it.
    ///
    /// A consequence worth having: a settings file written by a NEWER
    /// pdfce keeps its theme when an older one opens and re-saves it.
    pub theme: String,
    /// The gap, as a multiple of the current font size, at which
    /// text extraction inserts a word break.
    ///
    /// Already existed as `ExtractOptions::word_gap_ratio` with a
    /// documented default and a builder — and with **zero** CLI and GUI
    /// callers, which is what made it the register's cheapest win: the
    /// setting was built, just unreachable.
    pub word_gap_ratio: f32,
    /// How many degrees apart two lines may be and still be dimensioned as
    /// PARALLEL rather than as an angle (ce dimensions, two-line pick).
    ///
    /// # Why this is a setting and not a constant
    ///
    /// Nothing defines it. A search of the SolidWorks dimension/tolerance
    /// corpus at `D:\Dev\Rag-Specialized\SolidWorks_Dimensions\` for an
    /// epsilon, a threshold or a near-parallel snap rule found none — the
    /// catalog records the whole question as unverified. Standing rule R169
    /// says a choice no standard makes is a setting rather than a number
    /// buried in the geometry, and this is exactly that case.
    ///
    /// The default of half a degree is a judgement and is documented as one:
    /// CAD-exported geometry is usually exact, so a pair a hair off parallel
    /// is far more likely to be an exporter's rounding artefact than a
    /// deliberate shallow taper. An operator who genuinely dimensions
    /// shallow tapers should lower it.
    ///
    /// This governs only the AUTOMATIC classification. The operator can
    /// always force the parallel reading for one specific ce dimension —
    /// see the two-line authoring surface — so a wrong global default costs
    /// a checkbox, never the ability to get the dimension they want.
    pub parallel_epsilon_degrees: f64,
    /// Which filter resamples a size-mismatched `/SMask` or `/Mask`
    /// (`SM-A1`, §8.9.6.3 / Table 145). RENDER radius.
    pub mask_resample: MaskResample,
    /// How an image drawn smaller than its own pixel grid is sampled
    /// (`IM-A1`, §8.9.5.3). RENDER radius.
    pub image_minify: MinifyFilter,
    /// How a CMYK JPEG that declares no `/Decode` is read (`DCT-A1`,
    /// §7.4.8 + Table 13). RENDER radius, and BYTES wherever pdfce
    /// re-encodes — a re-encode under the wrong polarity bakes the
    /// inversion in permanently.
    pub cmyk_jpeg_polarity: CmykJpegPolarity,
    /// What extraction emits for a code the §9.10.2 ladder cannot map
    /// (`TX-A1`). **EXTRACT radius** — it moves character offsets, so it
    /// moves redaction-by-text coverage (R35).
    pub unmappable_code: UnmappableCode,
    /// Whether `/ActualText` replaces the glyphs it covers (`AT-A1`,
    /// §14.9.4). **EXTRACT radius**, same R35 note as
    /// [`Self::unmappable_code`].
    pub actual_text: ActualTextPrecedence,
    /// What to paint for a multi-entry `/AP` `/N` subdictionary with no
    /// `/AS` (`AS-A1`, §12.5.5). RENDER radius only — pdfce never writes
    /// an `/AS` to repair the file.
    pub missing_as: MissingAppearanceState,
    /// The two-byte terminator on a classic cross-reference entry
    /// (`EOL-A1`, §7.5.4). **BYTES radius.**
    pub xref_entry_eol: XrefEntryEol,
    /// Whether a byte follows the final `%%EOF` (`EOL-A2`, §7.5.5).
    /// **BYTES radius** — one byte.
    pub trailing_eol: TrailingEol,
}

impl Default for Settings {
    /// Every default, **taken from the type that owns it** rather than
    /// restated here.
    ///
    /// `#[derive(Default)]` was the obvious choice and it was wrong: it
    /// gives `word_gap_ratio = 0.0`, because `f32`'s default is zero and
    /// the engine's default is `0.20`. That is the exact failure mode this
    /// module warns about in its own docs — one answer to "what does pdfce
    /// do by default?", not two — and it shipped for about ten minutes
    /// until `every_setting_round_trips_through_the_file` caught it.
    ///
    /// Reading the value off [`ExtractOptions::default`] rather than
    /// copying the number means the two cannot drift at all, which is
    /// strictly better than a mirrored constant plus a test asserting the
    /// mirror still holds.
    fn default() -> Self {
        Self {
            separations: SeparationPolicy::default(),
            cmyk_intent: CmykIntent::default(),
            // The shell's default preset name, as a literal rather than
            // an import, for the layering reason on the field. The GUI's
            // `theme::Preset::default().key()` must agree, and
            // `the_core_default_theme_token_is_one_the_shell_knows` in
            // `pdfce-gui` is what checks that it does.
            theme: "quiet".to_owned(),
            word_gap_ratio: crate::text_extract::ExtractOptions::default().word_gap_ratio,
            // Taken from the geometry module's own policy default rather than
            // restated, so the settings file and the classifier cannot come to
            // disagree about the same number — the failure this module's
            // `word_gap_ratio` default already demonstrated once.
            parallel_epsilon_degrees: crate::vector::linepick::ParallelPolicy::default()
                .epsilon_degrees,
            // The ambiguity-register enums declare their own default on
            // the variant, the same way `CmykIntent` does, because the
            // *choice* is the thing they exist to model — there is no
            // other type that "owns the behaviour" for, say, a mask
            // resampling filter. The consuming option structs
            // (`ExtractOptions`, `RenderOptions`, `SaveOptions`) read
            // `Enum::default()` in turn, so there is still exactly one
            // answer to "what does pdfce do by default?", and tests in
            // this module and in `pdfce-render` pin that agreement.
            mask_resample: MaskResample::default(),
            image_minify: MinifyFilter::default(),
            cmyk_jpeg_polarity: CmykJpegPolarity::default(),
            unmappable_code: crate::text_extract::ExtractOptions::default().unmappable_code,
            actual_text: crate::text_extract::ExtractOptions::default().actual_text,
            missing_as: MissingAppearanceState::default(),
            xref_entry_eol: crate::writer::SaveOptions::default().xref_entry_eol,
            trailing_eol: crate::writer::SaveOptions::default().trailing_eol,
        }
    }
}

/// Lowest accepted `word_gap_ratio`. Zero would break a word at every
/// glyph pair.
///
/// Public so a front end can bound its own control by the **same** number
/// the parser clamps to. A slider whose range is a restated literal is a
/// slider that eventually disagrees with the file's own validation, and
/// then the operator drags to a value that silently clamps.
pub const MIN_WORD_GAP_RATIO: f32 = 0.01;
/// Highest accepted `word_gap_ratio`. Beyond this a line never breaks
/// into words at all. Public for the same reason as
/// [`MIN_WORD_GAP_RATIO`].
pub const MAX_WORD_GAP_RATIO: f32 = 5.0;

/// Lowest accepted `parallel_epsilon_degrees`.
///
/// Zero is allowed and means "exactly parallel only" — a legitimate strict
/// choice for someone working with exact CAD output, not a degenerate value,
/// so it is the floor rather than being rejected.
pub const MIN_PARALLEL_EPSILON_DEGREES: f64 = 0.0;
/// Highest accepted `parallel_epsilon_degrees`.
///
/// Above 45 degrees the classification inverts in spirit: more pairs would be
/// called parallel than angled, which is no longer a tolerance on "parallel"
/// but a different feature. Public for the same reason as
/// [`MIN_WORD_GAP_RATIO`] — a front end bounds its control by THIS number
/// rather than a restated literal.
pub const MAX_PARALLEL_EPSILON_DEGREES: f64 = 45.0;

/// The settings-file token for a separation policy.
///
/// Defined once and used by both [`Settings::apply`] (to say what it fell
/// back to) and [`Settings::write_to_string`] (to write it out). Spelling
/// a token in two places is how a writer and a parser come to disagree
/// about the same value — the same failure the `word_gap_ratio` default
/// already demonstrated in this module.
const fn separation_token(policy: SeparationPolicy) -> &'static str {
    match policy {
        SeparationPolicy::Repair => "repair",
        SeparationPolicy::Discard => "discard",
        SeparationPolicy::Refuse => "refuse",
    }
}

/// The settings-file token for a CMYK intent. See [`separation_token`].
const fn cmyk_token(intent: CmykIntent) -> &'static str {
    match intent {
        CmykIntent::Calibrated => "calibrated",
        CmykIntent::NeutralBlack => "neutral_black",
        CmykIntent::Naive => "naive",
    }
}

/// The settings-file token for a mask resampling filter. See
/// [`separation_token`] for why every enum gets one of these.
const fn mask_resample_token(filter: MaskResample) -> &'static str {
    match filter {
        MaskResample::Nearest => "nearest",
        MaskResample::BoxAverage => "box_average",
        MaskResample::Bilinear => "bilinear",
    }
}

/// The settings-file token for a minification filter. See
/// [`separation_token`].
const fn minify_token(filter: MinifyFilter) -> &'static str {
    match filter {
        MinifyFilter::PointSample => "point_sample",
        MinifyFilter::Smooth => "smooth",
    }
}

/// The settings-file token for a CMYK-JPEG polarity rule. See
/// [`separation_token`].
const fn cmyk_jpeg_polarity_token(polarity: CmykJpegPolarity) -> &'static str {
    match polarity {
        CmykJpegPolarity::NeverInvert => "never_invert",
        CmykJpegPolarity::InvertOnApp14 => "invert_on_app14",
    }
}

/// The settings-file token for an unmappable-code sentinel. See
/// [`separation_token`].
const fn unmappable_token(sentinel: UnmappableCode) -> &'static str {
    match sentinel {
        UnmappableCode::ReplacementChar => "replacement_char",
        UnmappableCode::QuestionMark => "question_mark",
        UnmappableCode::Omit => "omit",
    }
}

/// The settings-file token for an `/ActualText` precedence rule. See
/// [`separation_token`].
const fn actual_text_token(precedence: ActualTextPrecedence) -> &'static str {
    match precedence {
        ActualTextPrecedence::Always => "always",
        ActualTextPrecedence::TaggedOnly => "tagged_only",
        ActualTextPrecedence::Glyphs => "glyphs",
    }
}

/// The settings-file token for a missing-`/AS` policy. See
/// [`separation_token`].
const fn missing_as_token(policy: MissingAppearanceState) -> &'static str {
    match policy {
        MissingAppearanceState::PaintNothing => "paint_nothing",
        MissingAppearanceState::FirstEntry => "first_entry",
        MissingAppearanceState::OffElseNothing => "off_else_nothing",
    }
}

/// The settings-file token for a cross-reference entry terminator. See
/// [`separation_token`].
const fn xref_entry_eol_token(eol: XrefEntryEol) -> &'static str {
    match eol {
        XrefEntryEol::MatchSource => "match_source",
        XrefEntryEol::SpaceLf => "space_lf",
        XrefEntryEol::SpaceCr => "space_cr",
        XrefEntryEol::CrLf => "cr_lf",
    }
}

/// The settings-file token for the trailing-EOL rule. See
/// [`separation_token`].
const fn trailing_eol_token(eol: TrailingEol) -> &'static str {
    match eol {
        TrailingEol::Lf => "lf",
        TrailingEol::None => "none",
    }
}

impl Settings {
    /// Load the operator's settings, always successfully.
    ///
    /// Reads from `location`. A missing file, an unreadable one, or a file
    /// full of nonsense all yield usable settings; what went wrong is in
    /// the returned [`LoadReport`]. See the module docs' fail-soft table.
    #[must_use]
    pub fn load(location: StoreLocation) -> (Self, LoadReport) {
        let mut report = LoadReport {
            location,
            existed: false,
            notes: Vec::new(),
        };
        let Some(path) = report.location.path.clone() else {
            return (Self::default(), report);
        };
        if !path.exists() {
            // A first run is the expected state, not a fault.
            return (Self::default(), report);
        }
        report.existed = true;
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                report.notes.push(SettingNote::Unreadable {
                    path,
                    reason: error.to_string(),
                });
                return (Self::default(), report);
            }
        };
        let settings = Self::parse(&text, &mut report.notes);
        (settings, report)
    }

    /// Parse settings text, recovering per key.
    ///
    /// Split out from [`Settings::load`] so the whole grammar is testable
    /// without a filesystem — which is also what lets the fail-soft table
    /// in the module docs be pinned by tests rather than merely asserted
    /// in prose.
    #[must_use]
    pub fn parse(text: &str, notes: &mut Vec<SettingNote>) -> Self {
        let mut settings = Self::default();
        let mut seen: Vec<String> = Vec::new();

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else {
                notes.push(SettingNote::Malformed { line });
                continue;
            };
            let key = key.trim().to_owned();
            let value = value.trim();
            if key.is_empty() {
                notes.push(SettingNote::Malformed { line });
                continue;
            }
            if seen.contains(&key) {
                notes.push(SettingNote::Duplicate {
                    key: key.clone(),
                    line,
                });
            } else {
                seen.push(key.clone());
            }
            settings.apply(&key, value, line, notes);
        }
        settings
    }

    /// Apply one `key = value` pair, noting anything that did not take.
    ///
    /// The one place that knows the file's vocabulary. Every arm either
    /// sets a field or pushes a note — an arm that does neither would be a
    /// setting that silently does nothing, which is the failure this
    /// module exists to prevent.
    fn apply(&mut self, key: &str, value: &str, line: usize, notes: &mut Vec<SettingNote>) {
        match key {
            "separations" => match value {
                "repair" => self.separations = SeparationPolicy::Repair,
                "discard" => self.separations = SeparationPolicy::Discard,
                "refuse" => self.separations = SeparationPolicy::Refuse,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: separation_token(Self::default().separations).to_owned(),
                }),
            },
            "cmyk_intent" => match value {
                "calibrated" => self.cmyk_intent = CmykIntent::Calibrated,
                "neutral_black" => self.cmyk_intent = CmykIntent::NeutralBlack,
                "naive" => self.cmyk_intent = CmykIntent::Naive,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: cmyk_token(Self::default().cmyk_intent).to_owned(),
                }),
            },
            // Unvalidated on purpose — see the field docs.
            "theme" => self.theme = value.to_owned(),
            "parallel_epsilon_degrees" => match value.parse::<f64>() {
                Ok(parsed) if parsed.is_finite() => {
                    let clamped =
                        parsed.clamp(MIN_PARALLEL_EPSILON_DEGREES, MAX_PARALLEL_EPSILON_DEGREES);
                    if (clamped - parsed).abs() > f64::EPSILON {
                        notes.push(SettingNote::Clamped {
                            key: key.to_owned(),
                            value: value.to_owned(),
                            line,
                            using: clamped.to_string(),
                        });
                    }
                    self.parallel_epsilon_degrees = clamped;
                }
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: Self::default().parallel_epsilon_degrees.to_string(),
                }),
            },
            "word_gap_ratio" => match value.parse::<f32>() {
                Ok(parsed) if parsed.is_finite() => {
                    let clamped = parsed.clamp(MIN_WORD_GAP_RATIO, MAX_WORD_GAP_RATIO);
                    // `!=` on floats is exactly right here: the question is
                    // whether `clamp` returned a different number, not
                    // whether two computed values are near each other.
                    if (clamped - parsed).abs() > f32::EPSILON {
                        notes.push(SettingNote::Clamped {
                            key: key.to_owned(),
                            value: value.to_owned(),
                            line,
                            using: clamped.to_string(),
                        });
                    }
                    self.word_gap_ratio = clamped;
                }
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: Self::default().word_gap_ratio.to_string(),
                }),
            },
            "mask_resample" => match value {
                "nearest" => self.mask_resample = MaskResample::Nearest,
                "box_average" => self.mask_resample = MaskResample::BoxAverage,
                "bilinear" => self.mask_resample = MaskResample::Bilinear,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: mask_resample_token(Self::default().mask_resample).to_owned(),
                }),
            },
            "image_minify" => match value {
                "point_sample" => self.image_minify = MinifyFilter::PointSample,
                "smooth" => self.image_minify = MinifyFilter::Smooth,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: minify_token(Self::default().image_minify).to_owned(),
                }),
            },
            "cmyk_jpeg_polarity" => match value {
                "never_invert" => self.cmyk_jpeg_polarity = CmykJpegPolarity::NeverInvert,
                "invert_on_app14" => self.cmyk_jpeg_polarity = CmykJpegPolarity::InvertOnApp14,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: cmyk_jpeg_polarity_token(Self::default().cmyk_jpeg_polarity).to_owned(),
                }),
            },
            "unmappable_code" => match value {
                "replacement_char" => self.unmappable_code = UnmappableCode::ReplacementChar,
                "question_mark" => self.unmappable_code = UnmappableCode::QuestionMark,
                "omit" => self.unmappable_code = UnmappableCode::Omit,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: unmappable_token(Self::default().unmappable_code).to_owned(),
                }),
            },
            "actual_text" => match value {
                "always" => self.actual_text = ActualTextPrecedence::Always,
                "tagged_only" => self.actual_text = ActualTextPrecedence::TaggedOnly,
                "glyphs" => self.actual_text = ActualTextPrecedence::Glyphs,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: actual_text_token(Self::default().actual_text).to_owned(),
                }),
            },
            "missing_as" => match value {
                "paint_nothing" => self.missing_as = MissingAppearanceState::PaintNothing,
                "first_entry" => self.missing_as = MissingAppearanceState::FirstEntry,
                "off_else_nothing" => self.missing_as = MissingAppearanceState::OffElseNothing,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: missing_as_token(Self::default().missing_as).to_owned(),
                }),
            },
            "xref_entry_eol" => match value {
                "match_source" => self.xref_entry_eol = XrefEntryEol::MatchSource,
                "space_lf" => self.xref_entry_eol = XrefEntryEol::SpaceLf,
                "space_cr" => self.xref_entry_eol = XrefEntryEol::SpaceCr,
                "cr_lf" => self.xref_entry_eol = XrefEntryEol::CrLf,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: xref_entry_eol_token(Self::default().xref_entry_eol).to_owned(),
                }),
            },
            "trailing_eol" => match value {
                "lf" => self.trailing_eol = TrailingEol::Lf,
                "none" => self.trailing_eol = TrailingEol::None,
                _ => notes.push(SettingNote::BadValue {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    line,
                    using: trailing_eol_token(Self::default().trailing_eol).to_owned(),
                }),
            },
            _ => notes.push(SettingNote::UnknownKey {
                key: key.to_owned(),
                line,
            }),
        }
    }

    /// Render the settings as the file's text, with explanatory comments.
    ///
    /// The comments are not decoration: this file is meant to be opened in
    /// a text editor, and a bare `cmyk_intent = calibrated` tells an
    /// operator nothing about what the alternatives are or what flipping it
    /// would change. Every key therefore carries its legal values.
    #[must_use]
    pub fn write_to_string(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "# pdfce settings\n\
             #\n\
             # Plain text, one `key = value` per line. Lines starting with # are\n\
             # comments. An unknown key is ignored and reported, not deleted, and a\n\
             # value pdfce cannot read falls back to the default for that key alone —\n\
             # one bad line never discards the rest of the file.\n\
             #\n\
             # KEEP THIS FOLDER when you update pdfce. Updating means replacing the\n\
             # program files, and everything in this folder is yours, not the\n\
             # program's.\n\n",
        );

        out.push_str(
            "# What to do when a page operation splits a preseparated page set —\n\
             # a print-ready file where one logical page is several page objects,\n\
             # one per printing plate (ISO 32000-1 section 14.11.4).\n\
             #   repair  = keep the surviving plates and update them (default)\n\
             #   discard = keep the pages, forget they were separations\n\
             #   refuse  = decline the operation instead\n",
        );
        let _ = writeln!(
            out,
            "separations = {}\n",
            match self.separations {
                SeparationPolicy::Repair => "repair",
                SeparationPolicy::Discard => "discard",
                SeparationPolicy::Refuse => "refuse",
            }
        );

        out.push_str(
            "# How CMYK colour is converted for display. The PDF standard defines no\n\
             # conversion at all (section 8.6.4.4), so this is a choice, not a fact.\n\
             #   neutral_black = pure black ink renders true black (default). Right for\n\
             #                   CAD and line drawings, where every line is stroked in\n\
             #                   pure K. Only pure black differs; every mixed colour is\n\
             #                   still the calibrated one.\n\
             #   calibrated    = match how Acrobat and most viewers render it. Solid\n\
             #                   black ink shows as a very dark warm grey, not pure\n\
             #                   black, because that is what those viewers do. Use it to\n\
             #                   check how a document will look to someone else.\n\
             #   naive         = the simple formula pdfce used before calibration. Only\n\
             #                   for reproducing an older pdfce export.\n",
        );
        let _ = writeln!(
            out,
            "cmyk_intent = {}\n",
            match self.cmyk_intent {
                CmykIntent::Calibrated => "calibrated",
                CmykIntent::NeutralBlack => "neutral_black",
                CmykIntent::Naive => "naive",
            }
        );

        out.push_str(
            "# How far apart two glyphs must be, as a multiple of the font size,\n\
             # before extracted text gets a space between them. Raise it if\n\
             # extraction is splitting words; lower it if it is running them\n\
             # together. Accepted range 0.01 to 5.0.\n",
        );
        let _ = writeln!(out, "word_gap_ratio = {}\n", self.word_gap_ratio);

        out.push_str(
            "# When you dimension between two lines, how many degrees apart they may\n\
             # be and still be treated as parallel (giving a distance) rather than\n\
             # as an angle. Nothing in any standard fixes this, so it is yours to\n\
             # set: exported CAD geometry is usually exact, so a small value avoids\n\
             # calling a rounding artefact a taper. 0 means exactly parallel only.\n\
             # You can always force the parallel reading on one dimension without\n\
             # changing this. Accepted range 0 to 45.\n",
        );
        let _ = writeln!(
            out,
            "parallel_epsilon_degrees = {}\n",
            self.parallel_epsilon_degrees
        );

        out.push_str(
            "# When a picture carries a separate transparency image at a different\n\
             # size, this decides how that transparency is stretched to fit. The PDF\n\
             # standard fixes where the two line up and says nothing about how to\n\
             # stretch (section 8.9.6.3).\n\
             #   nearest     = take the single nearest transparency pixel (default).\n\
             #                 Keeps hard cut-out edges perfectly sharp and can never\n\
             #                 invent a half-transparent pixel that was not there.\n\
             #                 Can look stair-stepped.\n\
             #   box_average = average every transparency pixel the picture pixel\n\
             #                 covers. Best when the transparency is FINER than the\n\
             #                 picture, where `nearest` throws most of it away.\n\
             #   bilinear    = blend smoothly between transparency pixels. Best for a\n\
             #                 soft photographic fade supplied coarser than the\n\
             #                 picture; softens hard cut-out edges, which is usually\n\
             #                 not wanted.\n",
        );
        let _ = writeln!(
            out,
            "mask_resample = {}\n",
            mask_resample_token(self.mask_resample)
        );

        out.push_str(
            "# Which look the window uses. The application's own colours and spacing\n\
             # ONLY -- it never changes a document, and nothing here is written into\n\
             # a PDF you save.\n\
             #   quiet = muted greys, one accent, tight spacing (the default)\n\
             #   airy  = lighter, roomier, softer edges\n\
             #   dark  = a dark window against a light page, as CAD tools do it\n\
             # An unrecognised name is reported when pdfce starts and the default is\n\
             # used for that run; the name you wrote is kept, not overwritten.\n",
        );
        let _ = writeln!(out, "theme = {}\n", self.theme);

        out.push_str(
            "# How a picture is drawn when it is shown SMALLER than its own pixel\n\
             # grid. The standard only describes smoothing for making a picture\n\
             # bigger and never mentions making one smaller (section 8.9.5.3), so\n\
             # this direction is pdfce's choice.\n\
             #   point_sample = take one pixel per dot on screen (default). Exact, and\n\
             #                  what the document's own smoothing switch literally\n\
             #                  asks for; thin lines can shimmer or vanish when the\n\
             #                  picture is shrunk a lot.\n\
             #   smooth       = average when shrinking. Cleaner shrunken photographs;\n\
             #                  a deliberate departure from the document's switch.\n",
        );
        let _ = writeln!(out, "image_minify = {}\n", minify_token(self.image_minify));

        out.push_str(
            "# How to read a four-ink (CMYK) JPEG that does not say which way round\n\
             # its ink values are stored. No document anywhere defines this; some\n\
             # 1990s Photoshop output stores the values back-to-front and says so\n\
             # nowhere. Getting it wrong turns the picture into a photographic\n\
             # negative, so the mistake is at least obvious.\n\
             #   never_invert    = take the values as stored (default). What every\n\
             #                     other major PDF reader does; a document can still\n\
             #                     declare inverted storage the proper way, and pdfce\n\
             #                     honours that.\n\
             #   invert_on_app14 = flip the values when the file carries an Adobe\n\
             #                     marker and declares nothing. Only for a library of\n\
             #                     old Photoshop CMYK JPEGs that really are stored\n\
             #                     back-to-front.\n",
        );
        let _ = writeln!(
            out,
            "cmyk_jpeg_polarity = {}\n",
            cmyk_jpeg_polarity_token(self.cmyk_jpeg_polarity)
        );

        out.push_str(
            "# What copied or searched text shows for a character pdfce cannot read\n\
             # at all — a font that carries no way back to real characters. The\n\
             # standard names no stand-in (section 9.10.2). CHANGING THIS CHANGES\n\
             # WHICH TEXT A SEARCH OR A TEXT-BASED REDACTION MATCHES.\n\
             #   replacement_char = the standard black-diamond question mark, one per\n\
             #                      unreadable character (default). Keeps the text the\n\
             #                      same length and is unmistakably a failure.\n\
             #   question_mark    = a plain ? instead. Survives being pasted anywhere,\n\
             #                      but is indistinguishable from a real ? in the\n\
             #                      document.\n\
             #   omit             = show nothing. The text gets shorter with no sign\n\
             #                      in the text that anything was lost, and a line\n\
             #                      whose characters are ALL unreadable disappears\n\
             #                      from the results entirely. pdfce still counts\n\
             #                      every such character whichever setting you use.\n",
        );
        let _ = writeln!(
            out,
            "unmappable_code = {}\n",
            unmappable_token(self.unmappable_code)
        );

        out.push_str(
            "# Some documents attach a \"what this really says\" note to a piece of\n\
             # text — for a ligature, a logo, or an abbreviation. This decides\n\
             # whether that note replaces what is drawn on the page. The standard\n\
             # says one thing in section 14.9.4 and something else in a note to\n\
             # section 14.8, so both readings are defensible.\n\
             #   always      = the note wins wherever it appears (default).\n\
             #   tagged_only = the note wins only inside properly tagged content, and\n\
             #                 the drawn characters win everywhere else. Use it when a\n\
             #                 producer scatters bad notes outside its tagging.\n\
             #   glyphs      = the drawn characters always win; the note is reported\n\
             #                 but never substituted. Use it when you need what the\n\
             #                 page actually shows. Text whose ONLY real identity was\n\
             #                 in the note becomes unreadable.\n",
        );
        let _ = writeln!(
            out,
            "actual_text = {}\n",
            actual_text_token(self.actual_text)
        );

        out.push_str(
            "# What to show for a stamp, checkbox or other marked-up item that\n\
             # supplies several alternative appearances but forgets to say which one\n\
             # is current. Such a file is malformed and the standard states no\n\
             # recovery (section 12.5.5). pdfce never repairs the file; it only\n\
             # decides what to put on screen, and counts every occurrence.\n\
             #   paint_nothing    = show nothing, and report it (default). The honest\n\
             #                      answer: pdfce will not pick one for you.\n\
             #   first_entry      = show the first alternative the file lists.\n\
             #   off_else_nothing = show the \"off\" alternative if there is one,\n\
             #                      otherwise nothing. The checkbox-shaped guess.\n",
        );
        let _ = writeln!(out, "missing_as = {}\n", missing_as_token(self.missing_as));

        out.push_str(
            "# Two invisible bookkeeping bytes at the end of every line of a saved\n\
             # file's index table. The standard permits exactly these three and no\n\
             # others (section 7.5.4). Nothing on screen changes; only the saved\n\
             # bytes do.\n\
             #\n\
             # The default keeps whatever form the file you opened already used,\n\
             # so saving a document pdfce did not otherwise change does not\n\
             # rewrite two bytes in every line of its index for no reason.\n\
             #   match_source = keep the form the file already uses (default);\n\
             #                  space_lf for a file that has none\n\
             #   space_lf     = always space then line-feed\n\
             #   space_cr     = always space then carriage-return\n\
             #   cr_lf        = always carriage-return then line-feed\n",
        );
        let _ = writeln!(
            out,
            "xref_entry_eol = {}\n",
            xref_entry_eol_token(self.xref_entry_eol)
        );

        out.push_str(
            "# Whether a saved file ends with a line break after its final end-of-file\n\
             # marker. The standard requires every line to be terminated AND says the\n\
             # last line contains only the marker; both readings are legitimate.\n\
             #   lf   = end with a line break (default). Always safe.\n\
             #   none = end at the last character of the marker.\n",
        );
        let _ = writeln!(
            out,
            "trailing_eol = {}",
            trailing_eol_token(self.trailing_eol)
        );

        out
    }

    /// Write the settings to `location`, creating the directory if needed.
    ///
    /// # Errors
    ///
    /// [`SaveError`] — there is no writable home, the directory could not
    /// be created, or the write itself failed. Unlike loading, saving
    /// *does* fail loudly: the operator asked for something to be
    /// remembered and is owed the truth if it was not.
    pub fn save(&self, location: &StoreLocation) -> Result<(), SaveError> {
        let Some(path) = location.path.as_ref() else {
            return Err(SaveError::NoWritableLocation);
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| SaveError::Io {
                path: parent.to_path_buf(),
                reason: error.to_string(),
            })?;
        }
        std::fs::write(path, self.write_to_string()).map_err(|error| SaveError::Io {
            path: path.clone(),
            reason: error.to_string(),
        })
    }
}

/// Why settings could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
    /// Neither the portable location nor the platform fallback is usable.
    #[error(
        "no writable location for settings: the program folder is not writable and no \
         platform configuration directory could be determined"
    )]
    NoWritableLocation,
    /// The filesystem refused.
    #[error("could not write settings to {path}: {reason}")]
    Io {
        /// What was being written.
        path: PathBuf,
        /// The operating system's reason.
        reason: String,
    },
}

/// Decide where settings live, preferring the portable location.
///
/// Tries `<exe dir>/userdata/` first and falls back to the platform
/// configuration directory only if that cannot be written. Returns
/// [`StoreKind::None`] when neither works, which is still a usable state:
/// defaults load, the session runs, and only [`Settings::save`] fails.
///
/// # Why writability is tested rather than assumed
///
/// `ARCHITECTURE.md` §6 requires pdfce to run read-only-folder-clean, and
/// the failure this avoids is the one that only shows up in the field: a
/// program that assumes it can write beside itself works perfectly on the
/// developer's machine and fails the first time someone installs it under
/// `Program Files`. The probe is a create-and-remove of a temporary file,
/// which is the only test that answers the actual question — directory
/// permissions on Windows can permit `create_dir_all` and still refuse the
/// write.
#[must_use]
pub fn resolve_store() -> StoreLocation {
    // ★ RESOLVED ONCE PER PROCESS, and that is a correctness property rather
    // than a performance one.
    //
    // The `pdfceGUI` session's report (2026-08-13) found the write probe's
    // shared-filename race by way of its SHARPEST symptom: two callers in one
    // process DISAGREEING — the layout store resolving `Portable` while the
    // recent list resolved `PlatformFallback`, so two files meant to sit beside
    // each other did not.
    //
    // Fixing the probe makes that disagreement unlikely. Caching makes it
    // IMPOSSIBLE: every caller in a process now gets one answer by
    // construction, whatever the filesystem does underneath. That is the
    // difference between fixing an instance and closing the class, and it was
    // their suggestion — "a stronger property than making the probe reliable".
    //
    // It is also called at least three times per start-up (settings, layout,
    // recent list), each time doing filesystem work, so the saving is real; it
    // is simply not the reason.
    //
    // WHAT THIS DELIBERATELY GIVES UP: a directory that becomes writable
    // MID-RUN is not noticed. Accepted, because the inputs — `current_exe()`
    // and the platform env vars — do not meaningfully change within a process,
    // and because a store that moves under a running application is a worse
    // outcome than one that is stale. `store_in` remains the escape hatch for
    // an explicit directory (tests, and a future `--user-data-dir`), and it
    // does not consult this cache.
    static RESOLVED: std::sync::OnceLock<StoreLocation> = std::sync::OnceLock::new();
    RESOLVED.get_or_init(resolve_store_uncached).clone()
}

/// The uncached resolution [`resolve_store`] memoises. See its doc comment for
/// why the memoisation is a correctness property.
fn resolve_store_uncached() -> StoreLocation {
    if let Some(dir) = portable_dir()
        && directory_is_writable(&dir)
    {
        return StoreLocation {
            path: Some(dir.join(SETTINGS_FILE)),
            kind: StoreKind::Portable,
        };
    }
    if let Some(dir) = platform_dir()
        && directory_is_writable(&dir)
    {
        return StoreLocation {
            path: Some(dir.join(SETTINGS_FILE)),
            kind: StoreKind::PlatformFallback,
        };
    }
    StoreLocation {
        path: None,
        kind: StoreKind::None,
    }
}

/// A store rooted at an explicit directory — for tests and for a future
/// `--user-data-dir` override.
#[must_use]
pub fn store_in(dir: &Path) -> StoreLocation {
    StoreLocation {
        path: Some(dir.join(SETTINGS_FILE)),
        kind: StoreKind::Portable,
    }
}

/// `<directory of the running executable>/userdata`.
fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(USER_STATE_DIR))
}

/// The platform configuration directory, without a `dirs`-style
/// dependency.
///
/// Three environment variables cover the three supported platforms, and
/// each is the one the platform's own convention names. Doing this by hand
/// rather than adding a crate keeps a dependency out of `pdfce-core` for
/// roughly fifteen lines of logic — and this path is the *fallback*, so it
/// is exercised rarely and must stay simple enough to reason about
/// without running it.
fn platform_dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }?;
    Some(base.join("pdfce"))
}

/// Whether `dir` can actually be written to, creating it if necessary.
fn directory_is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    // ★ THE PROBE NAME MUST BE UNIQUE PER CALL.
    //
    // Until 2026-08-13 this was the fixed name `.pdfce-write-probe`, shared by
    // every caller in every thread and every process. One caller's
    // `remove_file` races another's `write` on the same path, the `write`
    // fails, and this function answers `false` FOR A DIRECTORY THAT IS PLAINLY
    // WRITABLE.
    //
    // Measured by the `pdfceGUI` session, which reported it: 8 threads x 2,000
    // iterations against one writable temp directory produced **1,223 false
    // negatives in 16,000 calls, ~7.6 %**. Not a rare interleaving.
    //
    // WHY A FALSE `false` IS WORSE THAN AN ERROR. `resolve_store` uses this to
    // choose between the PORTABLE directory beside the executable and the
    // PLATFORM FALLBACK. A spurious `false` does not surface as a failure — it
    // produces a different, valid-looking answer, silently relocating
    // settings, layout and the recent list to the platform config directory.
    // `package-portable.py`'s `BUILD-INFO.txt` tells the operator to "replace
    // the binaries but KEEP `userdata/`", which is only true if the portable
    // directory was the one chosen.
    //
    // The sharper failure, and how they found it: TWO CALLERS IN ONE PROCESS
    // DISAGREEING — the layout store resolving `Portable` while the recent list
    // resolves `PlatformFallback`, so two files meant to sit beside each other
    // do not.
    //
    // Process id AND a counter, because neither alone suffices: two processes
    // share a counter's starting value, and one process's threads share its
    // pid. Needs no dependency.
    //
    // A leftover probe from a killed process is now named distinctly and is
    // therefore harmless, where a stale `.pdfce-write-probe` was a name the
    // next run would collide with.
    static PROBE_SEQ: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    let probe = dir.join(format!(
        ".pdfce-write-probe.{}.{}",
        std::process::id(),
        PROBE_SEQ.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    ));
    if std::fs::write(&probe, b"").is_err() {
        return false;
    }
    // A failure to clean up does not make the directory unwritable — the
    // write already proved the point.
    let _ = std::fs::remove_file(&probe);
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod probe_race_tests {
    use super::directory_is_writable;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// ★ Concurrent probes of ONE writable directory must all answer true.
    ///
    /// Reported by the `pdfceGUI` session with a measured reproduction: the
    /// probe used a fixed filename, so one caller's `remove_file` raced
    /// another's `write` and the function answered `false` for a writable
    /// directory — **1,223 of 16,000 calls, ~7.6 %**.
    ///
    /// Thread and iteration counts mirror that reproduction closely enough to
    /// hit the same interleaving; at their rate 8 x 1,000 would expect ~600
    /// failures before the fix. **Zero** is asserted rather than "few", because
    /// unique names make the collision impossible by construction rather than
    /// merely unlikely.
    #[test]
    fn concurrent_probes_never_call_a_writable_directory_unwritable() {
        let dir = std::env::temp_dir().join(format!("pdfce-probe-race-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");

        let bad = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|s| {
            for _ in 0..8 {
                let dir = dir.clone();
                let bad = Arc::clone(&bad);
                s.spawn(move || {
                    for _ in 0..1000 {
                        if !directory_is_writable(&dir) {
                            bad.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        let bad = bad.load(Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            bad, 0,
            "{bad} of 8000 probes called a WRITABLE directory unwritable. In              production this does not error -- it silently relocates settings,              layout and the recent list out of the portable userdata/ directory."
        );
    }

    /// The probe cleans up after itself.
    ///
    /// Unique names make a leftover harmless, but 8,000 of them would be a
    /// different defect. Asserted separately so "unique" cannot be satisfied by
    /// "never deleted".
    #[test]
    fn probing_leaves_no_files_behind() {
        let dir = std::env::temp_dir().join(format!("pdfce-probe-litter-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        for _ in 0..50 {
            assert!(directory_is_writable(&dir));
        }
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("readable")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("write-probe"))
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            leftovers.is_empty(),
            "probe files left behind: {leftovers:?}"
        );
    }

    /// ★ Every caller in a process gets the SAME store, by construction.
    ///
    /// The reported defect's sharpest symptom was two callers in one process
    /// disagreeing — the layout store resolving `Portable` while the recent
    /// list resolved `PlatformFallback`. The probe fix makes that unlikely;
    /// the `OnceLock` makes it impossible, which is the property actually
    /// wanted. Hammered from threads because that is where the disagreement
    /// arose.
    #[test]
    fn every_caller_in_a_process_resolves_the_same_store() {
        let first = super::resolve_store();
        let all_same = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        std::thread::scope(|s| {
            for _ in 0..8 {
                let first = first.clone();
                let all_same = std::sync::Arc::clone(&all_same);
                s.spawn(move || {
                    for _ in 0..200 {
                        let got = super::resolve_store();
                        if got.kind != first.kind || got.path != first.path {
                            all_same.store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                });
            }
        });
        assert!(
            all_same.load(std::sync::atomic::Ordering::Relaxed),
            "two callers in one process resolved DIFFERENT stores — this is              the defect that put the layout file and the recent-file list in              different directories"
        );
    }

    /// An unwritable directory is still reported unwritable.
    ///
    /// The fix must not turn the function into one that always says yes --
    /// which is the cheapest way to make the race test pass and would be
    /// strictly worse than the bug.
    #[test]
    fn a_path_that_cannot_be_a_directory_is_still_refused() {
        let file = std::env::temp_dir().join(format!("pdfce-probe-file-{}", std::process::id()));
        std::fs::write(&file, b"x").expect("write");
        // A FILE, not a directory: create_dir_all must fail on it.
        let answer = directory_is_writable(&file);
        let _ = std::fs::remove_file(&file);
        assert!(!answer, "a regular file is not a writable directory");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn no_default_is_restated_in_this_module() {
        // A regression guard against someone "simplifying" the manual
        // Default impl back to a hard-coded number. Every default must
        // come from the type that owns the behaviour, or "the default"
        // starts meaning two things depending on whether a settings file
        // happens to exist.
        let engine = crate::text_extract::ExtractOptions::default();
        assert!((Settings::default().word_gap_ratio - engine.word_gap_ratio).abs() < f32::EPSILON);
        assert_eq!(Settings::default().separations, SeparationPolicy::default());
        assert_eq!(Settings::default().cmyk_intent, CmykIntent::default());

        // The ambiguity-register settings whose value is carried by an
        // option struct elsewhere in the crate: the same rule applies, and
        // the assertion is what stops the two drifting.
        assert_eq!(
            Settings::default().unmappable_code,
            engine.unmappable_code,
            "the extraction engine owns the sentinel default"
        );
        assert_eq!(
            Settings::default().actual_text,
            engine.actual_text,
            "the extraction engine owns the /ActualText precedence default"
        );
        let writer = crate::writer::SaveOptions::default();
        assert_eq!(Settings::default().xref_entry_eol, writer.xref_entry_eol);
        assert_eq!(Settings::default().trailing_eol, writer.trailing_eol);

        // And the ones whose only home is the enum itself.
        assert_eq!(Settings::default().mask_resample, MaskResample::default());
        assert_eq!(Settings::default().image_minify, MinifyFilter::default());
        assert_eq!(
            Settings::default().cmyk_jpeg_polarity,
            CmykJpegPolarity::default()
        );
        assert_eq!(
            Settings::default().missing_as,
            MissingAppearanceState::default()
        );
    }

    #[test]
    fn every_shipped_default_is_the_behaviour_that_shipped_before_the_setting() {
        // R169 says a shipped default is "the best guess of what is usually
        // followed", and for every entry the ambiguity register triaged out
        // of already-shipped code that guess is, by construction, WHAT
        // PDFCE ALREADY DID. This test is the guard against a later session
        // flipping one of them on its own authority: adding the knob must
        // not change a single observable behaviour, so each default is
        // pinned to the variant the pre-settings code hard-coded.
        let d = Settings::default();
        assert_eq!(d.mask_resample, MaskResample::Nearest, "mask.rs was NN");
        assert_eq!(
            d.image_minify,
            MinifyFilter::PointSample,
            "interpret.rs point-sampled in both directions"
        );
        assert_eq!(
            d.cmyk_jpeg_polarity,
            CmykJpegPolarity::NeverInvert,
            "R29: pdfce never inverted"
        );
        assert_eq!(
            d.unmappable_code,
            UnmappableCode::ReplacementChar,
            "the ladder's rung 4 emitted U+FFFD"
        );
        assert_eq!(
            d.actual_text,
            ActualTextPrecedence::Always,
            "/ActualText always won"
        );
        assert_eq!(
            d.missing_as,
            MissingAppearanceState::PaintNothing,
            "a multi-entry /N with no /AS painted nothing"
        );
        // THE ONE DELIBERATE EXCEPTION, and it is an operator ruling, not
        // a later session flipping a default on its own authority — which
        // is the thing this test exists to prevent.
        //
        // Ken, 2026-08-08: "change the shipped default so that we match
        // the file's existing 2-byte EOL." The register had recommended
        // exactly that and said the shipped fixed `SP LF` was "arguably
        // wrong on pdfce's own invariant": a full rewrite of a `CR LF`
        // file changes two bytes in every entry of a document nobody
        // edited, which is the diff rule 3 exists to prevent.
        //
        // The guarantee this test protects is NOT broken by that, and the
        // distinction is worth being precise about. `MatchSource` on an
        // `SP LF` source resolves to `SP LF` — so for every file pdfce
        // previously round-tripped byte-identically it still does. What
        // changed is the answer for files pdfce was previously getting
        // WRONG. Pinned from both sides: the resolution below, and
        // `tests/xref_eol.rs::a_full_rewrite_keeps_the_files_own_entry_eol`.
        assert_eq!(
            d.xref_entry_eol,
            XrefEntryEol::MatchSource,
            "operator ruling 2026-08-08: the default matches the source file"
        );
        assert_eq!(
            XrefEntryEol::MatchSource.resolve(b""),
            XrefEntryEol::SpaceLf,
            "with nothing to match, the answer is still what xref_out.rs always emitted"
        );
        assert_eq!(
            d.trailing_eol,
            TrailingEol::Lf,
            "xref_out.rs emitted an LF after %%EOF"
        );
    }

    #[test]
    fn an_empty_file_yields_defaults_quietly() {
        let mut notes = Vec::new();
        let settings = Settings::parse("", &mut notes);
        assert_eq!(settings, Settings::default());
        assert!(notes.is_empty());
    }

    #[test]
    fn comments_and_blank_lines_are_not_content() {
        let mut notes = Vec::new();
        let settings = Settings::parse("# a comment\n\n   \n\t# indented\n", &mut notes);
        assert_eq!(settings, Settings::default());
        assert!(notes.is_empty(), "no note for a file of pure commentary");
    }

    #[test]
    fn every_setting_round_trips_through_the_file() {
        // The test that keeps `write_to_string` and `apply` from drifting:
        // a setting that can be written but not read back is a setting
        // that silently resets on the next launch.
        //
        // Every field is set to a value that is NOT its default, so a key
        // that `write_to_string` forgot cannot pass by accidentally
        // matching the default on the way back in.
        let written = Settings {
            separations: SeparationPolicy::Discard,
            cmyk_intent: CmykIntent::Calibrated,
            word_gap_ratio: 0.35,
            // Deliberately NOT the default (0.5): this test exists to catch a
            // field `write_to_string` forgot, and a value equal to the default
            // would pass by accident on the way back in.
            parallel_epsilon_degrees: 1.25,
            mask_resample: MaskResample::BoxAverage,
            image_minify: MinifyFilter::Smooth,
            cmyk_jpeg_polarity: CmykJpegPolarity::InvertOnApp14,
            unmappable_code: UnmappableCode::Omit,
            actual_text: ActualTextPrecedence::Glyphs,
            missing_as: MissingAppearanceState::FirstEntry,
            xref_entry_eol: XrefEntryEol::CrLf,
            trailing_eol: TrailingEol::None,
            // A token core does NOT know, on purpose: this pins that the
            // round trip preserves whatever the shell wrote rather than
            // normalising it to something core recognises — which is the
            // whole point of storing it opaquely.
            theme: "dark".to_owned(),
        };
        assert_ne!(
            written,
            Settings::default(),
            "the round-trip fixture must not be the default settings"
        );
        let mut notes = Vec::new();
        let read = Settings::parse(&written.write_to_string(), &mut notes);
        assert_eq!(read, written);
        assert!(notes.is_empty(), "pdfce's own output must parse cleanly");
    }

    #[test]
    fn the_default_settings_round_trip_too() {
        let mut notes = Vec::new();
        let read = Settings::parse(&Settings::default().write_to_string(), &mut notes);
        assert_eq!(read, Settings::default());
        assert!(notes.is_empty());
    }

    #[test]
    fn one_bad_value_does_not_discard_the_good_ones() {
        // The whole reason this is not a serde derive.
        let mut notes = Vec::new();
        let settings = Settings::parse(
            "separations = discard\ncmyk_intent = purple\nword_gap_ratio = 0.4\n",
            &mut notes,
        );
        assert_eq!(settings.separations, SeparationPolicy::Discard);
        assert_eq!(
            settings.cmyk_intent,
            CmykIntent::default(),
            "an unreadable value falls back to the default, whatever it currently is"
        );
        assert!((settings.word_gap_ratio - 0.4).abs() < f32::EPSILON);
        assert_eq!(
            notes,
            vec![SettingNote::BadValue {
                key: "cmyk_intent".to_owned(),
                value: "purple".to_owned(),
                line: 2,
                using: cmyk_token(CmykIntent::default()).to_owned(),
            }]
        );
    }

    #[test]
    fn an_unknown_key_is_reported_at_its_line_and_nothing_else_breaks() {
        let mut notes = Vec::new();
        let settings = Settings::parse("ribbon_layout = wide\nseparations = refuse\n", &mut notes);
        assert_eq!(settings.separations, SeparationPolicy::Refuse);
        assert_eq!(
            notes,
            vec![SettingNote::UnknownKey {
                key: "ribbon_layout".to_owned(),
                line: 1,
            }]
        );
    }

    #[test]
    fn a_line_with_no_equals_is_malformed_and_skipped() {
        let mut notes = Vec::new();
        let settings = Settings::parse("this is not a setting\nseparations = refuse\n", &mut notes);
        assert_eq!(settings.separations, SeparationPolicy::Refuse);
        assert_eq!(notes, vec![SettingNote::Malformed { line: 1 }]);
    }

    #[test]
    fn an_out_of_range_number_is_clamped_and_said_so() {
        let mut notes = Vec::new();
        let settings = Settings::parse("word_gap_ratio = 99\n", &mut notes);
        assert!((settings.word_gap_ratio - MAX_WORD_GAP_RATIO).abs() < f32::EPSILON);
        assert_eq!(
            notes,
            vec![SettingNote::Clamped {
                key: "word_gap_ratio".to_owned(),
                value: "99".to_owned(),
                line: 1,
                using: MAX_WORD_GAP_RATIO.to_string(),
            }]
        );
    }

    #[test]
    fn a_non_finite_number_is_a_bad_value_not_a_clamp() {
        // `NaN.clamp(..)` and `inf.clamp(..)` do not do what a reader
        // expects, so they are rejected before the clamp rather than
        // silently becoming a bound.
        for text in ["word_gap_ratio = NaN\n", "word_gap_ratio = inf\n"] {
            let mut notes = Vec::new();
            let settings = Settings::parse(text, &mut notes);
            assert_eq!(settings.word_gap_ratio, Settings::default().word_gap_ratio);
            assert!(matches!(notes.as_slice(), [SettingNote::BadValue { .. }]));
        }
    }

    #[test]
    fn the_last_duplicate_wins_and_the_duplication_is_reported() {
        let mut notes = Vec::new();
        let settings = Settings::parse("separations = discard\nseparations = refuse\n", &mut notes);
        assert_eq!(settings.separations, SeparationPolicy::Refuse, "last wins");
        assert_eq!(
            notes,
            vec![SettingNote::Duplicate {
                key: "separations".to_owned(),
                line: 2,
            }]
        );
    }

    #[test]
    fn whitespace_around_keys_and_values_is_not_significant() {
        let mut notes = Vec::new();
        let settings = Settings::parse("   separations   =   refuse   \n", &mut notes);
        assert_eq!(settings.separations, SeparationPolicy::Refuse);
        assert!(notes.is_empty());
    }

    #[test]
    fn a_missing_file_is_silent_but_a_present_one_is_flagged_as_existing() {
        let dir = std::env::temp_dir().join(format!("pdfce-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let location = store_in(&dir);

        let (settings, report) = Settings::load(location.clone());
        assert_eq!(settings, Settings::default());
        assert!(!report.existed, "a first run has no file");
        assert!(report.is_quiet(), "and a first run is not a fault");

        let written = Settings {
            cmyk_intent: CmykIntent::Calibrated,
            ..Settings::default()
        };
        written.save(&location).expect("save must succeed");
        let (reloaded, report) = Settings::load(location);
        assert_eq!(reloaded, written);
        assert!(report.existed);
        assert!(report.is_quiet());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_without_a_location_is_a_named_refusal() {
        let nowhere = StoreLocation {
            path: None,
            kind: StoreKind::None,
        };
        assert_eq!(
            Settings::default().save(&nowhere),
            Err(SaveError::NoWritableLocation)
        );
    }

    #[test]
    fn the_written_file_names_every_legal_value_of_every_key() {
        // The file is meant to be hand-edited, so a key whose alternatives
        // are undiscoverable from the file itself is a key the operator
        // can only change by reading source.
        let text = Settings::default().write_to_string();
        for token in [
            "repair",
            "discard",
            "refuse",
            "calibrated",
            "neutral_black",
            "naive",
            "nearest",
            "box_average",
            "bilinear",
            "point_sample",
            "smooth",
            "never_invert",
            "invert_on_app14",
            "replacement_char",
            "question_mark",
            "omit",
            "always",
            "tagged_only",
            "glyphs",
            "paint_nothing",
            "first_entry",
            "off_else_nothing",
            "space_lf",
            "space_cr",
            "cr_lf",
            "lf",
            "none",
        ] {
            assert!(
                text.contains(token),
                "{token} is not documented in the file"
            );
        }
        assert!(
            text.contains("KEEP THIS FOLDER"),
            "R15's update instruction must be in the file the update would destroy"
        );
    }
}
