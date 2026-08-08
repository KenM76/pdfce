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
    /// The gap, as a multiple of the current font size, at which
    /// text extraction inserts a word break.
    ///
    /// Already existed as `ExtractOptions::word_gap_ratio` with a
    /// documented default and a builder — and with **zero** CLI and GUI
    /// callers, which is what made it the register's cheapest win: the
    /// setting was built, just unreachable.
    pub word_gap_ratio: f32,
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
            word_gap_ratio: crate::text_extract::ExtractOptions::default().word_gap_ratio,
        }
    }
}

/// Lowest accepted `word_gap_ratio`. Zero would break a word at every
/// glyph pair.
const MIN_WORD_GAP_RATIO: f32 = 0.01;
/// Highest accepted `word_gap_ratio`. Beyond this a line never breaks
/// into words at all.
const MAX_WORD_GAP_RATIO: f32 = 5.0;

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
        let _ = writeln!(out, "word_gap_ratio = {}", self.word_gap_ratio);

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
    let probe = dir.join(".pdfce-write-probe");
    if std::fs::write(&probe, b"").is_err() {
        return false;
    }
    // A failure to clean up does not make the directory unwritable — the
    // write already proved the point.
    let _ = std::fs::remove_file(&probe);
    true
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
        let written = Settings {
            separations: SeparationPolicy::Discard,
            cmyk_intent: CmykIntent::NeutralBlack,
            word_gap_ratio: 0.35,
        };
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
            cmyk_intent: CmykIntent::NeutralBlack,
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
