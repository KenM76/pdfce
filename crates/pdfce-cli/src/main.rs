//! # pdfce-cli — command-line batch shell
//!
//! The scriptable front end to the pdfce engine (docs/ARCHITECTURE.md §7).
//! Unlike Adobe Acrobat Pro — which has no real CLI, only in-GUI Action
//! Wizard batch sequences — pdfce ships a genuine command-line binary from
//! the start, so document pipelines can merge/split/stamp/convert/sign/
//! validate PDFs without opening a window.
//!
//! It depends on `pdfce-core` (and, from the Pass that needs it,
//! `pdfce-render`) exactly as `pdfce-gui` does, and is held to the same
//! zero-GUI-dependency invariant. Its very existence is the proof that the
//! GUI-core separation (docs/ARCHITECTURE.md §3) works: two completely
//! different front ends, one shared core, no logic duplicated.
//!
//! ## Implemented surface
//!
//! - `--version` / `--help` work and list the planned subcommand surface.
//! - `inspect <file>` (Pass 0): confirms the `%PDF-` header and prints the
//!   declared version (mirroring the GUI's Pass 0 bar).
//! - `render-page <file> [--page N] [--scale S] [--font-dir DIR]… -o
//!   <out.png>` (Pass 1; `--font-dir` decision 012): loads the document,
//!   resolves the page tree, rasterizes one page through `pdfce-render`,
//!   and writes a PNG. Each `--font-dir` supplies operator fonts for the
//!   document's NON-embedded fonts (shell-side folder walk, R61). See
//!   [`cmd_render_page`] for the full contract.
//! - `round-trip <file> [--mode M] [-o <out.pdf>] [--producer P]`
//!   (Pass 3.0): saves the document and verifies the
//!   `ARCHITECTURE.md` §5 round-trip invariant — byte identity in the
//!   shape the chosen mode promises, reloadability, and an identical
//!   page-1 raster. Its four verification-specific exit codes (5–8)
//!   are what make it usable as a corpus gate. See [`cmd_round_trip`].
//! - `set-info <in> -o <out> [--title …] [--clear …]` (Pass 3.1): edits
//!   the document information dictionary (§14.3.3). See [`cmd_set_info`].
//! - `rotate-page <in> -o <out> --page N --degrees D [--relative]`
//!   (Pass 3.1): sets one page's `/Rotate` (Table 30). See
//!   [`cmd_rotate_page`].
//! - Every other subcommand is a **documented stub** that exits with
//!   [`exit::UNIMPLEMENTED`]. The real bodies land alongside each feature's
//!   own Pass (docs/ROADMAP.md "CLI batch operations"). Stubs are listed
//!   now so the command surface — and the exit-code convention below — is
//!   established from the first commit rather than retrofitted.
//!
//! ## Exit-code contract (docs/ARCHITECTURE.md §7)
//!
//! pdfce-cli is meant to be genuinely scriptable, so it follows Unix
//! convention: `0` on success, non-zero on failure, with specific codes so
//! a calling script can distinguish failure modes. See the [`exit`] module
//! for the current assignments. Note `2` is reserved by `clap` for
//! argument/usage errors (its built-in convention) — pdfce's own runtime
//! failure codes deliberately avoid it.
//!
//! ## English-only by design; locale-invariant stdout (decision 002, R5)
//!
//! pdfce-cli is **permanently English-only** — a positive design ruling
//! (`docs/decisions/002-i18n-timing.md` §5.4), not a deferral. Reasons:
//! clap's own generated text (`Options:`, `error:`, "unexpected argument
//! found", …) is hardcoded English with no localization API (clap-rs/clap
//! #380, open since 2016), so a "localized" CLI would be a half-English
//! chimera; and localizing a scripting interface breaks callers — the GNU
//! `LC_ALL=C` convention exists precisely because localized tool output
//! breaks parsers. Acrobat Pro has no CLI, so nothing is conceded.
//!
//! The binding contract that follows: **stdout is machine-readable,
//! locale-invariant output, permanently** — it never varies with
//! `LANG`/`LC_ALL`, and neither does the exit-code table above. Human
//! diagnostics go to **stderr**. If stderr prose is ever localized, no
//! script breaks, because this separation is designed in from Pass 0.
//!
//! ## stdout result-line format (stable, parseable)
//!
//! Every subcommand that succeeds prints exactly **one LF-terminated,
//! pure-ASCII line to stdout** summarizing what it did. The lines below are
//! part of the compatibility surface and are versioned like any other
//! public API (a change is breaking).
//!
//! **A `key=` whose value is a §7.9.2 TEXT STRING is debug-QUOTED**
//! (`name="Home Phone"`), because such a value may legally contain spaces
//! and a bare one would make the line unsplittable — or, worse, tempt the
//! printer into altering the value to keep it splittable, which is exactly
//! the defect fixed on 2026-08-09: `list-fields` mangled whitespace to `_`
//! and so reported field names that every write verb rejected. A value that
//! is absent prints as the bare sentinel `-`, which is therefore
//! distinguishable from a present-but-empty `""`.
//!
//! ```text
//! inspect:      <input>: PDF <major>.<minor>
//! list-fields:  field name=<"quoted"|(unnamed)> type=<T> button=<B> \
//!               flags=0x<H> value=<"quoted"|-> widgets=<N> ap=<0|1> \
//!               fillable=<0|1> readonly=<0|1> aa=<0|1> caption=<"quoted"|->
//! round-trip:   round-trip <input> mode=<M> -> <output>; \
//!               identical=<0|1> in_bytes=<N> out_bytes=<N> appended=<N> \
//!               objects=<N> verbatim=<N> reserialized=<N> reloaded=<0|1> \
//!               raster_compared=<0|1> raster_identical=<0|1> delinearized=<0|1> \
//!               promoted=<N>
//! render-page:  rendered <input> page <N> -> <output> <W>x<H>; \
//!               substituted=<K> notdef=<L> unsupported=<J> unknown=<M> deferred=<P> \
//!               images=<Q> images_unsupported=<R> forms=<S> \
//!               images_codec_unsupported=<T> codec_features=<U> \
//!               codec_geometry_mismatch=<V> dct_cmyk=<W> lzw_anomalies=<X> \
//!               dct_cmyk_unverifiable=<Y> jpx_preblended=<Z> \
//!               annots=… need_appearances=<0|1> \
//!               unsupported_type3=<a> unsupported_noncmap=<b> \
//!               unsupported_vertical=<c> unsupported_composite_not_embedded=<d> \
//!               unsupported_unknown_subtype=<e> unsupported_unusable_program=<f> \
//!               supplied=<g> supplied_registered=<h> contents_unresolved=<i> \
//!               images_masked=<j> images_mask_unsupported=<k> masks_resampled=<l> \
//!               mattes_undone=<m> mattes_not_undone=<n> \n//!               oc_hidden=<o>
//! ```
//!
//! `render-page`'s line is deliberately split by the first `"; "` into a
//! **narrative half** and a **metrics half**:
//!
//! - The narrative half echoes paths verbatim. Paths may contain spaces,
//!   so it is for logs and humans, not for field-splitting.
//! - The metrics half is `key=<non-negative integer>` pairs, separated by
//!   single spaces, in the **fixed order shown**, with no spaces inside a
//!   pair. A script parses it with
//!   `line.split("; ").nth(1)` then `split(' ')` then `split('=')` —
//!   robust regardless of what the paths contain. New counters may be
//!   **appended** in later Passes; existing keys never change meaning,
//!   never move, and never disappear, so a parser that reads keys by name
//!   (rather than by position) keeps working.
//!
//! The counters are `pdfce_render::Diagnostics`'s honesty report
//! (decision 004 §6.4, rule R20) and their presence on stdout is
//! **mandatory, not decorative**: a batch pipeline that rasterizes 10,000
//! pages must be able to find the ones where pdfce did not draw the
//! document's own glyphs without a human looking at every image.
//!
//! | key | source field | question it answers |
//! |---|---|---|
//! | `substituted` | `glyphs_substituted` | "are these the document's own letterforms, or a BUNDLED substitute?" |
//! | `supplied` | `glyphs_supplied` | "how many glyphs came from an operator-supplied `--font-dir` face (shapes only; positions still from `/Widths`)?" (decision 012) |
//! | `supplied_registered` | (shell) | "how many name→file registrations did `--font-dir` add?" (0 without the flag) |
//! | `notdef` | `glyphs_notdef` | "is any glyph missing entirely?" |
//! | `unsupported` | `fonts_unsupported` | "was any text skipped outright?" |
//! | `unknown` | `unknown_ops` | "were there operators pdfce doesn't know?" |
//! | `deferred` | `deferred_ops` | "were there operators pdfce knows but hasn't implemented?" |
//! | `images` | `images_rendered` | "how many sampled images were painted?" |
//! | `images_unsupported` | `images_unsupported` | "how many images are simply MISSING from the raster?" |
//! | `contents_unresolved` | `contents_streams_unresolved` | "how many of this page's `/Contents` streams are not in the file at all, so their marks are MISSING from the raster?" (§7.3.10 + Table 30 — legal, but the page is incomplete) |
//! | `forms` | `forms_rendered` | "how many form XObjects were executed?" |
//! | `images_codec_unsupported` | `images_codec_unsupported` | "how many images need a codec this build doesn't have?" |
//! | `codec_features` | `codec_feature_unsupported` (summed) | "how many images need a codec *variant* this build doesn't have?" |
//! | `codec_geometry_mismatch` | `codec_geometry_mismatch` | "how many images disagree with their own codestream?" |
//! | `dct_cmyk` | `dct_cmyk_images` | "how many benign YCCK JPEGs appeared?" (census — decision 006 §4.4) |
//! | `lzw_anomalies` | `lzw_framing_anomalies` | "how many LZW streams were non-conformantly framed?" |
//! | `dct_cmyk_unverifiable` | `dct_cmyk_polarity_unverifiable` | "did the ONE polarity-ambiguous JPEG shape appear?" (decision 006 R30) |
//! | `jpx_preblended` | `jpx_smask_in_data_preblended` | "did any JPX image arrive preblended with a backdrop (`/SMaskInData 2`)?" |
//! | `images_masked` | `images_masked` | "how many images had their transparency COMPOSITED?" (census; a subset of `images`. The per-mechanism split — `smask` / `stencil` / `colour-key` / `jpx-embedded-alpha` — goes to stderr) |
//! | `images_mask_unsupported` | `images_mask_unsupported` | "how many images are on the page but TOO SOLID, because their `/SMask` or `/Mask` could not be applied?" (the transparency twin of `images_unsupported`: that one means missing, this one means opaque) |
//! | `masks_resampled` | `masks_resampled` | "how many masks had different pixel dimensions from their base image and were point-sampled across it?" (§8.9.6.3 / Table 145 — conformant and common; it exists so a pixel-parity investigation can tell resampling apart from decoding) |
//! | `mattes_undone` | `mattes_undone` | "how many `/Matte` preblends were inverted?" (§11.6.5.3 — census, but the inversion amplifies quantisation error by `1/α`, so a near-transparent fringe that disagrees with another engine is expected rather than a bug) |
//! | `mattes_not_undone` | `mattes_not_undone` | "how many `/Matte` preblends were NOT inverted, leaving colours shifted toward the matte colour?" (alpha still applied; the reason is in the image divergences) |
//!
//! `images` and `forms` are *volume*, not shortfall — they are non-zero
//! on a perfectly faithful render and exist so a batch pipeline can tell
//! "this page has no images" apart from "this page's images all failed."
//! `dct_cmyk` is likewise pure volume: decision 006 verified that
//! YCCK-storage JPEGs decode without polarity ambiguity
//! (pixel-matching pdfium), so the counter is a neutral census and no
//! stderr note accompanies it. Its former companion warning ("check
//! the colours") cried wolf on known-good files and was retired by the
//! 006 split. `dct_cmyk_unverifiable` is the half that still deserves
//! attention: a 4-component JPEG with effective `ColorTransform` 0 and
//! no `/Decode` is the one shape whose polarity genuinely cannot be
//! verified (rule R30 — reported, never repaired), and any sighting is
//! a decision 006 §9 revisit trigger. It sits at the END of the line
//! because keys are appended, never reordered.
//!
//! The six `unsupported_*` tokens are the **by-reason breakdown** of
//! `unsupported` (`fonts_unsupported_by_reason`, keyed by
//! `pdfce_render::text::UnsupportedFont::reason_key`): their sum equals
//! `unsupported`, and they are emitted in a fixed order even at zero so
//! the line stays diffable. They answer "*why* was text skipped?" without
//! re-instrumenting the loader (rule R20):
//!
//! | token | reason | meaning |
//! |---|---|---|
//! | `unsupported_type3` | `Type3` | Type 3 (content-stream) glyphs, deferred |
//! | `unsupported_noncmap` | `NonIdentityCmap` | `Type0` with a non-`Identity-H` CMap, deferred |
//! | `unsupported_vertical` | `VerticalWriting` | `Identity-V` vertical writing, deferred |
//! | `unsupported_composite_not_embedded` | `CompositeNotEmbedded` | `Identity-H` with no embedded program — supply the font |
//! | `unsupported_unknown_subtype` | `UnknownSubtype` | `/Subtype` absent/unrecognized |
//! | `unsupported_unusable_program` | `UnusableProgram` | an embedded program pdfce could not parse |
//!
//! `unsupported_unusable_program` is the load-bearing one for the
//! embedded-font-rendering class: a non-zero count is the exact signal
//! that once caught the `0x00010000`-sfnt whitespace-trim misroute which
//! sent every embedded TrueType to the CFF parser.
//!
//! `codec_features` is a **sum**, because the underlying counter is a
//! map keyed by feature name (`DCT/arithmetic`, `DCT/12-bit`, …) and the
//! machine line's contract is `key=<non-negative integer>`. The per-name
//! breakdown — which is the part an operator actually acts on — goes to
//! stderr, where it cannot break a parser.
//!
//! Any non-zero shortfall counter also triggers a **human-readable
//! expansion on stderr** (substituted font names, sample operator names,
//! the specific codec an image needed) — detail that would bloat the
//! machine line, placed where it cannot break a parser.
//!
//! ### `round-trip`'s counters (Pass 3.0)
//!
//! Same two-half split: `round-trip <input> mode=<M> -> <output>` is the
//! narrative half (paths may contain spaces; `mode` is a name, not an
//! integer, and lives here for exactly that reason), then `"; "`, then
//! `key=<non-negative integer>` pairs in the fixed order below.
//!
//! | key | meaning |
//! |---|---|
//! | `identical` | did the mode's byte-identity promise hold? (see below — the promise differs per mode) |
//! | `in_bytes` / `out_bytes` | input and output file sizes |
//! | `appended` | bytes written past the input's original length; `0` for a no-op incremental save |
//! | `objects` | object definitions emitted by this save |
//! | `verbatim` | of those, how many were copied byte-for-byte from the retained source |
//! | `reserialized` | of those, how many were rebuilt from values — every one is a byte-level divergence, counted rather than rounded away |
//! | `reloaded` | did `pdfce-core` parse back what it wrote? |
//! | `raster_compared` | was the semantic oracle able to run? (`0` when page 1 does not render, which is not a failure) |
//! | `raster_identical` | did page 1 re-render to identical pixels? |
//! | `delinearized` | did this save spend a live Annex F Fast Web View property? |
//!
//! **`identical=1` means three different things, by mode**, and this is
//! the distinction decision 007 W1 calls the likeliest source of a false
//! green or a false red:
//!
//! - `--mode incremental` — the output is byte-identical to the input,
//!   **whole file**. Zero edits means zero bytes.
//! - `--mode append-identity` — every byte below the input's original
//!   EOF is unchanged (§7.5.6), with a new revision appended.
//! - `--mode full` — every `File`-provenance object's **definition
//!   bytes** appear verbatim. Never whole-file: a full rewrite moves
//!   object offsets, so the cross-reference section must differ, and a
//!   whole-file comparison would fail on every input.
//!
//! ### The editing subcommands' counters (Pass 3.1)
//!
//! `set-info` and `rotate-page` share a counter tail, because they share
//! everything that matters: both go through the **same command log**
//! (`pdfce_core::edit::EditSession`) that the GUI uses, and both save
//! through the same writer. There is no CLI-only mutation path, and that
//! is the point of the GUI-core separation rather than an accident of
//! this Pass.
//!
//! ```text
//! set-info      <input> mode=<M> -> <output>; \
//!               changed=<N> objects=<N> verbatim=<N> reserialized=<N> \
//!               promoted=<N> appended=<N> out_bytes=<N> info_created=<0|1> \
//!               undo_verified=<0|1> undo_identical=<0|1> delinearized=<0|1>
//! rotate-page   <input> page <P> mode=<M> -> <output>; \
//!               rotate=<D> changed=<N> objects=<N> verbatim=<N> \
//!               reserialized=<N> promoted=<N> appended=<N> out_bytes=<N> \
//!               undo_verified=<0|1> undo_identical=<0|1> delinearized=<0|1>
//! ```
//!
//! | key | meaning |
//! |---|---|
//! | `changed` | objects that currently differ from the base revision — the save-time diff, **not** a count of commands run |
//! | `promoted` | objects moved out of an object stream because they were touched (R38) — a representation change worth disclosing |
//! | `info_created` | `1` when the file had no `/Info` dictionary and one was created for the operator's metadata |
//! | `undo_verified` | `1` when `--verify-undo` ran the edit → undo → save check |
//! | `undo_identical` | `1` when that check produced a file byte-identical to the input |
//!
//! `changed=0` is a legitimate, successful outcome: asking for a
//! rotation a page already has, or a title it already carries, changes
//! nothing and therefore writes nothing. The output file is then a byte
//! copy of the input, `appended=0`, and a note goes to stderr. Silently
//! appending an empty revision instead would be the exact "zero edits
//! means zero bytes" violation the writer refuses to commit.
//!
//! ### `--verify-undo`, and why it is a real flag rather than a test hook
//!
//! With it, the tool performs the edit, then **undoes it and saves
//! again**, and checks that the second save is byte-identical to the
//! input. That is `ARCHITECTURE.md` §11.1's contract — the dirty set is a
//! diff against the base, never the union of commands run — evaluated
//! against *this operator's document* rather than against a fixture. A
//! batch pipeline that is about to edit ten thousand signed contracts can
//! use it as a pre-flight on a sample. It costs one extra save, so it is
//! off by default; a failure exits [`exit::NOT_BYTE_IDENTICAL`], because
//! it is a correctness result, not a crash.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use pdfce_core::PdfError;
use pdfce_core::document::Document;
use pdfce_core::pageops::{DocumentView, InsertPosition, PageOpError, SplitCriterion};
use pdfce_core::signature::{SaveMode as CoreSaveMode, SignatureImpact};

/// Exit-code assignments for pdfce-cli's scriptable contract.
///
/// These are stable, documented values — changing one is a
/// backwards-incompatible change to any script that branches on the exit
/// code, so treat them the way the public API surface is treated. New
/// failure modes get new codes rather than reusing an existing one with a
/// broadened meaning.
mod exit {
    /// Everything succeeded.
    pub const SUCCESS: u8 = 0;
    /// A generic runtime failure with no more specific code.
    pub const RUNTIME_ERROR: u8 = 1;
    // 2 is reserved by clap for CLI-usage / argument-parse errors.
    /// The input file could not be opened or read (not found, permission
    /// denied, I/O error). Maps from [`pdfce_core::PdfError::Io`].
    pub const IO_ERROR: u8 = 3;
    /// The input is not a PDF, or its header is malformed. Maps from
    /// [`pdfce_core::PdfError::MissingHeader`] /
    /// [`pdfce_core::PdfError::MalformedVersion`].
    pub const NOT_A_PDF: u8 = 4;
    /// `round-trip`: the save completed, the output reloads, but it is
    /// **not byte-identical** to the input where the mode promised it
    /// would be.
    ///
    /// This is the Pass 3.0 headline gate's failure code. It is
    /// deliberately distinct from [`RUNTIME_ERROR`]: nothing crashed and
    /// nothing refused — pdfce produced a working PDF that differs from
    /// its input, which is a violation of `ARCHITECTURE.md` §5's
    /// round-trip invariant and a *correctness* result, not an error.
    /// A corpus script needs to count these separately.
    pub const NOT_BYTE_IDENTICAL: u8 = 5;
    /// `round-trip`: the save produced bytes, but `pdfce-core` could not
    /// load them back.
    ///
    /// Strictly worse than [`NOT_BYTE_IDENTICAL`] — the writer emitted a
    /// file that is not a valid PDF by pdfce's own reckoning.
    pub const RELOAD_FAILED: u8 = 6;
    /// `round-trip`: the output reloads, but re-rendering page 1 at the
    /// same scale produces a **different raster** than the input does.
    ///
    /// The semantic oracle. Byte identity is a syntactic claim; this is
    /// the one that says the document still *means* the same thing.
    /// Available only because the render stack shipped before the
    /// writer.
    pub const RASTER_DIFFERS: u8 = 7;
    /// `round-trip`: pdfce **refused** the requested save by name — e.g.
    /// a full rewrite of a §7.5.8.4 hybrid-reference file, which would
    /// destroy the file's pre-1.5 readability.
    ///
    /// A refusal is a correct outcome, not a defect, so it gets its own
    /// code: a corpus run must be able to tally "declined, by name" apart
    /// from "produced a wrong file".
    pub const SAVE_REFUSED: u8 = 8;
    /// An **edit** was refused by name before any save was attempted —
    /// a rotation that is not a multiple of 90 (ISO 32000-1 Table 30), a
    /// page index past the end of the document, a malformed `/Info`.
    ///
    /// Distinct from [`SAVE_REFUSED`] (which is about writing) and from
    /// [`RUNTIME_ERROR`] (which is about failing): the document was
    /// readable and pdfce declined to perform the operation as asked.
    /// A batch script needs to tell "this file is unsuitable for this
    /// edit" apart from "this file is broken".
    pub const EDIT_REFUSED: u8 = 9;
    /// A redaction **apply** completed the removal but the diligence
    /// carrier sweep disclosed a residual it could not scrub (XFA, a
    /// structure-tree ActualText copy, an embedded file), and the operator
    /// did **not** pass `--acknowledge-residuals`. The output was still
    /// written (the covered content IS removed), but the non-zero code
    /// forces a script to see the disclosure — the refusal-acknowledgement
    /// gate (ui-spec §4.4): no path where partial reads as complete.
    pub const REDACTION_RESIDUALS: u8 = 10;
    /// The document **opened**, but only via cross-reference **recovery**
    /// (decision 013): its stored cross-reference table could not be parsed
    /// and pdfce rebuilt it by scanning for `N G obj` headers
    /// (rebuild-by-scan). The content is available, but a batch script
    /// needs to tell "opened clean" from "opened via recovery" — a
    /// recovered document forces a full-rewrite save (incremental is
    /// refused) and its bytes were reconstructed, not read as authored.
    /// A distinct, documented status per the R20 counted-diagnostics
    /// tradition (fuzzy-never-sneaky).
    pub const OPENED_VIA_RECOVERY: u8 = 11;
    /// The subcommand exists in the surface but is not implemented yet
    /// (Pass 0 stub). Distinct code so a script can tell "you asked for a
    /// feature pdfce doesn't have yet" apart from a real failure.
    pub const UNIMPLEMENTED: u8 = 64;
}

/// pdfce — a scriptable PDF toolkit (open-source Acrobat-Pro-parity engine).
#[derive(Debug, Parser)]
#[command(
    name = "pdfce-cli",
    version,
    about = "pdfce command-line batch shell — scriptable PDF operations.",
    long_about = "pdfce-cli is the command-line front end to the pdfce PDF \
engine. Pass 0 implements `inspect`; the remaining subcommands are stubs \
whose real behaviour ships alongside each feature's own development Pass \
(see docs/ROADMAP.md)."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Password for an encrypted PDF (ISO 32000-1 §7.6). Either the user or
    /// the owner password opens the document.
    ///
    /// NOT NEEDED for most protected PDFs. A document with an empty user
    /// password — the common "permissions-only" PDF — opens with no password
    /// at all, because §7.6.3.1 requires a reader to try the empty one first
    /// and silently. Supply this only when pdfce asks for it.
    ///
    /// SECURITY: a password on the command line is visible to every other
    /// process on the machine (`ps`, Task Manager) and is written to your
    /// shell's history file. Prefer --open-password-file, which is not.
    ///
    /// NAMED `--open-password`, not `--password`, because `--password` is
    /// already taken: `add-text-field --password` is the Table 228 field flag
    /// that makes a form field mask its input. Two unrelated meanings, and
    /// `clap` refuses the collision outright (it panics at run time, which is
    /// how this was found). Renaming the shipped field flag would break
    /// existing scripts, so the newcomer takes the qualified name — and
    /// "open" is the more accurate word anyway: this password opens the
    /// document, it does not set one.
    #[arg(long, global = true, value_name = "PASSWORD")]
    open_password: Option<String>,

    /// Read the PDF password from a file, or from standard input with `-`.
    ///
    /// The first line is used, with a trailing newline (and CR) stripped; a
    /// file with no newline is used whole. Nothing else in the file is read,
    /// so a one-line secrets file works unchanged.
    ///
    /// Preferred over --open-password: the value never appears in a process
    /// listing or a shell history file.
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        conflicts_with = "open_password"
    )]
    open_password_file: Option<PathBuf>,
}

/// The password supplied by `--open-password` / `--open-password-file`,
/// resolved once.
///
/// # Why a process global rather than a threaded parameter
///
/// This is a genuinely process-wide option: it applies to every subcommand
/// that opens a document, and there are twenty-six such call sites. Threading
/// an `Option<&[u8]>` through all of them would change twenty-six function
/// signatures — and every future one — to carry a value that is constant for
/// the life of the process and that only two lines of code ever set. `clap`
/// models exactly this shape with `global = true`; this is its storage.
///
/// Written once, in [`run`], before any subcommand executes. [`OnceLock`]
/// rather than a mutable static so that ordering is enforced by the type
/// system: a read before the write yields `None`, which is the same answer as
/// "no password supplied" and therefore cannot produce a wrong decryption —
/// it can only produce a `PasswordRequired` the operator will understand.
static CLI_PASSWORD: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();

/// The password for opening encrypted documents, or `None` if none was given.
///
/// `None` is **not** the empty password. §7.6.3.1's silent empty-password
/// attempt happens inside `pdfce-core` for every document regardless; `None`
/// means only that if that attempt fails there is nothing else to try.
fn cli_password() -> Option<&'static [u8]> {
    CLI_PASSWORD
        .get()
        .and_then(Option::as_ref)
        .map(Vec::as_slice)
}

/// Resolve `--open-password` / `--open-password-file` into [`CLI_PASSWORD`].
///
/// Returns an error string for an `--open-password-file` that cannot be read, since
/// silently proceeding without a password the operator explicitly supplied
/// would surface as "this document is password-protected" and send them
/// hunting for the wrong problem.
fn resolve_cli_password(
    password: Option<String>,
    password_file: Option<PathBuf>,
) -> Result<Option<Vec<u8>>, String> {
    if let Some(p) = password {
        return Ok(Some(p.into_bytes()));
    }
    let Some(path) = password_file else {
        return Ok(None);
    };

    let raw = if path.as_os_str() == "-" {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)
            .map_err(|e| format!("reading the password from stdin: {e}"))?;
        s
    } else {
        std::fs::read_to_string(&path)
            .map_err(|e| format!("reading the password file {}: {e}", path.display()))?
    };

    // First line only, newline stripped. A password file is conventionally one
    // line, and a trailing newline that every editor adds must not silently
    // become part of the password — that failure looks exactly like a wrong
    // password and is unusually hard to see.
    let line = raw.split('\n').next().unwrap_or("");
    let line = line.strip_suffix('\r').unwrap_or(line);
    Ok(Some(line.as_bytes().to_vec()))
}

/// Open a document at `path`, supplying the CLI password if one was given.
///
/// Every subcommand that reads a file goes through here rather than calling
/// [`Document::load`] directly, so `--open-password` reaches all of them and a new
/// subcommand cannot forget it. That is the affordance half of the capability:
/// `pdfce-core` gained decryption, and a core capability no shell can reach is
/// not a feature yet.
fn open_document(
    path: &Path,
) -> Result<pdfce_core::document::Document, pdfce_core::document::DocError> {
    pdfce_core::document::Document::load_with_password(path, cli_password())
}

/// Parse a document from bytes, supplying the CLI password if one was given.
///
/// The `from_bytes` counterpart of [`open_document`], for the subcommands that
/// have already read the file (usually because they also need the raw bytes).
fn open_document_bytes(
    bytes: Vec<u8>,
) -> Result<pdfce_core::document::Document, pdfce_core::document::DocError> {
    pdfce_core::document::Document::from_bytes_with_password(bytes, cli_password())
}

/// The planned subcommand surface. Only [`Command::Inspect`] is implemented
/// at Pass 0; the rest are stubs (see the module docs). Each variant's doc
/// comment is what `pdfce-cli --help` and `pdfce-cli <cmd> --help` show.
#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect a PDF: confirm the %PDF- header and print its declared version.
    ///
    /// With `--text-blocks`, instead recognise and dump the page's
    /// **editable text-block** structure (ISO 32000-1 §14.8; Pass 14.0):
    /// the derived Run→Line→Column→Block hierarchy, every inference counted
    /// and disclosed. Strictly READ-ONLY — nothing is written. Because an
    /// untagged content stream defines no word/line/paragraph/column/reading
    /// order (§14.8, S1-S9), the whole structure is a reviewable HINT, and
    /// the sourced-only text is reported unchanged alongside it.
    Inspect {
        /// Path to the PDF file to inspect.
        file: PathBuf,
        /// Recognise and dump the editable text-block structure instead of
        /// printing the version line (read-only; §14.8, Pass 14.0).
        #[arg(long)]
        text_blocks: bool,
        /// With `--text-blocks`: 1-based pages to analyse: `all`, `3`,
        /// `1-4`, `5,1-2`. Order is honoured. Ignored without
        /// `--text-blocks`.
        #[arg(long, default_value = "all")]
        pages: String,
        /// With `--text-blocks`: emit a JSON document (full structure,
        /// per-line provenance, every diagnostic counter) instead of the
        /// human-readable report. Ignored without `--text-blocks`. Also
        /// selects JSON output for `--reflow-preview`.
        #[arg(long)]
        json: bool,
        /// Compute and dump a READ-ONLY within-block reflow PREVIEW for one
        /// recognised block (ISO 32000-1 §14.8; decision 015 / Pass 15.0):
        /// the auto-detected alignment, the greedy re-wrap's new break
        /// points and per-line origins, the new block box, and every
        /// disclosure. Strictly READ-ONLY — nothing is written, no content
        /// stream is mutated (that is Pass 15.1). Select the block with
        /// `--block` (and the page via `--pages`, first page used); tune the
        /// preview with `--width`/`--align`/`--leading`.
        #[arg(long)]
        reflow_preview: bool,
        /// With `--reflow-preview`: 0-based index of the block to preview on
        /// the selected page. Default `0`.
        #[arg(long, default_value_t = 0)]
        block: usize,
        /// With `--reflow-preview`: wrap width in points. Default = the
        /// recognised block's own box width.
        #[arg(long)]
        width: Option<f64>,
        /// With `--reflow-preview`: alignment override — `left`, `right`,
        /// `center`, or `justified` (aliases `l`/`r`/`c`/`j`/`justify`/
        /// `centre`). Default = auto-detected from glyph x-positions.
        #[arg(long)]
        align: Option<String>,
        /// With `--reflow-preview`: leading (baseline-to-baseline) in points.
        /// Default = the block's measured baseline gap.
        #[arg(long)]
        leading: Option<f64>,
    },

    /// Merge several PDFs into one, in argument order.
    ///
    /// Produces a brand-new document; every input is read and left
    /// untouched. Form fields whose fully-qualified names collide across
    /// inputs are auto-renamed with a `Doc<N>_` prefix, which is what
    /// stops same-named fields from becoming ONE logical field that
    /// fills every copy at once.
    ///
    /// PDF inputs only. Converting Word/Excel/images to PDF as part of a
    /// merge is a separate capability, not a flag on this one.
    Merge {
        /// Input PDFs, concatenated in the order given. At least two.
        inputs: Vec<PathBuf>,
        /// Output path for the merged PDF.
        #[arg(short, long)]
        output: PathBuf,
        /// Do not generate one top-level bookmark per source file.
        ///
        /// Generation is ON by default, matching Acrobat's documented
        /// Combine-Files default. The bookmark is named after the input
        /// file's stem.
        #[arg(long)]
        no_bookmarks: bool,
    },

    /// Split a PDF into several standalone files.
    ///
    /// Exactly one criterion may be given; `--every` is the default when
    /// none is. Nothing is written until every output name is known to be
    /// distinct and (unless `--force`) free.
    Split {
        /// Input PDF to split.
        input: PathBuf,
        /// Directory to write the parts into. Created if absent.
        #[arg(long)]
        out_dir: PathBuf,
        /// Fixed number of pages per output file.
        #[arg(long, default_value_t = 1, group = "criterion")]
        every: usize,
        /// Split AFTER these 1-based pages, e.g. `3,7,12`.
        #[arg(long, group = "criterion")]
        after: Option<String>,
        /// One output per top-level bookmark, breaking at the page each
        /// one targets. Nested bookmarks do not create boundaries.
        #[arg(long, group = "criterion")]
        bookmarks: bool,
        /// Output naming template. Placeholders: `{stem}` `{n}`
        /// `{start}` `{end}`; `{n}` is zero-padded to the part count so
        /// the files sort correctly.
        #[arg(long, default_value = pdfce_core::pageops::split::DEFAULT_NAME_TEMPLATE)]
        name_template: String,
        /// Overwrite existing files in the output directory.
        #[arg(long)]
        force: bool,
    },

    /// Extract pages into a new standalone PDF.
    ///
    /// The source is read and left untouched — use `delete-pages` on it
    /// afterwards for Acrobat's "extract and delete from original".
    /// Pages appear in the order given, so `--pages 5,1-2` is both a
    /// selection and an ordering.
    ExtractPages {
        /// Input PDF.
        input: PathBuf,
        /// Pages to extract, 1-based and inclusive, e.g. `3-7,9`.
        #[arg(long)]
        pages: String,
        /// Output path for the extracted pages.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Insert pages from another PDF into a target, producing a new file.
    InsertPages {
        /// The document being added to. Read, never modified.
        input: PathBuf,
        /// The PDF to take pages from.
        #[arg(long)]
        source: PathBuf,
        /// Which of the source's pages to insert, 1-based. Defaults to
        /// all of them.
        #[arg(long, default_value = "all")]
        source_pages: String,
        /// Insert BEFORE this 1-based target page. `0` means "at the
        /// start"; a value past the end means "at the end".
        #[arg(long)]
        before: Option<usize>,
        /// Insert AFTER this 1-based target page. Mutually exclusive
        /// with `--before`; the default is to append at the end.
        #[arg(long, conflicts_with = "before")]
        after: Option<usize>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Remove pages from a document.
    ///
    /// A page-tree splice: the pages leave the tree, ancestors' counts
    /// drop, and every object the removed pages owned exclusively is
    /// freed. Bookmarks and links that pointed at a removed page are
    /// reported, never silently repaired.
    ///
    /// NOT redaction. Under the default incremental save the removed
    /// pages' bytes remain in the file; deletion removes them from the
    /// document, not from the bytes.
    DeletePages {
        /// Input PDF.
        input: PathBuf,
        /// Pages to remove, 1-based and inclusive, e.g. `2,5-7`.
        #[arg(long)]
        pages: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte (ARCHITECTURE.md §11.1).
        #[arg(long)]
        verify_undo: bool,
    },

    /// Put a document's pages in a new order.
    ///
    /// `--order` is the complete new sequence of 1-based page numbers —
    /// every page exactly once. A list that drops or repeats a page is
    /// refused, because that would be a delete or a duplicate wearing a
    /// reorder's name.
    ReorderPages {
        /// Input PDF.
        input: PathBuf,
        /// The new page order, e.g. `3,1,2` or `5-8,1-4`.
        #[arg(long)]
        order: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Rotate every page, or a selection, by a multiple of 90°.
    ///
    /// `--degrees` is a turn RELATIVE to each page's current rotation,
    /// which is how a rotate-right button behaves and what ISO 32000-1
    /// Table 30 ends up storing (an absolute `/Rotate`, computed as
    /// existing + increment). Pages at different rotations therefore stay
    /// different.
    Rotate {
        /// Input PDF.
        input: PathBuf,
        /// Rotation in degrees; a multiple of 90. Negative turns left.
        #[arg(long, allow_hyphen_values = true)]
        degrees: i32,
        /// Which pages to turn, 1-based. Defaults to all of them.
        #[arg(long, default_value = "all")]
        pages: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Set one page's rotation (ISO 32000-1 Table 30 `/Rotate`).
    ///
    /// Writes the entry on the page object itself, which overrides any
    /// value inherited from an ancestor page-tree node — so rotating one
    /// page never disturbs its siblings. By default the document is
    /// saved as an incremental update, leaving every prior byte (and
    /// therefore every existing signature) intact.
    RotatePage {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number to rotate.
        #[arg(long)]
        page: u32,
        /// Rotation in degrees; must be a multiple of 90. Negative and
        /// ≥360 values are accepted and normalized (Table 30 constrains
        /// only "a multiple of 90").
        #[arg(long, allow_hyphen_values = true)]
        degrees: i32,
        /// Treat `--degrees` as a turn relative to the page's current
        /// effective rotation rather than an absolute value.
        #[arg(long)]
        relative: bool,
        /// Output path. Never the input path by default — see
        /// `--in-place`.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte (ARCHITECTURE.md §11.1). Costs one extra save.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Set or clear document information dictionary fields (§14.3.3).
    ///
    /// Creates an `/Info` dictionary if the file has none — the operator
    /// asked for the metadata by name, which is what distinguishes this
    /// from pdfce stamping its own producer id (see `--producer`).
    SetInfo {
        /// Input PDF.
        input: PathBuf,
        /// New `/Title`.
        #[arg(long)]
        title: Option<String>,
        /// New `/Author`.
        #[arg(long)]
        author: Option<String>,
        /// New `/Subject`.
        #[arg(long)]
        subject: Option<String>,
        /// New `/Keywords`.
        #[arg(long)]
        keywords: Option<String>,
        /// Remove a field entirely. Repeatable. Removal is a distinct
        /// flag rather than "pass an empty string", because an empty
        /// title and an absent title are different things in the file
        /// and a script must be able to ask for either.
        #[arg(long = "clear", value_enum)]
        clear: Vec<InfoFieldArg>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// `/Producer` handling for `--mode full` (ignored otherwise).
        #[arg(long, value_enum, default_value_t = ProducerArg::Preserve)]
        producer: ProducerArg,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte (ARCHITECTURE.md §11.1). Costs one extra save.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Add a geometric-markup annotation to a page (Pass 6.1, §12.5.6).
    ///
    /// Authors a fully-baked `/AP` appearance (R44) and patches the page's
    /// `/Annots` **without touching the page content stream** (R47), saved
    /// incrementally by default so every prior byte (and signature) stays
    /// intact. The subtype selects which geometry flag is read:
    ///
    /// | `--type` | geometry flag | example |
    /// |---|---|---|
    /// | `square`, `circle` | `--rect x0,y0,x1,y1` | `--rect 72,72,300,200` |
    /// | `line` | `--line x0,y0,x1,y1` | `--line 72,100,300,100` |
    /// | `polygon`, `polyline` | `--points "x,y x,y …"` | `--points "72,72 200,72 140,180"` |
    /// | `ink` | `--strokes "x,y x,y \| x,y …"` | `--strokes "72,72 90,120 \| 200,80 230,140"` |
    /// | `highlight`, `underline`, `strikeout`, `squiggly` | `--quads "x1,y1,…,x8,y8 ; …"` or `--rect` (one marquee quad) | `--rect 72,90,300,110` |
    ///
    /// Refusals (all exit `9`, EDIT_REFUSED): an encrypted document, an
    /// enforced certification signature (DocMDP), a page out of range,
    /// empty geometry, or a malformed `/Annots`. A file pdfce cannot open
    /// exits with the load error's own code (3/4).
    Annotate {
        /// Input PDF.
        input: PathBuf,
        /// The markup subtype to author.
        #[arg(long = "type", value_enum)]
        kind: AnnotKindArg,
        /// 1-based page number to annotate.
        #[arg(long)]
        page: u32,
        /// Rectangle `x0,y0,x1,y1` in default user space — for
        /// `square`/`circle`, and for a marquee text-markup (one quad).
        #[arg(long)]
        rect: Option<String>,
        /// Line `x0,y0,x1,y1` — for `line`.
        #[arg(long)]
        line: Option<String>,
        /// Vertices `x,y x,y …` — for `polygon`/`polyline`.
        #[arg(long)]
        points: Option<String>,
        /// Ink strokes `x,y x,y | x,y x,y` (`|` separates strokes) — for
        /// `ink`.
        #[arg(long)]
        strokes: Option<String>,
        /// Text-markup quads `x1,y1,…,x8,y8 ; …` (Z-order UL,UR,LL,LR) —
        /// overrides `--rect` for the text-markup subtypes.
        #[arg(long)]
        quads: Option<String>,
        /// The text to show — required for `freetext`, `text` (the note's
        /// popup body); optional for `stamp` (defaults to the stamp name).
        #[arg(long)]
        text: Option<String>,
        /// Standard-14 font `BaseFont` name for `freetext`/`stamp`
        /// (`Helvetica`, `Times-Roman`, `Courier`, …). Default `Helvetica`.
        #[arg(long, default_value = "Helvetica")]
        font: String,
        /// Font size in points for `freetext`. `0` = auto-size to the box
        /// height (a reviewable pdfce heuristic — §12.7.3.3 mandates no
        /// formula). Default `12`.
        #[arg(long, default_value_t = 12.0)]
        size: f64,
        /// Justification for `freetext`: `left`, `center`, or `right`
        /// (`/Q` 0/1/2). Default `left`.
        #[arg(long, value_enum, default_value_t = QuadArg::Left)]
        quad: QuadArg,
        /// Wrap `freetext` to multiple lines within the box.
        #[arg(long)]
        multiline: bool,
        /// Sticky-note icon for `text`: `note` (default), `comment`,
        /// `key`, `help`, `newparagraph`, `paragraph`, `insert`.
        #[arg(long, value_enum, default_value_t = IconArg::Note)]
        icon: IconArg,
        /// Standard stamp name for `stamp` (`draft` default, `approved`,
        /// `confidential`, `final`, `experimental`, `expired`, …).
        #[arg(long, value_enum, default_value_t = StampArg::Draft)]
        stamp_name: StampArg,
        /// Stroke/mark colour as `RRGGBB` hex. Default is per-subtype
        /// (yellow for highlight, red otherwise).
        #[arg(long)]
        color: Option<String>,
        /// Interior fill colour as `RRGGBB` hex (`square`/`circle`/
        /// `polygon`). Absent ⇒ transparent interior.
        #[arg(long)]
        fill: Option<String>,
        /// Border/stroke width in points.
        #[arg(long, default_value_t = 1.0)]
        width: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the edit reproduces the input file
        /// byte for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Mark content for redaction (ISO 32000-1 §12.5.6.23, MARK phase).
    ///
    /// This is the non-destructive first phase: it authors reviewable
    /// `/Redact` annotations, saved into the document, that a later
    /// `redact-apply` turns into true removal. Nothing is removed here —
    /// the marks can be reviewed, moved, or deleted first. Give exactly
    /// one of `--rect`, `--search`, or `--pattern`.
    ///
    /// The saved marks are drawn as a RED OUTLINE, never a filled box, so
    /// a marked-but-unapplied document can never be mistaken for a redacted
    /// one. Verify with `list-redactions`, then run `redact-apply`.
    RedactMark {
        /// Input PDF.
        input: PathBuf,
        /// Mark a single rectangle `x0,y0,x1,y1` (default user space) on
        /// `--page`.
        #[arg(long, group = "how")]
        rect: Option<String>,
        /// Mark every occurrence of this exact text (search-and-redact).
        #[arg(long, group = "how")]
        search: Option<String>,
        /// Mark every match of a simple pattern: literal text where `#`
        /// matches any digit and `?` matches any single character (e.g.
        /// `###-##-####` for a US SSN).
        #[arg(long, group = "how")]
        pattern: Option<String>,
        /// Case-insensitive matching for `--search`/`--pattern` (ASCII).
        #[arg(long)]
        ignore_case: bool,
        /// 1-based page for `--rect` (ignored by search/pattern, which
        /// scan every page).
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// Fill colour applied to the region ON APPLY, as `RRGGBB` hex.
        /// Default black — the Acrobat default.
        #[arg(long)]
        fill: Option<String>,
        /// Overlay text drawn over the region on apply (recorded on the
        /// mark; this build applies the fill and discloses overlay-text
        /// burn-in as a follow-up).
        #[arg(long)]
        overlay_text: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// **Remove embedded font programs** — a DRY RUN unless `--apply`.
    ///
    /// The first destructive font operation. It strikes `/FontFile`,
    /// `/FontFile2` or `/FontFile3` from the `/FontDescriptor` (§9.9
    /// Table 126), leaving a font reference the reader satisfies by
    /// substitution, and frees the program's object.
    ///
    /// ★ ONLY a font whose `list-fonts` verdict is `removable` may go.
    /// Every other font is refused **by name, with its reason printed** —
    /// never silently, never merely missing from the output. That is a
    /// deliberate divergence from Acrobat, which refuses the same fonts by
    /// leaving them out of its list with no explanation anywhere. Measured
    /// over 4,023 real files, refusal is the majority case by a wide margin:
    /// of 1,560 embedded fonts, 28.8 % removable, 53.6 % symbolic with a
    /// built-in encoding, 12.9 % glyph-index encoded, 4.4 % embedded CMap.
    ///
    /// ★ APPEARANCE CHANGES. `/Widths` is preserved, so every glyph keeps
    /// its exact advance, but the substituted face's own shapes and widths
    /// are not those numbers. Text sits in the same places and looks
    /// different. This is a certainty, not a risk.
    ///
    /// ★ BYTES ARE RECLAIMED BY `--mode full`, NOT by the default
    /// incremental save. An incremental update appends a revision; the
    /// freed program's bytes stay in the prior revision and the file gets
    /// LARGER. Both numbers are printed so the difference cannot be missed.
    ///
    /// By default the six-letter §9.6.4 subset tag is stripped from
    /// `/BaseFont` and `/FontName` together (Table 122 makes them equal by
    /// `shall`), because `ABCDEF+Arial` matches no installed font once the
    /// program is gone. `--keep-subset-tag` leaves both alone.
    ///
    /// A PDF/A-identified document is refused unless `--acknowledge-pdfa`:
    /// every part of ISO 19005 requires embedded fonts, so unembedding
    /// breaks the conformance the file claims about itself.
    UnembedFont {
        /// Input PDF.
        input: PathBuf,
        /// A font to unembed, by `/BaseFont` or by its family name — both
        /// `ABCDEF+Arial` and `Arial` work. Repeatable. A name that matches
        /// nothing is reported and exits non-zero.
        #[arg(long, group = "which")]
        font: Vec<String>,
        /// Unembed every font whose verdict is `removable`.
        #[arg(long, group = "which")]
        all_removable: bool,
        /// Actually write the output. Without it this is a DRY RUN: the
        /// full report is printed and no file is written.
        ///
        /// Inverted from most tools on purpose, the same way `print`
        /// requires `--send`: this removes something the file cannot get
        /// back, so the default has to be the one that cannot surprise
        /// anybody.
        #[arg(long)]
        apply: bool,
        /// Output path. Required with `--apply`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Which save path to use. `full` is the one that actually
        /// reclaims the bytes — see the command description.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Verify that undoing the operation reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
        /// Leave the §9.6.4 subset tag on `/BaseFont` and `/FontName`.
        #[arg(long)]
        keep_subset_tag: bool,
        /// Proceed on a document that identifies itself as PDF/A.
        /// Unembedding breaks that conformance; this says you know.
        #[arg(long)]
        acknowledge_pdfa: bool,
    },

    /// **Add the font programs a document is missing** — a DRY RUN unless
    /// `--apply`.
    ///
    /// The constructive mirror of `unembed-font`, and the fix for the one
    /// thing every print-on-demand service rejects a book for: a font the
    /// PDF names but does not carry. `list-fonts` reports it as
    /// `not-embedded=N`; this drives that number down and prints what is
    /// left.
    ///
    /// ★ THE SOURCE FONTS COME FROM `--font-dir`. pdfce never goes looking
    /// on its own. Point it at a folder holding the faces — on Windows,
    /// `--font-dir C:\Windows\Fonts` — and every face there is matched
    /// against the document's font names. A font nothing answers to is
    /// reported BY NAME, with what would satisfy it.
    ///
    /// ★ CHARACTER POSITIONS DO NOT MOVE. A PDF spaces text from its own
    /// `/Widths` array, never from the font program (§9.6.2.1 Table 111),
    /// and this command either leaves that array untouched or writes it from
    /// the Adobe Core-14 metrics a reader was already applying. What changes
    /// is the letterforms. That is a certainty in both directions: the
    /// layout is safe, and the shapes WILL differ where the face is not the
    /// original.
    ///
    /// ★ EXACT vs SUBSTITUTE is printed per font. `exact` means the folder
    /// held the face the document names. `alias` means a metric-compatible
    /// stand-in was used (`Helvetica` → `Arial`). `bundled` means one of
    /// pdfce's own substitute faces, which is off unless
    /// `--use-bundled-fonts` is passed.
    ///
    /// A font whose own licensing field says it may not be embedded is
    /// refused by name (§9.9). So are composite (CID) fonts, whose character
    /// codes are positions inside the specific program that is missing —
    /// no other face can stand in for one without drawing the wrong
    /// characters.
    ///
    /// The file gets BIGGER. Programs are compressed on the way in, and both
    /// save modes keep them.
    EmbedFont {
        /// Input PDF.
        input: PathBuf,
        /// A font to embed into, by `/BaseFont` or family name — both
        /// `ABCDEF+Arial` and `Arial` work. Repeatable. A name that matches
        /// nothing is reported and exits non-zero.
        #[arg(long, group = "which-embed")]
        font: Vec<String>,
        /// Embed into every font the document does not carry a program for.
        #[arg(long, group = "which-embed")]
        all_missing: bool,
        /// A folder of font files to resolve the document's font names
        /// against. Repeatable; later folders win a duplicate name.
        #[arg(long = "font-dir", value_name = "DIR")]
        font_dirs: Vec<PathBuf>,
        /// Also offer pdfce's own bundled standard-14 substitute faces when
        /// no supplied folder answers to a name.
        ///
        /// OFF by default, and not for a technical reason: the bundled
        /// faces are BSD-3-Clause (see `THIRD_PARTY_LICENSES.md`), and
        /// embedding one puts it inside a document you then distribute —
        /// which carries that licence's attribution condition with it.
        /// That is your decision to make, so pdfce does not make it for you.
        #[arg(long)]
        use_bundled_fonts: bool,
        /// Actually write the output. Without it this is a DRY RUN: the full
        /// report is printed and no file is written.
        #[arg(long)]
        apply: bool,
        /// Output path. Required with `--apply`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Which save path to use. Both keep the embedded programs;
        /// `incremental` leaves the input revision byte-identical.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Verify that undoing the operation reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Apply redactions: TRULY REMOVE the marked content (§12.5.6.23).
    ///
    /// The one destructive, irreversible operation in pdfce (R35). It
    /// removes the covered glyphs from the content stream (advance-
    /// preserving, so surviving text stays put), scrubs duplicating
    /// carriers (`/Info`, XMP), decomposes object streams so no removed
    /// object survives compressed, removes the marks, and writes a FORCED
    /// FULL REWRITE with no prior revision — so nothing is recoverable.
    ///
    /// It prints a REDACTION REPORT of exactly what was removed and which
    /// carriers were scrubbed or left. If any carrier could not be scrubbed
    /// (XFA, a tagged ActualText copy, an attachment), apply exits non-zero
    /// UNLESS you pass `--acknowledge-residuals` — there is no path where a
    /// partial redaction reads as complete. A region over an image is
    /// refused outright (pdfce cannot yet destroy image pixels).
    RedactApply {
        /// Input PDF carrying `/Redact` marks.
        input: PathBuf,
        /// Output path for the redacted document.
        #[arg(short, long)]
        output: PathBuf,
        /// Acknowledge disclosed, un-scrubbed carrier residuals and exit 0
        /// anyway (the removal itself always happens; this only governs the
        /// exit code for the disclosed residuals).
        #[arg(long)]
        acknowledge_residuals: bool,
    },

    /// List the `/Redact` marks awaiting apply in a document.
    ///
    /// Reports the count and per-page inventory computed from the
    /// document's own annotations (never a session counter), so a script
    /// can detect a marked-but-not-applied file before shipping it.
    ListRedactions {
        /// Input PDF.
        input: PathBuf,
    },

    /// Stamp Bates numbers across a batch of PDFs. [not yet implemented]
    BatesStamp {
        /// Input PDFs to stamp.
        inputs: Vec<PathBuf>,
        /// Starting number.
        #[arg(long, default_value_t = 1)]
        start: u64,
        /// Format string, e.g. `DOC-{:06}`.
        #[arg(long, default_value = "{:06}")]
        format: String,
    },

    /// Convert a PDF to a PDF/A conformance level. [not yet implemented]
    ToPdfa {
        /// Input PDF.
        input: PathBuf,
        /// PDF/A level, e.g. `2b`.
        #[arg(long)]
        level: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Validate a PDF against a PDF/A profile and print a report.
    /// [not yet implemented]
    ValidatePdfa {
        /// Input PDF to validate.
        input: PathBuf,
    },

    /// Sign a PDF with a PKCS#12 certificate (PAdES). [not yet implemented]
    Sign {
        /// Input PDF.
        input: PathBuf,
        /// PKCS#12 (.p12/.pfx) certificate file.
        #[arg(long)]
        cert: PathBuf,
        /// Output path for the signed PDF.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract a document's text content (ISO 32000-1 §9.10).
    ///
    /// Prints what the file actually says, plus the word spaces and line
    /// breaks pdfce had to DERIVE from glyph geometry — because outside
    /// a Tagged PDF the standard guarantees neither (§14.8.2.5, and the
    /// negative results S1–S9). `--json` splits the two apart run by
    /// run, so a caller that wants only the sourced characters can have
    /// exactly those.
    ///
    /// Diagnostics are never optional and never silent: how many
    /// character codes came from each rung of the §9.10.2 ladder, how
    /// many fell through it to U+FFFD, which fonts carry no recoverable
    /// Unicode at all, and how many spaces and line breaks pdfce
    /// invented.
    /// **List the document's bookmarks** (ISO 32000-1 §12.3.3).
    ///
    /// Reports the outline tree with each item's nesting level, its
    /// open/closed state, and where it points. Read-only.
    ListOutline {
        /// Input PDF.
        input: PathBuf,
        /// Print one line per bookmark with no indentation, for scripts
        /// that parse rather than read. `level=` is present either way.
        #[arg(long)]
        flat: bool,
    },

    /// **Report what each signature COVERS** — not whether it is valid.
    ///
    /// pdfce performs no cryptographic verification. This measures each
    /// signature's `/ByteRange` (§12.8.1) against the file's real length
    /// and reports what it protects, which answers a question a validity
    /// badge does not: was anything added beyond the signed range?
    ///
    /// A signature can be cryptographically perfect over the first 40 KB
    /// of a 900 KB file.
    ListSignatures {
        /// Input PDF.
        input: PathBuf,
    },

    /// **List a document's optional-content groups** — layers (§8.11).
    ///
    /// Reports each layer's name and whether a reader would DRAW it with
    /// no interaction, which is the fact a name cannot carry: a
    /// "Confidential" watermark layer that is off by default is a
    /// different document from one where it is on.
    ///
    /// Read-only. Toggling a layer is session state in a viewer with no
    /// file-format footprint unless explicitly saved, and pdfce has no
    /// save path for it — so there is no toggle to offer here.
    ListLayers {
        /// Input PDF.
        input: PathBuf,
    },

    /// **List a document's fonts** — what they are, what they cost, and
    /// which of them could safely have their embedded program removed
    /// (§9.5–9.10).
    ///
    /// One stable line per DISTINCT font object — a font referenced from
    /// forty pages is one row naming forty pages, not forty rows —
    /// followed by a document summary. Read-only; nothing is modified.
    ///
    /// Reports `/BaseFont` (and the family name when it carries a §9.6.4
    /// subset tag), the `/Subtype` and a composite font's descendant
    /// subtype, `/Encoding`, whether a program is embedded and under which
    /// descriptor key, **the program's byte size in this file**, whether
    /// `/ToUnicode` is present, the OpenType `fsType` permission bits where
    /// they can be read, and a removability verdict.
    ///
    /// The verdict is the point. For a `Type0` font on `Identity-H` the
    /// character codes in the content stream are glyph indices into that
    /// exact embedded program (§9.9 directs conforming writers to do
    /// this), so deleting the program leaves text no substitute font can
    /// draw. Each such font is named, with its reason, rather than being
    /// quietly left off a list.
    ///
    /// The summary line also states which font-bearing surfaces were
    /// searched and which were not.
    ListFonts {
        /// Input PDF.
        input: PathBuf,
        /// After each font, print the sentence explaining its verdict.
        ///
        /// Off by default so the one-line-per-font listing stays easy to
        /// parse and to scan. The distinct reasons present in the document
        /// are written to stderr regardless, so nothing is hidden by
        /// leaving this off — this flag only puts them next to the row
        /// they belong to.
        #[arg(long)]
        reasons: bool,
        /// Sort by embedded program size, largest first.
        ///
        /// The default order is first discovery, which is stable and
        /// diff-friendly. This is the order an operator asking "what is
        /// costing me the most" wants, and it is a separate question.
        #[arg(long)]
        by_size: bool,
    },

    /// **List a document's embedded files** (§7.11.4, §12.5.6.15).
    ///
    /// Reports BOTH kinds — document-level `/Names /EmbeddedFiles` and
    /// page-level `/FileAttachment` annotations — in one list, each
    /// labelled by kind, because they behave differently on save and on
    /// page deletion but an operator asking what is in a file should not
    /// need to know that to get a complete answer.
    ///
    /// Names are reported RAW. An attachment name is attacker-controlled
    /// and may contain path separators or a right-to-left override that
    /// makes `gnp.exe` render as `exe.png`; a sanitised alternative is
    /// printed alongside when the two differ.
    ///
    /// Read-only: it never writes a file out.
    ListAttachments {
        /// Input PDF.
        input: PathBuf,
    },

    /// **Report what printing this document WOULD do**, without printing.
    ///
    /// Resolves the printer, reads its resolution and printable area,
    /// and places every selected page onto the sheet — reporting the
    /// scale, the offset, and whether content would fall off the edge.
    /// It has no flag that starts a job.
    ///
    /// Acrobat clips an oversized page silently. This names the pages
    /// that would lose content, so a scripted caller can refuse before
    /// paper is consumed rather than discover it afterwards.
    /// **Send pages to a printer.** Does a DRY RUN unless `--send` is
    /// given.
    ///
    /// Every step runs either way — the device is opened, its resolution
    /// and printable area are read, placement is computed and the pages
    /// are rasterised. `--send` is the only thing that starts a job.
    ///
    /// That default is inverted from most tools on purpose: printing is
    /// irreversible, consumes paper, and occupies a device other people
    /// may share.
    Print {
        /// Input PDF.
        input: PathBuf,
        /// Printer name, as `list-printers` reports it. Defaults to the
        /// system default printer.
        #[arg(long)]
        printer: Option<String>,
        /// How the page is sized onto the sheet.
        #[arg(long, value_enum, default_value_t = PrintScaleArg::Fit)]
        scale: PrintScaleArg,
        /// An explicit percentage, where 100 is actual size. OVERRIDES
        /// `--scale` when given.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000))]
        scale_percent: Option<u32>,
        /// 1-based pages: `all`, `3`, `1-4`, `5,1-2`.
        #[arg(long, default_value = "all")]
        pages: String,
        /// **Actually print.** Without this the command stops before
        /// starting the job and reports what it would have done.
        #[arg(long)]
        send: bool,
        /// Cap the rendering resolution, in DPI.
        ///
        /// A memory decision, not a quality one: an A4 page at 600 DPI is
        /// 4960x7016 px, about 139 MB at RGBA for a single page. The cap
        /// is disclosed on stderr whenever it binds, because it is pdfce
        /// choosing a number the operator did not.
        #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(36..=2400))]
        max_dpi: u32,
        /// Write the job to a FILE instead of the printer's own port.
        ///
        /// What GDI's `lpszOutput` does. Most PDF writers sit on a
        /// `PORTPROMPT:` port and pop a Save dialog; this bypasses it,
        /// which makes them scriptable — and is a real capability rather
        /// than only a testing device.
        #[arg(long, value_name = "PATH")]
        to_file: Option<PathBuf>,
        /// How many copies.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u16).range(1..=999))]
        copies: u16,
        /// Print each page's copies together (1,1,2,2) rather than whole
        /// documents in order (1,2,1,2).
        #[arg(long)]
        uncollated: bool,
        /// Print only odd or only even DOCUMENT page numbers, within
        /// whatever `--pages` selected.
        #[arg(long, value_enum, default_value_t = SubsetArg::All)]
        subset: SubsetArg,
        /// Print the sequence back to front.
        #[arg(long)]
        reverse: bool,
        /// Sheet orientation. `auto` decides from the page's own shape.
        #[arg(long, value_enum, default_value_t = OrientationArg::Auto)]
        orientation: OrientationArg,
        /// Two-sided printing, if the device supports it. Never
        /// simulated: a printer that cannot duplex will print
        /// single-sided and `list-printers` says which can.
        #[arg(long, value_enum, default_value_t = DuplexArg::Simplex)]
        duplex: DuplexArg,
        /// Ask the driver to choose the input tray from each page's
        /// size rather than using its default tray.
        #[arg(long)]
        pick_tray: bool,
        /// Which annotation classes print.
        #[arg(long, value_enum, default_value_t = CommentsArg::Document)]
        comments: CommentsArg,
        /// Print several pages per sheet (2, 4, 6, 9, 16, …).
        ///
        /// The grid is chosen to place the first page as large as
        /// possible, rotation included — so 2-up on a portrait page
        /// turns the pages and stacks them, which is what fits.
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(2..=1024))]
        n_up: Option<u32>,
        /// Draw a border around each page's cell when using `--n-up`.
        #[arg(long)]
        n_up_border: bool,
        /// Impose as a folded booklet: two page-halves per sheet face,
        /// remapped so the fold reads in order.
        ///
        /// Pages are padded to a multiple of four with blanks, and the
        /// blanks SCATTER across sheets rather than grouping at the end
        /// — that is what a fold requires, and grouping them produces a
        /// booklet with a blank leaf in the middle.
        #[arg(long)]
        booklet: bool,
        /// Tile ONE oversized page across MANY sheets, to be taped
        /// together. The inverse of N-up.
        ///
        /// Mutually exclusive with `--n-up` and `--booklet`: all three
        /// change the shape of the job, and no two of them compose.
        #[arg(long)]
        poster: bool,
        /// Magnification applied before tiling, where 1.0 is 100%. This
        /// decides how big the assembled poster is, and therefore how many
        /// sheets it takes.
        #[arg(long, default_value_t = 1.0)]
        poster_scale: f64,
        /// Shared border in POINTS, duplicated onto adjacent tiles so the
        /// sheets can be aligned and taped without a gap at the seam.
        ///
        /// No default is invented: no source gives Acrobat's, so pdfce
        /// leaves it at zero and lets the operator choose.
        #[arg(long, default_value_t = 0.0)]
        poster_overlap: f64,
        /// Tile only pages larger than the printable area; pages that
        /// already fit print normally in the same job.
        #[arg(long)]
        poster_large_only: bool,
        /// Refuse a poster needing more sheets than this.
        #[arg(long, default_value_t = pdfce_print::imposition::DEFAULT_MAX_TILES)]
        poster_max_tiles: u32,
        /// Which edge the booklet is bound on.
        #[arg(long, value_enum, default_value_t = BindingArg::Left)]
        binding: BindingArg,
        /// Print one face of each sheet, for a printer without duplex.
        #[arg(long, value_enum, default_value_t = BookletSubsetArg::BothSides)]
        booklet_subset: BookletSubsetArg,
    },

    PrintPreview {
        /// Input PDF.
        input: PathBuf,
        /// Printer name, as `list-printers` reports it. Defaults to the
        /// system default printer.
        #[arg(long)]
        printer: Option<String>,
        /// How the page is sized onto the sheet.
        #[arg(long, value_enum, default_value_t = PrintScaleArg::Fit)]
        scale: PrintScaleArg,
        /// An explicit percentage, where 100 is actual size. OVERRIDES
        /// `--scale` when given.
        ///
        /// Reader accepts a free-form 1–1000% rather than a set of
        /// presets, so this is a number and not another enum value.
        /// Overriding rather than conflicting: `--scale` has a default,
        /// so making the two mutually exclusive would force every
        /// percentage caller to also pass a scale word they do not mean.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000))]
        scale_percent: Option<u32>,
        /// 1-based pages: `all`, `3`, `1-4`, `5,1-2`.
        #[arg(long, default_value = "all")]
        pages: String,
        /// Sheet orientation, exactly as `print` takes it. `auto`
        /// decides from the first page's own shape.
        ///
        /// It is here because orientation TURNS THE SHEET: the printable
        /// area a landscape job is placed on is the device's own,
        /// transposed. A preview that ignored it would report a scale the
        /// real print would not use, which is the one thing a preview
        /// must not do.
        #[arg(long, value_enum, default_value_t = OrientationArg::Auto)]
        orientation: OrientationArg,
    },

    /// **List the printers this machine can reach** (Windows only).
    ///
    /// Read-only. It queries the print spooler and reports nothing else;
    /// it does not open a document and cannot start a print job.
    ///
    /// The first slice of pdfce's printing support, which does not spool
    /// yet: printing consumes paper and occupies a shared device, so the
    /// half that can be built and checked without side effects is built
    /// first.
    ListPrinters,

    /// **Find text in a document's pages**, reporting where each hit is.
    ///
    /// Reports the page and the on-page bounding box of every occurrence,
    /// so a hit can be pointed at rather than merely counted. The
    /// geometry is the SAME scan `mark-redaction --search` uses, so what
    /// this finds and what that would cover cannot disagree.
    ///
    /// Searches **page content text only** — not form-field values,
    /// annotation contents, bookmarks or attachments. And matching is per
    /// text run, so a phrase the producer split across runs (at a kerning
    /// pair or a style change) is not found. Both limits are real and
    /// stated rather than left to be discovered.
    ///
    /// Changes nothing and gates on nothing: an encrypted or certified
    /// document is still searchable, because reading is not what a
    /// signature restricts.
    FindText {
        /// Input PDF.
        input: PathBuf,
        /// The text to find.
        #[arg(long)]
        needle: String,
        /// Match regardless of case.
        #[arg(long)]
        ignore_case: bool,
    },

    ExtractText {
        /// Input PDF.
        input: PathBuf,
        /// 1-based pages to extract: `all`, `3`, `1-4`, `5,1-2`.
        ///
        /// Order is honoured, so `--pages 3,1` extracts page 3 first.
        #[arg(long, default_value = "all")]
        pages: String,
        /// Write the text here instead of to stdout.
        ///
        /// When this is given, stdout carries the machine-readable
        /// result line instead of the text — so a script can capture the
        /// counters without parsing them out of the document's prose.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit a JSON document instead of plain text.
        ///
        /// The JSON exposes the sourced/derived split per run, the
        /// §9.10.2 ladder rung per glyph, and every diagnostic counter.
        #[arg(long)]
        json: bool,
        /// Include artifact content (running heads, folios, watermarks)
        /// in the extracted text.
        ///
        /// Off by default, which is a POLICY choice and not a
        /// conformance one: §14.8.2.2 states no `shall` requiring a
        /// reader to exclude artifacts — every reader-side verb there is
        /// `may`/`can`/`probably should`. Artifact runs appear in
        /// `--json` output either way, flagged.
        #[arg(long)]
        include_artifacts: bool,
    },

    /// Render a page to a PNG image.
    RenderPage {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number to render.
        ///
        /// A flag rather than a positional (the Pass 0 stub had it
        /// positional): rendering page 1 is overwhelmingly the common
        /// case, so it gets a default, and a defaulted positional in
        /// front of `-o` reads badly at a shell prompt.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// Device pixels per PDF user-space unit. 1.0 ≈ 72 DPI; for a
        /// target resolution use `scale = dpi / 72` (150 DPI ≈ 2.0833).
        ///
        /// Scale, not DPI, is the knob `pdfce-render` actually takes
        /// (`render_page(doc, page, scale)`), and passing the engine's
        /// own unit keeps the CLI from inventing a second one that has
        /// to be kept in sync.
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
        /// Output PNG path.
        #[arg(short, long)]
        output: PathBuf,
        /// Do not paint annotation appearances (markup, stamps, form-field
        /// widgets — ISO 32000-1 §12.5). Annotations are painted by
        /// default, matching what a reader shows; this flag reproduces the
        /// pre-6.0 content-only raster (for A/B comparison, or a document
        /// whose annotations you want excluded). The annotation *counters*
        /// on the result line are reported either way, so a suppressed
        /// render still discloses how many annotations the page carries.
        #[arg(long)]
        no_annotations: bool,
        /// Directory of font files to supply for the document's
        /// NON-embedded fonts (decision 012). Repeatable. pdfce walks each
        /// directory, registers every readable `.ttf`/`.otf`/`.ttc`/`.cff`/
        /// `.pfb` face under its advertised name(s) AND its filename stem,
        /// and draws a non-embedded font from a supplied face whose name
        /// matches the PDF's `/BaseFont` (e.g. `Calibri.ttf` covers a
        /// document that references `Calibri` or `ABCDEF+Calibri` without
        /// embedding it). Without this flag pdfce uses its bundled Base-14
        /// substitutes — the deterministic default (R19).
        ///
        /// Supplied fonts improve glyph SHAPES only: positions still come
        /// from the PDF's own `/Widths` (decision 004 §3.6), so layout is
        /// identical with or without `--font-dir`. Renders that use a
        /// supplied face are machine-dependent by definition and are
        /// disclosed separately (`supplied=` on the result line); they are
        /// outside pdfce's same-input-same-pixels guarantee (R63).
        /// Unreadable, oversized, or unparseable files are skipped and
        /// noted on stderr, never fatal.
        #[arg(long = "font-dir", value_name = "DIR")]
        font_dirs: Vec<PathBuf>,
        /// Force an optional-content layer (OCG) VISIBLE, by its `/Name`
        /// (ISO 32000-1 §8.11). Repeatable.
        ///
        /// Overrides the document's own default configuration
        /// (`/OCProperties /D`) for this render only — nothing is
        /// written. Use `list-layers` to see the names and which state
        /// the document itself asks for.
        ///
        /// A name that matches no layer is a NOTE on stderr, not a
        /// failure: a batch that renders a hundred drawings must not
        /// abort because one of them lacks a "Grid" layer, and silently
        /// ignoring it would let a typo produce a hundred wrong rasters
        /// with no sign anything was wrong.
        ///
        /// If a name is given to BOTH flags, that is refused rather than
        /// resolved by flag order — the operator asked for two things
        /// and pdfce cannot know which was meant.
        #[arg(long = "show-layer", value_name = "NAME")]
        show_layers: Vec<String>,
        /// Force an optional-content layer HIDDEN, by its `/Name`.
        /// Repeatable. See `--show-layer`.
        #[arg(long = "hide-layer", value_name = "NAME")]
        hide_layers: Vec<String>,
        /// Render the state a PRINTING or aggregating application would
        /// use: the `/D` default configuration alone, with `/AS` usage
        /// application dictionaries **not** applied (ISO 32000-1
        /// §8.11.4.5).
        ///
        /// Without this flag `render-page` behaves as a viewer and
        /// applies `View`-event usage at `--scale`, so a layer banded to
        /// a magnification range appears or disappears with it. With it,
        /// magnification is irrelevant and you get the state the
        /// document opens in.
        ///
        /// §8.11.4.5 NOTE 2 names this exact affordance: viewers "may
        /// also provide users with an option to view documents in this
        /// state … [permitting] an accurate preview of the content as it
        /// will appear when placed into an aggregating application or
        /// sent to a stand-alone printing system."
        #[arg(long)]
        print_state: bool,
    },

    /// List a document's annotations per page (ISO 32000-1 §12.5).
    ///
    /// Read-only inventory: for each page, every `/Annots` entry with its
    /// subtype, rectangle, flags, and whether pdfce would paint its
    /// appearance, refuse it by name (no `/AP`, unresolved `/AS`,
    /// degenerate placement), or suppress it (Hidden/NoView/Popup). This
    /// is the windowless companion to `render-page`'s annotation counters
    /// — it says *which* annotations are which, where the counters say
    /// *how many*. Emits the locale-invariant stable-line format; nothing
    /// is modified.
    ListAnnotations {
        /// Input PDF.
        input: PathBuf,
        /// 1-based pages to inventory: `all`, `3`, `1-4`, `5,1-2`.
        #[arg(long, default_value = "all")]
        pages: String,
    },

    /// List a PDF's interactive-form (AcroForm) fields (Pass 7).
    ///
    /// Prints one stable, locale-invariant line per terminal field —
    /// fully-qualified name, type, flags, value, widget count, and whether
    /// a baked `/AP` is present — followed by a document-level summary line
    /// with the form disclosures (`/NeedAppearances`, `/SigFlags`, `/CO`
    /// calculation-order length, XFA presence, fields carrying `/AA`
    /// JavaScript). Read-only; authors nothing.
    ListFields {
        /// Input PDF.
        input: PathBuf,
        /// Only list fillable fields (skip read-only, pushbuttons,
        /// signatures).
        #[arg(long)]
        fillable_only: bool,
        /// For each RICH-TEXT field, also print its formatting run by run.
        ///
        /// A rich-text field's row shows `rich=<n>runs`, which says the
        /// field HAS formatting without saying what it is. This prints the
        /// text of each run and the style resolved for it from `/RV` and
        /// `/DS` together (§12.7.3.4) — which is the question an operator
        /// asks before deciding whether a downgrade is acceptable.
        ///
        /// Off by default: it is several lines per field, and on a form of
        /// any size that would bury the one-line-per-field listing the rest
        /// of this command exists to give.
        #[arg(long)]
        rich_text: bool,
    },

    /// **Create a new text form field** (§12.7.2 + §12.5.6.19).
    ///
    /// Writes a merged field/widget dictionary, registers it in the
    /// document's `/AcroForm` `/Fields` (creating the `/AcroForm` if the
    /// document has none), adds it to the page's `/Annots`, and bakes an
    /// appearance — all additively, leaving every existing byte in place.
    ///
    /// Defaults match Acrobat's documented creation floor: Helvetica at size
    /// 0 (auto-size), black, thin solid border. The field is immediately
    /// fillable with `fill-field`.
    ///
    /// Refused by name on a document carrying an XFA layer (pdfce can write
    /// the AcroForm half but not the XFA half, and one-sided is worse than
    /// neither), when the name is already used by a field of a DIFFERENT
    /// type, and when the name belongs to a group that contains other fields.
    /// The same name and the SAME type is not a refusal — it merges.
    AddTextField {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name — also how `fill-field` and
        /// `list-fields` refer to it.
        ///
        /// A PERIOD SEPARATES LEVELS (§12.7.3.2): `Personal.Address.Zip`
        /// creates the group `Personal`, the group `Personal.Address`, and
        /// the field `Zip` inside it — reusing any of those that already
        /// exist. A name segment may not itself contain a period, so a
        /// leading, trailing or doubled one is refused rather than guessed at.
        ///
        /// REUSING AN EXISTING NAME OF THE SAME TYPE MERGES: a second widget
        /// is attached to the same field rather than a second field created.
        /// One value, two places to see and edit it — which is how a check box
        /// appears on every page of a form. A different type under the same
        /// name is refused, and so is a name that belongs to a group.
        #[arg(long)]
        name: String,
        /// 1-based page number to place the field on.
        #[arg(long)]
        page: usize,
        /// The field rectangle in PDF user space, `llx,lly,urx,ury`.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// Initial value. Omitted leaves the field empty.
        #[arg(long)]
        value: Option<String>,
        /// `/MaxLen` — maximum character count.
        #[arg(long)]
        max_len: Option<i64>,
        /// `/TU`, the accessibility name a screen reader announces.
        #[arg(long)]
        tooltip: Option<String>,
        /// Explicitly DECLINE an accessibility name (R105).
        ///
        /// Exactly one of `--tooltip` / `--no-tooltip` is required. Omitting
        /// both is an error, never a silent default: for a form field, `/TU`
        /// — not the tag tree — is what a screen reader announces, so a
        /// missing one is invisible to the person creating the field and
        /// load-bearing for the person who cannot see the form. Declining is
        /// a legitimate answer; it just has to be an ANSWER, and it is
        /// reported back in the operation's disclosures.
        #[arg(long, conflicts_with = "tooltip")]
        no_tooltip: bool,
        /// Accept multiple lines (`/Ff` bit 13).
        #[arg(long)]
        multiline: bool,
        /// Mark the field read-only (`/Ff` bit 1).
        #[arg(long)]
        read_only: bool,
        /// Mark the field required at submit time (`/Ff` bit 2).
        #[arg(long)]
        required: bool,
        /// Echo the value as bullets (`/Ff` bit 14).
        ///
        /// pdfce writes the flag; the obscuring is a viewer behaviour.
        #[arg(long)]
        password: bool,
        /// Lay the value out in equally-spaced cells (`/Ff` bit 25).
        ///
        /// REFUSED unless `--max-len` is given and neither `--multiline` nor
        /// `--password` is set. Table 228 bit 25 permits comb "only if" those
        /// hold, and a file that breaks the rule has no defined rendering —
        /// two viewers may legitimately draw it differently.
        #[arg(long)]
        comb: bool,
        /// Border line style (§12.5.4 Table 166).
        #[arg(long, value_enum, default_value_t = BorderArg::Solid)]
        border: BorderArg,
        /// Border width in points. Zero means no border.
        #[arg(long, default_value_t = 1.0)]
        border_width: f64,
        /// Where the widget is visible (§12.5.3 Table 165).
        #[arg(long, value_enum, default_value_t = VisibilityArg::Visible)]
        visibility: VisibilityArg,
        /// Pre-fill this field's properties from an existing field.
        ///
        /// Copies only NON-BOOLEAN, TYPE-MATCHED data — `--max-len` for a
        /// text field, the option list for a choice field, the on-state for
        /// a check box. A radio template copies nothing.
        ///
        /// Yes/no properties are never copied, and that is deliberate: these
        /// are presence flags, so a copied `--multiline` could be added but
        /// never turned off, and a single-line field could not be made from
        /// a multiline template. The accessibility name is never copied
        /// either — deciding it is the whole point of requiring
        /// `--tooltip`/`--no-tooltip`, and inheriting someone else's answer
        /// is not deciding.
        ///
        /// Anything given explicitly wins; this only fills gaps. When the
        /// template contributes nothing, it says so rather than silently
        /// doing nothing.
        #[arg(long, value_name = "FIELD")]
        defaults_from: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the add reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Author a new check box (ISO 32000-1 §12.7.4.2).
    ///
    /// Both appearance states are written at creation, so the box is
    /// immediately usable by `set-button-state` and immediately correct in a
    /// viewer — there is no `/NeedAppearances` fallback involved.
    AddCheckBox {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name — also how `fill-field` and
        /// `list-fields` refer to it.
        ///
        /// A PERIOD SEPARATES LEVELS (§12.7.3.2): `Personal.Address.Zip`
        /// creates the group `Personal`, the group `Personal.Address`, and
        /// the field `Zip` inside it — reusing any of those that already
        /// exist. A name segment may not itself contain a period, so a
        /// leading, trailing or doubled one is refused rather than guessed at.
        ///
        /// REUSING AN EXISTING NAME OF THE SAME TYPE MERGES: a second widget
        /// is attached to the same field rather than a second field created.
        /// One value, two places to see and edit it — which is how a check box
        /// appears on every page of a form. A different type under the same
        /// name is refused, and so is a name that belongs to a group.
        #[arg(long)]
        name: String,
        /// 1-based page number to place the box on.
        #[arg(long)]
        page: usize,
        /// The field rectangle in PDF user space, `llx,lly,urx,ury`.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// The ON state's name — the value this box exports when ticked.
        ///
        /// `Off` is reserved for the unticked state (§12.7.4.2.3) and is
        /// refused here. Override it when the form's submitted data needs a
        /// particular value, e.g. `--on-state Red`.
        #[arg(long, default_value = "Yes")]
        on_state: String,
        /// Create the box already ticked.
        #[arg(long)]
        checked: bool,
        /// `/TU`, the accessibility name a screen reader announces.
        #[arg(long)]
        tooltip: Option<String>,
        /// Explicitly DECLINE an accessibility name (R105).
        ///
        /// Exactly one of `--tooltip` / `--no-tooltip` is required. Omitting
        /// both is an error, never a silent default: for a form field, `/TU`
        /// — not the tag tree — is what a screen reader announces, so a
        /// missing one is invisible to the person creating the field and
        /// load-bearing for the person who cannot see the form. Declining is
        /// a legitimate answer; it just has to be an ANSWER, and it is
        /// reported back in the operation's disclosures.
        #[arg(long, conflicts_with = "tooltip")]
        no_tooltip: bool,
        /// Mark the field read-only (`/Ff` bit 1).
        #[arg(long)]
        read_only: bool,
        /// Mark the field required at submit time (`/Ff` bit 2).
        #[arg(long)]
        required: bool,
        /// Pre-fill this field's properties from an existing field.
        ///
        /// Copies only NON-BOOLEAN, TYPE-MATCHED data — `--max-len` for a
        /// text field, the option list for a choice field, the on-state for
        /// a check box. A radio template copies nothing.
        ///
        /// Yes/no properties are never copied, and that is deliberate: these
        /// are presence flags, so a copied `--multiline` could be added but
        /// never turned off, and a single-line field could not be made from
        /// a multiline template. The accessibility name is never copied
        /// either — deciding it is the whole point of requiring
        /// `--tooltip`/`--no-tooltip`, and inheriting someone else's answer
        /// is not deciding.
        ///
        /// Anything given explicitly wins; this only fills gaps. When the
        /// template contributes nothing, it says so rather than silently
        /// doing nothing.
        #[arg(long, value_name = "FIELD")]
        defaults_from: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the add reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
        /// Border line style (§12.5.4 Table 166).
        #[arg(long, value_enum, default_value_t = BorderArg::Solid)]
        border: BorderArg,
        /// Border width in points. Zero means no border.
        #[arg(long, default_value_t = 1.0)]
        border_width: f64,
        /// Where the widget is visible (§12.5.3 Table 165).
        #[arg(long, value_enum, default_value_t = VisibilityArg::Visible)]
        visibility: VisibilityArg,
    },

    /// Author one member of a radio group (ISO 32000-1 §12.7.4.2.1).
    ///
    /// ONE CALL PER MEMBER, not per group. Repeat with the same `--name` and
    /// a different `--export-value` to build the group up; the second call
    /// merges a widget into the field the first created, exactly as a check
    /// box repeated across pages does. There is no `add-radio-group` verb,
    /// because there is no moment at which pdfce could know you were
    /// finished — a one-member group is a legitimate intermediate state.
    ///
    /// Both appearance states are written per member, so the group is
    /// immediately usable by `set-button-state` and correct in a viewer.
    AddRadioButton {
        /// Input PDF.
        input: PathBuf,
        /// The GROUP's fully-qualified name — shared by every member, and how
        /// `set-button-state` and `list-fields` refer to it.
        ///
        /// A PERIOD SEPARATES LEVELS (§12.7.3.2): `Personal.Contact.Method`
        /// creates the groups `Personal` and `Personal.Contact` and the field
        /// `Method` inside it — reusing any that already exist.
        ///
        /// REUSING THIS NAME IS HOW A GROUP IS BUILT: each call adds a member.
        /// A check box or a text field under the same name is refused — a
        /// check box and a radio are both `/FT /Btn` and would otherwise
        /// merge into one field whose widgets disagree about whether they
        /// toggle independently or exclusively.
        #[arg(long)]
        name: String,
        /// 1-based page number to place this member on.
        ///
        /// Members may sit on DIFFERENT pages; the group is one field
        /// regardless.
        #[arg(long)]
        page: usize,
        /// This member's rectangle in PDF user space, `llx,lly,urx,ury`.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// This member's export value — its identity within the group.
        ///
        /// It is simultaneously the `/AP /N` key, the `/AS` when this member
        /// is chosen, and the `/V` the group takes (§12.7.4.2.1). Members are
        /// told apart by it and nothing else, so two members may not share
        /// one unless `--radios-in-unison` says they select together.
        ///
        /// `Off` is reserved for the unselected state (§12.7.4.2.3).
        #[arg(long)]
        export_value: String,
        /// Make this member the group's initial selection.
        #[arg(long)]
        selected: bool,
        /// `/TU`, the accessibility name a screen reader announces.
        #[arg(long)]
        tooltip: Option<String>,
        /// Explicitly DECLINE an accessibility name (R105).
        ///
        /// Exactly one of `--tooltip` / `--no-tooltip` is required.
        #[arg(long, conflicts_with = "tooltip")]
        no_tooltip: bool,
        /// `/Ff` bit 15 — once a member is chosen, clicking it again does not
        /// clear the group.
        ///
        /// Only the call that CREATES the group decides this; a later member
        /// passing a different value is told its flag was ignored rather than
        /// silently rewriting how the existing members behave.
        #[arg(long)]
        no_toggle_to_off: bool,
        /// `/Ff` bit 26 — members sharing an export value turn on together.
        ///
        /// This is also what permits a duplicate `--export-value`, which is
        /// otherwise refused. Only the creating call decides it.
        #[arg(long)]
        radios_in_unison: bool,
        /// Mark the field read-only (`/Ff` bit 1).
        #[arg(long)]
        read_only: bool,
        /// Mark the field required at submit time (`/Ff` bit 2).
        #[arg(long)]
        required: bool,
        /// Pre-fill this field's properties from an existing field.
        ///
        /// Copies only NON-BOOLEAN, TYPE-MATCHED data — `--max-len` for a
        /// text field, the option list for a choice field, the on-state for
        /// a check box. A radio template copies nothing.
        ///
        /// Yes/no properties are never copied, and that is deliberate: these
        /// are presence flags, so a copied `--multiline` could be added but
        /// never turned off, and a single-line field could not be made from
        /// a multiline template. The accessibility name is never copied
        /// either — deciding it is the whole point of requiring
        /// `--tooltip`/`--no-tooltip`, and inheriting someone else's answer
        /// is not deciding.
        ///
        /// Anything given explicitly wins; this only fills gaps. When the
        /// template contributes nothing, it says so rather than silently
        /// doing nothing.
        #[arg(long, value_name = "FIELD")]
        defaults_from: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the add reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
        /// Border line style (§12.5.4 Table 166).
        #[arg(long, value_enum, default_value_t = BorderArg::Solid)]
        border: BorderArg,
        /// Border width in points. Zero means no border.
        #[arg(long, default_value_t = 1.0)]
        border_width: f64,
        /// Where the widget is visible (§12.5.3 Table 165).
        #[arg(long, value_enum, default_value_t = VisibilityArg::Visible)]
        visibility: VisibilityArg,
    },

    /// Delete a form field entirely (ISO 32000-1 §12.7.3).
    ///
    /// Removes every widget from its page, the field dictionary, its
    /// `/AcroForm /Fields` registration, and any grouping node left childless
    /// — a named node owning nothing still occupies its slot in the
    /// fully-qualified-name space and would refuse a later field wanting the
    /// name.
    ///
    /// To remove ONE member of a radio group rather than the group, use
    /// `delete-widget`.
    DeleteField {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name, as `list-fields` reports it.
        #[arg(long)]
        name: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the deletion reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// **Delete a grouping node and every field beneath it** (§12.7.3.2).
    ///
    /// `delete-field` names ONE terminal. This names an interior node of the
    /// field tree — `Personal`, not `Personal.Name` — and removes the whole
    /// subtree: every terminal under it however deep, all their widgets, and
    /// the intermediate nodes themselves.
    ///
    /// **It removes fields you did not name**, which is why it will not run
    /// without `--yes`. Run it without that flag first: it prints exactly
    /// which terminals would go and exits without writing. That listing is
    /// the point of the command — a subtree is precisely the thing an
    /// operator cannot see the inside of before deleting it.
    ///
    /// A terminal field's name is REFUSED here rather than quietly treated
    /// as a one-field delete: the two commands remove different amounts, and
    /// guessing which you meant is how a mistyped name becomes silent data
    /// loss. Use `delete-field` for a terminal.
    DeleteFieldGroup {
        /// Input PDF.
        input: PathBuf,
        /// The grouping node's fully-qualified name — the dotted prefix
        /// `list-fields` shows on the terminals beneath it.
        #[arg(long)]
        name: String,
        /// Output path. Not written unless `--yes` is given.
        #[arg(short, long)]
        output: PathBuf,
        /// Actually perform the deletion. Without it, the affected fields
        /// are listed and nothing is written.
        #[arg(long)]
        yes: bool,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the deletion reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// **Rename a form field** (ISO 32000-1 §12.7.3.2).
    ///
    /// `--to` is a **partial** name — the one path segment this field
    /// contributes — not a fully-qualified one. Renaming `Address.City` to
    /// `Town` gives `Address.Town`; the field keeps its place in the tree,
    /// and this verb deliberately cannot re-parent it.
    ///
    /// RENAMING A GROUP RENAMES EVERYTHING UNDER IT. §12.7.3.2 builds a
    /// fully-qualified name by appending each node's partial name walking
    /// down, so renaming `Address` re-derives `Address.City` as
    /// `Location.City` — without writing to `City` at all. The output line
    /// reports `descendants_renamed` for exactly this reason: a one-field
    /// request can rename six, and every FDF, JavaScript reference and
    /// submit mapping that named them stops matching.
    ///
    /// A rename onto a name something else already holds is REFUSED, not
    /// merged — unlike `add-*`, which merges a same-type name because the
    /// caller asked for a field of that name. Here they asked for an
    /// existing field to take a new one, and fusing two identities is not
    /// something the request describes.
    RenameField {
        /// Input PDF.
        input: PathBuf,
        /// The field's current fully-qualified name, as `list-fields`
        /// reports it.
        #[arg(long)]
        name: String,
        /// The new PARTIAL name — one segment, no periods.
        #[arg(long = "to")]
        to: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the rename reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// **Delete an annotation** — any subtype, addressed as `list-annotations`
    /// reports it (ISO 32000-1 §12.5.2).
    ///
    /// The general deletion verb. Before it, pdfce could delete only the three
    /// annotation kinds that had a verb of their own (a redaction mark, a ce
    /// dimension, a form-field widget) — a highlight, a square, a stamp or a
    /// FreeText note, including ones pdfce itself authored, could not be
    /// removed at all.
    ///
    /// ADDRESSED BY `--page` + `--index`, the exact pair `list-annotations`
    /// prints, so the two commands compose: list, read the index, delete it.
    /// `--page` is 1-based, `--index` is 0-based within that page's `/Annots`
    /// array — the same convention `list-annotations` uses on its own output,
    /// not a second one invented here.
    ///
    /// FOUR THINGS CAN HAPPEN BESIDES THE OBVIOUS ONE, and the output line
    /// reports each:
    ///
    /// - Its `/Popup` window goes with it. §12.5.6.14 says a pop-up "shall not
    ///   appear alone", so this is the spec's requirement, not tidying —
    ///   `popup_removed=1`.
    /// - Replies to it (`/IRT`) SURVIVE, with their now-dangling link removed
    ///   — `replies_orphaned=N`. They are somebody's text and you asked to
    ///   delete one annotation; deleting a thread is N deletions.
    /// - `/RT /Group` subordinates of it are counted separately
    ///   (`group_promoted=N`) because the consequence is worse: while the
    ///   primary existed a reader was instructed to IGNORE their own author
    ///   and note text in favour of its, so removing it makes several other
    ///   comments start displaying text that was previously suppressed.
    /// - Appearance streams go only if nothing else uses them
    ///   (`ap_removed=N`) — forty stamps sharing one "DRAFT" stream keep it.
    ///
    /// A `/Widget` is REFUSED, not deleted: use `delete-widget` for that one
    /// widget or `delete-field` for the whole field. Deleting it here would
    /// leave the field registered in `/AcroForm /Fields` with nothing on the
    /// page, and which of the two you meant is not something this verb may
    /// guess. A `/Redact` mark and a ce dimension ARE accepted and are routed
    /// to their own verbs, so their sidecar/review semantics still apply —
    /// `route=` says which ran.
    DeleteAnnotation {
        /// Input PDF.
        input: PathBuf,
        /// Page, 1-BASED — the `page=` value `list-annotations` prints.
        #[arg(long)]
        page: usize,
        /// Index within that page's `/Annots`, 0-BASED — the `index=` value
        /// `list-annotations` prints.
        #[arg(long)]
        index: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the deletion reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Delete ONE widget of a form field (ISO 32000-1 §12.5.6.19).
    ///
    /// The usual case is dropping a member from a radio group, or one of the
    /// several places a check box appears across a form's pages.
    ///
    /// THREE THINGS CAN HAPPEN, and the output line says which:
    ///
    /// - Normally the widget goes and the field stays.
    /// - If the deleted widget held the field's VALUE, that value would name
    ///   a state no remaining widget can display, so it is cleared to `Off`
    ///   along with every survivor's appearance state — and
    ///   `selection_cleared=1` reports it.
    /// - If it was the LAST widget, the field goes too, exactly as
    ///   `delete-field` would.
    ///
    /// A group reduced to one member keeps its `/Kids` structure rather than
    /// collapsing back into a single merged dictionary: both shapes are
    /// legal, so the deletion does not rewrite object identities nobody asked
    /// it to change.
    DeleteWidget {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name, as `list-fields` reports it.
        #[arg(long)]
        name: String,
        /// Which widget to remove, numbered from 0 in the order
        /// `list-fields` reports the field's widgets.
        #[arg(long)]
        index: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the deletion reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// **Move a form field's widget** — translate its `/Rect` (§12.5.2).
    ///
    /// MOVES ONE APPEARANCE, NOT THE FIELD. A field can own widgets on
    /// several pages; this shifts the one you name and leaves its siblings
    /// where they are, reporting how many it left behind. That is the same
    /// widget-versus-field distinction `delete-widget` draws against
    /// `delete-field`.
    ///
    /// The artwork is NOT regenerated, and does not need to be: §12.5.5
    /// scales the appearance onto `/Rect` by the ratio of their extents, so
    /// a translation leaves both ratios at 1 and the existing picture is
    /// simply carried along at its original size.
    ///
    /// RESIZING IS A DIFFERENT OPERATION and is deliberately not this verb.
    /// Changing the extent makes those ratios ≠ 1, which §12.5.5 defines as
    /// a non-uniform stretch — a resized check box would get a distorted
    /// tick. A resize has to regenerate the appearance instead.
    MoveWidget {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name, as `list-fields` reports it.
        #[arg(long)]
        name: String,
        /// Which widget to move, numbered from 0 in the order `list-fields`
        /// reports the field's widgets.
        #[arg(long, default_value_t = 0)]
        index: usize,
        /// Horizontal shift in points, positive to the right.
        #[arg(long, allow_negative_numbers = true)]
        dx: f64,
        /// Vertical shift in points, positive UP — PDF user space has its
        /// origin at the bottom-left corner (§8.3.2.3), not the top.
        #[arg(long, allow_negative_numbers = true)]
        dy: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
    },

    /// Author a new push button (ISO 32000-1 §12.7.4.2.2).
    ///
    /// The button is created WITH NO ACTION and does nothing when clicked —
    /// pdfce recognises and preserves actions but never authors one. What
    /// this makes is a valid, inert control: a placeholder to be wired up
    /// elsewhere, and that is stated on every run rather than left to be
    /// discovered.
    ///
    /// A push button has no value in any state (§12.7.4.2.2 — it "shall not
    /// use the V and DV entries"), so `fill-field` cannot target it and
    /// there is no `--required` flag: a field that can never hold a value
    /// cannot be required to hold one.
    AddPushButton {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name — also how `list-fields` refers
        /// to it. This is the SCRIPT-FACING identifier, not the label; the
        /// label is `--caption`.
        ///
        /// A PERIOD SEPARATES LEVELS (§12.7.3.2): `Form.Actions.Submit`
        /// creates the group `Form`, the group `Form.Actions`, and the field
        /// `Submit` inside it — reusing any of those that already exist. A
        /// name segment may not itself contain a period, so a leading,
        /// trailing or doubled one is refused rather than guessed at.
        ///
        /// REUSING AN EXISTING PUSH BUTTON'S NAME MERGES: a second widget is
        /// attached to the same field rather than a second field created —
        /// one button, two places to press it. Each widget keeps its OWN
        /// caption, because the caption is a widget property (/MK /CA); the
        /// second add therefore does not relabel the first. A different type
        /// under the same name is refused, and so is a name that belongs to
        /// a group.
        #[arg(long)]
        name: String,
        /// 1-based page number to place the button on.
        #[arg(long)]
        page: usize,
        /// The button rectangle in PDF user space, `llx,lly,urx,ury`.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// The text printed on the button (`/MK` `/CA`).
        ///
        /// Distinct from `--name` (the script identifier) and `--tooltip`
        /// (what a screen reader announces). Defaulting any of the three
        /// from another would put an identifier on a control a person reads,
        /// so none of them is derived from the others.
        ///
        /// An empty caption is allowed and produces a blank plate; it is
        /// reported, because a blank button and a forgotten `--caption` are
        /// the same bytes.
        #[arg(long, default_value = "")]
        caption: String,
        /// `/TU`, the accessibility name a screen reader announces.
        #[arg(long)]
        tooltip: Option<String>,
        /// Explicitly DECLINE an accessibility name (R105).
        ///
        /// Exactly one of `--tooltip` / `--no-tooltip` is required. Omitting
        /// both is an error, never a silent default. This bites harder on a
        /// push button than on any other type: its `/T` is usually a script
        /// identifier and its caption is usually a bare verb, so a
        /// screen-reader user with neither a tooltip nor a meaningful name
        /// has nothing at all to go on.
        #[arg(long, conflicts_with = "tooltip")]
        no_tooltip: bool,
        /// Mark the button read-only (`/Ff` bit 1) — it renders but cannot
        /// be activated.
        #[arg(long)]
        read_only: bool,
        /// Pre-fill this button's properties from an existing push button.
        ///
        /// Copies the CAPTION and nothing else — it is the only non-boolean
        /// property a push button has. A template of any other type (or a
        /// captionless push button) contributes nothing and says so.
        ///
        /// An explicit `--caption` wins; this only fills a gap.
        #[arg(long, value_name = "FIELD")]
        defaults_from: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the add reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
        /// Border line style (§12.5.4 Table 166).
        #[arg(long, value_enum, default_value_t = BorderArg::Solid)]
        border: BorderArg,
        /// Border width in points. Zero means no border.
        #[arg(long, default_value_t = 1.0)]
        border_width: f64,
        /// Where the widget is visible (§12.5.3 Table 165).
        #[arg(long, value_enum, default_value_t = VisibilityArg::Visible)]
        visibility: VisibilityArg,
    },

    /// Author a new list box or drop-down (ISO 32000-1 §12.7.4.4).
    ///
    /// The field is created with its options and NO selection; `fill-field`
    /// puts the first value in.
    AddChoiceField {
        /// Input PDF.
        input: PathBuf,
        /// The field's fully-qualified name — also how `fill-field` and
        /// `list-fields` refer to it.
        ///
        /// A PERIOD SEPARATES LEVELS (§12.7.3.2): `Personal.Address.Zip`
        /// creates the group `Personal`, the group `Personal.Address`, and
        /// the field `Zip` inside it — reusing any of those that already
        /// exist. A name segment may not itself contain a period, so a
        /// leading, trailing or doubled one is refused rather than guessed at.
        ///
        /// REUSING AN EXISTING NAME OF THE SAME TYPE MERGES: a second widget
        /// is attached to the same field rather than a second field created.
        /// One value, two places to see and edit it — which is how a check box
        /// appears on every page of a form. A different type under the same
        /// name is refused, and so is a name that belongs to a group.
        #[arg(long)]
        name: String,
        /// 1-based page number to place the field on.
        #[arg(long)]
        page: usize,
        /// The field rectangle in PDF user space, `llx,lly,urx,ury`.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// One selectable option. Repeat for each.
        ///
        /// `LABEL` alone makes the exported value and the displayed label the
        /// same. `EXPORT=LABEL` splits them — the form submits `EXPORT` and
        /// the operator sees `LABEL`. That split is the whole point of a
        /// choice field's option list: `--option CA=Canada` submits `CA`.
        ///
        /// May be omitted: a choice field with no options is legal and
        /// saves, but cannot be filled until options are added, so creating
        /// one prints a warning rather than failing.
        #[arg(long = "option", value_name = "[EXPORT=]LABEL")]
        options: Vec<String>,
        /// Make this a drop-down (combo box) rather than a scrolling list.
        #[arg(long)]
        combo: bool,
        /// Allow typing a value that is not in the list. Combo boxes only
        /// (§12.7.4.4 Table 230).
        #[arg(long)]
        editable: bool,
        /// Allow more than one selection at a time (`/Ff` bit 22).
        #[arg(long)]
        multi_select: bool,
        /// Sort the options alphabetically by label.
        ///
        /// This REORDERS the written array, because §12.7.4.4 makes readers
        /// display `/Opt` order regardless of the sort flag.
        #[arg(long)]
        sort: bool,
        /// `/TU`, the accessibility name a screen reader announces.
        #[arg(long)]
        tooltip: Option<String>,
        /// Explicitly DECLINE an accessibility name (R105).
        ///
        /// Exactly one of `--tooltip` / `--no-tooltip` is required. Omitting
        /// both is an error, never a silent default: for a form field, `/TU`
        /// — not the tag tree — is what a screen reader announces, so a
        /// missing one is invisible to the person creating the field and
        /// load-bearing for the person who cannot see the form. Declining is
        /// a legitimate answer; it just has to be an ANSWER, and it is
        /// reported back in the operation's disclosures.
        #[arg(long, conflicts_with = "tooltip")]
        no_tooltip: bool,
        /// Mark the field read-only (`/Ff` bit 1).
        #[arg(long)]
        read_only: bool,
        /// Mark the field required at submit time (`/Ff` bit 2).
        #[arg(long)]
        required: bool,
        /// Pre-fill this field's properties from an existing field.
        ///
        /// Copies only NON-BOOLEAN, TYPE-MATCHED data — `--max-len` for a
        /// text field, the option list for a choice field, the on-state for
        /// a check box. A radio template copies nothing.
        ///
        /// Yes/no properties are never copied, and that is deliberate: these
        /// are presence flags, so a copied `--multiline` could be added but
        /// never turned off, and a single-line field could not be made from
        /// a multiline template. The accessibility name is never copied
        /// either — deciding it is the whole point of requiring
        /// `--tooltip`/`--no-tooltip`, and inheriting someone else's answer
        /// is not deciding.
        ///
        /// Anything given explicitly wins; this only fills gaps. When the
        /// template contributes nothing, it says so rather than silently
        /// doing nothing.
        #[arg(long, value_name = "FIELD")]
        defaults_from: Option<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the add reproduces the input byte for
        /// byte.
        #[arg(long)]
        verify_undo: bool,
        /// Border line style (§12.5.4 Table 166).
        #[arg(long, value_enum, default_value_t = BorderArg::Solid)]
        border: BorderArg,
        /// Border width in points. Zero means no border.
        #[arg(long, default_value_t = 1.0)]
        border_width: f64,
        /// Where the widget is visible (§12.5.3 Table 165).
        #[arg(long, value_enum, default_value_t = VisibilityArg::Visible)]
        visibility: VisibilityArg,
    },

    /// Recompute recognised Acrobat calculation scripts natively, without
    /// executing any JavaScript (decision 009 posture B).
    ///
    /// **Shows the plan and changes nothing unless `--apply` is given.** A
    /// recomputed total is something pdfce inferred from a script it did not
    /// run, so it is visible before it becomes document state (rule 4).
    ///
    /// Only exact-shape `AFSimple_Calculate` calls are recomputed. Anything
    /// else — author code, an edited built-in, a calculation naming a field
    /// this document does not contain — is left alone and reported.
    Recompute {
        /// Input PDF.
        input: PathBuf,
        /// Apply the plan and write `--output`. Without this, nothing is
        /// written and the plan is printed for review.
        #[arg(long)]
        apply: bool,
        /// Output path. Required with `--apply`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// How to read a comma in a stored value. A comma is ambiguous
        /// between a decimal point and a thousands separator, and pdfce
        /// refuses to guess by default.
        #[arg(long, value_enum, default_value_t = CommaArg::NotNumeric)]
        comma: CommaArg,
        /// Also verify that undoing the recompute reproduces the input file
        /// byte for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Reset form fields to their default values and save (§12.7.5.3).
    ///
    /// Sets each field's `/V` to its `/DV`, and **removes `/V` entirely**
    /// where there is no `/DV` — both halves are `shall` in the clause, and
    /// removal is not the same as blanking.
    ///
    /// Pushbuttons, signature fields and read-only fields are left alone and
    /// counted. **Destructive**: this discards typed answers, so it prints
    /// what it will clear unless `--apply` is given.
    ResetForm {
        /// Input PDF.
        input: PathBuf,
        /// Reset only this field, by fully-qualified name. Repeatable.
        /// Omit to reset every eligible field.
        #[arg(long = "field", value_name = "NAME")]
        fields: Vec<String>,
        /// Perform the reset and write `--output`. Without this, nothing is
        /// written and the fields that would be cleared are listed.
        #[arg(long)]
        apply: bool,
        /// Output path. Required with `--apply`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the reset reproduces the input file byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },

    /// List every form-field script, classified (decision 009 posture B).
    ///
    /// One stable line per script: which field, which `/AA` trigger, what
    /// pdfce recognised it as, and whether pdfce can natively reproduce its
    /// effect. **pdfce never executes any of it** — a recognised built-in is
    /// *read*, never run, and everything else is disclosed as unrun.
    ///
    /// Lines are locale-invariant and ordered by the field tree, so the
    /// output diffs cleanly between two revisions of the same form.
    ListScripts {
        /// Input PDF.
        input: PathBuf,
        /// Show only the scripts pdfce can natively reproduce.
        #[arg(long)]
        reproducible_only: bool,
    },

    /// Fill one or more interactive-form fields and save (Pass 7).
    ///
    /// Each `--set NAME=VALUE` sets a field by fully-qualified name: a text
    /// or choice field's value is set and its appearance regenerated
    /// (§12.7.3.3); a check-box/radio field's state is selected (VALUE is
    /// the on-state name, e.g. `Yes`, or `Off`/`on`/`true`/`1`). Saves
    /// incrementally by default (the minimal-diff path). Never flattens —
    /// the fields stay interactive.
    FillField {
        /// Input PDF.
        input: PathBuf,
        /// A field assignment `NAME=VALUE`. Repeatable.
        #[arg(long = "set", value_name = "NAME=VALUE", required = true)]
        sets: Vec<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the fill reproduces the input file byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
        /// Allow filling a RICH-TEXT field by converting it to a plain text
        /// field, discarding the stored formatting.
        ///
        /// Without this, a rich-text field is refused: writing /V while a
        /// live /RV remains would make conforming readers regenerate the
        /// appearance from the OLD text (§12.7.3.4), so the file would
        /// display something the operator never typed. Refusing is right,
        /// but it leaves the field unfillable, and this flag is the
        /// deliberate way through.
        ///
        /// It is LOSSY and irreversible within the fill: the /RV is removed
        /// and the RichText flag cleared, so bold, colour and every other
        /// span property in the stored rich text is gone. Every converted
        /// field is named individually on stderr — a count would not tell
        /// the operator WHICH field lost its formatting.
        #[arg(long)]
        downgrade_rich_text: bool,
    },

    /// Regenerate widget appearances and clear /NeedAppearances (Pass 7.1).
    ///
    /// For every text/choice field that has no baked appearance — or, when
    /// the document sets /NeedAppearances, every such field — the widget
    /// appearance is rebuilt from the field's stored value (§12.7.3.3), and
    /// the /NeedAppearances flag is removed so pdfce never emits a stale
    /// "appearances need regenerating" assertion on a file it just fixed
    /// (R51). Buttons are untouched (state selections, not generated).
    RegenerateAppearances {
        /// Input PDF.
        input: PathBuf,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
    },

    /// Flatten interactive form fields into page content (Pass 7.1).
    ///
    /// DESTRUCTIVE. Each field's appearance is burned into its page's
    /// content stream and the field is removed from /AcroForm and /Annots —
    /// the fields stop being interactive. Under the default incremental save
    /// the pre-flatten values remain recoverable in the prior revision;
    /// `--full-rewrite` writes a single revision that removes even that
    /// (R48). Refused on a certified document (flatten is structural).
    Flatten {
        /// Input PDF.
        input: PathBuf,
        /// Only flatten these fully-qualified field names (repeatable).
        /// Omit to flatten every field.
        #[arg(long = "field", value_name = "NAME")]
        fields: Vec<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Write a single-revision full rewrite that physically removes the
        /// pre-flatten field data (R48), instead of the default incremental
        /// save which leaves it recoverable in the prior revision.
        #[arg(long)]
        full_rewrite: bool,
    },

    /// Export a filled form's field data to FDF or XFDF (Pass 7.1).
    ///
    /// Read-only: writes the document's present field values to a standalone
    /// data file. FDF is a PDF-like data file; XFDF is its XML form. The
    /// source PDF path is embedded as a hint so a reader knows the data's
    /// origin.
    ExportData {
        /// Input PDF (read, never modified).
        input: PathBuf,
        /// Output data file.
        #[arg(short, long)]
        output: PathBuf,
        /// Data format to write.
        #[arg(long, value_enum, default_value_t = DataFormat::Fdf)]
        format: DataFormat,
    },

    /// Import form-field data from an FDF or XFDF file (Pass 7.1).
    ///
    /// Sets each named field's value (dispatched by the target field's type)
    /// and regenerates its appearance, then saves. A named field the
    /// document does not have is counted and skipped, never an error. The
    /// data format is detected from the file's content.
    ImportData {
        /// Input PDF.
        input: PathBuf,
        /// The FDF/XFDF data file to import.
        #[arg(long)]
        data: PathBuf,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
    },

    /// Save a PDF and verify the round-trip invariant (ARCHITECTURE.md §5).
    ///
    /// Loads the document, saves it in the chosen mode, and checks the
    /// result: byte identity where the mode promises it, reloadability
    /// always, and — unless `--no-raster` is given — that page 1
    /// re-renders to an identical raster. Exits 0 only if every check
    /// the mode promises passed; see the exit-code table in
    /// `pdfce-cli --help` and the module documentation.
    RoundTrip {
        /// Input PDF.
        input: PathBuf,
        /// Which save path to exercise.
        #[arg(long, value_enum, default_value_t = RoundTripMode::Incremental)]
        mode: RoundTripMode,
        /// Write the produced file here. Omit to verify in memory only —
        /// which is what a corpus sweep wants, since it never needs the
        /// bytes it just checked.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// `/Producer` handling for `--mode full` (ignored otherwise:
        /// incremental save never touches `/Info`).
        ///
        /// `preserve` is the default here, unlike the pdfce-core API
        /// default, because this subcommand's job is verification and a
        /// stamped `/Producer` is a deliberate byte change that would
        /// make the per-object identity check fail for one object by
        /// design. Ask for `set` explicitly when you want authorship.
        #[arg(long, value_enum, default_value_t = ProducerArg::Preserve)]
        producer: ProducerArg,
        /// Device pixels per PDF user-space unit for the raster oracle
        /// (`scale = dpi / 72`), matching `render-page --scale`.
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
        /// Skip the raster comparison. Faster, and the only option for
        /// a document whose page 1 does not render at all.
        #[arg(long)]
        no_raster: bool,
    },

    /// Edit a page's own text in place (Pass 14.1): re-encode a run + relayout.
    ///
    /// Locates `--find` within one show operator on `--page`, re-encodes
    /// `--replace` in that run's OWN font encoding (inverting /Encoding, never
    /// /ToUnicode which is one-way and lossy, ISO 32000-1 §9.6.6), preserves
    /// the §9.4.4 advance so un-edited text stays put, relayouts the edited
    /// line (reflow by default; the line may overflow the original margin,
    /// which is disclosed), and saves INCREMENTALLY. The prior text survives
    /// in the document's revision history by design (disclosed) -- to truly
    /// remove text, use `redact-apply` (a distinct, security operation). A
    /// character the run's font cannot provide is REFUSED by name (the
    /// font-on-edit gate); an embedded SUBSET refuses a glyph it does not
    /// already carry. `--font-dir` supplies non-embedded faces (decision 012).
    EditText {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number to edit.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Text to find within a single run on the page.
        #[arg(long)]
        find: String,
        /// Replacement text (re-encoded into the run's font).
        #[arg(long)]
        replace: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Pin survivors with a compensating TJ instead of reflowing the line.
        #[arg(long)]
        pin: bool,
        /// Operator-supplied font folder for non-embedded runs (decision 012).
        /// Repeatable.
        #[arg(long = "font-dir", value_name = "DIR")]
        font_dirs: Vec<PathBuf>,
    },

    /// Format a page's own text in place (Pass 14.2): size, colour, font family.
    ///
    /// Locates `--find` within one show operator on `--page` and applies any
    /// combination of three formatting changes to that run, reusing the Pass
    /// 14.1 advance-preserving surgery (only the changed text-state operators
    /// differ), then saves INCREMENTALLY (the prior state survives in history
    /// by design; to truly remove content use `redact-apply`):
    ///
    /// - `--set-size N` changes ONLY the `Tf` size operand (never the colour
    ///   operator). Size never needs new glyphs, so it always works on
    ///   existing text; the line is relaid out (reflow by default, `--pin` to
    ///   pin the tail). Arbitrary point values and multi-size flattening are
    ///   pdfce's own documented choices (Acrobat behaviour unconfirmed).
    /// - `--set-color MODEL:C,…` sets the fill colour and STORES THE CHOSEN
    ///   SPACE (`rgb:` -> `rg`, `cmyk:` -> `k`, `gray:` -> `g`) — pdfce does
    ///   NOT force-convert to DeviceRGB the way Acrobat does. A run originally
    ///   painted in a non-device space is DISCLOSED as a narrowing conversion.
    /// - `--set-font NAME` swaps to an existing Bold/Italic (or any) font
    ///   RESOURCE (by resource key or `/BaseFont`), re-encoding the run into
    ///   that face. It is gated on COVERAGE: a target that cannot show every
    ///   character in the run is REFUSED by name with nothing applied (never
    ///   `.notdef`, never a silent substitution). A successful change never
    ///   embeds a font. An outlined/vector run has no font to swap and is
    ///   refused. `--font-dir` supplies non-embedded faces (decision 012).
    ///
    /// Pass 19.1 adds three direct text-state controls, each emitted for the
    /// matched run ONLY and explicitly restored to the run's ambient value
    /// immediately after it (text state persists for the whole content stream
    /// per ISO 32000-1 §9.3, and `q`/`Q` are illegal inside a text object per
    /// §8.2 Table 51, so the scope is closed by restoring BY VALUE):
    ///
    /// - `--char-spacing V` sets character spacing `Tc` (§9.3.2). Accepts
    ///   `0.5` or `0.5pt` (ABSOLUTE — unscaled text-space units, written as
    ///   typed at any size) and `20em` (RELATIVE — 20 THOUSANDTHS of an em,
    ///   the typographic tracking unit, NOT 20 ems), which is re-derived
    ///   against the run's size so a later resize stays correct.
    /// - `--h-scale PCT` sets horizontal scaling `Tz` (§9.3.4) as a percentage
    ///   of normal width; 100 is normal. It stretches the glyphs themselves,
    ///   not just the gaps, and also scales the spacing parameters.
    /// - `--superscript` / `--subscript` / `--no-script` set the baseline via
    ///   `Ts` (§9.3.7) plus a reduced `Tf` size. The size and rise ratios are
    ///   pdfce's OWN documented defaults, NOT a parity claim (Acrobat's are
    ///   undocumented), and are printed by value in the report.
    ///
    /// Pass 19.4 completes the family with word spacing, which behaves
    /// differently from every flag above in two ways worth knowing BEFORE
    /// reaching for it:
    ///
    /// - `--word-spacing V` sets word spacing `Tw` (§9.3.3), same
    ///   `pt`/`em` unit grammar as `--char-spacing`. It applies to EVERY
    ///   occurrence of the single-byte character code 32 in the matched run —
    ///   leading spaces, trailing spaces and both halves of a doubled space
    ///   included. PDF has no per-gap word spacing; per-gap control is what
    ///   `TJ` numeric adjustments do, which is why `reflow --align justified`
    ///   distributes slack as `TJ` and not as `Tw`. The report prints how many
    ///   spaces were affected, including zero.
    /// - It is REFUSED, by name and with nothing applied, on a COMPOSITE
    ///   (Type 0 / CIDFont) run: §9.3.3 states word spacing "shall not apply
    ///   to occurrences of the byte value 32 in multiple-byte codes", so a
    ///   `Tw` there would be written into the file and do nothing. Use
    ///   `reflow` to redistribute inter-word space on a composite run.
    /// - `Tw` is multiplied by horizontal scaling (§9.4.4), so under a
    ///   `--h-scale 50` the visible gap is half the number given; the
    ///   disclosure quotes the effective value.
    ///
    /// If the run's ambient value for a parameter cannot be restored — it was
    /// inherited from outside the edited content stream — the edit is REFUSED
    /// by name with nothing applied, rather than guessing a default that would
    /// silently change content pdfce did not touch. Changing a run's width
    /// inside a JUSTIFIED line invalidates that line's slack; pdfce discloses
    /// that and offers re-justification instead of leaving it wrong.
    ///
    /// A formatting change inside a tagged (accessible) run PRESERVES its
    /// BDC/EMC+MCID wrapper and discloses that the structure tree went stale —
    /// pdfce does not reproduce Acrobat's tag-corruption defect (R72).
    FormatText {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number to format.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Text to find within a single run on the page.
        #[arg(long)]
        find: String,
        /// New font size in points (changes only the `Tf` size operand).
        #[arg(long)]
        set_size: Option<f64>,
        /// New fill colour as `MODEL:comps`, comma-separated components in
        /// `0..=1`: `rgb:1,0,0` (red), `cmyk:0,1,1,0`, `gray:0.5`. The chosen
        /// device space is STORED (never force-converted to DeviceRGB).
        #[arg(long, value_name = "MODEL:C,..")]
        set_color: Option<String>,
        /// New font family/style: an existing page font resource, named by
        /// its resource key (`F2`) or its `/BaseFont` (`Times-Bold`).
        #[arg(long, value_name = "NAME")]
        set_font: Option<String>,
        /// Character spacing `Tc` (§9.3.2) for the matched run. `0.5` or
        /// `0.5pt` is ABSOLUTE (unscaled text-space units); `20em` is
        /// RELATIVE and means 20 THOUSANDTHS of an em (the tracking unit) —
        /// not 20 ems — and is re-derived if the run is later resized.
        #[arg(long = "char-spacing", value_name = "V[pt|em]")]
        char_spacing: Option<String>,
        /// Word spacing `Tw` (§9.3.3) for the matched run — the final FF-H
        /// control. Same unit grammar as `--char-spacing`: `2` or `2pt` is
        /// ABSOLUTE (unscaled text-space units); `200em` is RELATIVE and
        /// means 200 THOUSANDTHS of an em, re-derived if the run is later
        /// resized. Applies to EVERY single-byte code 32 in the run —
        /// leading, trailing and doubled spaces included; there is no
        /// per-gap word spacing in PDF. REFUSED by name on a composite
        /// (Type 0 / CIDFont) run, where §9.3.3 makes it void.
        #[arg(long = "word-spacing", value_name = "V[pt|em]")]
        word_spacing: Option<String>,
        /// Horizontal scaling `Tz` (§9.3.4) for the matched run, as a
        /// percentage of normal glyph width. 100 is normal; must be > 0.
        #[arg(long = "h-scale", value_name = "PCT")]
        h_scale: Option<f64>,
        /// Raise the matched run to superscript (`Ts` rise + reduced size).
        #[arg(long, conflicts_with_all = ["subscript", "no_script"])]
        superscript: bool,
        /// Lower the matched run to subscript (`Ts` drop + reduced size).
        #[arg(long, conflicts_with = "no_script")]
        subscript: bool,
        /// Reset the matched run to the baseline (`0 Ts`, size unchanged) —
        /// how an inherited non-zero rise is flattened for one run.
        #[arg(long = "no-script")]
        no_script: bool,
        /// Free-form baseline rise `Ts` (§9.3.7) for the matched run — pdfce's
        /// deliberate EXCEED over Acrobat, which exposes only the coarse
        /// superscript/subscript toggle. `3.25` or `3.25pt` is ABSOLUTE
        /// (unscaled text-space units, written exactly as typed); `280em` is
        /// RELATIVE and means 280 THOUSANDTHS of an em, re-derived if the run
        /// is later resized. Positive raises the baseline. A rise moves the
        /// run WITHOUT changing its advance, so nothing after it shifts.
        /// Conflicts with the script toggles: both write `Ts`.
        #[arg(
            long,
            value_name = "V[pt|em]",
            conflicts_with_all = ["superscript", "subscript", "no_script"]
        )]
        rise: Option<String>,
        /// Apply SYNTHETIC bold: text rendering mode 2 (fill-then-stroke)
        /// with a user-space stroke width and the stroking colour matched to
        /// the fill (§9.3.6). This is a FALLBACK for when no real Bold face
        /// resolves on the page — if one does, the command REFUSES and names
        /// it, because synthesis is never an alternative to a real typeface
        /// (R90). Nothing is ever synthesized without this flag.
        #[arg(long = "bold-synthetic")]
        bold_synthetic: bool,
        /// Apply SYNTHETIC italic: a 12-degree oblique shear premultiplied
        /// into the run's text matrix. Same fallback-only gate as
        /// `--bold-synthetic`. REFUSED when a Td/TD/T* next-line operator
        /// follows the run inside the same text object (the injected matrix
        /// would displace that line), and refused with `--pin`.
        #[arg(long = "italic-synthetic")]
        italic_synthetic: bool,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Pin survivors with a compensating TJ instead of reflowing the line
        /// (only affects a size/font change; colour never shifts the line).
        #[arg(long)]
        pin: bool,
        /// Operator-supplied font folder for non-embedded runs (decision 012).
        /// Repeatable.
        #[arg(long = "font-dir", value_name = "DIR")]
        font_dirs: Vec<PathBuf>,
    },

    /// Re-wrap (reflow) a recognized paragraph in place (Pass 15.1).
    ///
    /// Applies an EXPLICIT within-block reflow: the recognized paragraph
    /// `--block` on `--page` is greedily re-wrapped to `--width` (default: the
    /// block's own detected box width) at `--align` (default: auto-detected
    /// from the block's glyph x-positions and preserved — left/right/center/
    /// justify) and `--leading` (default: the block's measured baseline gap),
    /// and ONLY that block's own content-stream object is re-emitted at the
    /// new per-line origins/breaks. JUSTIFIED full lines distribute their
    /// slack as per-gap `TJ` numbers (ISO 32000-1 §9.4.3); the last line of a
    /// paragraph is never stretched. The save is INCREMENTAL — the prior text
    /// survives in the document's revision history by design (disclosed); to
    /// truly remove text use `redact-apply` (a distinct, security operation).
    ///
    /// Preview first with `inspect --reflow-preview` (Pass 15.0, read-only).
    /// A reflow that grows the block past the page bottom EMITS the off-page
    /// content at its true position and DISCLOSES the overflow — it never
    /// silently clips or drops content (R76). A composite (Type0/CJK) block, a
    /// rotated/skewed block, or a block sharing a text object with other
    /// content is REFUSED by name (a clean, named non-zero exit — never a
    /// crash). A tagged block's BDC/EMC+MCID wrapper is preserved and its
    /// stale /ActualText disclosed (R72).
    Reflow {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number holding the block to reflow.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// 0-based index of the recognized block (paragraph) to reflow.
        #[arg(long, default_value_t = 0)]
        block: usize,
        /// Wrap width in points (default: the block's detected box width).
        #[arg(long)]
        width: Option<f64>,
        /// Alignment override: `left`, `right`, `center`, or `justified`
        /// (default: auto-detected from glyph x-positions and preserved).
        #[arg(long)]
        align: Option<String>,
        /// Leading (baseline-to-baseline) in points (default: the block's
        /// measured baseline gap).
        #[arg(long)]
        leading: Option<f64>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Add NEW text as real page content (Pass 16.0/16.1 / FF-D).
    ///
    /// Two modes: **point** (`--at "x,y"`, 16.0) shows the whole `--text` as one
    /// un-wrapped line; **boxed** (`--box "x,y,w,h"`, 16.1) wraps `--text` to the
    /// box width via the shipped 15.x greedy breaker, laid out top-anchored from
    /// the box top with `--align` (left|center|right|justify). Exactly one of
    /// `--at`/`--box` is required. Either mode APPENDS a fresh `BT…ET` run
    /// (default user space, §9.4.4) as a new content stream in the page
    /// `/Contents` array (ISO 32000-1 §7.7.3.3) — every ORIGINAL
    /// content stream stays byte-identical (R32/R46); only the page dict's
    /// `/Contents` reference, one new stream, and one new `/Font` entry change.
    /// The run defaults to a bundled Standard-14 face (`--font`, default
    /// Helvetica), written by name+code with NO embedding (R79 / §9.6.2.2) — so
    /// it is decision 014's most-editable font case and never hits the
    /// embedded-subset wall. A character the chosen face cannot represent is
    /// REFUSED by name (the F-refuse gate, R71) — a clean, named non-zero exit,
    /// never a crash or a faked glyph.
    ///
    /// This is genuine page content, NOT a `/FreeText` annotation (R78): the
    /// added run is thereafter editable with `edit-text`, formattable with
    /// `format-text`, and reflowable with `reflow`, exactly like the page's own
    /// text. The save is INCREMENTAL. On a TAGGED page the new run is untagged
    /// and that is disclosed (R73 — no structure element is fabricated). If the
    /// page inherited its `/Resources`, pdfce gives it its own (referencing the
    /// same shared sub-dictionaries) rather than mutating the shared ancestor
    /// (§7.7.3.4) — also disclosed. `--font-dir` registers an operator-supplied
    /// face so the disclosed provenance is `Supplied` (shapes only; the written
    /// dict is identical — decision 012).
    AddText {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number to add text to.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// POINT mode (16.0): origin `x,y` in points (default user space) — the
        /// run's absolute text-matrix translation (§9.4.2), e.g. `72,700`. The
        /// whole `--text` is one un-wrapped line. Mutually exclusive with
        /// `--box`; exactly one of `--at`/`--box` is required.
        #[arg(long, value_name = "X,Y")]
        at: Option<String>,
        /// BOXED mode (16.1): wrap `--text` to the rectangle `x,y,w,h` (points;
        /// `x,y` = lower-left corner), laid out top-anchored from the box top
        /// via the shipped 15.x greedy breaker. Multi-line, honours `--align`.
        /// Mutually exclusive with `--at`. Text taller than the box, or growing
        /// past the page, is DISCLOSED and still emitted in full (R76).
        #[arg(long = "box", value_name = "X,Y,W,H")]
        wrap_box: Option<String>,
        /// BOXED mode alignment: `left` (default) | `center` | `right` |
        /// `justify`. A fresh box has no glyphs to auto-detect from, so
        /// alignment is an explicit choice (justify distributes inter-word
        /// slack; the last line of each paragraph is left un-stretched).
        #[arg(long, value_name = "MODE")]
        align: Option<String>,
        /// BOXED mode leading (baseline-to-baseline, points). Omitted = the
        /// derived default `1.2 x size` (disclosed).
        #[arg(long, value_name = "PT")]
        leading: Option<f64>,
        /// The text to add. In BOXED mode a `\n` (literal newline in the
        /// argument) forces a hard line break; runs of spaces collapse.
        #[arg(long)]
        text: String,
        /// Standard-14 `BaseFont` name (`Helvetica`, `Times-Roman`, `Courier`,
        /// `Symbol`, …) or `auto` (= Helvetica). Exact §9.6.2.2 spelling.
        #[arg(long, default_value = "auto")]
        font: String,
        /// Font size in points.
        #[arg(long, default_value_t = 12.0)]
        size: f64,
        /// Fill colour as `r,g,b` components in `0..=1` (e.g. `1,0,0` red).
        /// Omitted = black.
        #[arg(long, value_name = "R,G,B")]
        color: Option<String>,
        /// Operator-supplied font folder (decision 012): registering a face for
        /// the chosen `--font` name discloses provenance `Supplied`. Repeatable.
        #[arg(long = "font-dir", value_name = "DIR")]
        font_dirs: Vec<PathBuf>,
        /// SUBSET AND EMBED this font file, so the saved PDF carries its own
        /// glyphs for the added text (FF-C, decision 021 / Pass 21.0).
        ///
        /// Without this, `add-text` writes a Standard-14 face by name with no
        /// embedding (R79), which means the text is limited to that face's
        /// repertoire — in practice WinAnsi, so no Greek, Cyrillic, CJK or
        /// Hebrew at all. With it, pdfce reads the given face, keeps only the
        /// glyphs this text needs, and adds them to the document as a new
        /// `/Type0` resource. Nothing already in the file is rewritten.
        ///
        /// This is ALWAYS explicit and never inferred — not from
        /// `--font-dir`, not from the text needing it (R108). Embedding
        /// changes the file size and redistributes someone else's font, so
        /// pdfce will refuse rather than decide for you.
        ///
        /// TrueType (`.ttf`) only in this first cut; a CFF/PostScript
        /// (`.otf`) face is refused by name. `--box` is not yet supported
        /// with an embedded face.
        #[arg(long = "embed-font", value_name = "FONT-FILE")]
        embed_font: Option<PathBuf>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Author a dimension (Pass 12.M2): a scaled measurement `/Line`
    /// `/IT /LineDimension` annotation with a baked appearance, on its group's
    /// optional-content layer, with the scale mirrored into a portable
    /// `/Measure` dict and the authoritative `/PieceInfo` sidecar updated.
    /// Purely additive — existing page content is left byte-verbatim.
    DimensionAdd {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// Dimension kind.
        #[arg(long, value_enum, default_value_t = DimKindArg::Linear)]
        kind: DimKindArg,
        /// Points as `x,y x,y ...` (space- or `;`-separated, in points).
        /// Linear uses the first two; radius/diameter fits a Taubin circle to
        /// all of them (needs at least 3 non-collinear points).
        #[arg(long)]
        points: String,
        /// Target group id (0 = the always-present default group).
        #[arg(long, default_value_t = 0)]
        group: u32,
        /// Linear alignment constraint.
        #[arg(long, value_enum, default_value_t = ConstraintArg::Aligned)]
        constraint: ConstraintArg,
        /// Standoff of the dimension line from the first point, in points,
        /// perpendicular to the measured axis (Pass 27.1).
        ///
        /// Positive is up for a horizontal dimension, right for a vertical
        /// one, and the sign does not depend on which point you gave first.
        /// 0 (the default) draws the dimension line through the first point,
        /// which is rarely what a drawing wants — a real drawing stands its
        /// dimensions off the geometry so the extension lines are visible.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        offset: f64,
        /// Where the value text sits along the dimension line, in points from
        /// its midpoint (Pass 27.1). 0 is centred.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        text_along: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// List the dimension groups and dimensions stored in a document
    /// (Pass 12.M2) — reads the authoritative `/PieceInfo` sidecar.
    DimensionList {
        /// Input PDF.
        input: PathBuf,
    },
    /// **Set a ce dimension group's drafting standard** (Pass 27.2) and
    /// regenerate every member to it.
    ///
    /// ANSI (the default) breaks the dimension line and centres the value in
    /// the gap, with all text horizontal. ISO runs the line unbroken with the
    /// value above it, aligned to the line, and uses a comma decimal marker —
    /// mandated by ISO 129-1:2018 cl. 4.1.1 — which is also mirrored into the
    /// portable `/Measure` dict so a conforming reader computes the same
    /// string pdfce drew.
    ///
    /// pdfce draws **ISO-style**, not "ISO 129-1 conformant": that standard's
    /// normative Annex A is paywalled and was not obtained, so the claim would
    /// be broader than the evidence.
    GroupSetStandard {
        /// Input PDF.
        input: PathBuf,
        /// Group id, as printed by `dimension-list`.
        #[arg(long, default_value_t = 0)]
        group: u32,
        /// The drafting standard.
        #[arg(long, value_enum)]
        standard: StandardArg,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Place a ce dimension** (Pass 27.1): set how far its dimension line
    /// stands off the geometry and where its value sits along that line.
    ///
    /// This is the batch form of dragging a dimension in the GUI, and like the
    /// drag it does NOT re-measure: the measured points stay where they were,
    /// the extension lines stretch, and the printed value is unchanged. Read
    /// the current values from `dimension-list`.
    ///
    /// Refused by name for a circular dimension, which has no axis to stand
    /// off from or slide along.
    DimensionOffset {
        /// Input PDF.
        input: PathBuf,
        /// The ce dimension id, as printed by `dimension-list`.
        #[arg(long)]
        dimension: u32,
        /// Standoff from the first measured point, in points, perpendicular to
        /// the measured axis. Positive is up for a horizontal dimension, right
        /// for a vertical one.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        offset: f64,
        /// Where the value sits along the dimension line, in points from its
        /// midpoint. 0 is centred.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        text_along: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Set a placed ce dimension's radius/diameter display** (Pass 34.2).
    ///
    /// Radius-versus-diameter used to be a draw-time choice only: whatever was
    /// picked when the ce dimension was authored was permanent, and the only
    /// way to change it was to delete and redraw — which also loses the
    /// dimension's id, its group and its placement. This changes the reading
    /// on an already-placed ce dimension.
    ///
    /// It does NOT re-measure: the fitted circle's centre, radius and fit
    /// residual are untouched, and only the flag deciding whether the label
    /// prints `r` or `2r` moves. Read the current reading from
    /// `dimension-list`.
    ///
    /// Refused by name for a LINEAR ce dimension, which has no circle and so
    /// no radius or diameter to choose between.
    DimensionDisplay {
        /// Input PDF.
        input: PathBuf,
        /// The ce dimension id, as printed by `dimension-list`.
        #[arg(long)]
        dimension: u32,
        /// Which reading the label should print.
        #[arg(long, value_enum)]
        show: DisplayReading,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Delete a ce dimension** (Pass 25.6): remove its `/Annots` reference,
    /// its annotation dictionary, its `/AP` appearance stream and its
    /// `/PieceInfo` sidecar record, together, as one undoable command.
    ///
    /// Find the id with `dimension-list`. The dimension's GROUP is left alone
    /// even when this was its last member — a group carries a calibrated scale
    /// that is not cheap to redo.
    DimensionDelete {
        /// Input PDF.
        input: PathBuf,
        /// The ce dimension id, as printed by `dimension-list`.
        #[arg(long)]
        dimension: u32,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// Create a named dimension group (Pass 12.M2). Prints the new group id.
    GroupAdd {
        /// Input PDF.
        input: PathBuf,
        /// The group name.
        #[arg(long)]
        name: String,
        /// The display unit: `mm|cm|m|in|ft|ft-in`.
        #[arg(long, default_value = "mm")]
        unit: String,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// Set a dimension group's scale + units, regenerating every member's
    /// baked appearance (Pass 12.M2).
    GroupSetScale {
        /// Input PDF.
        input: PathBuf,
        /// Target group id (0 = default group).
        #[arg(long, default_value_t = 0)]
        group: u32,
        /// Real-length path: the real-world length of a drawn reference line,
        /// written the way a drawing writes it.
        ///
        /// Accepts `55 5/8"`, `4'-7 1/2"`, `12'`, `1200mm`, `1.2m`, or a plain
        /// number (which uses `--unit`). A notation that names a unit sets the
        /// group's unit too, so `--unit` is only needed for a bare number —
        /// the same rule the GUI field follows, so a command and a click
        /// produce the same result from the same text.
        #[arg(long)]
        real_length: Option<String>,
        /// Real-length path: the drawn reference line's length in points.
        #[arg(long)]
        drawn: Option<f64>,
        /// Direct-ratio path, e.g. `1:100` (paper:real; inch paper-unit basis).
        #[arg(long)]
        ratio: Option<String>,
        /// Display unit: `mm|cm|m|in|ft|ft-in`.
        #[arg(long, default_value = "mm")]
        unit: String,
        /// Set an explicit 1:1 (full-size) scale instead of calibrating.
        #[arg(long)]
        one_to_one: bool,
        /// Decimal precision (decimal units) or fraction denominator (ft-in).
        #[arg(long)]
        precision: Option<u32>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// Toggle a dimension group's optional-content layer visibility
    /// (Pass 12.M2, §8.11 `/D` config).
    LayerToggle {
        /// Input PDF.
        input: PathBuf,
        /// Target group id.
        #[arg(long, default_value_t = 0)]
        group: u32,
        /// Hide the layer (default is to show it).
        #[arg(long)]
        hide: bool,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **List** a page's vector objects in paint order — the index discovery
    /// path for `object-move`, `object-delete` and `node-move`.
    ///
    /// Read-only; nothing is written. One `object …` line per selectable
    /// object, then an `object-list …` summary line. The `index=` on each
    /// line IS the value those three editing subcommands take as `--object`:
    /// both come from the same `pdfce_core::vector::decompose_page` walk, in
    /// the same paint order, so the correspondence is exact and not a
    /// convention this subcommand invents.
    ///
    /// Every geometry figure is in **PDF user space** (page space): origin at
    /// the page's lower-left, Y increasing upward, units of points (1/72 in).
    /// That is the same frame `object-move --dx/--dy`, `node-move --x/--y`
    /// and `dimension-add --points` use.
    ///
    /// `--hit X,Y` additionally answers "which object would a click here
    /// select?" by calling the SAME `pdfce_core::vector::hit_test_point` the
    /// GUI's object-edit tool calls, so the answer is authoritative for the
    /// GUI's behaviour rather than a second implementation of it. One
    /// difference, deliberately: the GUI receives a *canvas-space* pointer
    /// (Y-down device coordinates, page rotation applied) and converts it to
    /// PDF space before hit-testing, whereas `--hit` takes PDF space
    /// directly — so on a rotated or non-zero-origin page the number you
    /// would read off a screen ruler is NOT the number to pass here. Use the
    /// `bbox=` values this subcommand prints.
    ///
    /// `--all-hits` adds one `hit-candidate …` line per object under the
    /// point, front-most first — the same list the GUI's Alt+click cycling
    /// steps through, from the same
    /// `pdfce_core::vector::hit_test_point_all`. Without it the `hit …`
    /// line names only the winner, which cannot answer "why did my click
    /// select THAT?" when two objects overlap.
    ///
    /// A `--hit` MISS is a valid answer, not an error: the exit code stays 0
    /// and the `hit …` line reports `index=none`. Scripts branch on that
    /// field, not on the exit status.
    ///
    /// Example — inventory page 1:
    ///
    ///     pdfce-cli object-list drawing.pdf --page 1
    ///
    /// Example — ask what a click at (200, 200) would select:
    ///
    ///     pdfce-cli object-list drawing.pdf --page 1 --hit 200,200
    ///
    /// Example — ask what ELSE is under that point, in cycling order:
    ///
    ///     pdfce-cli object-list drawing.pdf --page 1 --hit 200,200 --all-hits
    ///
    /// Example — move whatever object index 2 turned out to be:
    ///
    ///     pdfce-cli object-move drawing.pdf --object 2 --dx=10 --dy=0 -o out.pdf
    ObjectList {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// Report which object a click at this page-space point would select,
        /// as `X,Y` in PDF user space (points).
        #[arg(long, value_name = "X,Y", allow_hyphen_values = true)]
        hit: Option<String>,
        /// List EVERY object under `--hit`, front-most first, as
        /// `hit-candidate …` lines with an `ordinal=` field — the order the
        /// GUI's Alt+click click-through cycling visits them in. The `hit …`
        /// line is still printed and still names the topmost. Ignored
        /// without `--hit`.
        #[arg(long)]
        all_hits: bool,
        /// Descend INTO this object index and report which of its subpaths the
        /// `--hit` point lands on, nearest first, as `subpath-hit …` lines.
        ///
        /// A CAD producer routinely emits an entire drawing view as ONE path
        /// object with hundreds of subpaths — one measured SolidWorks export
        /// has a single object holding 1194 subpaths and 6681 anchors for a
        /// whole isometric view. Per-object hit testing correctly names that
        /// object, which is useless if what you meant was one line of it. This
        /// is the level below: the same query the GUI runs after a double-click
        /// enters an object.
        ///
        /// Combine with `--hit`; ignored without it. Prints nothing for a
        /// non-path object or an out-of-range index.
        #[arg(long, value_name = "INDEX")]
        enter: Option<usize>,
        /// Page-space slack, in points, a `--hit` point may miss an object's
        /// edge by and still select it. Default 3.0 — the GUI's
        /// `FALLBACK_SELECT_TOLERANCE`, i.e. the catch radius a click gets at
        /// 100% zoom. Ignored without `--hit`.
        ///
        /// `allow_hyphen_values` matches the other numeric operands in this
        /// CLI (`--dx`, `--dy`, `--x`, `--y`): a leading `-` must reach the
        /// f64 parser as a value, so a negative reaches the handler's
        /// named refusal instead of dying as a clap usage error (exit 2)
        /// that tells the operator nothing about why it is wrong.
        #[arg(long, default_value_t = HIT_TOLERANCE_PT, allow_hyphen_values = true)]
        tolerance: f64,
    },
    /// **Move** a vector object (Pass 9c-min, decision 011 §2.5): translate
    /// all of an object's path-construction operands by a page-space
    /// `(dx, dy)` via content-stream surgery. Only the edited content stream
    /// changes; every other object stays byte-verbatim (the R46/§5.7 named
    /// exception). `--object` is the object's 0-based paint-order index —
    /// run `object-list` on the page to discover it.
    ObjectMove {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// Page-space x displacement, in points.
        #[arg(long, allow_hyphen_values = true)]
        dx: f64,
        /// Page-space y displacement, in points.
        #[arg(long, allow_hyphen_values = true)]
        dy: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Delete** a vector object (Pass 9c-min, decision 011 §2.5): remove an
    /// object's construction + painting operators from the content stream via
    /// surgery (R46/§5.7). Works on any object kind (path/text/image). NOT
    /// redaction — it removes a drawing object from a page, not covered
    /// content for security.
    ObjectDelete {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Move one subpath** of a path object (Pass 28.0): translate a single
    /// subpath's construction operands, leaving the object's other subpaths
    /// byte-verbatim.
    ///
    /// The companion to `subpath-delete`, for the same CAD-export case: when
    /// one path object holds a whole drawing view, moving "this line" means
    /// moving one of its subpaths.
    ///
    /// Refused for a subpath that starts implicitly (a segment after `h`,
    /// whose start point is inherited rather than written) — translating the
    /// operands that exist would tear it away from a start that stayed put.
    SubpathMove {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index.
        #[arg(long)]
        object: usize,
        /// 0-based subpath index within that object.
        #[arg(long)]
        subpath: usize,
        /// Page-space x displacement, in points.
        #[arg(long, allow_hyphen_values = true)]
        dx: f64,
        /// Page-space y displacement, in points.
        #[arg(long, allow_hyphen_values = true)]
        dy: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Export a page's vector geometry as DXF** — the format SOLIDWORKS,
    /// AutoCAD and plasma-table controllers import natively.
    ///
    /// WHY THIS EXISTS. SOLIDWORKS gates its own PDF import on Adobe Acrobat
    /// or Illustrator being installed and licensed. It imports DXF with no
    /// Adobe dependency at all — so this does not work around that gate, it
    /// makes it irrelevant.
    ///
    /// SCALE IS THE THING TO GET RIGHT. A PDF drawing is at PAPER scale, so a
    /// 1:2 detail view exports at half size and looks entirely plausible.
    /// Pass `--scale 2` for a 1:2 view. Every generic PDF-to-DXF converter
    /// skips this and says nothing.
    ///
    /// Circles and arcs are RECOGNISED, not flattened: PDF has no arc
    /// primitive, so a hole arrives as four Bezier curves, and emitting those
    /// as fine polylines is what turns forty washers into a 767 KB file.
    /// `--no-fit-arcs` disables it if you want the curves verbatim.
    ///
    /// The output carries no `MATERIAL` object and no group code 94, so it
    /// loads in AutoCAD LT 2004 and older CAM controllers that reject both.
    ///
    /// NOT A ROUND TRIP TO A MODEL. A PDF of a CAD drawing is printed output
    /// — derived geometry. You get sketch entities, never features and never
    /// dimensions as constraints.
    ExportDxf {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number. One page, one `--output` file.
        #[arg(long, default_value_t = 1, conflicts_with = "pages")]
        page: u32,
        /// 1-based pages to export, one DXF each into `--output-dir`:
        /// `all`, `3`, `1-4`, `5,1-2`.
        ///
        /// Files are named `<stem>_p<n>.dxf`, zero-padded to the widest
        /// page number in the run so they sort in page order — the same
        /// naming the GUI's multi-page export uses, deliberately, so a
        /// batch script and an operator produce interchangeable output.
        ///
        /// The drawing scale is derived from the ce dimensions on **all**
        /// the selected pages together, because one `--scale` serves the
        /// whole run. Pages at different scales are therefore a REFUSAL,
        /// exactly as two disagreeing groups on one page are: export them
        /// in separate runs, or pass `--scale`.
        #[arg(long, conflicts_with = "page")]
        pages: Option<String>,
        /// Output `.dxf` path (single-page mode).
        #[arg(short, long, conflicts_with = "output_dir")]
        output: Option<PathBuf>,
        /// Existing directory to write one DXF per page into
        /// (multi-page mode; requires `--pages`).
        #[arg(long, conflicts_with = "output", requires = "pages")]
        output_dir: Option<PathBuf>,
        /// Output units.
        #[arg(long, value_enum, default_value_t = DxfUnitArg::In)]
        units: DxfUnitArg,
        /// Drawing scale — real-world units per paper unit. `2` for a 1:2
        /// view, `0.5` for a 2:1 view.
        ///
        /// OMIT IT and pdfce derives the scale from the ce dimensions
        /// already on the page: if the drawing has been calibrated with the
        /// measure tool's "scale by known dimension", that answer is exactly
        /// what this needs, and the derived figure is printed before the
        /// file is written. With nothing calibrated, the export falls back
        /// to paper scale and says so loudly. If two dimension groups
        /// disagree — a 1:1 plan and a 1:5 detail on one sheet — the export
        /// is REFUSED and both are listed, because DXF carries one scale and
        /// picking either silently exports half the sheet wrong.
        #[arg(long)]
        scale: Option<f64>,
        /// Emit Bezier curves verbatim as SPLINEs instead of recognising
        /// circles and arcs.
        #[arg(long)]
        no_fit_arcs: bool,
        /// Leave the page's text out of the DXF entirely.
        ///
        /// By default each text run becomes a TEXT entity on its own layer
        /// (`PDFCE_TEXT`), so the drawing's dimensions and notes are
        /// readable and can be switched off in one click without touching
        /// the geometry. Pass this when the destination is a cutting table
        /// and any stray entity is a hazard.
        #[arg(long)]
        no_text: bool,
    },
    /// **Delete ONE text run** — one show operator — out of a text object
    /// (`Pass 32.0`, ISO 32000-1 §9.4).
    ///
    /// The text-side twin of `subpath-delete`. Deletion is otherwise
    /// object-granular, and a CAD exporter puts every label on a sheet inside
    /// ONE `BT`...`ET` — measured on a real drawing, one text object holding
    /// all 237 dimension labels — so deleting "a label" deleted all of them.
    ///
    /// `--run` is 0-based in content order, the same numbering `object-list`
    /// reports as `runs=`.
    ///
    /// REFUSED when the FOLLOWING run has no position of its own. §9.4.2
    /// leaves the pen advanced past the string just drawn, so such a run
    /// starts wherever this one ends; removing this one would slide it, in an
    /// edit that round-trips and passes `--verify-undo` and is still wrong.
    /// The remedy is in the message and always works: delete the later run
    /// first.
    ///
    /// Deleting the only run deletes the text object.
    TextRunDelete {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// 0-based show-operator index within that text object, content order.
        #[arg(long)]
        run: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Delete one subpath** of a path object (Pass 25.2): remove a single
    /// subpath's construction operators via surgery (R46/§5.7), leaving the
    /// object's other subpaths byte-verbatim.
    ///
    /// This is the operation for CAD output. A producer routinely emits a whole
    /// drawing view as ONE path object — a measured SolidWorks export has a
    /// single stroked path holding 1194 subpaths for one isometric view — so
    /// `object-delete` there removes the entire view. This removes one line of
    /// it. Find the index with
    /// `object-list --hit X,Y --enter OBJECT`, which prints `subpath-hit` lines
    /// nearest-first.
    ///
    /// NOT redaction: it removes a drawing element from a page, not covered
    /// content for security.
    ///
    /// Refused, by name and before any mutation, when the path defines a
    /// clipping region (deleting part of it would change what OTHER content is
    /// visible), and when the subpaths found in the operators disagree in count
    /// with the geometry (the index could then name a different line from the
    /// one intended). Deleting the only subpath deletes the object.
    SubpathDelete {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// 0-based subpath index within that object, in decomposition order —
        /// the same order `object-list --enter` reports.
        #[arg(long)]
        subpath: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Delete a node** of a path object (Pass 36.1): remove ONE anchor via
    /// surgery (R46/§5.7), joining its neighbours directly. `--node` is the
    /// anchor's 0-based index in decomposition order — the same numbering
    /// `node-move` takes.
    ///
    /// The segment operator that produced the anchor is excised. When the
    /// anchor is its subpath's FIRST, the following operator is rewritten into
    /// the new `m` instead; if that follower was a curve, its control points
    /// go with it and the loss is disclosed on stderr.
    ///
    /// Refused, by name and before any mutation, when the removal would leave
    /// a part with fewer than two points (delete the part instead), when the
    /// anchor is a corner of an `re` rectangle (no operand names it, and the
    /// result would be a triangle), when it is the inherited start of an
    /// `h`-reopened subpath (its coordinates belong to the part before it),
    /// and when the path defines a clipping region.
    NodeDelete {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// 0-based anchor node index (decomposition order).
        #[arg(long)]
        node: usize,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Drag a node** of a path object (Pass 9c-min, decision 011 §2.5):
    /// move ONE anchor to a page-space point via surgery (R46/§5.7).
    /// `--node` is the anchor's 0-based index in decomposition order (start,
    /// then each segment endpoint, across subpaths). Every anchor is
    /// draggable, including an `re` rectangle corner and the inherited start
    /// of a subpath reopened after `h` — each of which has no operand of its
    /// own, so one is materialized and the change of form is disclosed on
    /// stderr (Pass 30.0).
    NodeMove {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// 0-based anchor node index (decomposition order).
        #[arg(long)]
        node: usize,
        /// New anchor x, page space (points).
        #[arg(long, allow_hyphen_values = true)]
        x: f64,
        /// New anchor y, page space (points).
        #[arg(long, allow_hyphen_values = true)]
        y: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Move SEVERAL anchors of one path object at once**, as a single
    /// undoable surgery (`Pass 23.3`).
    ///
    /// The batch form of `node-move`. Repeat `--move NODE,X,Y` once per
    /// anchor; every anchor named goes to its own absolute page-space point,
    /// so this expresses a rigid translation and an arbitrary re-shaping
    /// equally well — nothing here requires the targets be a uniform offset.
    ///
    /// WHY THIS IS NOT JUST A LOOP OVER `node-move`. Two reasons, and only
    /// the first is about convenience:
    ///
    /// - One command, so ONE undo entry. A loop leaves N of them, and undoing
    ///   a batch then means pressing undo N times and knowing what N was.
    /// - Anchors that share an OPERATOR are rewritten together. All four
    ///   corners of a rectangle are the same four operands of one `re`, and an
    ///   `h`-reopened subpath's implicit start shares its byte range with the
    ///   segment that inherits it — cases where two independent edits would
    ///   overlap, and an overlapping edit is silently dropped rather than
    ///   applied.
    ///
    /// REFUSED, all before any byte changes: naming no anchors at all; naming
    /// the same anchor twice (last-wins and first-wins are equally defensible
    /// and give different geometry, so pdfce will not pick one for you); and
    /// any index the object does not have — which refuses the WHOLE batch,
    /// never a partial application.
    ///
    /// Disclosures go to stderr, as `node-move`'s do, so the stdout record
    /// stays machine-parseable. They are DE-DUPLICATED: rewriting three
    /// rectangles says so once, not three times.
    NodesMove {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// One anchor's destination, as `NODE,X,Y` — a 0-based anchor index
        /// (decomposition order, as `object-list` counts them) and an
        /// absolute page-space point. Repeat once per anchor.
        // NOT `num_args = 1..`: that makes the flag GREEDY, so
        // `--move 0,1,2 -o out.pdf` swallows `-o` and `out.pdf` as further
        // move tokens and the command dies reporting `--output` missing.
        // A `Vec` field already appends on repeat, which is the behaviour
        // wanted — one value per occurrence, occur as often as you like.
        #[arg(
            long = "move",
            value_name = "NODE,X,Y",
            required = true,
            allow_hyphen_values = true
        )]
        moves: Vec<String>,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },
    /// **Drag a curve handle** of a path object (Pass 30.1): move one Bézier
    /// control point, leaving the on-curve node itself where it is. This is
    /// the operation that changes a curve's SHAPE — `node-move` can only move
    /// the points a curve passes through.
    ///
    /// `--side incoming` is the control point governing the curve as it
    /// ARRIVES at the node, `--side outgoing` as it LEAVES. A straight
    /// segment has no handle and is refused rather than silently turned into
    /// a curve. `v` and `y` operators leave one control point implied by
    /// another point (§8.5.2.1 Table 59); dragging that one re-spells the
    /// segment as `c` and discloses it on stderr.
    HandleMove {
        /// Input PDF.
        input: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// 0-based paint-order object index on the page.
        #[arg(long)]
        object: usize,
        /// 0-based anchor node index (decomposition order).
        #[arg(long)]
        node: usize,
        /// Which of the node's two handles to move.
        #[arg(long, value_enum)]
        side: HandleArg,
        /// New control-point x, page space (points).
        #[arg(long, allow_hyphen_values = true)]
        x: f64,
        /// New control-point y, page space (points).
        #[arg(long, allow_hyphen_values = true)]
        y: f64,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Save mode.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Reload and verify the edit undoes byte-identically.
        #[arg(long)]
        verify_undo: bool,
    },

    /// Place a raster image on a page as an image XObject (ISO 32000-1 §8.9.5).
    ///
    /// PNG, JPEG and BMP are placed. Anything else is refused BY NAME, with a
    /// message saying which formats do work — never a silent failure and
    /// never a wrong-looking placement.
    ///
    /// NOTHING IS RE-ENCODED THAT DOES NOT HAVE TO BE. A JPEG's codestream is
    /// embedded byte for byte behind `/DCTDecode`, so placing a scan or a
    /// photograph costs no quality; a non-interlaced PNG's compressed image
    /// data is reused verbatim behind `/FlateDecode` with `/Predictor 15`.
    /// pdfce decodes and re-compresses only where PDF cannot express the
    /// source's layout — a PNG with an interleaved alpha channel (which
    /// becomes a base image plus a separate `/SMask`), or a BMP (which has no
    /// compressed form at all). Every such case is reported.
    ///
    /// `--compression jpeg` is the one way to ask for a re-encode anyway. It
    /// is lossy by definition and is never chosen for you.
    AddImage {
        /// Input PDF.
        input: PathBuf,
        /// The image file to place: PNG, JPEG or BMP.
        #[arg(long, value_name = "FILE")]
        image: PathBuf,
        /// 1-based page number to place the image on.
        #[arg(long)]
        page: usize,
        /// The rectangle to place the image in, `llx,lly,urx,ury`, in PDF
        /// user space (points, origin at the page's lower-left).
        ///
        /// By default the image keeps its shape and is CENTRED inside this
        /// rectangle, so one axis may end up smaller than asked — PDF itself
        /// preserves no aspect ratio (§8.9.4), so pdfce has to choose, and a
        /// distorted picture is a defect nobody asked for. Pass `--stretch`
        /// to fill the rectangle exactly instead.
        #[arg(long, value_name = "LLX,LLY,URX,URY", allow_hyphen_values = true)]
        rect: String,
        /// Fill `--rect` exactly, distorting the aspect ratio if it differs.
        ///
        /// The right answer when the rectangle came from a measurement —
        /// fitting a scan to a known paper size, replacing a stamp of fixed
        /// extent — rather than from a freehand drag.
        #[arg(long)]
        stretch: bool,
        /// Replace `--rect`'s SIZE with the image's natural size, keeping its
        /// lower-left corner.
        ///
        /// Natural size comes from the resolution the image file declares (a
        /// PNG `pHYs` chunk, a JFIF density, EXIF `XResolution`, a BMP's
        /// pixels-per-metre). When the file declares none, one pixel becomes
        /// one point (72 dpi) — and the reported `dpi_source=` says which of
        /// the two happened, so an assumed resolution is never mistaken for a
        /// declared one.
        ///
        /// Not the default: applying an embedded resolution silently would
        /// make the same picture land at wildly different sizes depending on
        /// metadata the operator never saw.
        #[arg(long, conflicts_with = "stretch")]
        natural: bool,
        /// How the image's pixels are stored in the PDF.
        ///
        /// `passthrough` (the DEFAULT) embeds the source's own compressed
        /// bytes unchanged — a JPEG's codestream verbatim, a PNG's compressed
        /// image data verbatim. Nothing is re-encoded, so nothing is
        /// degraded. Where a source has no compressed form to keep (a BMP) or
        /// its layout cannot be expressed in PDF (a PNG with an interleaved
        /// alpha channel), the result is lossless compression instead, and the
        /// reported `compression_applied=` says so.
        ///
        /// `lossless` stores the decoded samples with lossless compression.
        /// On a PNG or BMP this changes nothing. ON A JPEG IT RECOVERS
        /// NOTHING — it preserves exactly the pixels the JPEG decodes to,
        /// artefacts included, while typically multiplying the stored size
        /// several-fold. Useful before further editing; not a quality
        /// improvement.
        ///
        /// `jpeg` RE-ENCODES the image lossily at `--quality`. This is the
        /// only policy that degrades the picture, and it does so on purpose.
        /// ON A SOURCE THAT WAS ALREADY A JPEG IT IS A SECOND LOSSY PASS:
        /// the DCT runs again over the artefacts the first one left, which
        /// COMPOUNDS them rather than adding one predictable generation of
        /// loss, and no quality setting undoes that. The reported
        /// `jpeg_from_lossy=1` says when this happened; the honest fix is
        /// usually to place the original file instead. A transparent colour
        /// (a PNG `tRNS` on a truecolour image) is refused by name rather
        /// than re-encoded, because lossy encoding moves the exact sample
        /// values that transparency is matched against.
        ///
        /// Resolution capping ("downsample to N dpi") is still absent, but no
        /// longer for want of an encoder: a resampler is a visible quality
        /// decision (box vs. Lanczos) that deserves its own flag and its own
        /// disclosure, not a silent choice hidden inside this one.
        #[arg(long, value_enum, default_value_t = CompressionArg::Passthrough)]
        compression: CompressionArg,
        /// Encoder quality for `--compression jpeg`, 1-100. Ignored by every
        /// other policy.
        ///
        /// Larger is better and bigger. 100 is NOT lossless — it is the
        /// finest quantisation the scale defines, and on synthetic content
        /// (a screenshot, a CAD export) it routinely produces a file LARGER
        /// than `--compression lossless` would, while still losing detail.
        /// Values outside 1-100 are rejected here at parse time and, for
        /// library callers, refused by name rather than clamped.
        #[arg(long, default_value_t = 85, value_parser = clap::value_parser!(u8).range(1..=100))]
        quality: u8,
        /// Output path.
        #[arg(short, long)]
        output: PathBuf,
        /// Which save path to use.
        #[arg(long, value_enum, default_value_t = SaveMode::Incremental)]
        mode: SaveMode,
        /// Also verify that undoing the placement reproduces the input byte
        /// for byte.
        #[arg(long)]
        verify_undo: bool,
    },
}

/// `--compression` for [`Command::AddImage`].
///
/// A `clap`-side mirror of [`pdfce_core::image_import::ImageCompression`]
/// rather than a re-export, for the reason [`HandleArg`] gives: the core type
/// is not a `ValueEnum`, and making it one would put a CLI-parsing concern
/// into the GUI-free core crate. It also lets the CLI carry `--quality` as a
/// separate flag while the core type carries it inside the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CompressionArg {
    /// Embed the source's own compressed bytes unchanged. The default.
    Passthrough,
    /// Store the decoded samples with lossless compression.
    Lossless,
    /// Re-encode as JPEG at `--quality` — lossy, on purpose, and a SECOND
    /// lossy pass if the source was already a JPEG.
    Jpeg,
}

impl CompressionArg {
    /// The core policy this argument selects.
    fn policy(self, quality: u8) -> pdfce_core::image_import::ImageCompression {
        use pdfce_core::image_import::ImageCompression;
        match self {
            Self::Passthrough => ImageCompression::Passthrough,
            Self::Lossless => ImageCompression::Lossless,
            Self::Jpeg => ImageCompression::Jpeg { quality },
        }
    }
}

/// Which of a node's two Bézier handles [`Command::HandleMove`] moves.
///
/// A `clap`-side mirror of [`pdfce_core::vector::Handle`] rather than a
/// re-export: the core type is not a `ValueEnum`, and making it one would put
/// a CLI-parsing concern into the GUI-free core crate.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum HandleArg {
    /// The handle shaping the curve as it ARRIVES at the node.
    Incoming,
    /// The handle shaping the curve as it LEAVES the node.
    Outgoing,
}

impl HandleArg {
    /// The core enum this stands for.
    const fn to_core(self) -> pdfce_core::vector::Handle {
        match self {
            HandleArg::Incoming => pdfce_core::vector::Handle::Incoming,
            HandleArg::Outgoing => pdfce_core::vector::Handle::Outgoing,
        }
    }

    /// A stable token for CLI output.
    const fn token(self) -> &'static str {
        match self {
            HandleArg::Incoming => "incoming", // ui-text-exempt: stable output token
            HandleArg::Outgoing => "outgoing", // ui-text-exempt: stable output token
        }
    }
}

/// Which dimension kind [`Command::DimensionAdd`] authors. Radius and diameter
/// share one Taubin fit and differ only in DISPLAY (decision 011 §2.3).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum DimKindArg {
    /// A linear (distance) dimension between the first two points.
    Linear,
    /// A radius dimension over a best-fit circle.
    Radius,
    /// A diameter dimension over a best-fit circle (2×radius).
    Diameter,
}

impl DimKindArg {
    /// A stable token for CLI output.
    const fn token(self) -> &'static str {
        match self {
            DimKindArg::Linear => "linear",
            DimKindArg::Radius => "radius",
            DimKindArg::Diameter => "diameter",
        }
    }
}

/// Which reading [`Command::DimensionDisplay`] switches a placed circular ce
/// dimension to (Pass 34.2).
///
/// # Why this is a SECOND enum rather than a reuse of [`DimKindArg`]
///
/// [`DimKindArg`] answers "what kind of ce dimension am I authoring", and its
/// `Linear` variant is a legitimate answer to that question. Here `Linear` is
/// precisely the case being refused — the verb only applies to a circular ce
/// dimension. Reusing the wider enum would make `--show linear` parse cleanly
/// and then fail at runtime, which is a worse experience than clap refusing it
/// at the argument boundary with the two valid values listed.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum DisplayReading {
    /// Print the fitted radius.
    Radius,
    /// Print the diameter (2×radius) of the same fitted circle.
    Diameter,
}

impl DisplayReading {
    /// `true` when the label should print the diameter — the shape
    /// [`pdfce_core::edit::EditSession::set_dimension_display`] takes.
    const fn show_diameter(self) -> bool {
        matches!(self, DisplayReading::Diameter)
    }

    /// A stable token for CLI output.
    const fn token(self) -> &'static str {
        match self {
            DisplayReading::Radius => "radius",
            DisplayReading::Diameter => "diameter",
        }
    }
}

/// The linear alignment constraint for [`Command::DimensionAdd`].
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ConstraintArg {
    /// Free Euclidean direction.
    Aligned,
    /// Project onto the page X axis (measured length `|Δx|`).
    Horizontal,
    /// Project onto the page Y axis (measured length `|Δy|`).
    Vertical,
}

impl ConstraintArg {
    /// The `pdfce_core` constraint this maps to.
    const fn to_core(self) -> pdfce_core::vector::AxisConstraint {
        match self {
            ConstraintArg::Aligned => pdfce_core::vector::AxisConstraint::Aligned,
            ConstraintArg::Horizontal => pdfce_core::vector::AxisConstraint::Horizontal,
            ConstraintArg::Vertical => pdfce_core::vector::AxisConstraint::Vertical,
        }
    }
}

/// Which save path `round-trip` exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum RoundTripMode {
    /// §7.5.6 incremental save with an **empty** dirty set. Promises
    /// whole-file byte identity: zero edits means zero bytes.
    Incremental,
    /// Full rewrite. Promises per-object-definition byte identity, a
    /// reloadable file, and an identical raster — never whole-file
    /// identity, because object offsets legitimately move.
    Full,
    /// §7.5.6 incremental save that re-emits **every** object of the
    /// base revision **unchanged**, exercising the real append writer.
    ///
    /// This is a verification mode, not an editing feature: no object's
    /// value changes, so the result is semantically identical to the
    /// input by construction. It exists because the `incremental` mode's
    /// empty-dirty-set path is a `memcpy` — without this, the §7.5.6
    /// append machinery (object re-emission, update-section
    /// construction, `/Prev` chaining, trailer copying) would ship with
    /// no corpus coverage at all.
    AppendIdentity,
}

/// `/Producer` policy, as a CLI value.
/// `--units` for `export-dxf`, mapped to the DXF header's `$INSUNITS`.
///
/// Short names because they are typed: `--units mm` reads better than
/// `--units millimetres` and is what a drawing office would say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum DxfUnitArg {
    /// Inches ($INSUNITS 1).
    In,
    /// Millimetres ($INSUNITS 4).
    Mm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ProducerArg {
    /// Write `/Producer (pdfce <version>)` into an existing `/Info`.
    Set,
    /// Leave `/Info` byte-untouched (R41's no-fingerprint posture).
    Preserve,
}

/// How the CLI reads a comma in a stored field value.
///
/// Mirrors [`pdfce_core::form_script::calc::CommaPolicy`]. Kept as its own
/// clap enum rather than deriving `ValueEnum` on the core type, so the core
/// crate gains no CLI dependency — the GUI-core separation applies to
/// argument parsing too.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
enum CommaArg {
    /// A comma makes the value non-numeric, so it counts as a disclosed
    /// zero. The default: refusing to guess cannot turn `1,234` into `1.234`.
    NotNumeric,
    /// A comma is the decimal separator (`1,5` is 1.5).
    Decimal,
    /// A comma is the thousands separator (`1,234` is 1234).
    Grouping,
}

impl From<CommaArg> for pdfce_core::form_script::calc::CommaPolicy {
    fn from(arg: CommaArg) -> Self {
        match arg {
            CommaArg::NotNumeric => Self::NotNumeric,
            CommaArg::Decimal => Self::DecimalSeparator,
            CommaArg::Grouping => Self::GroupingSeparator,
        }
    }
}

/// `--border` on `add-text-field` (§12.5.4 Table 166).
///
/// Its own clap enum rather than a `ValueEnum` derive on the core type, so
/// `pdfce-core` gains no CLI dependency — the GUI-core separation applies to
/// argument parsing too.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
enum BorderArg {
    /// Solid rectangle. Table 166's default and pdfce's.
    Solid,
    /// Dashed.
    Dashed,
    /// Beveled — solid with an embossed highlight.
    Beveled,
    /// Inset — solid with an engraved lowlight.
    Inset,
    /// Underline — a line along the bottom edge only.
    Underline,
}

impl From<BorderArg> for pdfce_core::edit::BorderStyle {
    fn from(arg: BorderArg) -> Self {
        match arg {
            BorderArg::Solid => Self::Solid,
            BorderArg::Dashed => Self::Dashed,
            BorderArg::Beveled => Self::Beveled,
            BorderArg::Inset => Self::Inset,
            BorderArg::Underline => Self::Underline,
        }
    }
}

/// `--visibility` on `add-text-field` (§12.5.3 Table 165).
///
/// Four combinations, not eight bits. `hidden` and `print-only` are kept
/// distinct because Table 165 makes them different: `Hidden` suppresses
/// screen AND print "regardless of its annotation type", while `NoView`
/// suppresses only the screen and leaves printing to the `Print` flag.
/// Collapsing them would silently stop a field printing that the operator
/// asked to print.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
enum VisibilityArg {
    /// On screen and printed (`/F 4`). The default.
    Visible,
    /// On screen, never printed (`/F 0`).
    ScreenOnly,
    /// Printed, not shown on screen (`/F 36`).
    PrintOnly,
    /// Suppressed everywhere (`/F 2`).
    Hidden,
}

impl From<VisibilityArg> for pdfce_core::edit::Visibility {
    fn from(arg: VisibilityArg) -> Self {
        match arg {
            VisibilityArg::Visible => Self::VisibleAndPrints,
            VisibilityArg::ScreenOnly => Self::ScreenOnly,
            VisibilityArg::PrintOnly => Self::PrintOnly,
            VisibilityArg::Hidden => Self::Hidden,
        }
    }
}

/// Which save path an **editing** subcommand uses.
///
/// Deliberately a separate enum from [`RoundTripMode`], which carries a
/// verification-only `append-identity` variant that has no meaning for
/// an edit — merging them would put a mode in `--help` that cannot do
/// what its name suggests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum SaveMode {
    /// Append a §7.5.6 revision, leaving every prior byte intact. The
    /// default, and the only mode that preserves existing digital
    /// signatures (§12.8.1 NOTE 1).
    Incremental,
    /// Rewrite the file as a single revision. Smaller output, and it
    /// drops superseded revisions — but it **destroys every existing
    /// signature**, and it is refused outright for a hybrid-reference
    /// file (§7.5.8.4).
    Full,
}

impl SaveMode {
    /// The `mode=` token on the stdout line. Part of the stable output
    /// contract, so it is pinned by a test.
    const fn name(self) -> &'static str {
        match self {
            Self::Incremental => "incremental",
            Self::Full => "full",
        }
    }
}

/// The form-data interchange format for `export-data` / `import-data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum DataFormat {
    /// Forms Data Format (ISO 32000-1 §12.7.7) — a PDF-like data file.
    Fdf,
    /// XML Forms Data Format — the XML companion format.
    Xfdf,
    /// Two-column `name,value` CSV — the format a spreadsheet opens.
    ///
    /// Not a PDF-world format: FDF and XFDF interchange between PDF
    /// programs, and this one leaves that world. Values a spreadsheet would
    /// read as formulae are prefixed with an apostrophe and the change is
    /// reported.
    Csv,
}

/// A document-information field, as a CLI value for `--clear`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum InfoFieldArg {
    /// `/Title`.
    Title,
    /// `/Author`.
    Author,
    /// `/Subject`.
    Subject,
    /// `/Keywords`.
    Keywords,
}

impl From<InfoFieldArg> for pdfce_core::edit::InfoField {
    fn from(arg: InfoFieldArg) -> Self {
        match arg {
            InfoFieldArg::Title => Self::Title,
            InfoFieldArg::Author => Self::Author,
            InfoFieldArg::Subject => Self::Subject,
            InfoFieldArg::Keywords => Self::Keywords,
        }
    }
}

/// The geometric-markup subtype selected by `pdfce-cli annotate --type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum AnnotKindArg {
    /// `/Square` — an axis-aligned rectangle (`--rect`).
    Square,
    /// `/Circle` — an ellipse inscribed in a rectangle (`--rect`).
    Circle,
    /// `/Line` — a single segment, arrow-headed by default (`--line`).
    Line,
    /// `/Ink` — freehand strokes (`--strokes`).
    Ink,
    /// `/Polygon` — a closed multi-segment shape (`--points`).
    Polygon,
    /// `/PolyLine` — an open multi-segment path (`--points`).
    Polyline,
    /// `/Highlight` — a translucent wash over quads (`--quads`/`--rect`).
    Highlight,
    /// `/Underline` — a baseline line over quads.
    Underline,
    /// `/StrikeOut` — a strike-through line over quads.
    Strikeout,
    /// `/Squiggly` — a wavy line over quads.
    Squiggly,
    /// `/FreeText` — text drawn on the page (`--text`, `--rect`).
    Freetext,
    /// `/Text` — a sticky note whose body opens in a popup (`--text`).
    Text,
    /// `/Stamp` — a rubber stamp with a framed label (`--stamp-name`).
    Stamp,
}

/// Justification for a FreeText annotation (`/Q`, §12.7.3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum QuadArg {
    /// `/Q 0` — left-justified.
    Left,
    /// `/Q 1` — centred.
    Center,
    /// `/Q 2` — right-justified.
    Right,
}

impl QuadArg {
    fn to_quadding(self) -> pdfce_core::vartext::Quadding {
        use pdfce_core::vartext::Quadding;
        match self {
            Self::Left => Quadding::Left,
            Self::Center => Quadding::Center,
            Self::Right => Quadding::Right,
        }
    }
}

/// Sticky-note icon name (§12.5.6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum IconArg {
    /// `/Comment`.
    Comment,
    /// `/Key`.
    Key,
    /// `/Note` (default).
    Note,
    /// `/Help`.
    Help,
    /// `/NewParagraph`.
    NewParagraph,
    /// `/Paragraph`.
    Paragraph,
    /// `/Insert`.
    Insert,
}

impl IconArg {
    fn to_icon(self) -> pdfce_core::annot_author::StickyIcon {
        use pdfce_core::annot_author::StickyIcon;
        match self {
            Self::Comment => StickyIcon::Comment,
            Self::Key => StickyIcon::Key,
            Self::Note => StickyIcon::Note,
            Self::Help => StickyIcon::Help,
            Self::NewParagraph => StickyIcon::NewParagraph,
            Self::Paragraph => StickyIcon::Paragraph,
            Self::Insert => StickyIcon::Insert,
        }
    }
}

/// Standard rubber-stamp name (§12.5.6.12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum StampArg {
    /// `/Approved`.
    Approved,
    /// `/Experimental`.
    Experimental,
    /// `/NotApproved`.
    NotApproved,
    /// `/AsIs`.
    AsIs,
    /// `/Expired`.
    Expired,
    /// `/NotForPublicRelease`.
    NotForPublicRelease,
    /// `/Confidential`.
    Confidential,
    /// `/Final`.
    Final,
    /// `/Sold`.
    Sold,
    /// `/Departmental`.
    Departmental,
    /// `/ForComment`.
    ForComment,
    /// `/TopSecret`.
    TopSecret,
    /// `/Draft` (default).
    Draft,
    /// `/ForPublicRelease`.
    ForPublicRelease,
}

impl StampArg {
    fn to_stamp_name(self) -> pdfce_core::annot_author::StampName {
        use pdfce_core::annot_author::StampName as S;
        match self {
            Self::Approved => S::Approved,
            Self::Experimental => S::Experimental,
            Self::NotApproved => S::NotApproved,
            Self::AsIs => S::AsIs,
            Self::Expired => S::Expired,
            Self::NotForPublicRelease => S::NotForPublicRelease,
            Self::Confidential => S::Confidential,
            Self::Final => S::Final,
            Self::Sold => S::Sold,
            Self::Departmental => S::Departmental,
            Self::ForComment => S::ForComment,
            Self::TopSecret => S::TopSecret,
            Self::Draft => S::Draft,
            Self::ForPublicRelease => S::ForPublicRelease,
        }
    }
}

/// Run the CLI on a worker thread with a generous stack.
///
/// clap's **debug-build** argument-tree validation (`debug_assert`, compiled
/// only under `debug_assertions`) recurses deeply enough that, for a command
/// tree this size, it overflows the small default **main-thread** stack on
/// Windows/MSVC (~1 MB) — a release build is unaffected (no `debug_assert`,
/// and optimized frames). Running the whole program on a spawned thread with
/// a 16 MB stack sidesteps it portably; on failure to spawn, we fall back to
/// the main thread rather than aborting.
fn main() -> ExitCode {
    match std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(run)
    {
        Ok(handle) => handle.join().unwrap_or(ExitCode::FAILURE),
        Err(_) => run(),
    }
}

fn run() -> ExitCode {
    let cli = Cli::parse();

    // Resolve the password BEFORE any subcommand runs, so a bad
    // --password-file fails immediately and by name rather than surfacing
    // later as "this document is password-protected".
    match resolve_cli_password(cli.open_password, cli.open_password_file) {
        Ok(pw) => {
            let _ = CLI_PASSWORD.set(pw);
        }
        Err(msg) => {
            eprintln!("pdfce-cli: {msg}");
            return ExitCode::from(3);
        }
    }

    let code = match cli.command {
        Command::Inspect {
            file,
            text_blocks,
            pages,
            json,
            reflow_preview,
            block,
            width,
            align,
            leading,
        } => {
            if reflow_preview {
                cmd_inspect_reflow_preview(
                    &file,
                    &pages,
                    block,
                    width,
                    align.as_deref(),
                    leading,
                    json,
                )
            } else if text_blocks {
                cmd_inspect_text_blocks(&file, &pages, json)
            } else {
                cmd_inspect(&file)
            }
        }
        // The remaining stubs are named individually (rather than via a
        // catch-all) so `--help` still documents each one's real
        // argument shape.
        Command::Merge {
            inputs,
            output,
            no_bookmarks,
        } => cmd_merge(&inputs, &output, !no_bookmarks),
        Command::Split {
            input,
            out_dir,
            every,
            after,
            bookmarks,
            name_template,
            force,
        } => cmd_split(
            &input,
            &out_dir,
            every,
            after.as_deref(),
            bookmarks,
            &name_template,
            force,
        ),
        Command::ExtractPages {
            input,
            pages,
            output,
        } => cmd_extract_pages(&input, &pages, &output),
        Command::InsertPages {
            input,
            source,
            source_pages,
            before,
            after,
            output,
        } => cmd_insert_pages(&input, &source, &source_pages, before, after, &output),
        Command::DeletePages {
            input,
            pages,
            output,
            mode,
            verify_undo,
        } => cmd_delete_pages(&input, &pages, &output, mode, verify_undo),
        Command::ReorderPages {
            input,
            order,
            output,
            mode,
            verify_undo,
        } => cmd_reorder_pages(&input, &order, &output, mode, verify_undo),
        Command::Rotate {
            input,
            degrees,
            pages,
            output,
            mode,
            verify_undo,
        } => cmd_rotate(&input, degrees, &pages, &output, mode, verify_undo),
        Command::BatesStamp { .. } => unimplemented_stub("bates-stamp"),
        Command::ToPdfa { .. } => unimplemented_stub("to-pdfa"),
        Command::ValidatePdfa { .. } => unimplemented_stub("validate-pdfa"),
        Command::Sign { .. } => unimplemented_stub("sign"),
        Command::ListOutline { input, flat } => cmd_list_outline(&input, flat),
        Command::ListAttachments { input } => cmd_list_attachments(&input),
        Command::ListLayers { input } => cmd_list_layers(&input),
        Command::ListFonts {
            input,
            reasons,
            by_size,
        } => cmd_list_fonts(&input, reasons, by_size),
        Command::ListSignatures { input } => cmd_list_signatures(&input),
        Command::ListPrinters => cmd_list_printers(),
        Command::Print {
            input,
            printer,
            scale,
            scale_percent,
            pages,
            send,
            max_dpi,
            to_file,
            copies,
            uncollated,
            subset,
            reverse,
            orientation,
            duplex,
            pick_tray,
            comments,
            n_up,
            n_up_border,
            booklet,
            poster,
            poster_scale,
            poster_overlap,
            poster_large_only,
            poster_max_tiles,
            binding,
            booklet_subset,
        } => cmd_print(
            &input,
            printer.as_deref(),
            scale,
            scale_percent,
            &pages,
            send,
            max_dpi,
            to_file,
            copies,
            uncollated,
            subset,
            reverse,
            orientation,
            duplex,
            pick_tray,
            comments,
            n_up,
            n_up_border,
            booklet,
            poster,
            poster_scale,
            poster_overlap,
            poster_large_only,
            poster_max_tiles,
            binding,
            booklet_subset,
        ),
        Command::PrintPreview {
            input,
            printer,
            scale,
            pages,
            scale_percent,
            orientation,
        } => cmd_print_preview(
            &input,
            printer.as_deref(),
            scale,
            scale_percent,
            &pages,
            orientation,
        ),
        Command::FindText {
            input,
            needle,
            ignore_case,
        } => cmd_find_text(&input, &needle, ignore_case),
        Command::ExtractText {
            input,
            pages,
            output,
            json,
            include_artifacts,
        } => cmd_extract_text(&input, &pages, output.as_deref(), json, include_artifacts),
        Command::RenderPage {
            input,
            page,
            scale,
            output,
            no_annotations,
            font_dirs,
            show_layers,
            hide_layers,
            print_state,
        } => cmd_render_page(
            &input,
            page,
            scale,
            &output,
            !no_annotations,
            &font_dirs,
            &show_layers,
            &hide_layers,
            print_state,
        ),
        Command::ListAnnotations { input, pages } => cmd_list_annotations(&input, &pages),
        Command::ListFields {
            input,
            fillable_only,
            rich_text,
        } => cmd_list_fields(&input, fillable_only, rich_text),
        Command::AddTextField {
            input,
            name,
            page,
            rect,
            value,
            max_len,
            tooltip,
            no_tooltip,
            multiline,
            read_only,
            required,
            password,
            comb,
            border,
            border_width,
            visibility,
            output,
            mode,
            defaults_from,
            verify_undo,
        } => cmd_add_text_field(&AddTextFieldArgs {
            input: &input,
            name: &name,
            page,
            rect: &rect,
            value: value.as_deref(),
            max_len,
            tooltip: tooltip.as_deref(),
            no_tooltip,
            multiline,
            read_only,
            required,
            password,
            comb,
            border,
            border_width,
            visibility,
            output: &output,
            mode,
            defaults_from: defaults_from.as_deref(),
            verify_undo,
        }),
        Command::AddImage {
            input,
            image,
            page,
            rect,
            stretch,
            natural,
            compression,
            quality,
            output,
            mode,
            verify_undo,
        } => cmd_add_image(&AddImageArgs {
            input: &input,
            image: &image,
            page,
            rect: &rect,
            stretch,
            natural,
            compression: compression.policy(quality),
            output: &output,
            mode,
            verify_undo,
        }),
        Command::AddCheckBox {
            input,
            name,
            page,
            rect,
            on_state,
            checked,
            tooltip,
            no_tooltip,
            read_only,
            required,
            output,
            mode,
            defaults_from,
            verify_undo,
            border,
            border_width,
            visibility,
        } => cmd_add_check_box(&AddCheckBoxArgs {
            input: &input,
            name: &name,
            page,
            rect: &rect,
            on_state: &on_state,
            checked,
            tooltip: tooltip.as_deref(),
            no_tooltip,
            read_only,
            required,
            output: &output,
            mode,
            defaults_from: defaults_from.as_deref(),
            verify_undo,
            border,
            border_width,
            visibility,
        }),
        Command::AddRadioButton {
            input,
            name,
            page,
            rect,
            export_value,
            selected,
            tooltip,
            no_tooltip,
            no_toggle_to_off,
            radios_in_unison,
            read_only,
            required,
            output,
            mode,
            defaults_from,
            verify_undo,
            border,
            border_width,
            visibility,
        } => cmd_add_radio_button(&AddRadioButtonArgs {
            input: &input,
            name: &name,
            page,
            rect: &rect,
            export_value: &export_value,
            selected,
            tooltip: tooltip.as_deref(),
            no_tooltip,
            no_toggle_to_off,
            radios_in_unison,
            read_only,
            required,
            output: &output,
            mode,
            defaults_from: defaults_from.as_deref(),
            verify_undo,
            border,
            border_width,
            visibility,
        }),
        Command::DeleteField {
            input,
            name,
            output,
            mode,
            verify_undo,
        } => cmd_delete_form_field(&input, &name, None, &output, mode, verify_undo),
        Command::DeleteFieldGroup {
            input,
            name,
            output,
            yes,
            mode,
            verify_undo,
        } => cmd_delete_field_group(&input, &name, &output, yes, mode, verify_undo),
        Command::DeleteWidget {
            input,
            name,
            index,
            output,
            mode,
            verify_undo,
        } => cmd_delete_form_field(&input, &name, Some(index), &output, mode, verify_undo),
        Command::DeleteAnnotation {
            input,
            page,
            index,
            output,
            mode,
            verify_undo,
        } => cmd_delete_annotation(&input, page, index, &output, mode, verify_undo),
        Command::MoveWidget {
            input,
            name,
            index,
            dx,
            dy,
            output,
            mode,
        } => cmd_move_widget(&input, &name, index, dx, dy, &output, mode),
        Command::RenameField {
            input,
            name,
            to,
            output,
            mode,
            verify_undo,
        } => cmd_rename_field(&input, &name, &to, &output, mode, verify_undo),
        Command::AddPushButton {
            input,
            name,
            page,
            rect,
            caption,
            tooltip,
            no_tooltip,
            read_only,
            output,
            mode,
            defaults_from,
            verify_undo,
            border,
            border_width,
            visibility,
        } => cmd_add_push_button(&AddPushButtonArgs {
            input: &input,
            name: &name,
            page,
            rect: &rect,
            caption: &caption,
            tooltip: tooltip.as_deref(),
            no_tooltip,
            read_only,
            output: &output,
            mode,
            defaults_from: defaults_from.as_deref(),
            verify_undo,
            border,
            border_width,
            visibility,
        }),
        Command::AddChoiceField {
            input,
            name,
            page,
            rect,
            options,
            combo,
            editable,
            multi_select,
            sort,
            tooltip,
            no_tooltip,
            read_only,
            required,
            output,
            mode,
            defaults_from,
            verify_undo,
            border,
            border_width,
            visibility,
        } => cmd_add_choice_field(&AddChoiceFieldArgs {
            input: &input,
            name: &name,
            page,
            rect: &rect,
            options: &options,
            combo,
            editable,
            multi_select,
            sort,
            tooltip: tooltip.as_deref(),
            no_tooltip,
            read_only,
            required,
            output: &output,
            mode,
            defaults_from: defaults_from.as_deref(),
            verify_undo,
            border,
            border_width,
            visibility,
        }),
        Command::Recompute {
            input,
            apply,
            output,
            mode,
            comma,
            verify_undo,
        } => cmd_recompute(
            &input,
            apply,
            output.as_deref(),
            mode,
            comma.into(),
            verify_undo,
        ),
        Command::ResetForm {
            input,
            fields,
            apply,
            output,
            mode,
            verify_undo,
        } => cmd_reset_form(&input, &fields, apply, output.as_deref(), mode, verify_undo),
        Command::ListScripts {
            input,
            reproducible_only,
        } => cmd_list_scripts(&input, reproducible_only),
        Command::FillField {
            input,
            sets,
            output,
            mode,
            verify_undo,
            downgrade_rich_text,
        } => cmd_fill_field(
            &input,
            &sets,
            &output,
            mode,
            verify_undo,
            downgrade_rich_text,
        ),
        Command::RegenerateAppearances {
            input,
            output,
            mode,
        } => cmd_regenerate_appearances(&input, &output, mode),
        Command::Flatten {
            input,
            fields,
            output,
            full_rewrite,
        } => cmd_flatten(&input, &fields, &output, full_rewrite),
        Command::ExportData {
            input,
            output,
            format,
        } => cmd_export_data(&input, &output, format),
        Command::ImportData {
            input,
            data,
            output,
            mode,
        } => cmd_import_data(&input, &data, &output, mode),
        Command::RoundTrip {
            input,
            mode,
            output,
            producer,
            scale,
            no_raster,
        } => cmd_round_trip(&input, mode, output.as_deref(), producer, scale, !no_raster),
        Command::EditText {
            input,
            page,
            find,
            replace,
            output,
            pin,
            font_dirs,
        } => cmd_edit_text(&EditTextArgs {
            input: &input,
            output: &output,
            page,
            find: &find,
            replace: &replace,
            pin,
            font_dirs: &font_dirs,
        }),
        Command::FormatText {
            input,
            page,
            find,
            set_size,
            set_color,
            set_font,
            char_spacing,
            word_spacing,
            h_scale,
            superscript,
            subscript,
            no_script,
            rise,
            bold_synthetic,
            italic_synthetic,
            output,
            pin,
            font_dirs,
        } => cmd_format_text(&FormatTextArgs {
            input: &input,
            output: &output,
            page,
            find: &find,
            set_size,
            set_color: set_color.as_deref(),
            set_font: set_font.as_deref(),
            char_spacing: char_spacing.as_deref(),
            word_spacing: word_spacing.as_deref(),
            h_scale,
            rise: rise.as_deref(),
            synthetic: pdfce_core::text_edit::StyleSynthesis::new(bold_synthetic, italic_synthetic),
            // clap's `conflicts_with` guarantees at most one is set, so this
            // ladder cannot silently prefer one over another.
            script: if superscript {
                Some(pdfce_core::text_edit::ScriptPosition::Superscript)
            } else if subscript {
                Some(pdfce_core::text_edit::ScriptPosition::Subscript)
            } else if no_script {
                Some(pdfce_core::text_edit::ScriptPosition::Normal)
            } else {
                None
            },
            pin,
            font_dirs: &font_dirs,
        }),
        Command::Reflow {
            input,
            page,
            block,
            width,
            align,
            leading,
            output,
        } => cmd_reflow(
            &input,
            page,
            block,
            width,
            align.as_deref(),
            leading,
            &output,
        ),
        Command::AddText {
            input,
            page,
            at,
            wrap_box,
            align,
            leading,
            text,
            font,
            size,
            color,
            font_dirs,
            embed_font,
            output,
        } => cmd_add_text(&AddTextArgs {
            input: &input,
            output: &output,
            page,
            at: at.as_deref(),
            wrap_box: wrap_box.as_deref(),
            align: align.as_deref(),
            leading,
            text: &text,
            font: &font,
            size,
            color: color.as_deref(),
            font_dirs: &font_dirs,
            embed_font: embed_font.as_deref(),
        }),
        Command::DimensionAdd {
            input,
            page,
            kind,
            points,
            group,
            constraint,
            offset,
            text_along,
            output,
            mode,
            verify_undo,
        } => cmd_dimension_add(&DimensionAddArgs {
            input: &input,
            page,
            kind,
            points: &points,
            group,
            constraint,
            offset,
            text_along,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::DimensionList { input } => cmd_dimension_list(&input),
        Command::DimensionDelete {
            input,
            dimension,
            output,
            mode,
            verify_undo,
        } => cmd_dimension_delete(&input, dimension, &output, mode, verify_undo),
        Command::GroupSetStandard {
            input,
            group,
            standard,
            output,
            mode,
            verify_undo,
        } => cmd_group_set_standard(&input, group, standard, &output, mode, verify_undo),
        Command::DimensionOffset {
            input,
            dimension,
            offset,
            text_along,
            output,
            mode,
            verify_undo,
        } => cmd_dimension_offset(&DimensionOffsetArgs {
            input: &input,
            dimension,
            offset,
            text_along,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::DimensionDisplay {
            input,
            dimension,
            show,
            output,
            mode,
            verify_undo,
        } => cmd_dimension_display(&input, dimension, show, &output, mode, verify_undo),
        Command::GroupAdd {
            input,
            name,
            unit,
            output,
            mode,
            verify_undo,
        } => cmd_group_add(&input, &name, &unit, &output, mode, verify_undo),
        Command::GroupSetScale {
            input,
            group,
            real_length,
            drawn,
            ratio,
            unit,
            one_to_one,
            precision,
            output,
            mode,
            verify_undo,
        } => cmd_group_set_scale(&GroupSetScaleArgs {
            input: &input,
            group,
            real_length: real_length.as_deref(),
            drawn,
            ratio: ratio.as_deref(),
            unit: &unit,
            one_to_one,
            precision,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::LayerToggle {
            input,
            group,
            hide,
            output,
            mode,
            verify_undo,
        } => cmd_layer_toggle(&input, group, hide, &output, mode, verify_undo),
        Command::ObjectList {
            input,
            page,
            hit,
            all_hits,
            enter,
            tolerance,
        } => cmd_object_list(ObjectListArgs {
            input: &input,
            page_number: page,
            hit: hit.as_deref(),
            all_hits,
            enter,
            tolerance,
        }),
        Command::ObjectMove {
            input,
            page,
            object,
            dx,
            dy,
            output,
            mode,
            verify_undo,
        } => cmd_object_move(&ObjectMoveArgs {
            input: &input,
            page,
            object,
            dx,
            dy,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::ObjectDelete {
            input,
            page,
            object,
            output,
            mode,
            verify_undo,
        } => cmd_object_delete(&input, page, object, &output, mode, verify_undo),
        Command::SubpathMove {
            input,
            page,
            object,
            subpath,
            dx,
            dy,
            output,
            mode,
            verify_undo,
        } => cmd_subpath_move(&SubpathMoveArgs {
            input: &input,
            page,
            object,
            subpath,
            dx,
            dy,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::ExportDxf {
            input,
            page,
            pages,
            output,
            output_dir,
            units,
            scale,
            no_fit_arcs,
            no_text,
        } => cmd_export_dxf(ExportDxfArgs {
            input: &input,
            page,
            pages: pages.as_deref(),
            output: output.as_deref(),
            output_dir: output_dir.as_deref(),
            units,
            scale,
            fit_arcs: !no_fit_arcs,
            text: !no_text,
        }),
        Command::TextRunDelete {
            input,
            page,
            object,
            run,
            output,
            mode,
            verify_undo,
        } => cmd_text_run_delete(&input, page, object, run, &output, mode, verify_undo),
        Command::SubpathDelete {
            input,
            page,
            object,
            subpath,
            output,
            mode,
            verify_undo,
        } => cmd_subpath_delete(&SubpathDeleteArgs {
            input: &input,
            page,
            object,
            subpath,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::NodeDelete {
            input,
            page,
            object,
            node,
            output,
            mode,
            verify_undo,
        } => cmd_node_delete(&NodeDeleteArgs {
            input: &input,
            page,
            object,
            node,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::NodeMove {
            input,
            page,
            object,
            node,
            x,
            y,
            output,
            mode,
            verify_undo,
        } => cmd_node_move(&NodeMoveArgs {
            input: &input,
            page,
            object,
            node,
            x,
            y,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::NodesMove {
            input,
            page,
            object,
            moves,
            output,
            mode,
            verify_undo,
        } => cmd_nodes_move(&input, page, object, &moves, &output, mode, verify_undo),
        Command::HandleMove {
            input,
            page,
            object,
            node,
            side,
            x,
            y,
            output,
            mode,
            verify_undo,
        } => cmd_handle_move(&HandleMoveArgs {
            input: &input,
            page,
            object,
            node,
            side,
            x,
            y,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::RotatePage {
            input,
            page,
            degrees,
            relative,
            output,
            mode,
            verify_undo,
        } => cmd_rotate_page(&input, page, degrees, relative, &output, mode, verify_undo),
        Command::SetInfo {
            input,
            title,
            author,
            subject,
            keywords,
            clear,
            output,
            mode,
            producer,
            verify_undo,
        } => cmd_set_info(
            &input,
            &[
                (InfoFieldArg::Title, title),
                (InfoFieldArg::Author, author),
                (InfoFieldArg::Subject, subject),
                (InfoFieldArg::Keywords, keywords),
            ],
            &clear,
            &output,
            mode,
            producer,
            verify_undo,
        ),
        Command::Annotate {
            input,
            kind,
            page,
            rect,
            line,
            points,
            strokes,
            quads,
            color,
            fill,
            width,
            text,
            font,
            size,
            quad,
            multiline,
            icon,
            stamp_name,
            output,
            mode,
            verify_undo,
        } => cmd_annotate(&AnnotateArgs {
            input: &input,
            kind,
            page,
            rect: rect.as_deref(),
            line: line.as_deref(),
            points: points.as_deref(),
            strokes: strokes.as_deref(),
            quads: quads.as_deref(),
            color: color.as_deref(),
            fill: fill.as_deref(),
            width,
            text: text.as_deref(),
            font: &font,
            size,
            quad,
            multiline,
            icon,
            stamp_name,
            output: &output,
            mode,
            verify_undo,
        }),
        Command::RedactMark {
            input,
            rect,
            search,
            pattern,
            ignore_case,
            page,
            fill,
            overlay_text,
            output,
        } => cmd_redact_mark(&RedactMarkArgs {
            input: &input,
            rect: rect.as_deref(),
            search: search.as_deref(),
            pattern: pattern.as_deref(),
            ignore_case,
            page,
            fill: fill.as_deref(),
            overlay_text: overlay_text.as_deref(),
            output: &output,
        }),
        Command::UnembedFont {
            input,
            font,
            all_removable,
            apply,
            output,
            mode,
            verify_undo,
            keep_subset_tag,
            acknowledge_pdfa,
        } => cmd_unembed_font(&UnembedArgs {
            input: &input,
            fonts: &font,
            all_removable,
            apply,
            output: output.as_deref(),
            mode,
            verify_undo,
            keep_subset_tag,
            acknowledge_pdfa,
        }),
        Command::EmbedFont {
            input,
            font,
            all_missing,
            font_dirs,
            use_bundled_fonts,
            apply,
            output,
            mode,
            verify_undo,
        } => cmd_embed_font(&EmbedArgs {
            input: &input,
            fonts: &font,
            all_missing,
            font_dirs: &font_dirs,
            use_bundled_fonts,
            apply,
            output: output.as_deref(),
            mode,
            verify_undo,
        }),
        Command::RedactApply {
            input,
            output,
            acknowledge_residuals,
        } => cmd_redact_apply(&input, &output, acknowledge_residuals),
        Command::ListRedactions { input } => cmd_list_redactions(&input),
    };
    ExitCode::from(code)
}

/// Implement `pdfce-cli inspect <file>`: probe the header and print the
/// declared version, or print a diagnostic and map the error to the
/// documented exit code.
/// Warn when the visible document is a cover page for an encrypted payload.
///
/// # Why this is a warning and not a refusal
///
/// The wrapper really is a readable PDF, and everything pdfce says about it
/// is true *of the wrapper*. Refusing to open it would withhold a document
/// the operator can legitimately read — the cover page is often the only
/// instructions they have. What must not happen is the operator taking
/// "1 page, no fields" as a fact about the protected content.
///
/// So: open it, report it, and say plainly that the counts describe the
/// cover. On stderr, so a script capturing stdout still shows a human.
fn disclose_wrapper(file: &Path, doc: &pdfce_core::document::Document) {
    let info = pdfce_core::wrapper::detect(doc);
    if let Some(message) = info.message() {
        eprintln!("pdfce-cli: {}: {message}", file.display());
    }
}

fn cmd_inspect(file: &Path) -> u8 {
    let probe = pdfce_core::probe_file(file);
    // A full load additionally surfaces cross-reference RECOVERY (decision
    // 013): a file whose header probes fine can still have an unparseable
    // cross-reference table that pdfce rebuilds by scanning, and an
    // offset-start file fails the header probe but still recovers. `inspect`
    // stays lenient — a full-load *failure* does not fail the probe (a
    // header-valid-but-otherwise-broken file still reports its version) —
    // but a RECOVERED open is disclosed and gets a distinct exit status
    // (R20, fuzzy-never-sneaky).
    // The load ERROR is kept, not discarded. It used to be `.ok()`-ed away,
    // which is why an encrypted document reported the same clean line as a
    // readable one — the reason it could not be read was thrown away one
    // expression before anyone could report it.
    let full_result = std::fs::read(file)
        .map_err(pdfce_core::document::DocError::Io)
        .and_then(open_document_bytes);
    let full = full_result.as_ref().ok();

    match (&probe, &full) {
        // Opened via recovery — disclose + distinct status, whether or not
        // the header probe itself succeeded.
        (_, Some(doc)) if doc.recovery().is_some() => {
            let version = match &probe {
                Ok(v) => v.to_string(),
                Err(_) => doc.version().to_string(),
            };
            println!("{}: PDF {version} (recovered)", file.display());
            if let Some(report) = doc.recovery() {
                disclose_recovery(file, report);
            }
            exit::OPENED_VIA_RECOVERY
        }
        // Clean header (whether or not the full body loaded) — the stable
        // probe line, unchanged.
        (Ok(version), _) => {
            println!("{}: PDF {version}", file.display());
            // ★ The header probe succeeding is NOT the same as the document
            // being readable, and `inspect` used to say only the former.
            //
            // An encrypted PDF produced exactly the line above and exit 0 —
            // byte-identical in shape to a plain readable file. An operator
            // sweeping a directory to find what pdfce can handle would have
            // been told every file was fine, and found out otherwise one
            // command later.
            //
            // That is R186's shape a third time: the refusal fires correctly
            // at the LOAD layer, and the layer a sweep actually runs first
            // never mentioned it. Reported here rather than moved, because
            // the probe line is genuinely useful on a file whose body will
            // not load — it is how you learn it is a PDF at all.
            if let Err(err) = &full_result {
                eprintln!(
                    "pdfce-cli: {}: the header reads as PDF {version}, but pdfce could NOT load the document body: {err}. Anything reported above describes the header alone.",
                    file.display()
                );
                return exit_code_for_doc(err);
            }
            // ISO 32000-2 §7.6.7. Checked here rather than only in a forms
            // or attachments command because `inspect` is what a sweep runs
            // first, and the whole hazard is an operator concluding from a
            // clean result that they are looking at the document.
            if let Some(loaded) = full.as_ref() {
                disclose_wrapper(file, loaded);
            }
            exit::SUCCESS
        }
        // Header probe failed and recovery did not open it: not a PDF.
        (Err(err), _) => {
            eprintln!("pdfce-cli: {}: {err}", file.display());
            exit_code_for(err)
        }
    }
}

/// Print the honest cross-reference-recovery disclosure (decision 013,
/// R20). One counted diagnostic line to stderr plus a save-behaviour note.
///
/// Diagnostics go to **stderr** so a script reading `inspect`'s stdout for
/// the stable `path: PDF version` line is not disturbed; the distinct
/// [`exit::OPENED_VIA_RECOVERY`] status is the machine-readable signal.
fn disclose_recovery(file: &Path, report: &pdfce_core::recover::RecoveryReport) {
    eprintln!(
        "pdfce-cli: {}: opened via cross-reference recovery (rebuild-by-scan): \
         reason={:?}, file-level-objects={}, from-object-streams={}, \
         last-wins-collisions={}, trailer={:?}, offset-start={}, \
         stream-lengths-recovered={}, missing-endobj-recovered={}",
        file.display(),
        report.reason,
        report.file_level_objects,
        report.objstm_objects,
        report.last_wins_collisions,
        report.trailer_source,
        report.offset_start,
        report.stream_lengths_recovered,
        report.missing_endobj_recovered,
    );
    eprintln!(
        "pdfce-cli: {}: NOTE: the cross-reference table was rebuilt in memory; \
         saving will rewrite (normalize) the file, and incremental save is refused.",
        file.display()
    );
    if report.stream_lengths_recovered > 0 {
        eprintln!(
            "pdfce-cli: {}: NOTE: {} stream(s) had a /Length that did not agree with their \
             endstream keyword; their byte extents were re-derived from the keyword \
             (ISO 32000-1 \u{a7}7.3.8.2 defines /Length in terms of endstream). Those extents \
             are pdfce's reading of the file, not the file's own claim.",
            file.display(),
            report.stream_lengths_recovered
        );
    }
    if report.missing_endobj_recovered > 0 {
        eprintln!(
            "pdfce-cli: {}: NOTE: {} object definition(s) had no `endobj` keyword \
             (ISO 32000-1 \u{a7}7.3.10 requires one); each was bounded at the next object \
             header instead of being dropped. Dropping is what pdfce did before \
             2026-08-07, and when the dropped object was the page tree the saved file \
             named a /Pages object that was not in it.",
            file.display(),
            report.missing_endobj_recovered
        );
    }
}

/// Upper bound on a single supplied font file, in bytes (pdfce policy,
/// ARCHITECTURE.md §10 — never trust a file's size). 64 MiB comfortably
/// covers even large CJK OpenType faces while refusing a
/// pathologically-large or wrong-typed file before it is read into
/// memory. A file past this ceiling is skipped-and-noted, never fatal
/// (decision 012 acceptance).
const MAX_FONT_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Font-file extensions the `--font-dir` walk attempts to parse
/// (decision 012). Matched case-insensitively. A file with any other
/// extension is silently ignored (a font folder routinely also holds
/// `.txt` licences, `.json` metadata, etc.); a file WITH one of these
/// extensions that then fails to parse or is oversized IS noted, because
/// the operator meant it to be a usable font.
const FONT_FILE_EXTENSIONS: [&str; 7] = ["ttf", "otf", "ttc", "cff", "pfb", "pfa", "otc"];

/// Walk each `--font-dir` and build the [`pdfce_render::FontEnvironment`]
/// the renderer will consult for the document's NON-embedded fonts
/// (decision 012 — the SHELL owns the filesystem; the renderer stays
/// bytes-in, R61).
///
/// For every readable font-extension file under each directory (sorted
/// for determinism, so which face wins a duplicate name is stable), the
/// bytes are parsed ONCE through `pdfce-render`'s single skrifa parser
/// (R21) to read the face's advertised name(s), then registered via
/// [`pdfce_render::FontEnvironment::insert_named`] under **both** every
/// advertised name AND the filename stem — so `Calibri.ttf` matches a
/// PDF `/BaseFont` of `Calibri` whether or not the program's internal
/// name agrees. A file that cannot be read, exceeds
/// [`MAX_FONT_FILE_BYTES`], or fails to parse is skipped and pushed to
/// the returned notes; it never aborts the walk or the render.
///
/// Returns the environment plus a `(registered, notes)` pair:
/// `registered` is the count of faces (name→file registrations) added,
/// `notes` are the human-readable skip/registration lines for stderr.
/// When `font_dirs` is empty the environment is exactly
/// [`pdfce_render::FontEnvironment::bundled`] and both are empty — the
/// deterministic default path is untouched (R19/R63).
fn build_font_environment(
    font_dirs: &[PathBuf],
) -> (pdfce_render::FontEnvironment, usize, Vec<String>) {
    use pdfce_render::FontData;
    use pdfce_render::font::program::FontProgram;

    let mut env = pdfce_render::FontEnvironment::bundled();
    let mut registered = 0usize;
    let mut notes: Vec<String> = Vec::new();

    for dir in font_dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(err) => {
                notes.push(format!("font dir {}: {err}", dir.display()));
                continue;
            }
        };
        // Collect + sort so registration order (and therefore
        // duplicate-name precedence: last wins) is deterministic rather
        // than dependent on the OS directory-iteration order (R19 spirit,
        // even though the walk itself is shell-side).
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && has_font_extension(p))
            .collect();
        files.sort();

        for path in files {
            let meta = std::fs::metadata(&path);
            if let Ok(m) = &meta
                && m.len() > MAX_FONT_FILE_BYTES
            {
                notes.push(format!(
                    "skipped {}: {} bytes exceeds the {}-MiB font-file ceiling",
                    path.display(),
                    m.len(),
                    MAX_FONT_FILE_BYTES / (1024 * 1024)
                ));
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(err) => {
                    notes.push(format!("skipped {}: {err}", path.display()));
                    continue;
                }
            };
            // Parse ONCE (R21) to read the advertised name(s). The borrow
            // ends before `bytes` is moved into `FontData` below.
            let mut names: Vec<String> = match FontProgram::parse(&bytes) {
                Ok(program) => program.face_names(),
                Err(err) => {
                    notes.push(format!(
                        "skipped {}: not a usable font ({err})",
                        path.display()
                    ));
                    continue;
                }
            };
            // Always also register under the filename stem, so a match
            // works even when the internal name is odd or absent.
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && !names.iter().any(|n| n == stem)
            {
                names.push(stem.to_owned());
            }
            if names.is_empty() {
                notes.push(format!(
                    "skipped {}: parsed but advertises no name and has no usable filename",
                    path.display()
                ));
                continue;
            }
            let data = FontData::new(bytes);
            for name in &names {
                env.insert_named(name, data.clone());
                registered += 1;
            }
            notes.push(format!(
                "registered {} as: {}",
                path.display(),
                names.join(", ")
            ));
        }
    }

    (env, registered, notes)
}

/// Whether `path`'s extension is one of [`FONT_FILE_EXTENSIONS`]
/// (case-insensitive).
fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| FONT_FILE_EXTENSIONS.contains(&e.as_str()))
}

/// Implement `pdfce-cli render-page <input> [--page N] [--scale S] -o <out>`.
///
/// # The pipeline
///
/// Four stages, each with its own failure mode and exit code:
///
/// 1. **Load** — [`Document::load`] reads the file, walks the
///    cross-reference chain, and eagerly parses every in-use object.
///    Mapped by [`exit_code_for_doc`].
/// 2. **Resolve the page tree** — [`pdfce_core::page_tree::pages`]
///    flattens the tree with inheritance applied, yielding a `Vec<Page>`
///    in document order. Its index is what `--page` selects.
/// 3. **Rasterize** — [`pdfce_render::render_page_with`] at `scale`
///    device pixels per user-space unit. The default face set is the
///    **bundled** Base-14 substitutes ([`pdfce_render::RenderOptions::default`]);
///    `--font-dir` layers OPERATOR-supplied faces on top (decision 012).
///    The CLI never *auto-discovers* system fonts — rule R19 (decision
///    004) makes the default render deterministic, and a batch job whose
///    output silently depends on which fonts the runner happens to have
///    installed is not one anyone can trust. `--font-dir` is the explicit,
///    disclosed opt-in: the shell walks the folder (R61), and glyphs it
///    draws from a supplied face are reported via the `supplied` counter,
///    distinct from the bundled `substituted` counter (R62). Fonts the
///    document does not embed and that no supplied face matches are still
///    reported via `substituted`.
/// 4. **Encode + write** — `Pixmap::encode_png` (tiny-skia's
///    `png-format` feature) then a plain file write.
///
/// # Page selection
///
/// `--page` is **1-based**, matching how every PDF reader and every human
/// numbers pages. `0` and any value past the last page take the same
/// out-of-range path: a stderr message naming the actual page count, and
/// [`exit::RUNTIME_ERROR`]. Deliberately *not* a clap range constraint —
/// clap would exit `2` (usage error) for `--page 0` but this function
/// would exit `1` for `--page 999`, and a script branching on the exit
/// code should not have to care which flavour of "that page isn't there"
/// it hit.
///
/// # Output
///
/// One machine-readable line on stdout in the format documented in the
/// module header, and — only when the render was less than fully faithful
/// — a human-readable expansion on stderr. A clean render writes nothing
/// to stderr at all, so `2>/dev/null` is never needed and a non-empty
/// stderr is a real signal.
///
/// # Exit codes
///
/// `0` success; `3` the input could not be read or the output could not
/// be written; `4` the input is not a PDF; `1` everything else (structural
/// failure, page out of range, raster-size guard, PNG encoding).
#[allow(clippy::too_many_arguments)]
fn cmd_render_page(
    input: &Path,
    page_number: u32,
    scale: f32,
    output: &Path,
    annotations: bool,
    font_dirs: &[PathBuf],
    show_layers: &[String],
    hide_layers: &[String],
    print_state: bool,
) -> u8 {
    // Build the font environment from any `--font-dir` BEFORE loading the
    // document: the walk is pure shell-side I/O (R61), and a bad font dir
    // is a note, never a fatal error. With no `--font-dir` this is exactly
    // the bundled default and the deterministic path is untouched (R63).
    let (font_env, supplied_registered, font_notes) = build_font_environment(font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };

    let pages = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    // 1-based → 0-based. `checked_sub` handles `--page 0` without a
    // panic or a wrap; `get` handles past-the-end. Both land on the same
    // message so the failure reads the same way whichever end it came
    // from.
    let Some(page) = page_number
        .checked_sub(1)
        .and_then(|i| pages.get(i as usize))
    else {
        eprintln!(
            "pdfce-cli: {}: page {page_number} is out of range (document has {} page(s), \
numbered 1..={})",
            input.display(),
            pages.len(),
            pages.len()
        );
        return exit::RUNTIME_ERROR;
    };

    // Annotation painting is on by default (§12.5); `--no-annotations`
    // clears it to reproduce the pre-6.0 content-only raster. The font
    // environment carries any `--font-dir` supplied faces (decision 012);
    // with no `--font-dir` it is the bundled default (R63).
    // §8.6.4.4 mandates no CMYK conversion, so the operator's persisted
    // choice governs (R169). Read from the same `userdata/` store the GUI
    // uses, so `render-page` and the canvas cannot disagree about what
    // black looks like. Loading cannot fail — a missing or broken file
    // yields defaults plus notes, which are reported and never fatal.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let mut render_options = pdfce_render::RenderOptions::default()
        .with_annotations(annotations)
        .with_cmyk_intent(settings.cmyk_intent)
        // The other four R169 rendering knobs, all spec silences the
        // standard declines to fill: the mask resampling filter
        // (`SM-A1`, §8.9.6.3), the minification filter (`IM-A1`,
        // §8.9.5.3), the CMYK-JPEG polarity rule (`DCT-A1`, §7.4.8) and
        // the missing-`/AS` policy (`AS-A1`, §12.5.5). Every default is
        // the behaviour pdfce shipped before the setting existed, so a
        // machine with no settings file renders exactly as it always did.
        .with_mask_resample(settings.mask_resample)
        .with_image_minify(settings.image_minify)
        .with_cmyk_jpeg_polarity(settings.cmyk_jpeg_polarity)
        .with_missing_as(settings.missing_as);
    render_options.fonts = font_env;
    // `render-page` produces a raster for LOOKING AT, so it is a viewer
    // under §8.11.4.5 and applies `View`-event `/AS` usage at the
    // requested scale. The print path is the one the clause forbids this
    // on, and pdfce's printing does not come through here.
    // §8.11.4.5: only a viewer examines `/AS`; printing and aggregating
    // applications "shall not apply the changes based on usage
    // application dictionaries". `--print-state` is that mode, and NOTE 2
    // licenses offering it.
    if !print_state {
        render_options.view_magnification = Some(scale);
    }

    // §8.11 layer overrides. Resolved by NAME against the document's own
    // registry, because a name is what an operator has (`list-layers`
    // prints them) and an object number is not.
    if !show_layers.is_empty() || !hide_layers.is_empty() {
        match resolve_layer_override(&doc, show_layers, hide_layers) {
            Ok((visibility, unmatched)) => {
                for name in unmatched {
                    eprintln!(
                        "pdfce-cli: no layer named {name:?} in {} — the other --show-layer/--hide-layer names were still applied",
                        input.display()
                    );
                }
                render_options.layers = Some(visibility);
            }
            Err(name) => {
                eprintln!(
                    "pdfce-cli: layer {name:?} was given to both --show-layer and --hide-layer; pdfce will not guess which you meant"
                );
                // `RUNTIME_ERROR` rather than a new usage code: clap owns
                // the usage vocabulary and this is not a malformed command
                // line — both flags are spelled correctly and mean what
                // they say. What cannot be done is honouring both.
                return exit::RUNTIME_ERROR;
            }
        }
    }

    let rendered = match pdfce_render::render_page_with(&doc, page, scale, &render_options) {
        Ok(rendered) => rendered,
        Err(err) => {
            eprintln!("pdfce-cli: {}: page {page_number}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    let png = match rendered.pixmap.encode_png() {
        Ok(png) => png,
        Err(err) => {
            eprintln!(
                "pdfce-cli: {}: PNG encoding failed: {err}",
                output.display()
            );
            return exit::RUNTIME_ERROR;
        }
    };
    if let Err(err) = std::fs::write(output, &png) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }

    // The stable stdout line (module header, "stdout result-line format").
    // Counters last, key=value, fixed order — appended to, never reordered.
    let d = &rendered.diagnostics;
    // The Pass 6.0 annotation counters are APPENDED to the metrics half,
    // after every pre-existing key — the stable-line contract's
    // append-never-reorder rule (module docs). `annot_no_ap` is a SUM of
    // the per-subtype `annotations_without_ap` map (the machine line's
    // contract is `key=<integer>`); the per-subtype breakdown goes to
    // stderr where it cannot break a parser. `need_appearances` is the
    // document-scoped `/NeedAppearances` disclosure (R51).
    let annot_no_ap: usize = d.annotations_without_ap.values().sum();
    let need_appearances = usize::from(pdfce_core::annot::need_appearances(&doc));
    println!(
        "rendered {} page {page_number} -> {} {}x{}; \
substituted={} notdef={} unsupported={} unknown={} deferred={} \
images={} images_unsupported={} forms={} \
images_codec_unsupported={} codec_features={} codec_geometry_mismatch={} \
dct_cmyk={} lzw_anomalies={} dct_cmyk_unverifiable={} jpx_preblended={} \
annots={} annots_painted={} annots_no_ap={} annots_hidden={} \
annots_state_missing={} annots_widget={} annots_degenerate={} need_appearances={} \
unsupported_type3={} unsupported_noncmap={} unsupported_vertical={} \
unsupported_composite_not_embedded={} unsupported_unknown_subtype={} \
unsupported_unusable_program={} supplied={} supplied_registered={} \
contents_unresolved={} images_masked={} images_mask_unsupported={} \
masks_resampled={} mattes_undone={} mattes_not_undone={} oc_hidden={}",
        input.display(),
        output.display(),
        rendered.pixmap.width(),
        rendered.pixmap.height(),
        d.glyphs_substituted,
        d.glyphs_notdef,
        d.fonts_unsupported,
        d.unknown_ops,
        d.deferred_ops,
        d.images_rendered,
        d.images_unsupported,
        d.forms_rendered,
        d.images_codec_unsupported,
        d.codec_feature_unsupported.values().sum::<usize>(),
        d.codec_geometry_mismatch,
        d.dct_cmyk_images,
        d.lzw_framing_anomalies,
        d.dct_cmyk_polarity_unverifiable,
        d.jpx_smask_in_data_preblended,
        d.annotations_total,
        d.annotations_painted,
        annot_no_ap,
        d.annotations_hidden,
        d.annotations_appearance_state_missing,
        d.annotations_widget,
        d.annotations_placement_degenerate,
        need_appearances,
        // Per-reason breakdown of `unsupported` (R20): always emitted in
        // a fixed order, even at zero, so the line stays diffable. Sum ==
        // `unsupported`. `unusable_program` non-zero = an embedded program
        // pdfce could not parse (the class that hid the TrueType misroute).
        unsupported_reason(d, "Type3"),
        unsupported_reason(d, "NonIdentityCmap"),
        unsupported_reason(d, "VerticalWriting"),
        unsupported_reason(d, "CompositeNotEmbedded"),
        unsupported_reason(d, "UnknownSubtype"),
        unsupported_reason(d, "UnusableProgram"),
        // decision 012: the SUPPLIED trust level, appended after every
        // pre-existing key. `supplied` = glyphs drawn from an
        // operator-supplied face; `supplied_registered` = name→file
        // registrations the `--font-dir` walk added (0 without the flag).
        d.glyphs_supplied,
        supplied_registered,
        // Appended after every pre-existing key: `/Contents` entries this
        // page named that are not in the file, so their marks are simply
        // absent from the raster (§7.3.10 + Table 30).
        d.contents_streams_unresolved,
        // Image transparency (§8.9.6, §11.6.5.3), appended after every
        // pre-existing key. `images_masked` is a SUBSET of `images` —
        // those whose `/SMask`, `/Mask` or JPX opacity channel was
        // actually composited — and is census, not shortfall. The
        // per-mechanism breakdown goes to stderr, where a new key cannot
        // break a parser. `images_mask_unsupported` is the shortfall
        // twin: the picture is on the page but too solid.
        d.images_masked,
        d.images_mask_unsupported,
        d.masks_resampled,
        d.mattes_undone,
        d.mattes_not_undone,
        // §8.11.3.2 optional content, appended after every pre-existing
        // key. A page that renders emptier than expected because a
        // producer turned a layer OFF is indistinguishable from a render
        // that failed — unless this number is on the line. It is the
        // disclosure channel for the one feature whose correct behaviour
        // is "draw less" (R183).
        d.oc_sections_hidden,
    );
    report_diagnostics(d);

    exit::SUCCESS
}

/// Resolve `--show-layer` / `--hide-layer` names into a complete
/// [`pdfce_render::LayerVisibility`].
///
/// # The set REPLACES the document's configuration, so it is built from it
///
/// `LayerVisibility` is not a patch (see that type's module docs): the
/// renderer uses it *instead of* `/OCProperties /D`. So the answer starts
/// from [`pdfce_core::annot::optional_content_default_off`] — what the
/// document asks for — and applies the operator's names on top. Passing
/// only the named groups would show every layer the document had turned
/// off, which is a wrong raster that looks plausible.
///
/// # Errors
///
/// Returns the offending name when it appears in BOTH lists. That is
/// refused rather than resolved by flag order: the operator asked for two
/// contradictory things and there is no reading of the command line that
/// says which one they meant. Order-dependence would make the same two
/// flags mean different things depending on how a script assembled them.
///
/// Names matching no layer are returned as the second tuple element for
/// the caller to report — a note, not a failure, so a batch over a
/// hundred drawings does not abort because one lacks a "Grid" layer.
fn resolve_layer_override(
    doc: &pdfce_core::document::Document,
    show: &[String],
    hide: &[String],
) -> Result<(pdfce_render::LayerVisibility, Vec<String>), String> {
    if let Some(clash) = show.iter().find(|n| hide.contains(n)) {
        return Err(clash.clone());
    }
    let graph = doc.view();
    let read = pdfce_core::layers::read_layers(&graph);
    let mut hidden = pdfce_core::annot::optional_content_default_off(&graph);
    let mut unmatched = Vec::new();
    for (names, make_visible) in [(show, true), (hide, false)] {
        for name in names {
            let mut matched = false;
            for l in read.layers.iter().filter(|l| &l.name == name) {
                matched = true;
                if make_visible {
                    hidden.remove(&l.id);
                } else {
                    hidden.insert(l.id);
                }
            }
            if !matched {
                unmatched.push(name.clone());
            }
        }
    }
    Ok((pdfce_render::LayerVisibility::hiding(hidden), unmatched))
}

/// Count of `unsupported` fonts attributed to one reason key (0 when the
/// reason never occurred) — the accessor behind the fixed-order tokens on
/// `render-page`'s stdout line.
fn unsupported_reason(d: &pdfce_render::Diagnostics, key: &str) -> usize {
    d.fonts_unsupported_by_reason.get(key).copied().unwrap_or(0)
}

/// A `" (reason=count, …)"` suffix naming only the reasons that actually
/// occurred, for the stderr note. Empty when the breakdown is empty (it
/// never is when `fonts_unsupported > 0`, but the guard keeps the caller
/// honest). Reasons are emitted in `UnsupportedFont::all_reason_keys`
/// order so the note is stable.
fn fonts_unsupported_breakdown(d: &pdfce_render::Diagnostics) -> String {
    let parts: Vec<String> = pdfce_render::text::UnsupportedFont::all_reason_keys()
        .iter()
        .filter_map(|key| {
            let n = unsupported_reason(d, key);
            (n > 0).then(|| format!("{key}={n}"))
        })
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

/// Write the human-readable half of the R20 honesty report to stderr —
/// the detail that is too unbounded (font names, operator names) to put
/// on the machine-readable stdout line.
///
/// Silent when the render was fully faithful. That silence is the point:
/// it makes "stderr had output" a usable signal in a batch script rather
/// than noise the operator learns to ignore.
fn report_diagnostics(d: &pdfce_render::Diagnostics) {
    if !d.substituted_fonts.is_empty() {
        eprintln!(
            "pdfce-cli: note: {} glyph(s) drawn with bundled substitute faces, not the \
document's own: {}",
            d.glyphs_substituted,
            d.substituted_fonts.join(", ")
        );
    }
    // decision 012 / R62: supplied faces are disclosed SEPARATELY from
    // bundled — the operator's own shapes, not pdfce's guess. Positions
    // still come from the PDF's `/Widths`, so this improves shapes, not
    // layout (R63: such a render is machine-dependent by definition).
    if !d.supplied_fonts.is_empty() {
        eprintln!(
            "pdfce-cli: note: {} glyph(s) drawn with operator-supplied faces (shapes only — \
positions still come from the document's own widths, and this render is machine-dependent): {}",
            d.glyphs_supplied,
            d.supplied_fonts.join(", ")
        );
    }
    if d.fonts_unsupported > 0 {
        eprintln!(
            "pdfce-cli: note: {} font(s) use machinery this build does not implement; \
their text was SKIPPED, not approximated{}",
            d.fonts_unsupported,
            fonts_unsupported_breakdown(d)
        );
    }
    if d.glyphs_notdef > 0 {
        eprintln!(
            "pdfce-cli: note: {} glyph(s) had no mapping and were drawn as .notdef or omitted",
            d.glyphs_notdef
        );
    }
    if (d.unknown_ops > 0 || d.deferred_ops > 0) && !d.sample_ops.is_empty() {
        eprintln!(
            "pdfce-cli: note: {} unknown and {} deferred content operator(s); \
first distinct names: {}",
            d.unknown_ops,
            d.deferred_ops,
            d.sample_ops.join(", ")
        );
    }
    if d.contents_streams_unresolved > 0 {
        eprintln!(
            "pdfce-cli: note: {} /Contents entr(y/ies) on this page name an object that is NOT \
in the file; that content is MISSING from the raster (ISO 32000-1 \u{a7}7.3.10: a reference to an \
absent object is the null object; Table 30: absent /Contents = an empty page)",
            d.contents_streams_unresolved
        );
    }
    if d.images_unsupported > 0 {
        eprintln!(
            "pdfce-cli: note: {} image(s) could not be decoded and are MISSING from the raster \
(nothing was substituted for them)",
            d.images_unsupported
        );
    }
    if d.images_codec_unsupported > 0 {
        eprintln!(
            "pdfce-cli: note: {} of those need an image codec this build does not implement",
            d.images_codec_unsupported
        );
    }
    // The per-name breakdown is the actionable half of R27: "pdfce has
    // no JPEG decoder" and "pdfce has a JPEG decoder but not the
    // arithmetic-coded variant" are different problems with different
    // answers, and only the name distinguishes them.
    if !d.codec_feature_unsupported.is_empty() {
        let named: Vec<String> = d
            .codec_feature_unsupported
            .iter()
            .map(|(feature, count)| format!("{feature} x{count}"))
            .collect();
        eprintln!(
            "pdfce-cli: note: unsupported codec feature(s): {}",
            named.join(", ")
        );
    }
    if d.codec_geometry_mismatch > 0 {
        eprintln!(
            "pdfce-cli: note: {} image(s) whose codestream geometry disagrees with the image \
dictionary; the dictionary was used for placement and the codestream for sample layout",
            d.codec_geometry_mismatch
        );
    }
    // `dct_cmyk_images` (the benign YCCK census) deliberately prints
    // NOTHING here: decision 006 verified those images decode without
    // polarity ambiguity, and the pre-006 "check the colours" note
    // cried wolf on known-good files. The count stays on the stdout
    // metrics line; stderr stays a shortfall-only channel. Only the
    // R30 shape below warrants a warning.
    if d.dct_cmyk_polarity_unverifiable > 0 {
        eprintln!(
            "pdfce-cli: note: {} four-component CMYK JPEG(s) with ColorTransform 0 and no \
/Decode: the polarity of this shape is UNVERIFIABLE — if the producer used the undocumented \
inverted-CMYK convention the image renders as its own negative. pdfce draws the raw samples, \
as every reference engine does, and reports rather than guesses (docs/decisions/006, R30). \
Please keep the file and report it",
            d.dct_cmyk_polarity_unverifiable
        );
    }
    // /SMaskInData 2 alters the COLOUR SAMPLES themselves (a backdrop
    // is mixed into them), which is why it warrants a note where the
    // benign YCCK census does not: the picture on the page is not the
    // picture the document describes, merely the closest one pdfce can
    // draw without clause 11 Matte machinery.
    if d.jpx_smask_in_data_preblended > 0 {
        eprintln!(
            "pdfce-cli: note: {} JPEG2000 image(s) with /SMaskInData 2: their colour channels \
are preblended with a backdrop and the opacity channel needs a /Matte entry to undo. Drawn from \
the preblended channels as stored - correct wherever the image is opaque, showing the backdrop \
where it is not. Un-premultiplication arrives with the transparency model",
            d.jpx_smask_in_data_preblended
        );
    }
    if d.lzw_framing_anomalies > 0 {
        eprintln!(
            "pdfce-cli: note: {} LZW stream(s) missing a ClearCode or EndOfInformation; \
recovered, but the producer is non-conformant",
            d.lzw_framing_anomalies
        );
    }
    // `images_masked` deliberately prints NOTHING here — it is
    // verified-correct volume, and decision 006 §4.4 records what a note
    // on known-good files does to an operator's trust in this channel.
    // The per-mechanism breakdown is offered only as context beside a
    // shortfall, never on its own.
    if d.images_mask_unsupported > 0 {
        let named: Vec<String> = d
            .mask_refused
            .iter()
            .map(|(reason, count)| format!("{reason} x{count}"))
            .collect();
        eprintln!(
            "pdfce-cli: note: {} image(s) carry an /SMask or /Mask that could not be applied \
({}); they are drawn FULLY OPAQUE, so the page shows content the document intended to be \
hidden or see-through",
            d.images_mask_unsupported,
            named.join(", ")
        );
    }
    if d.mattes_not_undone > 0 {
        eprintln!(
            "pdfce-cli: note: {} soft mask(s) carry /Matte (preblended colour) whose inversion \
was not applied; the alpha IS applied, but colours in the partially-transparent regions stay \
shifted toward the matte colour. The reason is in the image divergences below",
            d.mattes_not_undone
        );
    }
    if !d.image_notes.is_empty() {
        eprintln!(
            "pdfce-cli: note: image divergences: {}",
            d.image_notes.join("; ")
        );
    }
    if d.xobject_depth_overflows > 0 {
        eprintln!(
            "pdfce-cli: note: {} form XObject invocation(s) refused as too deeply nested or \
cyclic; their content is missing from the raster",
            d.xobject_depth_overflows
        );
    }
    if d.tolerated > 0 {
        eprintln!(
            "pdfce-cli: note: {} structural oddity(ies) tolerated while interpreting the page",
            d.tolerated
        );
    }
    // --- Pass 6.0 annotation honesty (R43/R50/R27) -------------------
    if !d.annotations_without_ap.is_empty() {
        // R43: these annotations have no usable /AP and pdfce paints
        // NOTHING for them (it never synthesises a look). The per-subtype
        // breakdown is the actionable half — it names what kind of
        // appearance generation a later Pass would need.
        let named: Vec<String> = d
            .annotations_without_ap
            .iter()
            .map(|(subtype, count)| format!("{subtype} x{count}"))
            .collect();
        eprintln!(
            "pdfce-cli: note: {} annotation(s) have no usable appearance stream and were NOT \
painted (pdfce never synthesises a look): {}",
            d.annotations_without_ap.values().sum::<usize>(),
            named.join(", ")
        );
    }
    if d.annotations_appearance_state_missing > 0 {
        eprintln!(
            "pdfce-cli: note: {} annotation(s) have an appearance-state (/AS) that could not be \
resolved (missing, or naming an absent state); displayed as nothing, never guessed",
            d.annotations_appearance_state_missing
        );
    }
    if d.annotations_hidden > 0 {
        // R50: a hidden annotation is content the operator cannot see —
        // disclosed, because it is a document-forensics-relevant fact.
        eprintln!(
            "pdfce-cli: note: {} annotation(s) are suppressed on screen by the Hidden or NoView \
flag; honoured (not painted) AND disclosed",
            d.annotations_hidden
        );
    }
    if d.annotations_placement_degenerate > 0 {
        eprintln!(
            "pdfce-cli: note: {} annotation(s) carry an appearance that could not be placed \
(missing /Rect or /BBox, or a degenerate transformed box); refused by name, never mis-placed",
            d.annotations_placement_degenerate
        );
    }
    if !d.annotation_notes.is_empty() {
        eprintln!(
            "pdfce-cli: note: annotation placement notes: {}",
            d.annotation_notes.join("; ")
        );
    }
}

/// Implement `pdfce-cli list-annotations <input> [--pages …]`: a read-only
/// per-page annotation inventory (ISO 32000-1 §12.5).
///
/// # Output (locale-invariant, stable, parseable)
///
/// One `annot …` line per annotation, in page then `/Annots`-array order
/// (deterministic), followed by one `list-annotations …` summary line.
/// Both are pure-ASCII `key=value` and go to **stdout**; the leading token
/// distinguishes them. Per line:
///
/// ```text
/// annot page=<P> index=<I> subtype=<Name|none> rect=<llx,lly,urx,ury|none> \
///       flags=0x<hex> widget=<0|1> disposition=<D> ap=<A> \
///       author=<none|"…"> note=<none|"…"> modified=<none|"…">
/// list-annotations <input> pages=<N>; annots=<T> paint_ready=<P> no_ap=<Q> \
///       state_missing=<S> suppressed=<H> popup=<U> widget=<W> \
///       with_note=<C> with_author=<A> need_appearances=<0|1>
/// ```
///
/// `disposition` is the **model-level** classification a reader would
/// apply, in the render path's precedence order:
/// - `popup` — a `/Popup`: never page content (§12.5.6.14), whatever its
///   `/AP`.
/// - `suppressed` — Hidden or NoView flag set (§12.5.3): not shown on
///   screen (R50: disclosed anyway).
/// - `paint-ready` — a resolvable normal appearance stream (`render-page`
///   would paint it, unless its transformed box is degenerate — a
///   placement fact `render-page`'s `annots_degenerate` counter reports).
/// - `no-ap` — no usable `/AP` `/N` (R43 named-not-painted).
/// - `state-missing` — an `/AS` that could not be resolved (§12.5.5 NOTE 3).
/// - `no-rect` — a paintable appearance but no `/Rect` placement target.
///
/// `ap` is the appearance shape: `stream`, `state-dict`, or `none`.
///
/// # The three note columns (Pass 38.5, closing this command's own named gap)
///
/// `author`, `note` and `modified` are `/T`, `/Contents` and `/M`
/// (§12.5.2 Table 164, §12.5.6.2 Table 170), decoded by
/// [`pdfce_core::annot`]'s §7.9.2 text-string reader so a UTF-16BE
/// `/Contents` prints as text and not as mojibake. They are appended
/// **last**, after `ap=`, so a parser that reads through the pre-existing
/// columns is unaffected.
///
/// Each is either the bare token `none` or a `quoted_token` string, and
/// the distinction is load-bearing rather than cosmetic — a document
/// really can carry the literal author name `none`, and it prints as
/// `author="none"`. Quoting also makes a note containing spaces,
/// newlines or quotes a single field, which an unquoted value could not
/// be.
///
/// **`author=none` never means "anonymous".** `/T` is a **Table 170
/// markup-only** key: a `/Link` or a `/Widget` has no author concept at
/// all, so its absence there is a statement about the subtype, not about
/// the person. The same distinction the core model draws
/// ([`pdfce_core::annot::Annotation::title`]) is preserved here rather
/// than flattened into an empty string.
///
/// **`modified` is emitted RAW**, exactly as the file stores it, because
/// §12.5.2 types `/M` as *"date **or** text string"* and obliges a reader
/// to accept any format. Normalising it to ISO-8601 here would invent
/// precision the document does not have, and would silently discard the
/// producer-specific formats that are the whole reason the key is typed
/// so loosely.
///
/// The summary line's `with_note` / `with_author` counts are over the
/// **selected pages only**, like every other counter on that line.
///
/// # Exit codes
///
/// `0` success; `3`/`4` unreadable / not-a-PDF; `1` for a structural
/// failure or an out-of-range `--pages` selection.
fn cmd_list_annotations(input: &Path, pages_spec: &str) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let pages = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(pages_spec, pages.len()) {
        Ok(sel) => sel,
        Err(msg) => {
            eprintln!("pdfce-cli: {}: {msg}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    let need_appearances = usize::from(pdfce_core::annot::need_appearances(&doc));
    let (mut total, mut paint_ready, mut no_ap) = (0usize, 0usize, 0usize);
    let (mut state_missing, mut suppressed, mut popup, mut widget) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut with_note, mut with_author) = (0usize, 0usize);

    for &page_index in &selected {
        let Some(page) = pages.get(page_index) else {
            continue;
        };
        let annots = pdfce_core::annot::page_annotations(&doc, page.id);
        for (array_index, annot) in annots.iter().enumerate() {
            total += 1;
            if annot.is_widget() {
                widget += 1;
            }
            let (disposition, ap_shape) = classify_for_listing(annot);
            match disposition {
                "popup" => popup += 1,
                "suppressed" => suppressed += 1,
                "paint-ready" | "no-rect" => paint_ready += 1,
                "no-ap" => no_ap += 1,
                "state-missing" => state_missing += 1,
                _ => {}
            }
            let subtype = if annot.subtype.is_empty() {
                "none".to_owned()
            } else {
                // Names cannot legally contain whitespace; sanitise
                // defensively so the line stays field-splittable.
                sanitize_token(&annot.subtype_label())
            };
            let rect = match annot.rect {
                Some(r) => format!("{},{},{},{}", r.llx, r.lly, r.urx, r.ury),
                None => "none".to_owned(),
            };
            if annot.contents.is_some() {
                with_note += 1;
            }
            if annot.title.is_some() {
                with_author += 1;
            }
            // `none` (bare) vs `"…"` (quoted): see this function's doc
            // comment — the bare token is what makes an ABSENT key
            // distinguishable from a key whose value happens to be the
            // word "none".
            let opt_token = |v: Option<&String>| match v {
                Some(s) => quoted_token(s),
                None => "none".to_owned(),
            };
            println!(
                "annot page={} index={array_index} subtype={subtype} rect={rect} \
flags=0x{:X} widget={} disposition={disposition} ap={ap_shape} author={} note={} modified={}",
                page_index + 1,
                annot.flags.0,
                usize::from(annot.is_widget()),
                opt_token(annot.title.as_ref()),
                opt_token(annot.contents.as_ref()),
                opt_token(annot.mod_date.as_ref()),
            );
        }
    }

    println!(
        "list-annotations {} pages={}; annots={total} paint_ready={paint_ready} no_ap={no_ap} \
state_missing={state_missing} suppressed={suppressed} popup={popup} widget={widget} \
with_note={with_note} with_author={with_author} need_appearances={need_appearances}",
        input.display(),
        selected.len(),
    );
    exit::SUCCESS
}

/// `list-fields`: inventory a document's AcroForm fields (Pass 7).
///
/// Read-only. One `field …` line per terminal field, then a `list-fields …`
/// summary line carrying the document-level form disclosures. The value is
/// emitted as a sanitised token so the line stays field-splittable.
/// `list-outline` — the document's bookmarks, as a tree.
///
/// # Why the indentation is real output and not decoration
///
/// An outline's SHAPE is its meaning: "Chapter 3" nested under "Part II"
/// says something a flat list of titles does not. So the level is both
/// rendered as indentation for a person and printed as `level=` for a
/// script, rather than one or the other.
///
/// # `read_outline`, not `parse_outline`
///
/// The core module offers both. `parse_outline` is the thin wrapper that
/// returns items alone and **silently discards the diagnostics** —
/// including "this tree was truncated because it contained a cycle".
/// A command that reported a partial outline as if it were the whole
/// thing would be making a claim about the document it cannot support.
fn cmd_list_outline(input: &Path, flat: bool) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let outline = pdfce_core::outline::read_outline(&session.graph());

    fn emit(items: &[pdfce_core::outline::OutlineItem], flat: bool) -> usize {
        let mut n = 0;
        for it in items {
            n += 1;
            let dest = match &it.destination {
                Some(d) => format!("{d:?}"),
                // Distinguished from a destination pdfce could not
                // resolve: an item with no destination at all is a
                // heading, which is legal and common.
                None => "-".to_owned(),
            };
            let indent = if flat {
                String::new()
            } else {
                "  ".repeat(it.level)
            };
            println!(
                "{indent}bookmark level={} open={} title={:?} dest={dest}",
                it.level,
                u32::from(it.open),
                it.title,
            );
            n += emit(&it.children, flat);
        }
        n
    }
    let shown = emit(&outline.items, flat);

    // The diagnostics are not a footnote. A truncated tree looks exactly
    // like a short one from the outside, and only this line distinguishes
    // them.
    //
    // Reported as only the NON-ZERO counters. The struct carries twenty-odd
    // fields and on a healthy document every one is zero — dumping them all
    // put the two that matter inside a wall of `foo: 0`, which is how a real
    // warning gets skimmed past. The cycle fixture made it obvious:
    // `cycles_broken: 3` was in there, and invisible. Found by reading the
    // command's own output (R174).
    let d = &outline.diagnostics;
    let mut notes: Vec<String> = Vec::new();
    if d.item_budget_exhausted {
        notes.push("item_budget_exhausted".to_owned());
    }
    if d.root_count_disagreement {
        notes.push("root_count_disagreement".to_owned());
    }
    for (c, name) in [
        (d.depth_truncations, "depth_truncations"),
        (d.cycles_broken, "cycles_broken"),
        (d.unreadable_items, "unreadable_items"),
        (d.titles_unreadable, "titles_unreadable"),
        (d.titles_inexact, "titles_inexact"),
        (d.unmapped_pages, "unmapped_pages"),
        (d.unresolved_names, "unresolved_names"),
        (d.count_disagreements, "count_disagreements"),
        (d.unknown_views, "unknown_views"),
        (d.malformed_views, "malformed_views"),
        (d.cross_namespace_resolutions, "cross_namespace_resolutions"),
        (d.non_reference_links, "non_reference_links"),
        (d.unreadable_actions, "unreadable_actions"),
        (
            d.dest_and_action_both_present,
            "dest_and_action_both_present",
        ),
    ] {
        if c > 0 {
            notes.push(format!("{name}={c}"));
        }
    }
    if let Some(e) = &d.page_tree_error {
        notes.push(format!("page_tree_error={e:?}"));
    }
    let warnings = if notes.is_empty() {
        "clean".to_owned()
    } else {
        notes.join(" ")
    };
    println!(
        "list-outline {} bookmarks={shown} max_depth={} {warnings}",
        input.display(),
        d.max_depth,
    );
    exit::SUCCESS
}

/// `list-signatures` — what each signature COVERS, not whether it is valid.
///
/// # The caveat is on the output, not in the help text
///
/// This is the one command in the CLI where a reader is most likely to
/// take away more than was said. "list-signatures" on a signed document,
/// printing offsets and byte counts, looks exactly like a verification
/// report — and pdfce performs no cryptography at all.
///
/// So every run ends with a line saying so. Not a `--verbose` extra, not
/// the man page: the summary line itself, on every invocation, because
/// the operator who most needs it is the one who did not read the docs.
///
/// # What the numbers mean
///
/// `covered` is how many bytes the digest spans. `tail` is how many lie
/// PAST the end of everything it covers — the number that matters, and
/// the shape an incremental save takes when a revision is appended after
/// signing. A non-zero tail does not mean the signature is broken; it
/// means it protects less than the whole file.
///
/// §12.8.1 makes whole-file coverage a `should`, not a `shall`, so a
/// short range is reported and never called malformed. Overlapping
/// ranges violate Table 252's "exact byte range" and ARE reported as
/// malformed. The two are deliberately distinguishable in the output.
fn cmd_list_signatures(input: &Path) -> u8 {
    // The file's real length on disk. `/ByteRange` is a claim about
    // BYTES, and only the bytes can check it — the object model cannot
    // check a claim about the file against itself.
    let file_len = match std::fs::metadata(input) {
        Ok(m) => m.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let graph = session.graph();

    let census = pdfce_core::signature::census(&graph);
    let coverage = pdfce_core::signature::byte_range_coverage(&graph, file_len);

    for c in &coverage {
        let ranges: Vec<String> = c.ranges.iter().map(|(o, l)| format!("{o}+{l}")).collect();
        let mut flags: Vec<&str> = Vec::new();
        if !c.ranges_well_formed {
            flags.push("MALFORMED_RANGE");
        }
        if c.uncovered_tail > 0 {
            // Named as what it is, not as an error. A short range is
            // conforming (§12.8.1's `should`).
            flags.push("does-not-cover-whole-file");
        }
        if c.pair_count == 1 {
            // One pair means /Contents sits inside its own digest, which
            // cannot verify — a different and worse problem than a short
            // range.
            flags.push("SINGLE_RANGE_CANNOT_VERIFY");
        }
        println!(
            "signature field={:?} covered={} of {} tail={} pairs={} ranges=[{}]{}",
            c.field_name.as_deref().unwrap_or("-"),
            c.covered,
            c.file_len,
            c.uncovered_tail,
            c.pair_count,
            ranges.join(" "),
            if flags.is_empty() {
                String::new()
            } else {
                format!(" {}", flags.join(" "))
            },
        );
    }

    // TWO warnings, and they must not be conflated. The first draft
    // emitted the "this is permitted, the document is not malformed"
    // reassurance for a document whose ranges OVERLAP — so one run
    // printed `MALFORMED_RANGE` on the row and a line saying nothing was
    // malformed underneath it. Found by reading the output across all
    // three fixtures rather than by any test (R174).
    let malformed = coverage.iter().any(|c| !c.ranges_well_formed);
    if malformed {
        eprintln!(
            "pdfce-cli: WARNING — at least one signature's /ByteRange is MALFORMED: its \
             ranges overlap or run backwards, which Table 252's \"exact byte range\" does \
             not permit. The numbers above are what the file DECLARES; a reader that \
             rejects the array will compute something else, or nothing at all."
        );
    }
    // Only reassure about conformance when there is nothing to be
    // unreassured about.
    if !malformed && coverage.iter().any(|c| c.uncovered_tail > 0) {
        eprintln!(
            "pdfce-cli: WARNING — at least one signature does not cover the whole file. \
             Content exists beyond what it protects. This is permitted by ISO 32000-1 \
             §12.8.1 (whole-file coverage is a \"should\"), so the document is not \
             malformed — but the signature guarantees less than its presence suggests."
        );
    }

    println!(
        "list-signatures {} signatures={} certifications={} with_byte_range={} \
         (COVERAGE ONLY — pdfce performs no cryptographic verification, so this says \
         what each signature would protect, never whether it is valid)",
        input.display(),
        census.signatures,
        census.certifications,
        coverage.len(),
    );
    exit::SUCCESS
}

/// `list-layers` — the document's optional-content groups.
///
/// # Why the default-visibility column is the interesting one
///
/// A layer's name says what it is; `visible=` says whether a reader
/// showing this document with no interaction would draw it. Those come
/// apart constantly — a "Confidential" watermark layer that is OFF by
/// default is a very different document from one where it is ON, and the
/// name alone cannot tell them apart.
///
/// The value comes from `annot.rs`'s `optional_content_default_off`, the
/// same resolver the renderer uses to decide whether an annotation is
/// drawn. Sharing it is the point: a listing that said "on" about
/// content the renderer hides would be worse than no listing.
///
/// # Read-only, and layers are not editable here
///
/// Toggling a layer in a viewer is **session-scoped with zero
/// file-format footprint** unless the operator explicitly saves
/// (`Acrobat_Features/layers__ocg_visibility_and_defaults.md`). pdfce has
/// no save path for it, so there is nothing to offer — and offering a
/// toggle that silently did not persist would be worse than not offering
/// one (R83).
fn cmd_list_layers(input: &Path) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let read = pdfce_core::layers::read_layers(&session.graph());

    for l in &read.layers {
        // An undeclared name is reported as `-`, never invented. Table 98
        // marks `/Name` Required, so its absence is a real malformation,
        // and a synthesised "Layer 3" would hide it behind something that
        // looks like data from the file.
        let name = if l.name_declared {
            format!("{:?}", l.name)
        } else {
            "-".to_owned()
        };
        // Only the flags that are TRUE, and only where true is the
        // interesting case. `in_order=false` matters (the layer will not
        // appear in a conforming panel); `in_order=true` is the norm and
        // says nothing.
        let mut flags: Vec<&str> = Vec::new();
        if l.locked {
            flags.push("locked");
        }
        if !l.in_default_config {
            flags.push("UNREGISTERED");
        }
        if !l.in_order {
            flags.push("not-in-order");
        }
        if !l.name_exact {
            flags.push("name-inexact");
        }
        // §8.11.2.3: a group whose `/Intent` excludes `View` does not
        // participate in visibility, so `visible=1` on it is not a
        // statement about the document's `/OFF` array — it is a
        // statement that the array does not reach this group.
        //
        // Printed only when it is NOT `View`, like every other flag
        // here: the common case says nothing, and a token on every line
        // is a token nobody reads. Without it, a `Design` layer named in
        // `/OFF` prints `visible=1` with no way to tell intent
        // filtering from a pdfce defect.
        if !l.intent_view {
            flags.push("intent-not-view");
        }
        let rb = match l.radio_group {
            Some(g) => format!(" radio_group={g}"),
            None => String::new(),
        };
        println!(
            "layer name={name} visible={}{rb}{}{}",
            u32::from(l.visible_by_default),
            if flags.is_empty() {
                String::new()
            } else {
                format!(" {}", flags.join(" "))
            },
            // Inlined rather than a nested `format!`: clippy's
            // `format_in_format_args` is right that the inner allocation
            // is pointless when the outer macro can format it directly.
            format_args!(" via={:?}", l.discovered_via),
        );
    }

    // Only the diagnostics that fired — the struct has seventeen fields
    // and on a healthy document every one is quiet. Burying the two that
    // matter in fifteen `false`s is how a real warning gets skimmed past;
    // that lesson cost an outline listing earlier today.
    let d = &read.diagnostics;
    let mut notes: Vec<String> = Vec::new();
    for (on, name) in [
        (d.no_optional_content, "no_optional_content"),
        (d.missing_default_config, "missing_default_config"),
        (d.missing_registry, "missing_registry"),
        (d.order_node_truncation, "order_node_truncation"),
        (d.base_state_off_in_default, "base_state_off_in_default"),
        (d.base_state_unrecognised, "base_state_unrecognised"),
        (
            d.base_state_off_with_unregistered,
            "BASE_STATE_OFF_WITH_UNREGISTERED",
        ),
        (d.layer_truncation, "layer_truncation"),
        (d.resource_scan_truncated, "resource_scan_truncated"),
        (d.page_scan_failed, "page_scan_failed"),
    ] {
        if on {
            notes.push(name.to_owned());
        }
    }
    for (c, name) in [
        (d.unregistered_groups, "unregistered_groups"),
        // Not a fault — the file is fine and its state simply moves.
        // Reported because `visible=` on the rows above is the
        // /D-initial answer, and for these groups a viewer's answer
        // depends on magnification (§8.11.4.5).
        (d.auto_managed_groups, "auto_managed_groups"),
        // Decision 038: the file says two things about these groups and
        // pdfce resolved it. The count is emitted here; the sentence
        // naming the RESOLUTION goes to stderr below, because an
        // operator who sees only a count cannot predict what they are
        // looking at.
        (d.contradictory_on_off_groups, "contradictory_on_off_groups"),
        (d.groups_without_name, "groups_without_name"),
        (d.names_inexact, "names_inexact"),
        (d.direct_group_dicts, "direct_group_dicts"),
        (d.dangling_group_references, "dangling_group_references"),
        (d.order_depth_truncations, "order_depth_truncations"),
        (d.order_cycles, "order_cycles"),
        (d.malformed_group_elements, "malformed_group_elements"),
        (d.overlapping_radio_groups, "overlapping_radio_groups"),
    ] {
        if c > 0 {
            notes.push(format!("{name}={c}"));
        }
    }
    let warnings = if notes.is_empty() {
        "clean".to_owned()
    } else {
        notes.join(" ")
    };
    // The sentence that makes the count actionable. A bare
    // `contradictory_on_off_groups=2` says a file is self-contradictory
    // and leaves the operator unable to predict which state pdfce chose;
    // naming the rule lets them work it out for any group (decision 038).
    if read.diagnostics.contradictory_on_off_groups > 0 {
        eprintln!(
            "pdfce-cli: {} group(s) are listed in BOTH /D /ON and /D /OFF. Resolved per ISO 32000-1 §8.11.4.5 b): the array OPPOSITE /BaseState decides, so with the usual /BaseState ON they are OFF. The document is not malformed — nothing forbids a writer from listing a group twice.",
            read.diagnostics.contradictory_on_off_groups
        );
    }
    if read.diagnostics.auto_managed_groups > 0 {
        eprintln!(
            "pdfce-cli: {} group(s) have their state managed automatically by /AS usage application dictionaries (ISO 32000-1 §8.11.4.4). The visible= column above is the state the document OPENS in; a viewer re-computes it from the current magnification, so what render-page draws at a given --scale may differ. Use --print-state on render-page for the state a printing or aggregating application uses.",
            read.diagnostics.auto_managed_groups
        );
    }
    if read.diagnostics.base_state_unrecognised {
        eprintln!(
            "pdfce-cli: /D /BaseState is a name other than ON or OFF. Table 101 requires the default configuration's /BaseState to be ON, so this file is non-conforming; pdfce recovers by treating it as ON, which is both the stated default and the only value /D was allowed to carry."
        );
    }
    let config = read.config_name.as_deref().unwrap_or("-");
    println!(
        "list-layers {} layers={} config={config:?} radio_groups={} {warnings}",
        input.display(),
        read.layers.len(),
        read.radio_groups.len(),
    );
    exit::SUCCESS
}

/// Render one font's `fsType` state as a single stable token.
///
/// ★ The four states must never collapse into each other, and in particular
/// none of them may look like `0`.
///
/// `fsType == 0` genuinely **means** Installable — the most permissive value
/// the field can express — so "we could not read it" and "this format has no
/// such field" have to be visibly different from it and from one another. A
/// report that printed a blank, a dash, or a zero for all three would be
/// asserting the broadest embedding right there is on the strength of bytes
/// nobody read (`PDF_Spec/fonts/font__opentype_os2_fstype.md` N1).
///
/// The raw value is printed alongside the word so the reading can be checked
/// against the specification's own table without re-deriving it.
fn format_fs_type(fs: &pdfce_core::fontinfo::FsType) -> String {
    use pdfce_core::fontinfo::{FsType, FsTypeError};
    match fs {
        FsType::NotApplicable => "n/a-no-field".to_owned(),
        FsType::ProgramNotDecoded => "unknown-not-decoded".to_owned(),
        // The CAUSE, not just "unknown". A sweep of 3,901 corpus documents
        // found 998 of 1,560 embedded programs reading unknown here, and one
        // token could not say whether that was a subsetter stripping `OS/2`
        // (the common, benign case -- the tool that made the subset simply
        // did not carry the table forward) or a damaged font. Those are
        // different facts about the file, and an operator triaging a corpus
        // needs to bucket them apart.
        FsType::Unreadable(why) => match why {
            FsTypeError::NotSfnt => "unknown-not-sfnt",
            FsTypeError::Collection => "unknown-collection",
            FsTypeError::BadTableDirectory => "unknown-bad-directory",
            FsTypeError::NoOs2Table => "unknown-no-os2",
            FsTypeError::Os2Truncated => "unknown-os2-truncated",
            _ => "unknown-unrecognised-cause",
        }
        .to_owned(),
        // `FsType` is `#[non_exhaustive]`, so a future state compiles here
        // rather than breaking the CLI — but it must not silently render as
        // one of the existing ones, least of all as a permission. An
        // unrecognised state prints as unrecognised.
        FsType::Known(bits) => {
            let mut s = format!("{}/0x{:04X}", bits.permission.label(), bits.raw);
            if bits.no_subsetting {
                s.push_str("+nosubset");
            }
            if bits.bitmap_only {
                s.push_str("+bitmaponly");
            }
            if bits.version_gated_bits_ignored {
                s.push_str("+v0v1-bits-ignored");
            }
            if bits.reserved_bit0 {
                s.push_str("+reserved-bit0");
            }
            s
        }
        _ => "unknown-unrecognised-state".to_owned(),
    }
}

/// `list-fonts` — the document's fonts, what they cost, and what could be
/// removed.
///
/// # Why the byte size is here and nowhere else
///
/// ★ Acrobat exposes a per-font byte size **nowhere**: Document Properties →
/// Fonts gives type, encoding and embedded status with no size at all, and
/// Audit Space Usage gives one aggregate "Fonts" bucket for the whole
/// document with no per-font attribution
/// (`Acrobat_Features/optimize__font_reporting.md`, recorded as a GAP with no
/// source found either way). An operator asking "which font is costing me the
/// most" has to infer it there by toggling fonts through the Optimizer one at
/// a time and diffing output sizes.
///
/// pdfce computes it directly from data already parsed. This is a deliberate
/// exceed rather than a parity target — there is no Acrobat behaviour to
/// match, only a gap it leaves open.
///
/// The number is the program's **stored** size: the bytes it occupies in the
/// file, which is what removing it recovers. The decoded size is printed
/// beside it because it answers the different question of how large the font
/// actually is.
///
/// # Why the verdict is a word and not a flag
///
/// Acrobat refuses to unembed a font whose text is glyph-index-keyed, and
/// refuses **silently** — the font simply does not appear in its unembed
/// list, with no reason shown anywhere (sourced to Dov Isaacs, former Adobe
/// Principal Scientist, in `optimize__font_unembedding.md`; independently
/// corroborated by a user whose largest font was absent from the list with no
/// explanation). A shorter list is not actionable. "This font's text is
/// stored as glyph indices into this exact program" is.
///
/// So every font appears, every verdict is named, and the reasons present in
/// the document go to stderr whether or not `--reasons` was passed. That is
/// project rule 4 applied to a refusal: the inference pdfce made is visible
/// before anyone acts on it.
///
/// # Why coverage is on the summary line
///
/// A font inventory that quietly misses a surface and prints a confident list
/// is this project's most-repeated defect (R186). The summary states which
/// surfaces were walked **and which were not**, so the listing carries the
/// shape of its own evidence. Acrobat's coverage here is an unconfirmed GAP,
/// so pdfce states its own scope rather than assuming parity with a behaviour
/// nobody has measured.
fn cmd_list_fonts(input: &Path, reasons: bool, by_size: bool) -> u8 {
    use pdfce_core::fontinfo::{self, Program, Removability};

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let inv = fontinfo::inventory(&doc.view());

    // Borrowed, so the default order stays first-discovery — stable across
    // runs and diff-friendly — and `--by-size` is a view over it rather than
    // a different inventory.
    let mut rows: Vec<&fontinfo::FontRecord> = inv.fonts.iter().collect();
    if by_size {
        // Descending by stored bytes, ties broken by discovery order, which
        // `sort_by_key` preserves (it is stable).
        rows.sort_by_key(|f| std::cmp::Reverse(f.stored_bytes()));
    }

    for f in &rows {
        let name = f.base_font.as_deref().unwrap_or("-");
        // Printed only when it differs, because on the ~13% of embedded
        // fonts that are not subsets it would repeat the name field exactly
        // — and a token that is always present and usually redundant is a
        // token nobody reads (the lesson `list-layers` records).
        let family = match f.family_name() {
            Some(fam) if fam != name => format!(" family={fam:?}"),
            _ => String::new(),
        };
        let ty = match &f.descendant_subtype {
            Some(d) => format!("{}/{}", f.subtype.label(), d.label()),
            None => f.subtype.label().to_owned(),
        };
        let (embedded, bytes, decoded, fstype) = match &f.program {
            Program::NotEmbedded => (
                "no".to_owned(),
                "0".to_owned(),
                "-".to_owned(),
                "n/a-not-embedded".to_owned(),
            ),
            // "Declared but unreadable" is not "not embedded": the first is
            // damage, the second is a document relying on substitution.
            Program::Unreadable { key, .. } => (
                format!("{}!unreadable", key.label()),
                "0".to_owned(),
                "-".to_owned(),
                "n/a-program-unreadable".to_owned(),
            ),
            Program::Embedded(p) => (
                match &p.subtype {
                    Some(s) => format!("{}/{s}", p.key.label()),
                    None => p.key.label().to_owned(),
                },
                p.stored_bytes.to_string(),
                p.decoded_bytes
                    .map_or_else(|| "-".to_owned(), |n| n.to_string()),
                format_fs_type(&p.fs_type),
            ),
            // `Program` is `#[non_exhaustive]`. A state this build does not
            // know must not be rendered as "no" — that would report a font
            // as unembedded on the strength of not recognising it.
            _ => (
                "unrecognised".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "unknown-unrecognised-state".to_owned(),
            ),
        };
        let surfaces = f
            .surfaces
            .iter()
            .map(|s| s.token())
            .collect::<Vec<_>>()
            .join(",");
        let names = if f.resource_names.is_empty() {
            "-".to_owned()
        } else {
            let mut joined = f.resource_names.join(",");
            if f.resource_names_truncated {
                joined.push_str(",…");
            }
            joined
        };
        let obj =
            f.id.map_or_else(|| "direct".to_owned(), |id| format!("{}", id.num));
        println!(
            "font name={name:?}{family} type={ty:?} encoding={:?} embedded={embedded} \
bytes={bytes} decoded={decoded} fstype={fstype} tounicode={} std14={} verdict={} \
pages={} surfaces={surfaces} resources={names} obj={obj}",
            f.encoding.label(),
            u32::from(f.has_to_unicode),
            u32::from(f.standard_14),
            f.removability.token(),
            pdfce_core::fontinfo::format_page_ranges(&f.pages),
        );
        if reasons {
            println!("  reason: {}", f.removability.reason());
        }
    }

    // The document total, computed from the same per-font numbers the rows
    // above show — so the total and the listing cannot disagree. Acrobat's
    // equivalent (Audit Space Usage's aggregate "Fonts" bucket) is
    // Pro-exclusive; this ships in every build.
    let counts = inv.verdict_counts();
    let verdicts = if counts.is_empty() {
        "-".to_owned()
    } else {
        counts
            .iter()
            .map(|(token, n)| format!("{token}={n}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    let d = &inv.diagnostics;
    let mut notes: Vec<String> = Vec::new();
    for (on, name) in [
        (d.resource_scan_truncated, "RESOURCE_SCAN_TRUNCATED"),
        (d.font_limit_reached, "FONT_LIMIT_REACHED"),
        (d.page_scan_failed, "PAGE_SCAN_FAILED"),
    ] {
        if on {
            notes.push(name.to_owned());
        }
    }
    for (c, name) in [
        (d.direct_font_dicts, "direct_font_dicts"),
        (d.dangling_font_references, "dangling_font_references"),
        (d.descriptors_missing, "descriptors_missing"),
        (d.programs_unreadable, "programs_unreadable"),
        (d.programs_undecodable, "programs_undecodable"),
        (d.descendants_missing, "descendants_missing"),
    ] {
        if c > 0 {
            notes.push(format!("{name}={c}"));
        }
    }
    let warnings = if notes.is_empty() {
        "clean".to_owned()
    } else {
        notes.join(" ")
    };

    let walked = inv
        .coverage
        .walked()
        .iter()
        .map(|s| s.token())
        .collect::<Vec<_>>()
        .join(",");
    let not_walked = inv.coverage.not_walked();
    let not_walked_token = if not_walked.is_empty() {
        "-".to_owned()
    } else {
        not_walked
            .iter()
            .map(|s| s.token())
            .collect::<Vec<_>>()
            .join(",")
    };

    println!(
        "list-fonts {} fonts={} embedded={} bytes={} {verdicts} walked={walked} \
not_walked={not_walked_token} {warnings}",
        input.display(),
        inv.fonts.len(),
        inv.embedded_count(),
        inv.embedded_bytes(),
    );

    // ★ The disclosure Acrobat does not make. One sentence per DISTINCT
    // non-removable verdict present, on stderr so stdout stays a clean
    // machine-readable listing. Deduplicated because a document with forty
    // Identity-H fonts needs the mechanism explained once, not forty times —
    // repetition is how a real warning gets skimmed past.
    let mut seen: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
    for f in &rows {
        if matches!(f.removability, Removability::Removable) {
            continue;
        }
        if seen.insert(f.removability.token()) {
            eprintln!(
                "pdfce-cli: {}: verdict {} — {}",
                input.display(),
                f.removability.token(),
                f.removability.reason()
            );
        }
    }
    // Said unconditionally, not only when it bites. An operator reading a
    // font inventory to decide what to delete needs the shape of the
    // evidence, and "there is one place pdfce did not look" is part of the
    // answer rather than a caveat on it.
    if !not_walked.is_empty() {
        eprintln!(
            "pdfce-cli: {}: NOT searched: {not_walked_token}. Font dictionaries reachable from \
none of the walked surfaces still occupy bytes in the file but do not appear above.",
            input.display(),
        );
    }
    if d.page_scan_failed {
        eprintln!(
            "pdfce-cli: {}: the page tree would not walk, so NO page-reachable font is in this \
listing. An empty or short list here is not a statement about the document's fonts.",
            input.display(),
        );
    }
    exit::SUCCESS
}

/// `list-attachments` — embedded files, both kinds, in one list.
///
/// # Why both kinds in one list, each labelled
///
/// Document-level (`/Names /EmbeddedFiles`) and page-level
/// (`/FileAttachment` annotations) are structurally distinct and behave
/// differently on save and on page deletion, so a caller must be able to
/// tell them apart. But an operator asking "what is in this file" should
/// not have to know the distinction exists in order to get a complete
/// answer, so it is one command and one list.
///
/// # ★ The encryption warning is a REFUSAL condition, not a note
///
/// Since PDF 1.5 an otherwise-unencrypted document can carry ENCRYPTED
/// embedded files via `/EFF` + `DefEmbeddedFile` (§7.6.5). The intuitive
/// guard — no password prompt, so plaintext — is wrong, and wrong
/// silently: the filter chain runs and returns garbage that looks like
/// success. pdfce cannot decrypt yet, so when the flag is set this
/// command says so loudly on stderr rather than letting a caller treat
/// the listing as safe to extract from.
///
/// # What a listing means, and what it does not
///
/// Complete enumeration is impossible **by the standard's own admission**
/// (§7.11.7 NOTE 1/3), not by pdfce's limitation: no `shall` requires an
/// embedded file to appear in `/EmbeddedFiles`. So this reports what is
/// reachable by the two standard paths, and the summary line says so
/// rather than implying exhaustiveness.
fn cmd_list_attachments(input: &Path) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let (items, notes) = pdfce_core::attachments::list_attachments_with_notes(&session.graph());

    for a in &items {
        // The RAW name, deliberately — it is what the document says, and
        // an operator investigating a suspicious file needs pdfce's
        // evidence rather than pdfce's cleanup. `safe_name()` exists for
        // the moment bytes are written somewhere, which this command
        // never does.
        let safe = a.safe_name();
        let changed = if safe.changed {
            format!(
                " UNSAFE_NAME safe={:?} hazards={:?}",
                safe.value, safe.hazards
            )
        } else {
            String::new()
        };
        // A COMPACT kind, not the derived Debug. The real one prints
        // `tree_key: [254, 255, 0, 115, ...]` — UTF-16BE bytes where a
        // reader expects a name, plus object ids nobody asked for. The
        // page number is the part that matters for a page-level
        // attachment; the tree key IS the name, already printed.
        let kind = match &a.kind {
            pdfce_core::attachments::AttachmentKind::DocumentLevel { .. } => "document".to_owned(),
            pdfce_core::attachments::AttachmentKind::PageAnnotation { page_index, .. } => {
                format!("page:{}", page_index + 1)
            }
            // `AttachmentKind` is `#[non_exhaustive]`, so a kind added
            // later lands here. Reported as unknown rather than folded
            // into "document" — a wrong label is worse than an honest
            // gap, because only one of the two prompts anyone to look.
            _ => "unknown".to_owned(),
        };
        println!(
            "attachment name={:?} kind={kind} desc={:?} source={:?}{changed}",
            a.name,
            a.description.as_deref().unwrap_or("-"),
            a.name_source,
        );
    }

    if notes.may_be_encrypted {
        eprintln!(
            "pdfce-cli: WARNING — this document's embedded files may be ENCRYPTED (§7.6.5 \
             /EFF). pdfce cannot decrypt them yet, and extracting one would produce \
             ciphertext that looks like a successful read. Do not treat these bytes as \
             the file's contents."
        );
    }
    // Same reasoning as the outline diagnostics: only what is not clean.
    let mut n: Vec<String> = Vec::new();
    if notes.page_tree_unwalkable {
        n.push("page_tree_unwalkable".to_owned());
    }
    if notes.truncated {
        n.push("truncated".to_owned());
    }
    if notes.name_tree_budget_exhausted {
        n.push("name_tree_budget_exhausted".to_owned());
    }
    for (c, name) in [
        (notes.name_tree_cycles, "name_tree_cycles"),
        (notes.malformed_tree_entries, "malformed_tree_entries"),
        (
            notes.annotations_without_filespec,
            "annotations_without_filespec",
        ),
        (notes.filespecs_without_stream, "filespecs_without_stream"),
        (notes.unresolvable_streams, "unresolvable_streams"),
    ] {
        if c > 0 {
            n.push(format!("{name}={c}"));
        }
    }
    if notes.may_be_encrypted {
        n.push("MAY_BE_ENCRYPTED".to_owned());
    }
    let warnings = if n.is_empty() {
        "clean".to_owned()
    } else {
        n.join(" ")
    };
    println!(
        "list-attachments {} attachments={} {warnings} (reachable by the two standard \
         paths; ISO 32000-1 §7.11.7 does not require completeness)",
        input.display(),
        items.len(),
    );
    exit::SUCCESS
}

/// The scaling modes, as command-line words.
///
/// A separate type from `pdfce_print::ScaleMode` because that one carries a
/// free-form `Custom(f64)` which clap cannot express as a value-enum
/// variant, and because the CLI's vocabulary is allowed to differ from
/// the engine's — `shrink` reads better than `ShrinkOversized` in a
/// shell.
/// `--binding` on `print`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum BindingArg {
    /// Side-by-side halves, bound on the left.
    Left,
    /// Side-by-side halves, bound on the right.
    Right,
    /// Stacked halves, bound on the left (a horizontal fold).
    LeftTall,
    /// Stacked halves, bound on the right.
    RightTall,
}

impl BindingArg {
    /// The imposition type this maps to.
    const fn to_binding(self) -> pdfce_print::imposition::Binding {
        match self {
            Self::Left => pdfce_print::imposition::Binding::Left,
            Self::Right => pdfce_print::imposition::Binding::Right,
            Self::LeftTall => pdfce_print::imposition::Binding::LeftTall,
            Self::RightTall => pdfce_print::imposition::Binding::RightTall,
        }
    }
}

/// `--booklet-subset` on `print`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum BookletSubsetArg {
    /// Every face.
    BothSides,
    /// Front faces only — the first pass on a printer without duplex.
    FrontOnly,
    /// Back faces only — the second pass, after re-feeding.
    BackOnly,
}

impl BookletSubsetArg {
    /// The imposition type this maps to.
    const fn to_subset(self) -> pdfce_print::imposition::BookletSubset {
        match self {
            Self::BothSides => pdfce_print::imposition::BookletSubset::BothSides,
            Self::FrontOnly => pdfce_print::imposition::BookletSubset::FrontOnly,
            Self::BackOnly => pdfce_print::imposition::BookletSubset::BackOnly,
        }
    }
}

/// `--comments` on `print` — which annotation classes reach the paper.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum CommentsArg {
    /// Page content and form fields only, no review markup. The
    /// DEFAULT, matching Reader rather than Acrobat Pro: a comment
    /// reaching paper unasked is the costlier mistake.
    Document,
    /// Page content plus all markup annotations.
    Markups,
    /// Page content plus stamps only — narrower than markup.
    Stamps,
    /// Form fields alone. The page itself is NOT printed, which is the
    /// point: this is for printing onto a pre-printed form.
    FieldsOnly,
}

impl CommentsArg {
    /// The render type this maps to.
    const fn to_scope(self) -> pdfce_render::AnnotationScope {
        match self {
            Self::Document => pdfce_render::AnnotationScope::Document,
            Self::Markups => pdfce_render::AnnotationScope::DocumentAndMarkups,
            Self::Stamps => pdfce_render::AnnotationScope::DocumentAndStamps,
            Self::FieldsOnly => pdfce_render::AnnotationScope::FormFieldsOnly,
        }
    }
}

/// `--orientation` on `print`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OrientationArg {
    /// Decide per page from its own aspect ratio.
    Auto,
    /// Force portrait.
    Portrait,
    /// Force landscape.
    Landscape,
}

impl OrientationArg {
    /// The core type this maps to.
    const fn to_orientation(self) -> pdfce_print::Orientation {
        match self {
            Self::Auto => pdfce_print::Orientation::Auto,
            Self::Portrait => pdfce_print::Orientation::Portrait,
            Self::Landscape => pdfce_print::Orientation::Landscape,
        }
    }
}

/// `--duplex` on `print`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum DuplexArg {
    /// One side only.
    Simplex,
    /// Flip on the long edge (book binding).
    LongEdge,
    /// Flip on the short edge (notepad binding).
    ShortEdge,
}

impl DuplexArg {
    /// The core type this maps to.
    const fn to_duplex(self) -> pdfce_print::Duplex {
        match self {
            Self::Simplex => pdfce_print::Duplex::Simplex,
            Self::LongEdge => pdfce_print::Duplex::LongEdge,
            Self::ShortEdge => pdfce_print::Duplex::ShortEdge,
        }
    }
}

/// `--subset` on `print`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum SubsetArg {
    /// Every selected page.
    All,
    /// Only odd DOCUMENT page numbers.
    Odd,
    /// Only even DOCUMENT page numbers.
    Even,
}

impl SubsetArg {
    /// The core type this maps to.
    const fn to_subset(self) -> pdfce_print::PageSubset {
        match self {
            Self::All => pdfce_print::PageSubset::All,
            Self::Odd => pdfce_print::PageSubset::Odd,
            Self::Even => pdfce_print::PageSubset::Even,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum PrintScaleArg {
    /// Scale to fill the printable area, enlarging a small page.
    /// Reader's own default.
    Fit,
    /// 1 PDF point = 1/72 inch on paper, clipping if it must.
    Actual,
    /// Actual size, except reduce a page too big for the sheet. Never
    /// enlarges — which is the whole difference from `fit`.
    Shrink,
}

impl PrintScaleArg {
    fn to_mode(self) -> pdfce_print::ScaleMode {
        match self {
            Self::Fit => pdfce_print::ScaleMode::Fit,
            Self::Actual => pdfce_print::ScaleMode::ActualSize,
            Self::Shrink => pdfce_print::ScaleMode::ShrinkOversized,
        }
    }

    /// `#[cfg(windows)]` because every caller is: the mode name is only
    /// ever printed on a result line that reports a real device, and the
    /// non-Windows build has no device to report. Without the gate this is
    /// a dead-code warning that `-D warnings` turns into a failed build —
    /// which was invisible for as long as the crate did not compile on
    /// non-Windows at all.
    #[cfg(windows)]
    fn name(self) -> &'static str {
        match self {
            Self::Fit => "fit",
            Self::Actual => "actual",
            Self::Shrink => "shrink",
        }
    }
}

/// `print` — send pages to a printer.
///
/// # ★ It does a DRY RUN unless told otherwise
///
/// `--send` is required to start a job. Without it every step runs —
/// device context, capability query, placement, rasterisation, the page
/// loop — and stops before `StartDoc`.
///
/// That default is the opposite of most tools and is deliberate. Printing
/// is irreversible, consumes a physical resource, and occupies a device
/// other people may be waiting for. A command whose safe mode is the one
/// you get by *not* thinking is a command that fails safely for the
/// person who mistyped a page range at 2 a.m.
///
/// It also makes the command testable by its own author on a machine
/// with one printer whose owner is sitting at it — which is how this was
/// written.
///
/// # Rasterised, and it says so
///
/// pdfce renders each page to pixels and sends the bitmap. Reader sends
/// vector and text to the driver and lets it RIP, keeping "print as
/// image" as an explicitly-invoked fallback for driver bugs
/// (`printing__rendering_pipeline_and_resolution.md`).
///
/// So pdfce's default IS Reader's fallback. On a CAD drawing that is
/// visibly coarser than the driver's own output, and an operator
/// printing a drawing needs telling before the paper comes out, not
/// after. The result line says `mode=raster` on every run for that
/// reason.
// Twelve arguments, five over clippy's bound. They are `clap`'s own
// parsed flags handed straight through, and bundling them into a struct
// would mean a second definition of the command's surface that has to be
// kept in step with the derive — the same reasoning `interpret::run`
// carries for its decomposed render inputs.
#[allow(clippy::too_many_arguments)]
fn cmd_print(
    input: &Path,
    printer: Option<&str>,
    scale: PrintScaleArg,
    scale_percent: Option<u32>,
    pages_spec: &str,
    send: bool,
    dpi_cap: u32,
    to_file: Option<PathBuf>,
    copies: u16,
    uncollated: bool,
    subset: SubsetArg,
    reverse: bool,
    orientation: OrientationArg,
    duplex: DuplexArg,
    pick_tray: bool,
    comments: CommentsArg,
    n_up: Option<u32>,
    n_up_border: bool,
    booklet: bool,
    poster: bool,
    poster_scale: f64,
    poster_overlap: f64,
    poster_large_only: bool,
    poster_max_tiles: u32,
    binding: BindingArg,
    booklet_subset: BookletSubsetArg,
) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let all = match pdfce_print::list_printers() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::IO_ERROR;
        }
    };
    let name = match printer {
        Some(name) => name.to_owned(),
        None => match all.iter().find(|p| p.is_default) {
            Some(p) => p.name.clone(),
            None => {
                eprintln!(
                    "pdfce-cli: no default printer is set — pass --printer with one of the names from `pdfce-cli list-printers`"
                );
                return exit::EDIT_REFUSED;
            }
        },
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let page_list = match session.pages() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(pages_spec, page_list.len()) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("pdfce-cli: {msg}");
            return exit::RUNTIME_ERROR;
        }
    };
    let caps = match pdfce_print::printer_caps(&name) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::EDIT_REFUSED;
        }
    };

    let device_settings = pdfce_print::DeviceSettings {
        orientation: orientation.to_orientation(),
        duplex: duplex.to_duplex(),
        pick_tray_by_page_size: pick_tray,
    };
    let mode = match scale_percent {
        Some(pct) => pdfce_print::ScaleMode::Custom(f64::from(pct) / 100.0),
        None => scale.to_mode(),
    };
    // The placement and resolution arithmetic comes from `pdfce-print`
    // rather than being repeated here, so the GUI and the CLI cannot
    // come to disagree about where a page lands — the drift whose
    // symptom is a GUI print landing differently from a CLI print of the
    // same document, which nobody thinks to compare.
    let page_sizes: Vec<(f64, f64)> = page_list
        .iter()
        .map(|p| {
            let mb = p.media_box;
            ((mb.urx - mb.llx).abs(), (mb.ury - mb.lly).abs())
        })
        .collect();
    let spec = pdfce_print::JobSpec {
        pages: selected.clone(),
        mode,
        max_dpi: dpi_cap,
        subset: subset.to_subset(),
        reverse,
        copies,
        collate: if uncollated {
            pdfce_print::Collate::Uncollated
        } else {
            pdfce_print::Collate::Collated
        },
    };
    // ★ TURNED for this job before ANY layout is computed against it —
    // and it must be built after `spec`, because the page that decides
    // `--orientation auto` is the first page the SEQUENCE sends, not
    // `pages[0]`.
    //
    // `printer_caps` reports the device's default `DEVMODE`, so on a
    // portrait-default printer it hands back a portrait printable area
    // while a landscape job prints on a sheet the driver has turned.
    // Every consumer below reads `device.printable_pt` — plain placement,
    // n-up cells, poster tiles, booklet halves — so turning it here is
    // what keeps all four honest rather than four separate fixes that
    // would eventually disagree.
    let device = pdfce_print::DeviceGeometry::from_caps(
        &caps,
        device_settings.orientation,
        spec.first_page_pt(&page_sizes),
    );
    // ---- The three job-shape modes are mutually exclusive ----
    //
    // N-up, booklet and poster each REMAP the job rather than scale it,
    // and no two of them compose. Before this guard existed the three
    // branches ran in sequence and the last one to fire silently
    // overwrote the others' work: `--poster --booklet` composed nine
    // poster tiles, threw them away, and printed a booklet. The operator
    // got a plausible job that was not the one they asked for, with no
    // indication anything had been discarded.
    //
    // Refusing is right rather than picking a precedence. There is no
    // reading of `--poster --booklet` that is obviously intended, so any
    // precedence pdfce chose would be a guess presented as a result.
    {
        let modes = [
            (n_up.is_some(), "--n-up"),
            (booklet, "--booklet"),
            (poster, "--poster"),
        ];
        let named: Vec<&str> = modes
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, n)| *n)
            .collect();
        if named.len() > 1 {
            eprintln!(
                "pdfce-cli: {} cannot be combined — each one changes the shape of the job, and no two of them compose. Pick one.",
                named.join(" and ")
            );
            return exit::EDIT_REFUSED;
        }
    }

    let resolution = pdfce_print::job_resolution(&device, &spec);
    let plans = pdfce_print::plan_job(&device, &page_sizes, &spec);
    let dpi = resolution.dpi;
    let capped = resolution.capped;
    let clipped = plans.iter().filter(|p| p.placement.clipped).count();
    let mut bitmaps: Vec<pdfce_print::PageBitmap> = Vec::new();

    // ---- N-up: several source pages composited onto one sheet ----
    //
    // Handled as its own path rather than as another `ScaleMode`,
    // because it changes the SHAPE of the job: N source pages become one
    // sheet, so the one-plan-per-page arithmetic above no longer
    // describes it. Trying to express that as a placement would mean a
    // plan whose `index` is a lie.
    if let Some(count) = n_up {
        let nup = pdfce_print::imposition::NUpSpec {
            grid: pdfce_print::imposition::NUpGrid::Count(count),
            order: pdfce_print::imposition::PageOrder::Horizontal,
            border: n_up_border,
            auto_rotate: true,
        };
        let sequence = spec.sequence();
        let ordered_sizes: Vec<(f64, f64)> = sequence
            .iter()
            .filter_map(|&i| page_sizes.get(i).copied())
            .collect();
        let layout =
            match pdfce_print::imposition::plan_n_up(device.printable_pt, &ordered_sizes, &nup) {
                Ok(l) => l,
                Err(err) => {
                    eprintln!("pdfce-cli: {err}");
                    return exit::RUNTIME_ERROR;
                }
            };
        let mut sheets: Vec<pdfce_print::PageBitmap> = Vec::new();
        for sheet_index in 0..layout.sheets {
            // One pixmap per SHEET, at the device resolution, with each
            // source page drawn into its own cell. Compositing here
            // rather than sending one blit per cell keeps the spooler
            // loop unchanged — it still sees one bitmap per physical
            // sheet, which is what a sheet is.
            let px = |pt: f64| (pt * f64::from(resolution.dpi) / 72.0).round().max(1.0) as u32;
            let (sw, sh) = (px(device.printable_pt.0), px(device.printable_pt.1));
            let Some(mut sheet) = pdfce_render::tiny_skia::Pixmap::new(sw, sh) else {
                eprintln!("pdfce-cli: a sheet of {sw}x{sh} pixels is too large to compose");
                return exit::RUNTIME_ERROR;
            };
            sheet.fill(pdfce_render::tiny_skia::Color::WHITE);
            for slot in layout.slots.iter().filter(|s| s.sheet == sheet_index) {
                let Some(&source) = sequence.get(slot.source) else {
                    continue;
                };
                let (Some(page), Some(&size)) = (page_list.get(source), page_sizes.get(source))
                else {
                    continue;
                };
                let scale = (f64::from(resolution.dpi) / 72.0) * slot.fit.scale;
                let options = pdfce_render::RenderOptions::default()
                    .with_annotation_scope(comments.to_scope());
                let rendered = match pdfce_render::render_page_with_view(
                    &session.view(),
                    page,
                    scale as f32,
                    &options,
                ) {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!("pdfce-cli: page {}: {err}", source + 1);
                        return exit::RUNTIME_ERROR;
                    }
                };
                let _ = size;
                sheet.draw_pixmap(
                    px(slot.fit.rect.x) as i32,
                    px(slot.fit.rect.y) as i32,
                    rendered.pixmap.as_ref(),
                    &pdfce_render::tiny_skia::PixmapPaint::default(),
                    pdfce_render::tiny_skia::Transform::identity(),
                    None,
                );
            }
            sheets.push(pdfce_print::PageBitmap {
                width: sheet.width(),
                height: sheet.height(),
                rgba: sheet.data().to_vec(),
                // The sheet is already the printable area at device
                // resolution, so it is placed 1:1 with no further
                // scaling — the imposition did the fitting.
                placement: pdfce_print::Placement {
                    scale: 1.0,
                    offset_x_pt: 0.0,
                    offset_y_pt: 0.0,
                    clipped: false,
                },
                page_pt: device.printable_pt,
            });
        }
        bitmaps = sheets;
    }

    // ---- Poster: ONE page tiled across MANY sheets ----
    //
    // The inverse of N-up, and its own path for the same reason: it
    // changes the SHAPE of the job. N-up puts many pages on one sheet by
    // scaling them into cells; poster puts one page on many sheets by
    // cropping it into tiles, and no `Placement` expresses a crop.
    //
    // Planned PER PAGE rather than once for the document, because
    // `plan_poster` takes one page size: a document whose pages differ in
    // size tiles each to its own grid, which is the only answer that does
    // not silently letterbox the odd one out.
    if poster {
        let spec_p = pdfce_print::imposition::PosterSpec {
            tile_scale: poster_scale,
            overlap_pt: poster_overlap,
            cut_marks: false,
            labels: false,
            tile_only_large_pages: poster_large_only,
            max_tiles: poster_max_tiles,
        };
        let px = |pt: f64| (pt * f64::from(resolution.dpi) / 72.0).round().max(1.0) as u32;
        let (sw, sh) = (px(device.printable_pt.0), px(device.printable_pt.1));
        let mut sheets: Vec<pdfce_print::PageBitmap> = Vec::new();
        let mut tiled_pages = 0usize;
        let mut untiled_pages = 0usize;

        for &index in &spec.sequence() {
            let (Some(page), Some(&size)) = (page_list.get(index), page_sizes.get(index)) else {
                continue;
            };
            // `tile_only_large_pages` asks the planner, not this loop: the
            // predicate is the planner's to own so the CLI and the GUI
            // cannot disagree about what counts as "large" (R171 — read the
            // value off the one place that owns it, never restate it).
            if !spec_p.tiles_page(device.printable_pt, size) {
                untiled_pages += 1;
                // Printed at its natural placement, in sequence, so a
                // mixed document comes off the printer in reading order
                // rather than with the small pages collected at the end.
                let render_scale = f64::from(resolution.dpi) / 72.0;
                let options = pdfce_render::RenderOptions::default()
                    .with_annotation_scope(comments.to_scope());
                let rendered = match pdfce_render::render_page_with_view(
                    &session.view(),
                    page,
                    render_scale as f32,
                    &options,
                ) {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!("pdfce-cli: page {}: {err}", index + 1);
                        return exit::RUNTIME_ERROR;
                    }
                };
                sheets.push(pdfce_print::PageBitmap {
                    width: rendered.pixmap.width(),
                    height: rendered.pixmap.height(),
                    rgba: rendered.pixmap.data().to_vec(),
                    placement: pdfce_print::Placement {
                        scale: 1.0,
                        offset_x_pt: 0.0,
                        offset_y_pt: 0.0,
                        clipped: false,
                    },
                    page_pt: size,
                });
                continue;
            }
            let layout =
                match pdfce_print::imposition::plan_poster(device.printable_pt, size, &spec_p) {
                    Ok(l) => l,
                    Err(err) => {
                        eprintln!("pdfce-cli: page {}: {err}", index + 1);
                        return exit::RUNTIME_ERROR;
                    }
                };
            tiled_pages += 1;
            // The page is rendered ONCE at the tile scale and each tile
            // copies its own window out of it. Rendering per tile would
            // re-rasterise the whole page for every sheet — on a 4x5
            // poster, twenty times the work for identical pixels.
            let render_scale = (f64::from(resolution.dpi) / 72.0) * spec_p.tile_scale;
            let options =
                pdfce_render::RenderOptions::default().with_annotation_scope(comments.to_scope());
            let rendered = match pdfce_render::render_page_with_view(
                &session.view(),
                page,
                render_scale as f32,
                &options,
            ) {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("pdfce-cli: page {}: {err}", index + 1);
                    return exit::RUNTIME_ERROR;
                }
            };
            for tile in &layout.tiles {
                let Some(mut sheet) = pdfce_render::tiny_skia::Pixmap::new(sw, sh) else {
                    eprintln!("pdfce-cli: a sheet of {sw}x{sh} pixels is too large to compose");
                    return exit::RUNTIME_ERROR;
                };
                sheet.fill(pdfce_render::tiny_skia::Color::WHITE);
                // `source_pt` is in the page's own points; the rendered
                // pixmap is at `render_scale`, so the window is scaled by
                // exactly that. Dividing by `tile_scale` here would undo
                // the magnification the operator asked for.
                let sx = px(tile.source_pt.x * spec_p.tile_scale) as i32;
                let sy = px(tile.source_pt.y * spec_p.tile_scale) as i32;
                sheet.draw_pixmap(
                    px(tile.sheet_pt.x) as i32 - sx,
                    px(tile.sheet_pt.y) as i32 - sy,
                    rendered.pixmap.as_ref(),
                    &pdfce_render::tiny_skia::PixmapPaint::default(),
                    pdfce_render::tiny_skia::Transform::identity(),
                    None,
                );
                sheets.push(pdfce_print::PageBitmap {
                    width: sheet.width(),
                    height: sheet.height(),
                    rgba: sheet.data().to_vec(),
                    placement: pdfce_print::Placement {
                        scale: 1.0,
                        offset_x_pt: 0.0,
                        offset_y_pt: 0.0,
                        clipped: false,
                    },
                    page_pt: device.printable_pt,
                });
            }
            eprintln!(
                "pdfce-cli: page {}: poster of {} x {} tiles ({} sheet(s)), assembled size \
{:.0} x {:.0} pt, {:.0} pt overlap.",
                index + 1,
                layout.columns,
                layout.rows,
                layout.tiles.len(),
                layout.poster_pt.0,
                layout.poster_pt.1,
                layout.overlap_pt,
            );
        }
        if untiled_pages > 0 {
            eprintln!(
                "pdfce-cli: {untiled_pages} page(s) already fit the paper and were printed \
untiled; {tiled_pages} page(s) were tiled."
            );
        }
        bitmaps = sheets;
    }

    // ---- Booklet: folded imposition, two page-halves per sheet face ----
    //
    // Its own path for the same reason N-up is: it changes the SHAPE of
    // the job. A booklet is not a scaling of the page sequence, it is a
    // REMAPPING of it — sheet 1 carries the last page beside the first —
    // and no `Placement` can express that.
    //
    // The blank positions are real slots with no source. They are
    // rendered as empty sheet halves rather than skipped, because a
    // booklet's blanks are structural: dropping them shortens the fold
    // and every subsequent sheet carries the wrong pages.
    if booklet {
        let spec_b = pdfce_print::imposition::BookletSpec {
            binding: binding.to_binding(),
            subset: booklet_subset.to_subset(),
            sheets: None,
            auto_rotate: true,
        };
        let sequence = spec.sequence();
        let ordered_sizes: Vec<(f64, f64)> = sequence
            .iter()
            .filter_map(|&i| page_sizes.get(i).copied())
            .collect();
        let layout = match pdfce_print::imposition::plan_booklet(
            device.printable_pt,
            &ordered_sizes,
            &spec_b,
        ) {
            Ok(l) => l,
            Err(err) => {
                eprintln!("pdfce-cli: {err}");
                return exit::RUNTIME_ERROR;
            }
        };
        let px = |pt: f64| (pt * f64::from(resolution.dpi) / 72.0).round().max(1.0) as u32;
        let (sw, sh) = (px(device.printable_pt.0), px(device.printable_pt.1));
        let mut faces: Vec<pdfce_print::PageBitmap> = Vec::new();
        // One bitmap per SHEET FACE, in the order they must be fed.
        let mut keys: Vec<(usize, bool)> = layout
            .slots
            .iter()
            .map(|s| {
                (
                    s.sheet,
                    matches!(s.side, pdfce_print::imposition::BookletSide::Back),
                )
            })
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for (sheet_no, is_back) in keys {
            let Some(mut face) = pdfce_render::tiny_skia::Pixmap::new(sw, sh) else {
                eprintln!("pdfce-cli: a sheet of {sw}x{sh} pixels is too large to compose");
                return exit::RUNTIME_ERROR;
            };
            face.fill(pdfce_render::tiny_skia::Color::WHITE);
            for slot in layout.slots.iter().filter(|s| {
                s.sheet == sheet_no
                    && matches!(s.side, pdfce_print::imposition::BookletSide::Back) == is_back
            }) {
                let (Some(source_pos), Some(fit)) = (slot.source, slot.fit) else {
                    // A structural blank. The half stays white.
                    continue;
                };
                let Some(&source) = sequence.get(source_pos) else {
                    continue;
                };
                let Some(page) = page_list.get(source) else {
                    continue;
                };
                let scale = (f64::from(resolution.dpi) / 72.0) * fit.scale;
                let options = pdfce_render::RenderOptions::default()
                    .with_annotation_scope(comments.to_scope());
                let rendered = match pdfce_render::render_page_with_view(
                    &session.view(),
                    page,
                    scale as f32,
                    &options,
                ) {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!("pdfce-cli: page {}: {err}", source + 1);
                        return exit::RUNTIME_ERROR;
                    }
                };
                face.draw_pixmap(
                    px(fit.rect.x) as i32,
                    px(fit.rect.y) as i32,
                    rendered.pixmap.as_ref(),
                    &pdfce_render::tiny_skia::PixmapPaint::default(),
                    pdfce_render::tiny_skia::Transform::identity(),
                    None,
                );
            }
            faces.push(pdfce_print::PageBitmap {
                width: face.width(),
                height: face.height(),
                rgba: face.data().to_vec(),
                placement: pdfce_print::Placement {
                    scale: 1.0,
                    offset_x_pt: 0.0,
                    offset_y_pt: 0.0,
                    clipped: false,
                },
                page_pt: device.printable_pt,
            });
        }
        eprintln!(
            "pdfce-cli: booklet of {} sheet(s), {} page(s) after padding, {} blank position(s). \
             Print two-sided on the long edge, or print one side and re-feed.",
            layout.total_sheets, layout.padded_pages, layout.blank_positions
        );
        bitmaps = faces;
    }

    if n_up.is_none() && !booklet && !poster {
        for plan in &plans {
            let (Some(page), Some(&size)) = (page_list.get(plan.index), page_sizes.get(plan.index))
            else {
                continue;
            };
            let placement = plan.placement;
            let render_scale = plan.render_scale;
            let options =
                pdfce_render::RenderOptions::default().with_annotation_scope(comments.to_scope());
            let rendered = match pdfce_render::render_page_with_view(
                &session.view(),
                page,
                render_scale as f32,
                &options,
            ) {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("pdfce-cli: page {}: {err}", plan.index + 1);
                    return exit::RUNTIME_ERROR;
                }
            };
            bitmaps.push(pdfce_print::PageBitmap {
                width: rendered.pixmap.width(),
                height: rendered.pixmap.height(),
                rgba: rendered.pixmap.data().to_vec(),
                placement,
                page_pt: size,
            });
        }
    }

    let dry = if send {
        pdfce_print::DryRun::No
    } else {
        pdfce_print::DryRun::Yes
    };
    // The orientation page is passed EXPLICITLY, and it is the same one
    // `device` was turned for. The imposition paths hand the spooler one
    // bitmap per SHEET, so letting `spool` re-derive the page from the
    // bitmaps would resolve `--orientation auto` from the sheet in those
    // paths and from a source page in this one — two answers where the
    // job has room for only one.
    let report = match pdfce_print::spool(
        &name,
        &bitmaps,
        dry,
        to_file.as_deref(),
        device_settings,
        spec.first_page_pt(&page_sizes),
    ) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::RUNTIME_ERROR;
        }
    };

    if capped {
        eprintln!(
            "pdfce-cli: rendering at {dpi} DPI, below the printer's {}x{}. A full-resolution page \
             costs about {} MB of memory each; raise --max-dpi if you need the detail and have \
             the memory.",
            caps.dpi_x,
            caps.dpi_y,
            resolution.uncapped_page_mb()
        );
    }
    if clipped > 0 {
        eprintln!(
            "pdfce-cli: {clipped} page(s) do not fit the printable area and will lose content off \
             the edges. Acrobat clips silently here; pdfce says so. Use --scale fit to avoid it."
        );
    }
    if !report.printed {
        eprintln!(
            "pdfce-cli: DRY RUN — nothing was printed and no job was queued. Everything up to \
             starting the job ran against the real device. Add --send to print."
        );
    }

    println!(
        "print {} printer={name:?} pages={} printed={} dpi={}x{} clipped={} mode=raster job={}",
        input.display(),
        report.pages,
        u8::from(report.printed),
        report.dpi.0,
        report.dpi.1,
        report.clipped_pages,
        report
            .job_id
            .map_or_else(|| "-".to_owned(), |j| j.to_string()),
    );
    exit::SUCCESS
}

/// `print-preview` — what a print WOULD do, without doing it.
///
/// # Why this exists before `print` does
///
/// Printing is an outward-facing, irreversible side effect: paper is
/// consumed and a shared device is occupied. So the surface that answers
/// "what would happen" ships before the one that makes it happen, and
/// this command deliberately has no flag that starts a job.
///
/// It is not a placeholder either. Everything a real print needs —
/// resolving the printer, reading its resolution and printable area,
/// and placing each page onto the sheet — happens here and is reported.
/// When spooling lands it will consume this exact result, so a preview
/// that reads correctly is evidence about the print, not a separate
/// approximation of it.
///
/// # The clip report is the point
///
/// Acrobat clips an oversized page **silently**
/// (`Acrobat_Features/printing__scaling_modes.md`). pdfce names the pages
/// that would lose content, and the exit code reflects it, so a scripted
/// caller can refuse to print rather than discover the loss on paper.
#[cfg(windows)]
fn cmd_print_preview(
    input: &Path,
    printer: Option<&str>,
    scale: PrintScaleArg,
    scale_percent: Option<u32>,
    pages: &str,
    orientation: OrientationArg,
) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };

    // Resolve the printer: the named one, else the system default. An
    // unnamed preview on a machine with no default is a real dead end,
    // so it says which of the two problems it is.
    let all = match pdfce_print::list_printers() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::IO_ERROR;
        }
    };
    let chosen = match printer {
        Some(name) => name.to_owned(),
        None => match all.iter().find(|p| p.is_default) {
            Some(p) => p.name.clone(),
            None => {
                eprintln!(
                    "pdfce-cli: no default printer is set — pass --printer with one of the names from `pdfce-cli list-printers`"
                );
                return exit::EDIT_REFUSED;
            }
        },
    };

    let caps = match pdfce_print::printer_caps(&chosen) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::EDIT_REFUSED;
        }
    };

    // Through a session, matching every other page-addressing command:
    // `pages()` lives on `EditSession`, and reading through the same
    // type the editing commands use keeps one page-index space.
    let session = pdfce_core::edit::EditSession::new(doc);
    let page_list = match session.pages() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let indices = match parse_pages(pages, page_list.len()) {
        Ok(i) => i,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::EDIT_REFUSED;
        }
    };

    // ★ The sheet as the DRIVER will present it, not as it was reported.
    //
    // `printer_caps` reads the device's default `DEVMODE`, so a
    // portrait-default printer reports a portrait sheet even for a job
    // that will print landscape. Reporting and placing against that
    // un-turned sheet is what made a landscape page come out at about
    // 77% of correct size, and a preview repeating the same mistake would
    // agree with the wrong print instead of catching it.
    //
    // The orientation page is the FIRST selected page, matching what
    // `print` resolves `auto` from — one `DEVMODE` covers the whole job.
    let page_sizes: Vec<(f64, f64)> = page_list
        .iter()
        .map(|p| {
            let mb = p.media_box;
            ((mb.urx - mb.llx).abs(), (mb.ury - mb.lly).abs())
        })
        .collect();
    let first_page_pt = indices
        .first()
        .and_then(|&i| page_sizes.get(i).copied())
        .unwrap_or(pdfce_print::US_LETTER_PORTRAIT_PT);
    let device =
        pdfce_print::DeviceGeometry::from_caps(&caps, orientation.to_orientation(), first_page_pt);
    let turned = device.default_orientation();

    println!(
        "printer name={:?} dpi={}x{} sheet_pt={:.1}x{:.1} printable_pt={:.1}x{:.1} \
         margin_pt={:.1},{:.1} orientation={}",
        chosen,
        device.dpi.0,
        device.dpi.1,
        device.physical_pt.0,
        device.physical_pt.1,
        device.printable_pt.0,
        device.printable_pt.1,
        device.offset_pt.0,
        device.offset_pt.1,
        match turned {
            pdfce_print::Orientation::Landscape => "landscape",
            pdfce_print::Orientation::Auto | pdfce_print::Orientation::Portrait => "portrait",
        },
    );

    // A percentage wins over the word. Clap has already bounded it to
    // 1..=1000, so the conversion cannot produce a non-positive or
    // non-finite multiplier.
    let mode = match scale_percent {
        Some(pct) => pdfce_print::ScaleMode::Custom(f64::from(pct) / 100.0),
        None => scale.to_mode(),
    };
    let mode_name = match scale_percent {
        Some(pct) => format!("{pct}%"),
        None => scale.name().to_owned(),
    };
    let mut clipped = 0usize;
    for i in &indices {
        let Some(page) = page_list.get(*i) else {
            continue;
        };
        // The MEDIA box is the sheet the page declares. A crop box would
        // be the right input for a viewer, but printing a cropped view
        // and printing the page are different operations, and Reader
        // prints the page.
        let mb = page.media_box;
        let size = ((mb.urx - mb.llx).abs(), (mb.ury - mb.lly).abs());
        let p = pdfce_print::place_page(size, device.printable_pt, mode);
        if p.clipped {
            clipped += 1;
        }
        println!(
            "page {} size_pt={:.1}x{:.1} scale={:.4} offset_pt={:.1},{:.1} clipped={}",
            i + 1,
            size.0,
            size.1,
            p.scale,
            p.offset_x_pt,
            p.offset_y_pt,
            u32::from(p.clipped),
        );
    }

    if clipped > 0 {
        // stderr, and named as a count: this is the fact that should stop
        // a scripted print, and it must not be lost in a stdout capture.
        eprintln!(
            "pdfce-cli: WARNING — {clipped} page(s) would lose content off the edge of the \
             paper at this scale. Acrobat clips these silently; pdfce does not. Try \
             --scale fit or --scale shrink."
        );
    }
    println!(
        "print-preview {} printer={:?} scale={} pages={} clipped={clipped}",
        input.display(),
        chosen,
        mode_name,
        indices.len(),
    );
    // Zero even when pages would clip: the PREVIEW succeeded, and its
    // whole job is to report that fact. A non-zero exit would make "this
    // layout loses content" indistinguishable from "the file would not
    // open", and a caller that wants to branch has `clipped=` on the
    // summary line.
    exit::SUCCESS
}

/// The non-Windows arm — reports rather than vanishing, for the reason
/// given on `cmd_list_printers`.
#[cfg(not(windows))]
fn cmd_print_preview(
    _input: &Path,
    _printer: Option<&str>,
    _scale: PrintScaleArg,
    _scale_percent: Option<u32>,
    _pages: &str,
    _orientation: OrientationArg,
) -> u8 {
    eprintln!(
        "pdfce-cli: printing is available on Windows only in this build \
         (docs/decisions/003-distribution-posture.md §4.1)"
    );
    exit::EDIT_REFUSED
}

/// `list-printers` — what the print spooler can see.
///
/// # Why a whole subcommand for a list
///
/// Every later printing feature needs a printer NAME, and an operator
/// cannot supply one they cannot see. Shipping the query before the
/// action also means the platform binding is exercised and correct
/// before anything can put marks on paper.
///
/// Not built on non-Windows: the subcommand exists in the parser on
/// every platform (so `--help` is honest about what pdfce offers) and
/// reports that it is unavailable rather than being silently missing —
/// a command that vanishes by platform is indistinguishable from a typo.
#[cfg(windows)]
fn cmd_list_printers() -> u8 {
    let printers = match pdfce_print::list_printers() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {err}");
            return exit::IO_ERROR;
        }
    };
    for p in &printers {
        println!(
            "printer name={:?} driver={:?} port={:?} default={}",
            p.name,
            p.driver,
            p.port,
            u32::from(p.is_default),
        );
    }
    // Zero printers is a successful query of a machine with none — not a
    // failure. A non-zero exit would make "no printers installed"
    // indistinguishable from "the spooler is down", which is the one
    // distinction a caller actually needs here.
    println!("list-printers count={}", printers.len());
    exit::SUCCESS
}

/// The non-Windows arm — see the Windows version's docs for why this
/// reports rather than disappears.
#[cfg(not(windows))]
fn cmd_list_printers() -> u8 {
    eprintln!(
        "pdfce-cli: printing is available on Windows only in this build          (docs/decisions/003-distribution-posture.md §4.1)"
    );
    exit::EDIT_REFUSED
}

/// `find-text` — locate every occurrence of a string in the page text.
///
/// # Why the geometry is in the output and not just the page number
///
/// "page 3" is not an answer when a word appears six times on it. Each
/// hit reports its bounding box in unrotated page space, which is what a
/// caller needs to draw a box, crop an image, or hand a coordinate to
/// `mark-redaction`. It is the SAME quad `mark-redaction --search` would
/// cover, because both come from one scan in core — so a script that
/// finds first and redacts second cannot get two different rectangles.
///
/// # Exit code
///
/// `0` whether or not anything matched. Finding nothing is a successful
/// search, not a failure, and a non-zero exit would make "no hits"
/// indistinguishable from "could not read the file" in a shell pipeline.
/// The count is on the summary line for a caller that wants to branch.
fn cmd_find_text(input: &Path, needle: &str, ignore_case: bool) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let mut session = pdfce_core::edit::EditSession::new(doc);
    let hits = session.find_text(needle, ignore_case);

    for h in &hits {
        // Bounds over all FOUR corners rather than reading `ll`/`ur`.
        // Today every quad here comes from `Quad::from_rect` and is
        // axis-aligned, so the two agree — but `Quad` is a general
        // quadrilateral (§12.5.6.10 `/QuadPoints`), and a corner-pair
        // shortcut would silently under-report the box the day a rotated
        // one arrives.
        let xs = [h.quad.ul.0, h.quad.ur.0, h.quad.ll.0, h.quad.lr.0];
        let ys = [h.quad.ul.1, h.quad.ur.1, h.quad.ll.1, h.quad.lr.1];
        let min = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // 1-based page, matching every other page-addressing surface in
        // this CLI. The extraction is 0-based and the operator is not.
        println!(
            "match page={} text={:?} rect={:.2},{:.2},{:.2},{:.2}",
            h.page_index + 1,
            h.text,
            min(xs),
            min(ys),
            max(xs),
            max(ys),
        );
    }
    println!(
        "find-text {} needle={needle:?} ignore_case={} matches={}",
        input.display(),
        u32::from(ignore_case),
        hits.len(),
    );
    exit::SUCCESS
}

/// One line describing a resolved rich-text run style.
///
/// # Why only the SET properties appear
///
/// [`pdfce_core::richtext::Style`] uses `None` for "neither the run nor
/// `/DS` specified this", which is deliberately not the same as "the
/// default". Printing `weight=none` for every plain run would bury the
/// handful of properties that are actually set, and — worse — would read
/// as an assertion about the field that the file does not make. What is
/// absent here is absent from the document.
///
/// `unstyled` rather than an empty string when nothing is set, because a
/// blank tail in a `key=value` line reads as truncated output.
fn describe_style(s: &pdfce_core::richtext::Style) -> String {
    use pdfce_core::richtext::{Align, Stretch};
    let mut parts: Vec<String> = Vec::new();
    if let Some(f) = s.size_pt {
        parts.push(format!("{f}pt"));
    }
    if !s.family.is_empty() {
        parts.push(s.family.join("/"));
    }
    // Reported as the number the spec normalises to, with the familiar
    // keyword alongside for the two values that have one — an operator
    // reading `700` should not have to remember that it means bold.
    if let Some(w) = s.weight {
        parts.push(match w {
            400 => "weight=400(normal)".to_owned(),
            700 => "weight=700(bold)".to_owned(),
            other => format!("weight={other}"),
        });
    }
    if let Some(i) = s.italic {
        parts.push(if i { "italic" } else { "upright" }.to_owned());
    }
    if s.underline == Some(true) {
        parts.push("underline".to_owned());
    }
    if s.strikethrough == Some(true) {
        parts.push("strikethrough".to_owned());
    }
    if let Some([r, g, b]) = s.color {
        // Back to the #rrggbb the file wrote. The model holds DeviceRGB
        // 0.0-1.0 because RT-M12 requires that conversion, but three
        // decimals are not what an operator recognises as "the red one".
        let byte = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
        parts.push(format!("#{:02X}{:02X}{:02X}", byte(r), byte(g), byte(b)));
    }
    if let Some(a) = s.align {
        parts.push(
            match a {
                Align::Left => "align=left",
                Align::Center => "align=center",
                Align::Right => "align=right",
            }
            .to_owned(),
        );
    }
    if let Some(v) = s.baseline_shift_pt {
        // Named by what it MEANS, not by its sign. Table 225's convention
        // is positive-is-superscript, which is the opposite of the
        // intuition a reader brings from CSS's `vertical-align`.
        let kind = if v > 0.0 { "superscript" } else { "subscript" };
        parts.push(format!("{kind}({v:+}pt)"));
    }
    // `Normal` is suppressed rather than printed: it is the width every
    // font already has, so naming it adds a token to every run without
    // distinguishing any of them.
    if let Some(st) = s.stretch.filter(|st| *st != Stretch::Normal) {
        parts.push(format!("stretch={st:?}"));
    }
    if parts.is_empty() {
        "unstyled".to_owned()
    } else {
        parts.join(",")
    }
}

fn cmd_list_fields(input: &Path, fillable_only: bool, rich_text: bool) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let Some(form) = pdfce_core::forms::parse_acroform(&doc) else {
        // No form is not an error — report zero and exit clean, so a batch
        // sweep can tally form-bearing vs form-free files.
        println!("list-fields {} fields=0 no_acroform=1", input.display());
        return exit::SUCCESS;
    };

    let mut fields_with_aa = 0usize;
    let mut shown = 0usize;
    for field in &form.fields {
        if field.has_additional_actions {
            fields_with_aa += 1;
        }
        if fillable_only && !field.is_fillable() {
            continue;
        }
        shown += 1;
        let ty = match field.field_type {
            Some(pdfce_core::forms::FieldType::Button) => "Btn",
            Some(pdfce_core::forms::FieldType::Text) => "Tx",
            Some(pdfce_core::forms::FieldType::Choice) => "Ch",
            Some(pdfce_core::forms::FieldType::Signature) => "Sig",
            None => "none",
        };
        let button = match field.button_kind {
            Some(pdfce_core::forms::ButtonKind::Push) => "push",
            Some(pdfce_core::forms::ButtonKind::Check) => "check",
            Some(pdfce_core::forms::ButtonKind::Radio) => "radio",
            None => "-",
        };
        // ★ QUOTED, NOT WHITESPACE-MANGLED — and this was a real defect.
        //
        // These three columns carry §7.9.2 TEXT STRINGS (`/T`, `/V`, `/MK`
        // `/CA`), and a text string may contain spaces. They used to run
        // through `sanitize_token`, whose doc comment justified itself with
        // *"names cannot legally contain whitespace (§7.3.5 uses `#20` for a
        // space), so this only fires on pathological input."* §7.3.5 governs
        // **name objects** (`/Foo`). It has nothing to say about `/T`.
        //
        // What that cost, measured on a real form (Arizona courts' Health
        // Care Power of Attorney, 2026-08-09): `/T` values of `Home Phone`,
        // `Address 1_3`, `Cell Phone_2` and eleven more printed as
        // `Home_Phone`, `Address_1_3`, `Cell_Phone_2` — and this verb's own
        // help calls its output *"also how `fill-field` and `list-fields`
        // refer to it"*, while `--name` on every write verb says *"as
        // `list-fields` reports it"*. So for every field whose name contains
        // a space — which on Acrobat-authored forms is most of them, because
        // Acrobat derives field names from nearby label text — the
        // documented discovery path emitted a name that `fill-field`,
        // `rename-field`, `delete-field`, `delete-widget` and `move-widget`
        // all reject with "no fillable form field with the fully-qualified
        // name". Five of that form's six broken fields were unreachable.
        //
        // Debug-quoting is not a new convention: `delete-widget`,
        // `rename-field` and `delete-field` already print `name={:?}` in
        // their own result lines. This verb — the DISCOVERY one, the only
        // one whose output is meant to be fed back in — was the odd one out.
        //
        // The bare sentinels stay bare, so `-` (absent) stays distinguishable
        // from `""` (present and empty), which quoting everything would have
        // merged.
        let name = if field.fully_qualified_name.is_empty() {
            "(unnamed)".to_owned()
        } else {
            format!("{:?}", field.fully_qualified_name)
        };
        let value = {
            let v = field.value.display_text();
            if v.is_empty() {
                "-".to_owned()
            } else {
                format!("{v:?}")
            }
        };
        // `/MK` `/CA`, from the first widget that has one. Appended LAST so
        // a parser reading through `aa=` is unaffected.
        //
        // Worth a column of its own because `value=` cannot carry it: a push
        // button has no `/V` in any state (§12.7.4.2.2), so without this
        // every push button in a form lists identically and the only thing
        // telling *Submit* from *Reset* is a string inside an appearance
        // stream. `-` for a field with no caption, which for a non-button is
        // every one of them.
        let caption = field
            .widgets
            .iter()
            .find_map(|w| w.caption.as_deref())
            .map_or_else(
                || "-".to_owned(),
                |c| format!("{:?}", String::from_utf8_lossy(c)),
            );
        // The field's rich text, parsed once and used for both the row's
        // compact token and the optional detail below.
        //
        // Parsed from `/RV` UNGATED by the RichText flag, matching the
        // export path: a file may legally-ish carry `/RV` with bit 26
        // clear, and that is precisely the case where reporting "no
        // formatting" would hide the only copy of it. A parse failure
        // reports as `rich=unparsed` rather than as absent, because "this
        // field has formatting pdfce could not read" and "this field has
        // no formatting" are different facts and only one of them is a
        // reason to stop.
        let runs = field.rich_value.as_ref().map(|rv| {
            let ds = field
                .default_style
                .as_ref()
                .map(|d| String::from_utf8_lossy(d).into_owned());
            String::from_utf8(rv.clone())
                .map_err(|_| "not UTF-8".to_owned())
                .and_then(|s| {
                    pdfce_core::richtext::parse(&s, ds.as_deref()).map_err(|e| e.to_string())
                })
        });
        let rich = match &runs {
            None => "-".to_owned(),
            Some(Ok(r)) => format!("{}runs", r.len()),
            Some(Err(_)) => "unparsed".to_owned(),
        };

        println!(
            "field name={name} type={ty} button={button} flags=0x{:X} value={value} \
widgets={} ap={} fillable={} readonly={} aa={} caption={caption} rich={rich}",
            field.flags.0,
            field.widgets.len(),
            u32::from(field.has_appearance()),
            u32::from(field.is_fillable()),
            u32::from(field.flags.read_only()),
            u32::from(field.has_additional_actions),
        );

        if rich_text {
            match &runs {
                None => {}
                Some(Ok(r)) => {
                    for (i, run) in r.iter().enumerate() {
                        println!(
                            "  run {i} p={} text={:?} style={}",
                            run.paragraph,
                            run.text,
                            describe_style(&run.style),
                        );
                    }
                }
                // Named, not swallowed. A field whose formatting pdfce
                // cannot read is the one an operator most needs told
                // about, since every downstream decision about it is
                // being made blind.
                Some(Err(e)) => println!("  rich text could not be read: {e}"),
            }
        }
    }

    let xfa = match form.xfa {
        pdfce_core::forms::XfaPresence::None => "none".to_owned(),
        pdfce_core::forms::XfaPresence::Stream => "stream".to_owned(),
        pdfce_core::forms::XfaPresence::PacketArray { packets } => format!("packets:{packets}"),
    };
    // Decision 009 posture-A JavaScript disclosure histogram (recognition
    // only — pdfce NEVER executes any of it). Network/launch action counts
    // flag the R12/R13 hazards loudly.
    let js = pdfce_core::forms::scan_javascript(&doc);
    println!(
        "list-fields {} fields={} shown={shown} need_appearances={} sig_flags=0x{:X} \
calc_order={} fields_with_aa={fields_with_aa} xfa={xfa} default_resources={} \
js_calc={} js_format={} js_validate={} js_keystroke={} js_custom={} js_doc_level={} \
open_action_js={} js_network_actions={} js_launch_actions={}",
        input.display(),
        form.fields.len(),
        u32::from(form.need_appearances),
        form.sig_flags,
        form.calc_order_count,
        u32::from(form.has_default_resources),
        js.fields_with_calculate_script,
        js.fields_with_format_script,
        js.fields_with_validate_script,
        js.fields_with_keystroke_script,
        js.custom_scripts,
        js.doc_level_scripts,
        u32::from(js.open_action_is_javascript),
        js.network_action_count,
        js.launch_action_count,
    );
    if js.network_action_count > 0 || js.launch_action_count > 0 {
        eprintln!(
            "pdfce-cli: {}: this form carries {} network and {} process-launch action trigger(s) \
that Adobe Acrobat/Reader would run; pdfce recognizes them but NEVER executes any (R12/R13/R54).",
            input.display(),
            js.network_action_count,
            js.launch_action_count,
        );
    }
    exit::SUCCESS
}

/// `recompute`: natively recompute recognised calculation scripts.
///
/// # Plan first, apply second — and that is not a convenience
///
/// Without `--apply` this writes nothing. Decision 009 §5.1 makes a recompute
/// an operator-invoked act rather than a side effect, and project rule 4
/// requires anything pdfce inferred to be visible before it becomes document
/// state. A recomputed total is an inference: pdfce read a script it did not
/// run and reproduced what it believes the script means.
///
/// On a batch surface the dry run is doing more work than on a screen. A
/// script that pipes `recompute --apply` across a directory has no operator
/// watching; the plan is what makes it possible to look first.
///
/// # Output contract
///
/// One `change` line per field, one `skip` line per recognised calculation
/// left alone, then a summary. All locale-invariant and stable across runs:
///
/// ```text
/// change field="Total" from="0" to="132.5" op=SUM operands=2 coerced=0
/// skip   field="Bad" reason=refused detail="..."
/// recompute <path> changes=1 skipped=1 order=calc_order applied=0
/// ```
///
/// # Exit status
///
/// A dry run with changes pending still exits `SUCCESS`: it did what it was
/// asked and found work. Distinguishing "nothing to do" from "changes
/// pending" is what the `changes=` count is for — an exit code that varied
/// would make `recompute` unusable in a `set -e` script that only wanted the
/// report.
fn cmd_recompute(
    input: &Path,
    apply: bool,
    output: Option<&Path>,
    mode: SaveMode,
    policy: pdfce_core::form_script::calc::CommaPolicy,
    verify_undo: bool,
) -> u8 {
    use pdfce_core::form_script::recompute::{OrderSource, Skip};

    if apply && output.is_none() {
        eprintln!("pdfce-cli: --apply needs --output");
        return exit::EDIT_REFUSED;
    }

    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let plan = {
        let view = session.view();
        pdfce_core::form_script::recompute::plan(&view, policy)
    };

    for change in &plan.changes {
        println!(
            "change field={:?} from={:?} to={:?} op={} operands={} coerced={}",
            change.field,
            change.previous,
            change.proposed,
            change.computation.op.code(),
            change.computation.operands.len(),
            change.computation.coerced_operands(),
        );
    }
    for skipped in &plan.skipped {
        let reason = match skipped.reason {
            Skip::Refused(_) => "refused",
            Skip::CircularDependency => "circular",
            Skip::AlreadyCorrect => "already_correct",
            Skip::NotAValueField => "not_a_value_field",
        };
        println!(
            "skip   field={:?} reason={reason} detail={:?}",
            skipped.field,
            skipped.reason.to_string(),
        );
    }

    let order = match plan.order_source {
        OrderSource::CalculationOrder => "calc_order",
        OrderSource::Mixed => "mixed",
        OrderSource::Derived => "derived",
        OrderSource::Empty => "none",
    };

    // The caveats, on stderr, before any write. Each is a fact that changes
    // how much the numbers should be trusted, and burying them under the
    // summary line would put them after the thing they qualify.
    if plan.order_source.is_pdfce_choice() {
        eprintln!(
            "pdfce-cli: {}: this form has {} calculated field(s) its /CO array does not \
list, which ISO 32000-1 requires it to. The standard gives no recovery rule, so pdfce \
ordered them by their own dependencies — another reader may legitimately compute \
different values.",
            input.display(),
            plan.unlisted_calculations,
        );
    }
    if plan.coerced_operands() > 0 {
        eprintln!(
            "pdfce-cli: {}: {} operand(s) were blank or non-numeric and counted as zero, \
matching Acrobat. The totals are arithmetically correct for a partly-empty form.",
            input.display(),
            plan.coerced_operands(),
        );
    }
    if plan.not_reproducible > 0 {
        eprintln!(
            "pdfce-cli: {}: {} script(s) were NOT considered — pdfce recognises no \
built-in in them, so their fields keep the values last saved. Run list-scripts to see \
which.",
            input.display(),
            plan.not_reproducible,
        );
    }

    if !apply {
        println!(
            "recompute {} changes={} skipped={} order={order} applied=0",
            input.display(),
            plan.changes.len(),
            plan.skipped.len(),
        );
        if !plan.is_empty() {
            eprintln!(
                "pdfce-cli: {}: nothing was written. Re-run with --apply --output FILE to \
store these values. The source scripts stay in the file either way, so a \
JavaScript-running reader still recomputes independently.",
                input.display()
            );
        }
        return exit::SUCCESS;
    }

    for change in &plan.changes {
        if let Err(err) = session.fill_text_field(&change.field, &change.proposed) {
            return report_edit_error(input, &err);
        }
    }

    let Some(output) = output else {
        // Unreachable: guarded at entry. Handled rather than unwrapped so a
        // future edit to the guard cannot turn this into a panic.
        eprintln!("pdfce-cli: --apply needs --output");
        return exit::EDIT_REFUSED;
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    println!(
        "recompute {} changes={} skipped={} order={order} applied={}",
        input.display(),
        plan.changes.len(),
        plan.skipped.len(),
        plan.changes.len(),
    );
    finish_edit(input, &outcome)
}

/// `reset-form`: restore form fields to their defaults (§12.7.5.3).
///
/// # Why this shows the damage before doing it
///
/// A reset DISCARDS what the operator typed, and unlike a fill it does so to
/// many fields at once. `fill-field` writes one named value and the operator
/// can see what they asked for; `reset-form` with no arguments touches
/// everything, and "everything" is exactly the scope where a wrong guess is
/// unrecoverable from the command line.
///
/// So the default lists the fields it would clear and writes nothing. That is
/// the same shape as `recompute`, and for a stronger reason: recompute's
/// mistake is a wrong number, this one's is lost data.
///
/// # Output contract
///
/// One line per field, then a summary, all locale-invariant:
///
/// ```text
/// reset  field="Keep" from="typed" to="factory" source=default
/// reset  field="Drop" from="typed" to=<removed> source=none
/// skip   field="Push" reason=pushbutton
/// reset-form <path> reset=2 defaulted=1 removed=1 skipped=1 applied=0
/// ```
///
/// `to=<removed>` rather than `to=""` on purpose: the clause removes the key,
/// and an operator reading `to=""` would reasonably expect an empty string in
/// the file.
fn cmd_reset_form(
    input: &Path,
    fields: &[String],
    apply: bool,
    output: Option<&Path>,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    if apply && output.is_none() {
        eprintln!("pdfce-cli: --apply needs --output");
        return exit::EDIT_REFUSED;
    }
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let only = (!fields.is_empty()).then_some(fields);

    // The preview comes from the CORE, not from a second copy of the
    // eligibility rule written here. The GUI panel and this dry run were
    // each deriving it independently until `reset_preview` existed, which is
    // two implementations of one rule free to drift — R171's exact shape,
    // and the drift would have shown only as the CLI and the GUI disagreeing
    // about how many fields a reset touches.
    let preview = session.reset_preview(only.map(<[String]>::as_ref));
    if preview.is_empty() {
        eprintln!(
            "pdfce-cli: {}: the document has no interactive form",
            input.display()
        );
        return exit::EDIT_REFUSED;
    }
    let mut clearing = 0usize;
    for row in &preview {
        if let Some(reason) = row.ineligible {
            println!("skip   field={:?} reason={}", row.field, reason.token());
            continue;
        }
        if !row.would_change {
            println!("ok     field={:?} reason=already_default", row.field);
            continue;
        }
        clearing += 1;
        // `<removed>` rather than `""`: the clause removes the KEY, and an
        // operator reading `to=""` would reasonably expect an empty string in
        // the file.
        let to = if row.would_remove {
            "<removed>".to_owned()
        } else {
            format!("{:?}", row.target)
        };
        println!(
            "reset  field={:?} from={:?} to={to} source={}",
            row.field,
            row.current,
            if row.would_remove { "none" } else { "default" },
        );
    }

    if !apply {
        println!(
            "reset-form {} reset={clearing} defaulted={} removed={} skipped={} applied=0",
            input.display(),
            preview
                .iter()
                .filter(|r| r.would_change && !r.would_remove)
                .count(),
            preview
                .iter()
                .filter(|r| r.would_change && r.would_remove)
                .count(),
            preview.iter().filter(|r| r.ineligible.is_some()).count(),
        );
        eprintln!(
            "pdfce-cli: {}: nothing was written. The lines above are what a reset WOULD \
clear. Re-run with --apply --output FILE to perform it.",
            input.display()
        );
        return exit::SUCCESS;
    }

    let out = match session.reset_form(only.map(<[String]>::as_ref)) {
        Ok(out) => out,
        Err(err) => return report_edit_error(input, &err),
    };
    let Some(output) = output else {
        eprintln!("pdfce-cli: --apply needs --output");
        return exit::EDIT_REFUSED;
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    println!(
        "reset-form {} reset={} defaulted={} removed={} skipped={} applied={}",
        input.display(),
        out.fields_reset,
        out.values_defaulted,
        out.values_removed,
        out.skipped_pushbuttons + out.skipped_signatures + out.skipped_read_only,
        out.fields_reset,
    );
    if out.skipped_signatures > 0 {
        eprintln!(
            "pdfce-cli: {}: {} signature field(s) were left alone — a signature's value IS \
the signature, and removing it would destroy it.",
            input.display(),
            out.skipped_signatures,
        );
    }
    finish_edit(input, &outcome)
}

/// `list-scripts`: classify every form-field script (decision 009 posture B).
///
/// # Why this exists as its own subcommand rather than more columns on
/// `list-fields`
///
/// `list-fields` already prints a posture-A *histogram* — how many fields
/// calculate, how many format, how many are custom. That answers "is this
/// form script-driven?" and stops. The question posture B raises is
/// per-field and has four parts: **which** field, on **which trigger**,
/// recognised as **what**, and **can pdfce reproduce it**. Four facts per
/// script do not fit on a summary line, and folding them in would make the
/// summary line unstable in width — the property that makes it greppable.
///
/// # Output contract
///
/// One line per script, fields space-separated `key=value`, locale-invariant
/// and stable across runs:
///
/// ```text
/// script field=Total trigger=calculate helper=AFSimple_Calculate reproducible=1 source=string bytes=38
/// ```
///
/// Then one summary line with the histogram. A form with no scripts prints
/// the summary with a zero count rather than nothing at all — silence would
/// be indistinguishable from a failed read, and "this form has no scripts"
/// is a positive finding worth stating (R162's shape: an absence claim is
/// only meaningful once the reader has shown it can find the thing).
///
/// # The non-execution disclaimer is unconditional
///
/// Printed to stderr whenever any script exists, whether or not pdfce
/// recognised any of them. An operator reading a list of recognised Acrobat
/// built-ins is at their most likely to assume the values on the page are
/// live, and that is exactly the moment to say they are not.
fn cmd_list_scripts(input: &Path, reproducible_only: bool) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let view = pdfce_core::view::DocumentView::new(&doc, doc.bytes(), doc.version());
    let inv = pdfce_core::form_script::inventory::inventory(&view);

    let mut shown = 0usize;
    for s in &inv.scripts {
        if reproducible_only && !s.is_reproducible() {
            continue;
        }
        shown += 1;
        let source = match s.source {
            pdfce_core::form_script::inventory::ScriptSource::LiteralString => "string",
            pdfce_core::form_script::inventory::ScriptSource::Stream => "stream",
            pdfce_core::form_script::inventory::ScriptSource::Unreadable => "unreadable",
        };
        // The field name is quoted because a fully-qualified name may
        // contain spaces (`/T` is a text string, not a name object), and an
        // unquoted one would silently break the key=value parse this line
        // promises.
        println!(
            "script field={:?} trigger={} helper={} reproducible={} source={source} bytes={}",
            s.field,
            s.trigger.token(),
            s.class.token(),
            u32::from(s.is_reproducible()),
            s.length,
        );
    }

    let histogram = inv
        .histogram()
        .iter()
        .map(|(token, n)| format!("{token}={n}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "list-scripts {} scripts={} shown={shown} reproducible={} {histogram}",
        input.display(),
        inv.scripts.len(),
        inv.reproducible().count(),
    );

    if !inv.scripts.is_empty() {
        eprintln!(
            "pdfce-cli: {}: this form carries {} script(s) that Adobe Acrobat/Reader would \
run. pdfce NEVER executes any of them (R53/R54). A recognised built-in is read, not run; \
its stored value is shown as last saved and may be stale until you recompute it.",
            input.display(),
            inv.scripts.len(),
        );
    }
    exit::SUCCESS
}

/// `fill-field`: set form-field values and save (Pass 7).
///
/// Each `NAME=VALUE` is dispatched by the field's modelled type: text/choice
/// through [`EditSession::fill_text_field`], check-box/radio through
/// [`EditSession::set_button_state`]. All assignments land in one session
/// (so the save carries them as one incremental revision), then the shared
/// [`save_edited`]/[`finish_edit`] plumbing writes and reports.
///
/// # `downgrade_rich_text`, and why the CLI needed it
///
/// Until this flag existed, a rich-text field was **unfillable from the
/// CLI at all**: this function called [`EditSession::fill_text_field`],
/// which refuses one with [`EditError::FieldIsRichText`], and never
/// exposed [`EditSession::fill_text_field_downgrading_rich_text`]. The GUI
/// had shipped the disclosed downgrade; the CLI had no route to it, and
/// `docs/FEATURES.md` asserted the exact opposite of both facts until
/// `aac321c`.
///
/// The flag is **opt-in and lossy**, which is the whole design. Making it
/// the default would silently discard `/RV` formatting on a plain
/// `fill-field` — the "sneaky" half of rule 4, on a batch surface where
/// nobody is watching a screen. Refusing without any escape leaves a real
/// document permanently unfillable. An explicit flag is the only option
/// that is neither.
///
/// Disclosure is **per field, by name, on stderr** — not a count. A count
/// tells the operator that something lost its formatting; it does not tell
/// them WHICH, and on a scripted run that is the only question worth
/// answering. This is the same reasoning as `R181`, arrived at from the
/// other direction: there, a count described the wrong thing; here, a count
/// would be the wrong SHAPE for the thing.
///
/// # Loop safety (`R179`)
///
/// The assignment loop mutates and returns early on error, which is
/// `R179`'s shape. It is safe here because the early return happens
/// **before** [`save_edited`] — the partially-mutated session is dropped
/// and the output file is never written, so a failed run leaves no partial
/// fill anywhere an operator can observe. Atomicity by not saving, not by
/// rollback.
fn cmd_fill_field(
    input: &Path,
    sets: &[String],
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
    downgrade_rich_text: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // Model the form once up front to know each field's type. The session is
    // still at the base revision here, so the model is the file as loaded;
    // each fill re-reads through the overlay, so later fills see earlier ones.
    let Some(form) = pdfce_core::forms::parse_acroform(&session.graph()) else {
        eprintln!(
            "pdfce-cli: {}: the document has no interactive form",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };

    let mut applied = 0usize;
    for set in sets {
        let Some((name, value)) = set.split_once('=') else {
            eprintln!("pdfce-cli: --set must be NAME=VALUE, got {set:?}");
            return exit::EDIT_REFUSED;
        };
        // Look up the field's type from the model to choose the fill path.
        use pdfce_core::forms::FieldType;
        let field_type = form.field_by_name(name).and_then(|f| f.field_type);
        let result = match field_type {
            Some(FieldType::Button) => {
                // Convenience aliases for a checkbox's single on-state.
                let state = match value.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" | "checked" => resolve_on_state(&form, name),
                    "off" | "false" | "0" | "" | "unchecked" => "Off".to_owned(),
                    _ => value.to_owned(),
                };
                session.set_button_state(name, &state)
            }
            Some(FieldType::Choice) => {
                // A choice value may name several selections for a
                // multi-select field, `|`-separated (`Red|Blue`).
                let sels: Vec<&str> = value.split('|').collect();
                session
                    .set_choice_value(name, &sels)
                    .map(|out| disclose_fill(name, &out))
            }
            // Text, and the `None`/unmodelled fallback.
            //
            // The lossy verb is taken ONLY for a field the model says is
            // actually rich text, never merely because the flag is set.
            // Routing every text field through it would be wrong twice
            // over: `--downgrade-rich-text` must not change the outcome
            // for a field that has no formatting to lose, and the note
            // below would then have to guess whether anything happened.
            // Asking `is_rich_text()` — which resolves `/FT` first, so it
            // cannot mistake a radio group's bit 26 for RichText
            // (`587e520`) — makes both exact.
            //
            // Without the flag this falls through to `fill_text_field`,
            // which refuses a rich-text field. That refusal is the
            // default and stays the default.
            _ if downgrade_rich_text
                && form
                    .field_by_name(name)
                    .is_some_and(pdfce_core::forms::Field::is_rich_text) =>
            {
                // Announced BEFORE the write, so an operator watching a
                // batch run sees which field is about to lose formatting
                // even if a later assignment aborts the whole run. stderr,
                // so a script capturing stdout still shows a human.
                eprintln!(
                    "pdfce-cli: {name}: rich-text formatting discarded \
                     (--downgrade-rich-text) — /RV removed, RichText flag \
                     cleared"
                );
                session
                    .fill_text_field_downgrading_rich_text(name, value)
                    .map(|out| disclose_fill(name, &out))
            }
            _ => session
                .fill_text_field(name, value)
                .map(|out| disclose_fill(name, &out)),
        };
        if let Err(err) = result {
            return report_edit_error(input, &err);
        }
        applied += 1;
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };

    let r = &outcome.report;
    println!(
        "fill-field {} sets={applied} mode={} -> {}; changed={} objects={} verbatim={} \
reserialized={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// Print the disclosures a vector surgery owes, to stderr.
///
/// Stderr, not stdout, because each of these commands prints ONE fixed-shape
/// record line that scripts parse; interleaving a variable-length prose block
/// into that stream would break them. The operator still sees it on a
/// terminal, where both streams land together.
///
/// These say the surgery had to change the *form* of an operator to do what
/// was asked — expand a rectangle whose corner was dragged out of square,
/// write the `m` an implicitly-started subpath never had. The drawing is
/// unchanged; the bytes are not recoverable by reversing the gesture, and
/// rule 4 forbids leaving the operator to discover that from a diff.
fn report_disclosures(disclosures: &[String]) {
    for d in disclosures {
        eprintln!("pdfce-cli: {d}");
    }
}

/// Print the fuzzy-never-sneaky disclosures a fill owes (an applied
/// auto-size, any unencodable characters) to stderr.
fn disclose_fill(name: &str, out: &pdfce_core::edit::FillOutcome) {
    // Stated FIRST, before the cosmetic caveats. The others describe how the
    // value was drawn; this one says the value may not be the one a reader
    // sees at all, which is a different order of consequence.
    if out.xfa_may_disagree {
        eprintln!(
            "pdfce-cli: field {name:?}: this form also carries an XFA packet. pdfce filled the \
AcroForm half, which most viewers read, but cannot write the XFA half — so an XFA-aware viewer \
may still show the OLD value."
        );
    }
    if let Some(sz) = out.applied_autosize {
        eprintln!(
            "pdfce-cli: field {name:?}: auto-sized to {sz} pt (a reviewable pdfce heuristic; \
§12.7.3.3 mandates no formula)"
        );
    }
    if out.unencodable_chars > 0 {
        eprintln!(
            "pdfce-cli: field {name:?}: {} character(s) had no WinAnsi code and were substituted \
with '?' (Base-14 Latin only)",
            out.unencodable_chars
        );
    }
    if let Some(ti) = out.top_index {
        eprintln!(
            "pdfce-cli: field {name:?}: this list box was SCROLLED to option {ti} (/TI) so the \
selection is on screen — the selected option sits below the first visible window at this \
field's size. pdfce derived the position; nothing about the field's value changed."
        );
    }
}

/// `regenerate-appearances`: rebuild widget appearances and clear
/// /NeedAppearances (Pass 7.1, R51).
fn cmd_regenerate_appearances(input: &Path, output: &Path, mode: SaveMode) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let outcome = match session.regenerate_appearances() {
        Ok(o) => o,
        Err(err) => return report_edit_error(input, &err),
    };
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        false,
    ) {
        Ok(o) => o,
        Err(code) => return code,
    };
    println!(
        "regenerate-appearances {} regenerated={} need_appearances_cleared={} mode={} -> {}; \
objects={} out_bytes={}",
        input.display(),
        outcome.regenerated,
        u32::from(outcome.need_appearances_cleared),
        mode.name(),
        output.display(),
        saved.report.objects_written,
        saved.report.bytes_written,
    );
    finish_edit(input, &saved)
}

/// `flatten`: burn form fields into page content and remove them (Pass 7.1,
/// R48). `--full-rewrite` maps to a single-revision full save that removes
/// even the prior-revision-recoverable pre-flatten data.
fn cmd_flatten(input: &Path, fields: &[String], output: &Path, full_rewrite: bool) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let names: Option<Vec<&str>> = if fields.is_empty() {
        None
    } else {
        Some(fields.iter().map(String::as_str).collect())
    };
    let outcome = match session.flatten_fields(names.as_deref()) {
        Ok(o) => o,
        Err(err) => return report_edit_error(input, &err),
    };
    let mode = if full_rewrite {
        SaveMode::Full
    } else {
        SaveMode::Incremental
    };
    if !full_rewrite {
        eprintln!(
            "pdfce-cli: {}: flatten saved incrementally — the pre-flatten field values remain \
recoverable in the prior revision. Re-run with --full-rewrite to remove them physically (R48).",
            input.display()
        );
    }
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        false,
    ) {
        Ok(o) => o,
        Err(code) => return code,
    };
    println!(
        "flatten {} fields_flattened={} widgets_burned={} pages_touched={} mode={} -> {}; \
objects={} out_bytes={}",
        input.display(),
        outcome.fields_flattened,
        outcome.widgets_burned,
        outcome.pages_touched,
        mode.name(),
        output.display(),
        saved.report.objects_written,
        saved.report.bytes_written,
    );
    finish_edit(input, &saved)
}

/// `export-data`: write a filled form's field data to FDF or XFDF (Pass 7.1).
fn cmd_export_data(input: &Path, output: &Path, format: DataFormat) -> u8 {
    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let Some(data) = session.export_form_data() else {
        eprintln!(
            "pdfce-cli: {}: the document has no interactive form",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };
    let src_hint = input.to_string_lossy();
    let bytes = match format {
        DataFormat::Fdf => data.to_fdf(Some(&src_hint)),
        DataFormat::Xfdf => data.to_xfdf(Some(&src_hint)),
        DataFormat::Csv => {
            let export = pdfce_core::formcsv::to_csv(&data);
            // Reported BEFORE the success line, because it describes a
            // difference between the CSV and the PDF that an operator
            // comparing the two would otherwise have to explain to
            // themselves.
            if let Some(message) = export.message() {
                eprintln!("pdfce-cli: {}: {message}", input.display());
            }
            export.csv
        }
    };
    // Rich-text disclosure, on stderr in prose like every other one this
    // binary emits. Counted from the data itself rather than re-derived from
    // the form, so it describes the FILE that was written.
    //
    // Note what it does NOT say. Until Pass 37.3's first slice this export
    // dropped the formatting entirely and the GUI warned about that; the
    // warning is now false there and has been corrected. The CLI never had
    // one at all, which is its own gap — the two shells must not develop
    // different accounts of the same behaviour.
    let rich = data
        .fields
        .iter()
        .filter(|f| f.rich_value.is_some())
        .count();
    if rich > 0 {
        eprintln!(
            "pdfce-cli: {}: {rich} field(s) hold formatted (rich) text, and the formatting IS in the data file. pdfce cannot yet apply it on import, though — another reader can, but a round trip back through pdfce will not restore it.",
            input.display()
        );
    }
    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }
    let fmt = match format {
        DataFormat::Fdf => "fdf",
        DataFormat::Xfdf => "xfdf",
        DataFormat::Csv => "csv",
    };
    println!(
        "export-data {} fields={} format={fmt} -> {}; out_bytes={}",
        input.display(),
        data.fields.len(),
        output.display(),
        bytes.len(),
    );
    exit::SUCCESS
}

/// `import-data`: set field values from an FDF/XFDF file and save (Pass 7.1).
/// The format is detected from the data file's content.
fn cmd_import_data(input: &Path, data_path: &Path, output: &Path, mode: SaveMode) -> u8 {
    let data_bytes = match std::fs::read(data_path) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", data_path.display());
            return exit::IO_ERROR;
        }
    };
    // Detect the format by CONTENT rather than by extension: a file named
    // `.txt` that is plainly an XFDF should still import, and an operator
    // who renamed one should not have to know that renaming mattered.
    //
    // The three tests are ordered by how specific their marker is. FDF
    // carries a `%FDF` header, XFDF opens with `<`, and CSV is the residue —
    // which is right, because CSV has no marker of its own and anything that
    // is neither of the other two is at least worth *trying* to read as two
    // columns before giving up.
    let first = data_bytes
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .copied();
    let looks_pdfish = data_bytes.starts_with(b"%FDF") || first == Some(b'%');
    let parsed = if first == Some(b'<') {
        pdfce_core::fdf::FormData::parse_xfdf(&data_bytes).map_err(|e| e.to_string())
    } else if looks_pdfish {
        pdfce_core::fdf::FormData::parse_fdf(&data_bytes).map_err(|e| e.to_string())
    } else {
        pdfce_core::formcsv::parse_csv(&data_bytes).map_err(|e| e.to_string())
    };
    let data = match parsed {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", data_path.display());
            return exit::EDIT_REFUSED;
        }
    };

    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    // Counted BEFORE the import, off the live form, because a rich-text
    // field is skipped and so leaves no trace in the outcome to count after.
    let rich_targets = pdfce_core::forms::parse_acroform(&session.graph()).map_or(0, |form| {
        data.fields
            .iter()
            .filter(|e| {
                form.field_by_name(&e.name)
                    .is_some_and(pdfce_core::forms::Field::is_rich_text)
            })
            .count()
    });
    let outcome = match session.import_form_data(&data) {
        Ok(o) => o,
        Err(err) => return report_edit_error(input, &err),
    };
    // WHY a field was skipped, not just that it was. `skipped=1` on the
    // result line is a number an operator cannot act on; this is the
    // sentence that tells them the field still holds what it held, and that
    // pdfce declined on purpose rather than failed.
    if rich_targets > 0 {
        eprintln!(
            "pdfce-cli: {}: {rich_targets} rich-text field(s) were left untouched — not even their plain value was applied. Writing plain text beside a field's existing formatting makes conforming readers display the OLD text (ISO 32000-1 §12.7.3.3), so pdfce leaves such a field alone rather than corrupt what it shows.",
            input.display()
        );
    }
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        false,
    ) {
        Ok(o) => o,
        Err(code) => return code,
    };
    println!(
        "import-data {} applied={} skipped={} mode={} -> {}; objects={} out_bytes={}",
        input.display(),
        outcome.applied,
        outcome.skipped,
        mode.name(),
        output.display(),
        saved.report.objects_written,
        saved.report.bytes_written,
    );
    finish_edit(input, &saved)
}

/// The on-state name a check-box/radio convenience alias (`on`/`true`/…)
/// selects: the field's first widget's first non-`Off` on-state, or `Yes`
/// (the §12.7.4.2.3 convention) when none is discoverable.
fn resolve_on_state(form: &pdfce_core::forms::AcroForm, name: &str) -> String {
    form.field_by_name(name)
        .and_then(|f| f.widgets.iter().find_map(|w| w.on_states.first()))
        .map_or_else(
            || "Yes".to_owned(),
            |s| String::from_utf8_lossy(s).into_owned(),
        )
}

/// Classify one modelled annotation for `list-annotations`, returning
/// `(disposition, ap_shape)` in the render path's precedence order (Popup
/// → suppressed → appearance selection). Model-level only — the degenerate
/// -placement refusal needs the appearance stream's geometry and is a
/// `render-page` concern.
fn classify_for_listing(annot: &pdfce_core::annot::Annotation) -> (&'static str, &'static str) {
    use pdfce_core::annot::Appearance;
    if annot.is_popup {
        return ("popup", "none");
    }
    if annot.flags.suppressed_on_screen() {
        // Still report the appearance shape it *would* have, so a hidden
        // annotation's nature is disclosed (R50).
        let shape = match annot.appearance {
            Appearance::Normal { .. } => "stream",
            Appearance::StateUnresolved => "state-dict",
            Appearance::None => "none",
        };
        return ("suppressed", shape);
    }
    match annot.appearance {
        Appearance::Normal { .. } => {
            if annot.rect.is_some() {
                ("paint-ready", "stream")
            } else {
                ("no-rect", "stream")
            }
        }
        Appearance::None => ("no-ap", "none"),
        Appearance::StateUnresolved => ("state-missing", "state-dict"),
    }
}

/// Replace ASCII whitespace in a token with `_` so a stable stdout line
/// stays splittable on spaces.
///
/// # ⚠ ONLY for tokens that are genuinely PDF NAME OBJECTS (§7.3.5)
///
/// Annotation subtypes (`/Widget`, `/Redact`) are name objects, and §7.3.5
/// writes a space in one as `#20`, so a decoded subtype containing raw
/// whitespace really is pathological and mangling it costs nothing.
///
/// **It must never be applied to a §7.9.2 TEXT STRING.** This function's
/// doc comment used to assert the §7.3.5 rule as though it covered every
/// caller, and on that reasoning it was applied to a form field's `/T`, its
/// `/V` and a widget's `/MK` `/CA` — none of which are name objects and all
/// of which may legally contain spaces.
///
/// The cost, measured on a real government form (2026-08-09): `list-fields`
/// printed `Home_Phone` for a field whose `/T` is `Home Phone`, while every
/// write verb's `--name` documents itself as taking the name *"as
/// `list-fields` reports it"*. The discovery path emitted names the write
/// path rejected, for the majority of fields on any Acrobat-authored form —
/// Acrobat derives field names from nearby label text, so spaces are the
/// norm rather than the exception. Those columns are now debug-quoted; see
/// the comment at their construction.
fn sanitize_token(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// Map a [`PdfError`] to the CLI's documented exit code. The wildcard arm
/// exists because `PdfError` is `#[non_exhaustive]` (later Passes add
/// variants) — unmapped future errors report the generic runtime code
/// rather than failing to compile.
fn exit_code_for(err: &PdfError) -> u8 {
    match err {
        PdfError::Io(_) => exit::IO_ERROR,
        PdfError::MissingHeader { .. } | PdfError::MalformedVersion { .. } => exit::NOT_A_PDF,
        _ => exit::RUNTIME_ERROR,
    }
}

/// Map a [`DocError`] (full-document load) to the CLI's exit code.
///
/// Only two cases are more specific than "something went wrong": the file
/// was unreadable ([`exit::IO_ERROR`]) and the file is not a PDF at all
/// ([`exit::NOT_A_PDF`], delegated to [`exit_code_for`] since the header
/// probe's own error type carries that distinction).
///
/// Everything else — a broken cross-reference chain, an object that does
/// not match its xref entry, a missing `/Root`, and the deliberate
/// xref-stream / hybrid-reference *"not yet supported"* refusals — is
/// [`exit::RUNTIME_ERROR`]. That last group is worth naming: those files
/// are perfectly valid PDFs that this build declines to open, and the
/// distinction is currently carried by the **stderr message**, not by the
/// exit code. If a script ever needs to branch on "unsupported structure"
/// versus "corrupt file", that earns a new, dedicated exit code rather
/// than a broadened meaning for an existing one.
fn exit_code_for_doc(err: &pdfce_core::document::DocError) -> u8 {
    use pdfce_core::document::DocError;
    match err {
        DocError::Io(_) => exit::IO_ERROR,
        DocError::Header(inner) => exit_code_for(inner),
        _ => exit::RUNTIME_ERROR,
    }
}

/// Implement `pdfce-cli round-trip`: save, then verify the
/// `ARCHITECTURE.md` §5 invariant that the chosen mode promises.
///
/// ## The three checks, and why they are three
///
/// Decision 007 W1/R32 names conflating these *"the single likeliest
/// source of a false green or a false red"*. Each maps to its own exit
/// code so a corpus sweep can tally them apart:
///
/// 1. **Byte identity** — whole-file for `--mode incremental` (which
///    promises it), *prefix* identity for `--mode append-identity`
///    (§7.5.6: prior bytes are left intact), and **per object
///    definition** for `--mode full`, where offsets legitimately move
///    and a whole-file comparison would fail universally.
/// 2. **Reload** — `pdfce-core` must be able to parse what it just
///    wrote. A writer that emits an unloadable file is worse than one
///    that emits a differing file.
/// 3. **Raster** — page 1 must re-render to identical pixels. This is
///    the *semantic* oracle: byte identity is a syntactic claim, and a
///    file can satisfy it while meaning something different. It is a
///    **self**-comparison (pdfce-before vs pdfce-after), which needs no
///    reference renderer — deliberately NOT the outstanding
///    pdfce-vs-pdfium pixel-parity harness, which remains owed.
///
/// A refused save ([`exit::SAVE_REFUSED`]) is a fourth, separate
/// outcome: pdfce declining a hybrid full rewrite by name is correct
/// behaviour, and a corpus run that counted it as a failure would be
/// reporting a lie.
fn cmd_round_trip(
    input: &Path,
    mode: RoundTripMode,
    output: Option<&Path>,
    producer: ProducerArg,
    scale: f32,
    compare_raster: bool,
) -> u8 {
    use pdfce_core::document::Document;
    use pdfce_core::writer::{DirtySet, ProducerPolicy, SaveOptions};

    let source = match std::fs::read(input) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source.clone()) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    // Decision 013: if this file opened only via cross-reference recovery,
    // disclose it (R20). A recovered document also refuses incremental save
    // by name (`RoundTripMode::Incremental`/`AppendIdentity` → SAVE_REFUSED)
    // and its `save_full` emits a fresh valid classic xref — the demo path.
    if let Some(report) = doc.recovery() {
        disclose_recovery(input, report);
    }

    let options = SaveOptions::default().with_producer(match producer {
        ProducerArg::Set => ProducerPolicy::Set,
        ProducerArg::Preserve => ProducerPolicy::Preserve,
    });

    // The whole `round-trip` subcommand is an INSTRUMENT, not an editing
    // feature. Every arm below passes `DirtySet::empty()` or
    // `identity_reemission` — it mutates nothing, and exists to prove the
    // writer reproduces a file byte-for-byte (the content-identity harness).
    // Routing it through `EditSession` would mean opening a session to make no
    // edit, and would measure the session rather than the writer.
    //
    // bypass-exempt: identity re-emission only, mutates nothing (see above)
    let saved = match mode {
        RoundTripMode::Incremental => {
            pdfce_core::writer::save_incremental(&doc, &DirtySet::empty(), &options)
        }
        RoundTripMode::Full => pdfce_core::writer::save_full(&doc, &DirtySet::empty(), &options),
        RoundTripMode::AppendIdentity => {
            // Every object of the base revision, re-emitted unchanged.
            let ids: Vec<_> = doc.objects().map(|io| io.id).collect();
            // bypass-exempt: round-trip instrument — the `DirtySet` below is
            // `identity_reemission`, which re-emits every base object
            // unchanged and mutates nothing. This proves the writer reproduces
            // a file byte-for-byte; it is an instrument, not an edit.
            pdfce_core::writer::save_incremental(
                // bypass-exempt: round-trip instrument, identity re-emission
                &doc,
                &DirtySet::identity_reemission(ids),
                &options,
            )
        }
    };
    let (bytes, report) = match saved {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("pdfce-cli: {}: save refused: {err}", input.display());
            return exit::SAVE_REFUSED;
        }
    };

    if let Some(path) = output
        && let Err(err) = std::fs::write(path, &bytes)
    {
        eprintln!("pdfce-cli: {}: {err}", path.display());
        return exit::IO_ERROR;
    }

    // Check 2 is evaluated BEFORE check 1, because a full rewrite's
    // byte-identity test needs the RELOADED document — see
    // `full_rewrite_is_per_object_verbatim` for why comparing through
    // the reload is both linear and a strictly stronger claim.
    //
    // --- check 2: reload, and the object graph survived ---------------
    //
    // Compared ACROSS buffers: `Object`'s derived `PartialEq` compares a
    // stream's `ByteSpan`, which a save legitimately relocates, so a
    // span-sensitive comparison reports a phantom change on every
    // stream-bearing file. See `object::equivalent_across_buffers`.
    //
    // Two objects are excluded, and both exclusions are principled
    // rather than convenient:
    //
    //  - the base file's own cross-reference-stream object, which the
    //    new section supersedes (`is_section_object`);
    //  - under `--producer set`, the document information dictionary,
    //    which the policy rewrote ON PURPOSE. Reporting an intentional
    //    metadata change as a round-trip regression would be a false
    //    red, and would make the one flag R41 requires unusable.
    let deliberately_rewritten = match producer {
        ProducerArg::Set => doc
            .trailer()
            .get(b"Info")
            .and_then(pdfce_core::object::Object::as_reference),
        ProducerArg::Preserve => None,
    };
    let reloaded = open_document_bytes(bytes.clone());
    let reload_ok = match &reloaded {
        Ok(back) => doc.objects().all(|io| {
            is_section_object(&doc, io.id)
                || Some(io.id) == deliberately_rewritten
                || back.get(io.id).is_some_and(|b| {
                    pdfce_core::object::equivalent_across_buffers(
                        &b.value,
                        back.bytes(),
                        &io.value,
                        doc.bytes(),
                    )
                })
        }),
        Err(_) => false,
    };

    // --- check 1: byte identity, in the shape this mode promises -----
    let (identical, identity_note) = match mode {
        RoundTripMode::Incremental => (
            bytes == source,
            "whole-file byte identity (empty dirty set)",
        ),
        RoundTripMode::AppendIdentity => (
            // The one permitted insertion is a separating EOL when the
            // base file's final byte is not one (§7.2.3's
            // comment-runs-to-end-of-line rule).
            bytes.starts_with(&source)
                || (bytes.get(..source.len()) == Some(&source[..])
                    && matches!(bytes.get(source.len()), Some(b'\n'))),
            "prior bytes unchanged (§7.5.6 append)",
        ),
        RoundTripMode::Full => (
            reloaded.as_ref().is_ok_and(|back| {
                full_rewrite_is_per_object_verbatim(&doc, back, deliberately_rewritten)
            }),
            "per-object-definition byte identity",
        ),
    };

    // --- check 2b: the saved file is still USABLE ---------------------
    //
    // `reload_ok` above asks "did every object I HAVE survive?" — a
    // survivorship test over the model. It cannot see a saved file that
    // REFERENCES something absent, because §7.3.10 makes a dangling
    // reference resolve to null rather than an error, so the model reads
    // clean while the file is broken.
    //
    // That blindness shipped. On 2026-08-07 the veraPDF parse gate found
    // pdfce writing a catalog that said `/Pages 2 0 R` with object 2
    // absent, and `round-trip --mode full` reported SUCCESS on it — the
    // verb whose entire job is verifying the save invariant was the thing
    // that missed it. Fixing only the dropped object would have left this
    // check just as blind to the next one (R163: strengthen the gate, do
    // not write a note asking future authors to look harder).
    //
    // The criterion is COMPARATIVE, and that is deliberate. Asking "does
    // the saved file have a page tree?" would fail every round-trip over
    // a legitimately broken corpus file, where faithfully preserving the
    // damage is correct behaviour. What must never happen is a save that
    // DESTROYS a page tree the source had. §7.7.2 Table 28 makes /Pages
    // required, so a document that loses it is one no conforming reader
    // can open.
    let resolves_page_tree = |d: &Document| -> bool {
        d.catalog()
            .ok()
            .and_then(|c| c.get(b"Pages").cloned())
            .is_some_and(|p| d.resolve(&p).as_dict().is_some())
    };
    // Three outcomes, not two — the same shape `tools/verapdf-parse-gate.py`
    // settled on, and for the same reason. A save that DESTROYS a page
    // tree fails. A save that faithfully preserves an already-missing one
    // is not a failure, but it must not be SILENT either: the saved file
    // still names a /Pages object it does not contain, and an operator who
    // sees "round-trip ... identical=1" and nothing else will reasonably
    // conclude the output is usable. It is not.
    //
    // Stated plainly because it is the honest limit of this check: the
    // 2026-08-07 `bad6.pdf` defect lands in the PRESERVED bucket, not the
    // destroyed one, because pdfce could not resolve the page tree on the
    // input either. So check 2b alone would not have caught it — the NOTE
    // below is the part that would have, by making the broken output
    // visible instead of letting a clean-looking result line speak for it.
    let page_tree_before = resolves_page_tree(&doc);
    let page_tree_after = reloaded.as_ref().is_ok_and(resolves_page_tree);
    let page_tree_kept = !page_tree_before || page_tree_after;
    if !page_tree_after && reloaded.is_ok() {
        eprintln!(
            "pdfce-cli: {}: NOTE: the saved file's catalog does not resolve to a page \
tree{}. \u{a7}7.7.2 Table 28 requires /Pages, so this output is not openable by a \
conforming reader.",
            input.display(),
            if page_tree_before {
                " — and the SOURCE resolved one, so the save destroyed it"
            } else {
                " — the source did not resolve one either, so this is preserved \
damage rather than new damage"
            }
        );
    }

    // --- check 3: raster self-comparison ------------------------------
    let mut raster_compared = 0u32;
    let mut raster_identical = 0u32;
    if compare_raster
        && let Ok(after) = &reloaded
        && let (Some(before_px), Some(after_px)) = (
            render_first_page(&doc, scale),
            render_first_page(after, scale),
        )
    {
        raster_compared = 1;
        raster_identical = u32::from(before_px == after_px);
    }

    if report.delinearized {
        // "Fuzzy, never sneaky": Annex F.1 makes de-linearization on
        // append normative and unavoidable, but it is still a property
        // the operator did not ask to spend.
        eprintln!(
            "pdfce-cli: {}: this file is linearized (Fast Web View); saving \
invalidates that. Per ISO 32000-1 Annex F.1 the result 'shall be treated as \
ordinary PDF'. pdfce does not repair or re-linearize, and does not strip the \
/Linearized dictionary.",
            input.display()
        );
    }

    // The stable stdout line: narrative half, then `key=<integer>` pairs
    // in fixed order (module header, "stdout result-line format").
    println!(
        "round-trip {} mode={} -> {}; \
identical={} in_bytes={} out_bytes={} appended={} objects={} verbatim={} \
reserialized={} reloaded={} raster_compared={} raster_identical={} delinearized={} \
promoted={} page_tree_kept={}",
        input.display(),
        mode_name(mode),
        output.map_or_else(|| "<memory>".to_owned(), |p| p.display().to_string()),
        u32::from(identical),
        source.len(),
        report.bytes_written,
        report.bytes_appended,
        report.objects_written,
        report.objects_verbatim,
        report.objects_reserialized,
        u32::from(reload_ok),
        raster_compared,
        raster_identical,
        u32::from(report.delinearized),
        // Appended, never inserted: keys are added at the END so a
        // parser that reads by name keeps working (module docs). Only
        // ever non-zero in `append-identity` mode, where re-emitting a
        // compressed object necessarily promotes it out of its
        // container.
        report.promoted.len(),
        // Appended at the END for the same reason `promoted` was.
        u32::from(page_tree_kept),
    );

    // Ordered worst-first: an unloadable file is a more serious result
    // than a differing one, and a differing one than a raster mismatch
    // — a script branching on the code gets the most severe finding.
    match &reloaded {
        Err(err) => {
            eprintln!(
                "pdfce-cli: {}: the saved file did not reload: {err}",
                input.display()
            );
            return exit::RELOAD_FAILED;
        }
        Ok(_) if !reload_ok => {
            eprintln!(
                "pdfce-cli: {}: the saved file reloaded, but its object graph changed.",
                input.display()
            );
            return exit::RELOAD_FAILED;
        }
        Ok(_) if !page_tree_kept => {
            eprintln!(
                "pdfce-cli: {}: the source resolved a page tree and the saved file does \
not — the catalog names a /Pages object the file does not contain (ISO 32000-1 \
\u{a7}7.7.2 Table 28 requires it). No conforming reader can open the result.",
                input.display()
            );
            return exit::RELOAD_FAILED;
        }
        Ok(_) => {}
    }
    if !identical {
        eprintln!(
            "pdfce-cli: {}: {identity_note} FAILED — this is an \
ARCHITECTURE.md §5 round-trip violation.",
            input.display()
        );
        return exit::NOT_BYTE_IDENTICAL;
    }
    if raster_compared == 1 && raster_identical == 0 {
        eprintln!(
            "pdfce-cli: {}: page 1 re-renders differently after the save.",
            input.display()
        );
        return exit::RASTER_DIFFERS;
    }
    exit::SUCCESS
}

/// The `--mode` value as it appears on the stdout line.
const fn mode_name(mode: RoundTripMode) -> &'static str {
    match mode {
        RoundTripMode::Incremental => "incremental",
        RoundTripMode::Full => "full",
        RoundTripMode::AppendIdentity => "append-identity",
    }
}

/// Whether every `File`-provenance object's definition bytes are
/// **byte-identical and reachable through the new cross-reference
/// table** — the R32 per-object assertion for a full rewrite.
///
/// Compares each object's retained `ByteSpan` slice on both sides,
/// rather than searching the output for the bytes. Two reasons, and the
/// second matters more:
///
/// 1. **It is linear.** A substring search per object is
///    `objects × filesize`. The veraPDF corpus contains a deliberate
///    Annex C implementation-limits file with ~80,000 objects in 4 MB —
///    roughly 320 GB of byte comparisons, which blows any wall-clock
///    budget.
/// 2. **It is stricter.** "These bytes appear somewhere in the output"
///    is a weak claim: it passes even when the object landed at an
///    offset its own cross-reference entry does not name. Comparing the
///    span the RELOADED document resolved for that object id proves the
///    bytes are reachable *through the xref*, which is what §5 actually
///    promises.
///
/// Two objects are excluded: the base file's own cross-reference-stream
/// object (superseded by the newly generated section, so its old bytes
/// are *supposed* to be absent), and `rewritten` — the object a
/// `--producer set` policy deliberately changed. Counting an intentional
/// metadata edit as a round-trip violation would be a false red.
fn full_rewrite_is_per_object_verbatim(
    before: &pdfce_core::document::Document,
    after: &pdfce_core::document::Document,
    rewritten: Option<pdfce_core::object::ObjId>,
) -> bool {
    use pdfce_core::object::Provenance;

    for io in before.objects() {
        // Only `Provenance::File` promises verbatim re-emission. A
        // compressed object has no file-level bytes, and a
        // `Provenance::RecoveredFile` object has bytes that CONTRADICT its
        // parsed value (a recovered stream extent), so the writer
        // deliberately re-serializes it — asserting byte identity for
        // either would be asserting the opposite of the contract.
        let Provenance::File(span) = io.provenance else {
            continue;
        };
        if is_section_object(before, io.id) || Some(io.id) == rewritten {
            continue;
        }
        let want = span.slice(before.bytes());
        let got = after
            .get(io.id)
            .and_then(|o| o.file_span())
            .and_then(|s| s.slice(after.bytes()));
        if want.is_none() || got != want {
            eprintln!(
                "pdfce-cli: object {} lost its verbatim definition bytes",
                io.id
            );
            return false;
        }
    }
    true
}

/// Whether `id` is the object that *is* the base file's newest
/// cross-reference section (§7.5.8.1) rather than document content.
///
/// Excluded from every comparison: a save supersedes it, so its
/// dictionary legitimately differs (fresh `/Prev`, delta `/Index`, new
/// `/Length`) and its old bytes are supposed to be absent.
fn is_section_object(doc: &pdfce_core::document::Document, id: pdfce_core::object::ObjId) -> bool {
    use pdfce_core::xref::SectionShape;
    matches!(doc.section_shape(), SectionShape::Stream { id: sid, .. } if sid == id)
}

/// Rasterize page 1 for the self-comparison oracle, or `None` if the
/// document has no renderable first page.
///
/// A render failure is not a round-trip failure — plenty of corpus files
/// are deliberately non-conformant — so this yields `None` and the
/// caller reports `raster_compared=0` rather than a false red.
fn render_first_page(doc: &pdfce_core::document::Document, scale: f32) -> Option<Vec<u8>> {
    let pages = pdfce_core::page_tree::pages(doc).ok()?;
    let page = pages.first()?;
    let rendered = pdfce_render::render_page(doc, page, scale).ok()?;
    Some(rendered.pixmap.data().to_vec())
}

// ---------------------------------------------------------------------------
// Editing subcommands (Pass 3.1)
// ---------------------------------------------------------------------------

/// What an edit-and-save produced, for the stdout line.
struct EditOutcome {
    report: pdfce_core::writer::SaveReport,
    /// Objects that currently differ from the base revision — the
    /// save-time diff, **not** a count of commands run.
    changed: usize,
    undo_verified: bool,
    undo_identical: bool,
}

/// Load `input` into an editing session, or print a diagnostic and
/// return the mapped exit code.
fn open_for_edit(input: &Path) -> Result<(Vec<u8>, pdfce_core::edit::EditSession), u8> {
    let source = std::fs::read(input).map_err(|err| {
        eprintln!("pdfce-cli: {}: {err}", input.display());
        exit::IO_ERROR
    })?;
    let doc = open_document_bytes(source.clone()).map_err(|err| {
        eprintln!("pdfce-cli: {}: {err}", input.display());
        exit_code_for_doc(&err)
    })?;
    Ok((source, pdfce_core::edit::EditSession::new(doc)))
}

/// Save an edited session, optionally verifying that undoing every edit
/// reproduces the input byte for byte.
///
/// ## The undo check always uses the incremental path, whatever `mode` is
///
/// The contract being verified is `ARCHITECTURE.md` §11.1's — *an object
/// edited and then undone must not appear in the update section* — and
/// its observable form is "zero edits means zero bytes", which only
/// incremental save promises. A full rewrite legitimately produces
/// different bytes for an unedited document (offsets move), so running
/// the check through it would assert something that is false by design.
///
/// The session is left with the edits **undone** afterwards. That is
/// deliberate rather than tidied up: the output file has already been
/// written, and re-applying the history only to throw the session away
/// would be motion without meaning.
fn save_edited(
    session: &mut pdfce_core::edit::EditSession,
    source: &[u8],
    output: &Path,
    mode: SaveMode,
    producer: ProducerArg,
    verify_undo: bool,
) -> Result<EditOutcome, u8> {
    use pdfce_core::writer::{ProducerPolicy, SaveOptions};

    // The two BYTES-radius R169 knobs: §7.5.4's cross-reference entry
    // terminator (`EOL-A1`) and §7.5.5's trailing end-of-line (`EOL-A2`).
    // Both are genuine spec ambiguities — three legal forms and no stated
    // preference, and two self-consistent readings of the last line — and
    // both default to exactly what pdfce has always emitted, so a machine
    // with no settings file writes byte-identical output.
    //
    // Applied to BOTH save modes, unlike `producer`: these describe bytes
    // the appended revision writes for itself, not a rewrite of anything
    // the operator did not touch, so rule 3 is untroubled. The
    // undo-verification save further down deliberately keeps pure
    // `identity()` — it is a byte comparison against the source and must
    // not acquire a dependency on a settings file.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let eol = |options: SaveOptions| {
        options
            .with_xref_entry_eol(settings.xref_entry_eol)
            .with_trailing_eol(settings.trailing_eol)
    };

    let options = eol(SaveOptions::default().with_producer(match producer {
        ProducerArg::Set => ProducerPolicy::Set,
        ProducerArg::Preserve => ProducerPolicy::Preserve,
    }));
    let changed = session.dirty_set().len();

    let saved = match mode {
        SaveMode::Incremental => session.to_incremental_bytes(&eol(SaveOptions::identity())),
        SaveMode::Full => session.to_full_bytes(&options),
    };
    let (bytes, report) = saved.map_err(|err| {
        eprintln!("pdfce-cli: save refused: {err}");
        exit::SAVE_REFUSED
    })?;

    std::fs::write(output, &bytes).map_err(|err| {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        exit::IO_ERROR
    })?;

    let mut undo_identical = false;
    if verify_undo {
        while session.undo().is_some() {}
        let undone = session
            .to_incremental_bytes(&SaveOptions::identity())
            .map_err(|err| {
                eprintln!("pdfce-cli: undo verification could not save: {err}");
                exit::SAVE_REFUSED
            })?
            .0;
        undo_identical = undone == source;
    }

    Ok(EditOutcome {
        report,
        changed,
        undo_verified: verify_undo,
        undo_identical,
    })
}

/// Print the shared warnings and return the final exit code for an
/// editing subcommand.
fn finish_edit(input: &Path, outcome: &EditOutcome) -> u8 {
    if outcome.changed == 0 {
        // "Zero edits means zero bytes" is the writer's contract, so an
        // unchanged document produces a byte copy rather than an empty
        // revision. Saying so is the difference between a no-op the
        // operator understands and one they mistake for a failure.
        eprintln!(
            "pdfce-cli: {}: nothing changed — the document already had the requested \
value(s), so the output is a byte-for-byte copy of the input and no revision was appended.",
            input.display()
        );
    }
    if !outcome.report.promoted.is_empty() {
        // R38: a representation change to an object whose value the
        // operator may not have edited. Named, not just counted.
        let names: Vec<String> = outcome
            .report
            .promoted
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        eprintln!(
            "pdfce-cli: {}: {} object(s) were moved out of an object stream because they were \
touched (ISO 32000-1 §7.5.7 objects cannot be edited in place): {}. Their previous values remain \
inside the untouched container, which is normal for an edit and is NOT sufficient for redaction.",
            input.display(),
            outcome.report.promoted.len(),
            names.join(", ")
        );
    }
    if outcome.report.delinearized {
        eprintln!(
            "pdfce-cli: {}: this file is linearized (Fast Web View); saving invalidates that. \
Per ISO 32000-1 Annex F.1 the result 'shall be treated as ordinary PDF'. pdfce does not repair \
or re-linearize, and does not strip the /Linearized dictionary.",
            input.display()
        );
    }
    if outcome.undo_verified && !outcome.undo_identical {
        eprintln!(
            "pdfce-cli: {}: UNDO VERIFICATION FAILED — undoing the edit did not reproduce the \
input byte for byte. This is an ARCHITECTURE.md §11.1 violation (the dirty set must be a diff \
against the base revision, not the union of every command run).",
            input.display()
        );
        return exit::NOT_BYTE_IDENTICAL;
    }
    exit::SUCCESS
}

/// Map an [`EditError`](pdfce_core::edit::EditError) to an exit code.
///
/// Everything except a genuine structural failure of the page tree is
/// [`exit::EDIT_REFUSED`]: the file was readable and pdfce declined the
/// operation as asked, which a batch script must be able to tell apart
/// from a broken file.
fn report_edit_error(input: &Path, err: &pdfce_core::edit::EditError) -> u8 {
    use pdfce_core::edit::EditError;
    eprintln!("pdfce-cli: {}: {err}", input.display());
    match err {
        EditError::PageTree(_) => exit::RUNTIME_ERROR,
        _ => exit::EDIT_REFUSED,
    }
}

/// Implement `pdfce-cli rotate-page`.
///
/// `--page` is 1-based, matching every PDF reader and every human;
/// `pdfce-core` is 0-based, and the conversion happens here rather than
/// in the engine. `0` and past-the-end take the same path — a named
/// refusal from the engine, reported with the real page count.
fn cmd_rotate_page(
    input: &Path,
    page: u32,
    degrees: i32,
    relative: bool,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // 1-based -> 0-based. `checked_sub` handles `--page 0` without a
    // wrap; the engine handles past-the-end and names the real count.
    let Some(index) = page.checked_sub(1).map(|i| i as usize) else {
        eprintln!(
            "pdfce-cli: {}: --page is 1-based; 0 is not a page",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };

    let result = if relative {
        session.rotate_page_by(index, degrees)
    } else {
        session.set_page_rotation(index, degrees)
    };
    if let Err(err) = result {
        return report_edit_error(input, &err);
    }

    // Read the resulting rotation back out of the session rather than
    // recomputing it here: the normalization rules (positive modulo,
    // inheritance) live in one place, and a second copy would drift.
    let rotate = session
        .pages()
        .ok()
        .and_then(|pages| pages.get(index).map(|p| p.rotate))
        .unwrap_or(0);

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };

    let r = &outcome.report;
    println!(
        "rotate-page {} page {page} mode={} -> {}; \
rotate={rotate} changed={} objects={} verbatim={} reserialized={} promoted={} \
appended={} out_bytes={} undo_verified={} undo_identical={} delinearized={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.promoted.len(),
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
        u32::from(r.delinearized),
    );
    finish_edit(input, &outcome)
}

/// Implement `pdfce-cli set-info`.
///
/// `sets` is every `(field, Option<value>)` pair the flags produced;
/// `clears` is the `--clear` list. A field that appears in both is
/// **cleared**, because `--clear` is the more explicit request — and
/// that resolution is stated here rather than left to argument order,
/// which a script author cannot see.
fn cmd_set_info(
    input: &Path,
    sets: &[(InfoFieldArg, Option<String>)],
    clears: &[InfoFieldArg],
    output: &Path,
    mode: SaveMode,
    producer: ProducerArg,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let had_info = session
        .document()
        .trailer()
        .get(b"Info")
        .and_then(pdfce_core::object::Object::as_reference)
        .is_some();

    if sets.iter().all(|(_, v)| v.is_none()) && clears.is_empty() {
        eprintln!(
            "pdfce-cli: {}: no fields given — pass at least one of --title/--author/\
--subject/--keywords, or --clear <field>",
            input.display()
        );
        return exit::EDIT_REFUSED;
    }

    for (field, value) in sets {
        if clears.contains(field) {
            continue;
        }
        let Some(text) = value else { continue };
        if let Err(err) = session.set_info_field((*field).into(), Some(text.as_str())) {
            return report_edit_error(input, &err);
        }
    }
    for field in clears {
        if let Err(err) = session.set_info_field((*field).into(), None) {
            return report_edit_error(input, &err);
        }
    }

    let created = !had_info && session.dirty_set().trailer_patch().contains_key(b"Info");

    let outcome = match save_edited(&mut session, &source, output, mode, producer, verify_undo) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };

    let r = &outcome.report;
    println!(
        "set-info {} mode={} -> {}; \
changed={} objects={} verbatim={} reserialized={} promoted={} appended={} out_bytes={} \
info_created={} undo_verified={} undo_identical={} delinearized={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.promoted.len(),
        r.bytes_appended,
        r.bytes_written,
        u32::from(created),
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
        u32::from(r.delinearized),
    );
    if created {
        // Creating an object is a bigger structural act than editing
        // one, and R41's whole discipline is that pdfce does not add
        // things to a file unasked — so when it legitimately does,
        // because the operator asked, it says so.
        eprintln!(
            "pdfce-cli: {}: this file had no document information dictionary; one was created \
to hold the metadata you asked for, and the trailer now references it.",
            input.display()
        );
    }
    finish_edit(input, &outcome)
}

/// The parsed arguments of `pdfce-cli annotate`, grouped into one struct
/// so [`cmd_annotate`] stays under clippy's argument-count limit.
struct AnnotateArgs<'a> {
    input: &'a Path,
    kind: AnnotKindArg,
    page: u32,
    rect: Option<&'a str>,
    line: Option<&'a str>,
    points: Option<&'a str>,
    strokes: Option<&'a str>,
    quads: Option<&'a str>,
    color: Option<&'a str>,
    fill: Option<&'a str>,
    width: f64,
    text: Option<&'a str>,
    font: &'a str,
    size: f64,
    quad: QuadArg,
    multiline: bool,
    icon: IconArg,
    stamp_name: StampArg,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// Whether an [`AnnotKindArg`] is one of the Pass-6.2 text-bearing
/// subtypes (which take the variable-text path) or a Pass-6.1 geometric
/// one.
fn is_text_bearing(kind: AnnotKindArg) -> bool {
    matches!(
        kind,
        AnnotKindArg::Freetext | AnnotKindArg::Text | AnnotKindArg::Stamp
    )
}

/// Implement `pdfce-cli annotate` (Pass 6.1). Parses the per-subtype
/// geometry flags into a [`MarkupSpec`](pdfce_core::annot_author::MarkupSpec)
/// and authors it through the same [`EditSession`] path the GUI uses.
fn cmd_annotate(args: &AnnotateArgs<'_>) -> u8 {
    let input = args.input;
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let Some(index) = args.page.checked_sub(1).map(|i| i as usize) else {
        eprintln!(
            "pdfce-cli: {}: --page is 1-based; 0 is not a page",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };

    // Pass 6.2 text-bearing subtypes take the variable-text path
    // (add_text_annotation); the Pass 6.1 geometric subtypes take
    // add_markup. Both share every guard and the same save/undo plumbing.
    let add_result = if is_text_bearing(args.kind) {
        match build_text_annot_spec(args) {
            Ok(spec) => session.add_text_annotation(index, &spec).map(|_| ()),
            Err(msg) => {
                eprintln!("pdfce-cli: {}: {msg}", input.display());
                return exit::EDIT_REFUSED;
            }
        }
    } else {
        match build_markup_spec(args) {
            Ok(spec) => session.add_markup(index, &spec).map(|_| ()),
            Err(msg) => {
                eprintln!("pdfce-cli: {}: {msg}", input.display());
                return exit::EDIT_REFUSED;
            }
        }
    };
    if let Err(err) = add_result {
        return report_edit_error(input, &err);
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };

    let r = &outcome.report;
    println!(
        "annotate {} type={:?} page={} mode={} -> {}; \
changed={} objects={} verbatim={} reserialized={} promoted={} appended={} out_bytes={} \
undo_verified={} undo_identical={} delinearized={}",
        input.display(),
        args.kind,
        args.page,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.promoted.len(),
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
        u32::from(r.delinearized),
    );
    finish_edit(input, &outcome)
}

// ---------------------------------------------------------------------------
// Redaction subcommands (Pass 8, ISO 32000-1 §12.5.6.23)
// ---------------------------------------------------------------------------

/// Parsed `redact-mark` flags.
struct RedactMarkArgs<'a> {
    input: &'a Path,
    rect: Option<&'a str>,
    search: Option<&'a str>,
    pattern: Option<&'a str>,
    ignore_case: bool,
    page: u32,
    fill: Option<&'a str>,
    overlay_text: Option<&'a str>,
    output: &'a Path,
}

/// `redact-mark`: author reviewable `/Redact` marks (the non-destructive
/// MARK phase). Removal is a separate `redact-apply` (R52).
fn cmd_redact_mark(args: &RedactMarkArgs<'_>) -> u8 {
    use pdfce_core::annot_author::{Quad, RedactSpec};
    use pdfce_core::vartext::Quadding;

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let fill = match args.fill {
        Some(h) => match parse_color(h) {
            Ok(c) => Some(c),
            Err(msg) => {
                eprintln!("pdfce-cli: {}: {msg}", args.input.display());
                return exit::EDIT_REFUSED;
            }
        },
        None => None,
    };
    let overlay_text = args.overlay_text.map(str::to_string);

    let created = if let Some(rectspec) = args.rect {
        let Some(index) = args.page.checked_sub(1).map(|i| i as usize) else {
            eprintln!(
                "pdfce-cli: {}: --page is 1-based; 0 is not a page",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        };
        let rect = match rect_from(rectspec) {
            Ok(r) => r,
            Err(msg) => {
                eprintln!("pdfce-cli: {}: {msg}", args.input.display());
                return exit::EDIT_REFUSED;
            }
        };
        let spec = RedactSpec {
            quads: vec![Quad::from_rect(rect)],
            fill,
            overlay_text,
            quadding: Quadding::Left,
        };
        match session.add_redaction(index, &spec) {
            Ok(_) => 1usize,
            Err(err) => return report_edit_error(args.input, &err),
        }
    } else if let Some(query) = args.search {
        if args.fill.is_some() || args.overlay_text.is_some() {
            eprintln!(
                "pdfce-cli: note: --fill/--overlay-text are ignored for --search marks this build \
                 (default black fill applied on apply)"
            );
        }
        match session.mark_redactions_by_search(query, args.ignore_case) {
            Ok(ids) => ids.len(),
            Err(err) => return report_edit_error(args.input, &err),
        }
    } else if let Some(pattern) = args.pattern {
        match session.mark_redactions_by_pattern(pattern, args.ignore_case) {
            Ok(ids) => ids.len(),
            Err(err) => return report_edit_error(args.input, &err),
        }
    } else {
        eprintln!(
            "pdfce-cli: {}: give exactly one of --rect, --search, or --pattern",
            args.input.display()
        );
        return exit::EDIT_REFUSED;
    };

    if created == 0 {
        eprintln!(
            "pdfce-cli: {}: no content matched — no redaction marks authored",
            args.input.display()
        );
    }

    // Marks are additive, so an incremental save is correct and
    // signature-safe (they are not the destructive apply).
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        SaveMode::Incremental,
        ProducerArg::Preserve,
        false,
    ) {
        Ok(o) => o,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "redact-mark {} marks_created={} -> {}; changed={} appended={} out_bytes={}",
        args.input.display(),
        created,
        args.output.display(),
        outcome.changed,
        r.bytes_appended,
        r.bytes_written,
    );
    if created > 0 {
        println!(
            "  {created} /Redact mark(s) authored — REVIEW then run `redact-apply` to remove the \
             content. The document is NOT yet redacted."
        );
    }
    exit::SUCCESS
}

/// `redact-apply`: TRULY REMOVE the marked content (the destructive R35
/// phase). Prints the redaction report; the refusal-acknowledgement gate
/// (ui-spec §4.4) forces a non-zero exit on un-scrubbed carrier residuals
/// unless `--acknowledge-residuals` is passed.
/// Arguments for [`cmd_edit_text`], grouped so the handler stays under the
/// clippy `too_many_arguments` bound (same pattern as `RedactMarkArgs`).
struct EditTextArgs<'a> {
    input: &'a Path,
    output: &'a Path,
    /// 1-based page number.
    page: usize,
    find: &'a str,
    replace: &'a str,
    pin: bool,
    font_dirs: &'a [PathBuf],
}

/// `edit-text`: Pass 14.1 in-place text editing.
///
/// Locates `--find` within one show operator on `--page`, re-encodes
/// `--replace` in that run's OWN font encoding (inverting `/Encoding`, never
/// `/ToUnicode` — §9.6.6 / `iso32000__ref__inverse_encoding.md`), preserves
/// the §9.4.4 advance so un-edited text stays put, relayouts the line
/// (reflow by default; `--pin` compensates instead), and saves
/// INCREMENTALLY. The font-on-edit gate REFUSES by name any character the
/// run's font cannot provide (rule 4 / R71) — a refusal is a clean, named
/// non-zero exit ([`exit::EDIT_REFUSED`]), never a crash. All disclosures
/// (three-trust-level, incremental/prior-text, tagged-stale, relayout
/// overflow, R-INV-5 ambiguity) are surfaced verbatim.
fn cmd_edit_text(args: &EditTextArgs<'_>) -> u8 {
    use pdfce_core::text_edit::{
        EditError, EditGlyphSource, EditOptions, EditRequest, FollowerDisposition,
    };

    // The shell owns font discovery (R61): `--font-dir` supplies operator
    // faces for a NON-embedded run's preview/coverage (decision 012).
    let (font_env, supplied_registered, font_notes) = build_font_environment(args.font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }

    let source = match std::fs::read(args.input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit_code_for_doc(&err);
        }
    };

    if args.page == 0 {
        eprintln!("pdfce-cli: --page is 1-based; 0 is not a valid page number");
        return exit::EDIT_REFUSED;
    }
    let req = EditRequest::find_replace(args.page - 1, args.find, args.replace);
    let opts = EditOptions::default().with_disposition(if args.pin {
        FollowerDisposition::Pin
    } else {
        FollowerDisposition::Reflow
    });

    let outcome = match pdfce_core::text_edit::edit_text(&doc, &req, &opts) {
        Ok(o) => o,
        Err(err) => {
            eprintln!("pdfce-cli: edit-text refused: {err}");
            return match err {
                EditError::Refused(_)
                | EditError::NoMatch(_)
                | EditError::Unsupported(_)
                | EditError::PageIndex(_)
                | EditError::Encrypted => exit::EDIT_REFUSED,
                EditError::Write(_) => exit::SAVE_REFUSED,
                EditError::Content(_) | EditError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::RUNTIME_ERROR,
            };
        }
    };

    if let Err(err) = std::fs::write(args.output, &outcome.bytes) {
        eprintln!("pdfce-cli: {}: {err}", args.output.display());
        return exit::IO_ERROR;
    }

    let report = &outcome.report;
    println!(
        "edit-text {} -> {}",
        args.input.display(),
        args.output.display()
    );
    println!(
        "  page={} find={:?} replace={:?}",
        args.page, args.find, args.replace
    );
    println!(
        "  base_font={} content_object={} advance_delta={:.3} followers_repositioned={}",
        report.base_font,
        report.content_object,
        report.advance_delta,
        report.followers_repositioned
    );

    // Refine the core's Embedded/NonEmbedded into the three decision-012
    // trust levels via the ONE shared classifier on `FontEnvironment`
    // (Pass 14.3 §7 hoist): a NON-embedded run is Supplied when a `--font-dir`
    // face is registered for its name, else Bundled (shapes only; positions
    // still come from `/Widths`).
    let trust = match report.glyph_source {
        EditGlyphSource::Embedded => "Embedded",
        EditGlyphSource::NonEmbedded => match font_env.classify_nonembedded(&report.base_font) {
            pdfce_render::GlyphSource::Supplied => "Supplied",
            _ => "Bundled",
        },
        _ => "unknown",
    };
    println!(
        "  glyph_source={trust} subset={} supplied_registered={}",
        report.subset, supplied_registered
    );
    if let Some(mcid) = report.tagged_mcid {
        println!("  tagged_mcid={mcid}");
    }
    println!("  disclosures:");
    for d in &report.disclosures {
        println!("    - {d}");
    }
    exit::SUCCESS
}

/// Named arguments for [`cmd_add_text`] (grouped to dodge clippy's
/// `too_many_arguments`, matching [`EditTextArgs`]).
struct AddTextArgs<'a> {
    input: &'a Path,
    output: &'a Path,
    /// 1-based page number.
    page: usize,
    /// POINT mode origin `"x,y"` in points, or `None` in boxed mode.
    at: Option<&'a str>,
    /// BOXED mode rectangle `"x,y,w,h"` in points, or `None` in point mode.
    wrap_box: Option<&'a str>,
    /// BOXED mode alignment keyword, or `None` (defaults to left).
    align: Option<&'a str>,
    /// BOXED mode leading (points), or `None` for the derived default.
    leading: Option<f64>,
    text: &'a str,
    /// Standard-14 `BaseFont` name or `auto`.
    font: &'a str,
    size: f64,
    /// `"r,g,b"` fill colour, or `None` for black.
    color: Option<&'a str>,
    font_dirs: &'a [PathBuf],
    /// Path to a donor font file to SUBSET AND EMBED (FF-C, decision 021).
    ///
    /// `None` keeps the shipped R79 behaviour: a Standard-14 face written by
    /// name with no embedding. Embedding is never inferred from anything
    /// else — not from `--font-dir`, not from the text containing non-Latin
    /// characters — because R108 makes it an explicit per-action choice, and
    /// an "I noticed you needed this" default is exactly the silent
    /// file-size and font-redistribution change that rule exists to prevent.
    embed_font: Option<&'a Path>,
}

/// `add-text`: Pass 16.0 add NEW page text (decision 016 / FF-D).
///
/// Synthesizes a single-line `BT…ET` run at `--at "x,y"` in the chosen
/// Standard-14 face and APPENDS it as a new content stream (ISO 32000-1
/// §7.7.3.3), leaving every ORIGINAL content stream byte-identical (R32/R46).
/// No glyph embedding (R79): the run is written by `/BaseFont` name + code, so
/// a character the face cannot represent is REFUSED by name (the F-refuse gate,
/// R71) — a clean, named non-zero exit ([`exit::EDIT_REFUSED`]), never a crash
/// or a faked glyph. This is genuine page content, NOT a `/FreeText`
/// annotation (R78): the result is editable/formattable/reflowable like the
/// page's own text. The save is INCREMENTAL. Font provenance
/// (`Bundled`/`Supplied`), the tagged-untagged disclosure (R73), and the
/// inheritance-safe `/Resources` note (§7.7.3.4) are surfaced verbatim.
fn cmd_add_text(args: &AddTextArgs<'_>) -> u8 {
    use pdfce_core::fontdata::{Std14, std14_base_font_name, std14_by_base_font};
    use pdfce_core::text_edit::{
        AddTextError, AddTextRequest, BlockAlignment, FontProvenance, NewTextColor, add_text,
    };

    if args.page == 0 {
        eprintln!("pdfce-cli: --page is 1-based; 0 is not a valid page number");
        return exit::EDIT_REFUSED;
    }

    // Resolve the placement mode up front (R27 fail-clean): exactly one of
    // `--at` (point, 16.0) or `--box` (boxed, 16.1). Parsing the geometry and
    // the alignment before any document work means a typo fails cleanly.
    enum Placement {
        Point {
            origin: (f64, f64),
        },
        Boxed {
            rect: (f64, f64, f64, f64),
            align: BlockAlignment,
        },
    }
    let placement = match (args.at, args.wrap_box) {
        (Some(_), Some(_)) => {
            eprintln!("pdfce-cli: --at and --box are mutually exclusive; pass exactly one");
            return exit::EDIT_REFUSED;
        }
        (None, None) => {
            eprintln!(
                "pdfce-cli: add-text needs a placement — pass --at \"x,y\" (point text) or \
                 --box \"x,y,w,h\" (boxed, wrapped text)"
            );
            return exit::EDIT_REFUSED;
        }
        (Some(at), None) => match parse_at_pair(at) {
            Some(origin) => Placement::Point { origin },
            None => {
                eprintln!(
                    "pdfce-cli: --at expects two comma-separated numbers \"x,y\" (points), got {at:?}"
                );
                return exit::EDIT_REFUSED;
            }
        },
        (None, Some(bx)) => {
            let rect = match parse_box_quad(bx) {
                Some(r) => r,
                None => {
                    eprintln!(
                        "pdfce-cli: --box expects four comma-separated numbers \"x,y,w,h\" \
                         (points), got {bx:?}"
                    );
                    return exit::EDIT_REFUSED;
                }
            };
            let align = match args.align {
                None => BlockAlignment::Left,
                Some(s) => match BlockAlignment::parse(s) {
                    Some(a) => a,
                    None => {
                        eprintln!("pdfce-cli: --align {s:?}: expected left|center|right|justify");
                        return exit::EDIT_REFUSED;
                    }
                },
            };
            Placement::Boxed { rect, align }
        }
    };

    // `--font auto` = Helvetica (pdfce's documented default; Acrobat's is a
    // GAP, decision 016 §3.3); otherwise an EXACT §9.6.2.2 spelling.
    let font = if args.font.eq_ignore_ascii_case("auto") {
        Std14::Helvetica
    } else {
        match std14_by_base_font(args.font) {
            Some(f) => f,
            None => {
                eprintln!(
                    "pdfce-cli: --font {:?} is not a Standard-14 BaseFont name \
                     (e.g. Helvetica, Times-Roman, Courier-Bold, Symbol, ZapfDingbats)",
                    args.font
                );
                return exit::EDIT_REFUSED;
            }
        }
    };

    let color = match args.color {
        None => NewTextColor::Black,
        Some(s) => match parse_rgb_triple(s) {
            Some((r, g, b)) => NewTextColor::Rgb(r, g, b),
            None => {
                eprintln!(
                    "pdfce-cli: --color expects three comma-separated components in 0..=1 \
                     \"r,g,b\", got {s:?}"
                );
                return exit::EDIT_REFUSED;
            }
        },
    };

    // The shell owns font discovery (R61): a `--font-dir` face registered for
    // the chosen name lifts the disclosed provenance to `Supplied` (decision
    // 012). The WRITTEN dict is identical either way (no embedding, R79).
    let (font_env, supplied_registered, font_notes) = build_font_environment(args.font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }
    let base_font_name = std14_base_font_name(font);
    let provenance = match font_env.classify_nonembedded(base_font_name) {
        pdfce_render::GlyphSource::Supplied => FontProvenance::Supplied,
        _ => FontProvenance::Bundled,
    };

    let source = match std::fs::read(args.input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit_code_for_doc(&err);
        }
    };

    // The `origin` a point add uses; a boxed add supersedes it via `with_box`.
    let base_origin = match placement {
        Placement::Point { origin } => origin,
        Placement::Boxed { .. } => (0.0, 0.0),
    };
    let mut req = AddTextRequest::new(args.page - 1, base_origin, args.text.to_owned())
        .with_font(font)
        .with_provenance(provenance)
        .with_size(args.size)
        .with_color(color);

    // FF-C (decision 021 / Pass 21.0): --embed-font subsets a donor face and
    // embeds it, so the saved file carries its own glyphs.
    //
    // The subset is computed HERE, before anything is written, and its real
    // numbers are printed. That is R108/R98 applied: subsetting is a pure
    // function, so there is no reason to describe the outcome in the future
    // tense. "will add roughly N KB" is a prediction; "added 11,240 bytes for
    // 14 glyph(s)" is a measurement, and only one of them can be wrong.
    if let Some(donor_path) = args.embed_font {
        let donor = match std::fs::read(donor_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "pdfce-cli: cannot read the font file {}: {e}",
                    donor_path.display()
                );
                return exit::IO_ERROR;
            }
        };
        // Deduplicated and ordered: the plan only needs each distinct
        // character once, and `plan_subset` reports coverage gaps against
        // exactly what it was asked for — passing "AAB" would otherwise
        // report 'A' missing twice.
        let mut wanted: Vec<char> = args.text.chars().collect();
        wanted.sort_unstable();
        wanted.dedup();

        let stem = donor_path.file_stem().map_or_else(
            || "EmbeddedFont".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        // A subset tag must be exactly six uppercase ASCII letters (§9.6.4).
        // Derived from the face name so repeated runs over the same font are
        // reproducible — a random tag would make byte-comparison of two
        // otherwise identical outputs impossible, which the round-trip
        // harness depends on.
        let tag = pdfce_render::font::subset::subset_tag_for(&stem);

        let plan = match pdfce_render::font::subset::plan_subset(&donor, 0, &wanted, &stem, &tag) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("pdfce-cli: add-text refused: {e}");
                return exit::EDIT_REFUSED;
            }
        };

        // One line, no continuation. A `\` line-continuation inside the
        // format string looked tidier in source and printed a run of
        // spaces to the terminal, because the leading indentation of the
        // continued line is only stripped when nothing follows the
        // backslash. Readable source is not worth unreadable output.
        println!(
            "embedding a subset of '{}': {} glyph(s), {} byte(s) of font program, covering {} character(s) — subset tag {}",
            plan.base_name,
            plan.glyphs.len(),
            plan.program.len(),
            wanted.len(),
            plan.subset_tag
        );
        req = req.with_embedded_face(plan);
    }
    if let Placement::Boxed {
        rect: (x, y, w, h),
        align,
    } = placement
    {
        req = req
            .with_box(x, y, w, h)
            .with_alignment(align)
            .with_leading(args.leading);
    }

    let outcome = match add_text(&doc, &req) {
        Ok(o) => o,
        Err(err) => {
            eprintln!("pdfce-cli: add-text refused: {err}");
            return match err {
                AddTextError::Refused(_)
                | AddTextError::PageIndex(_)
                | AddTextError::EmptyText
                | AddTextError::InvalidSize(_)
                | AddTextError::InvalidBox(..)
                | AddTextError::NoWordsToWrap
                | AddTextError::Encrypted
                | AddTextError::CertificationForbidsChange { .. }
                | AddTextError::HiddenObjects { .. }
                | AddTextError::ObjectNumbersExhausted
                // Both FF-C refusals are operator-facing: something about
                // the request cannot be honoured, and the operator can
                // change it. They belong with the refusals, not in the
                // `_ => RUNTIME_ERROR` catch-all, which would have told a
                // script that pdfce had crashed rather than declined.
                | AddTextError::EmbeddedBoxedUnsupported
                | AddTextError::EmbeddedPlanIncomplete { .. }
                | AddTextError::Embed(_)
                | AddTextError::Unsupported(_) => exit::EDIT_REFUSED,
                AddTextError::Write(_) => exit::SAVE_REFUSED,
                AddTextError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::RUNTIME_ERROR,
            };
        }
    };

    if let Err(err) = std::fs::write(args.output, &outcome.bytes) {
        eprintln!("pdfce-cli: {}: {err}", args.output.display());
        return exit::IO_ERROR;
    }

    let report = &outcome.report;
    println!(
        "add-text {} -> {}",
        args.input.display(),
        args.output.display()
    );
    match placement {
        Placement::Point { origin } => println!(
            "  mode=point page={} at={},{} text={:?}",
            args.page, origin.0, origin.1, args.text
        ),
        Placement::Boxed {
            rect: (x, y, w, h),
            align,
        } => println!(
            "  mode=boxed page={} box={x},{y},{w},{h} align={} text={:?}",
            args.page,
            align.as_str(),
            args.text
        ),
    }
    if let Some(n) = report.wrapped_lines {
        println!(
            "  wrapped_lines={n} box_overflow_lines={} page_overflow_pt={:.2}",
            report.box_overflow_lines, report.page_overflow_pt
        );
    }
    let provenance = match report.provenance {
        FontProvenance::Bundled => "Bundled",
        FontProvenance::Supplied => "Supplied",
        _ => "unknown",
    };
    println!(
        "  base_font={} provenance={provenance} font_resource=/{} size={}",
        report.base_font, report.font_resource_name, args.size
    );
    println!(
        "  content_object={} font_object={} gave_page_own_resources={} tagged_untagged={} \
         supplied_registered={}",
        report.content_object,
        report.font_object,
        report.gave_page_own_resources,
        report.tagged_untagged,
        supplied_registered
    );
    println!("  disclosures:");
    for d in &report.disclosures {
        println!("    - {d}");
    }
    exit::SUCCESS
}

/// Parse `"x,y"` into two `f64` points, or `None` on any malformed input.
fn parse_at_pair(s: &str) -> Option<(f64, f64)> {
    let (x, y) = s.split_once(',')?;
    let x: f64 = x.trim().parse().ok()?;
    let y: f64 = y.trim().parse().ok()?;
    if x.is_finite() && y.is_finite() {
        Some((x, y))
    } else {
        None
    }
}

/// Parse `"x,y,w,h"` into four `f64` points for the boxed add, or `None` on
/// any malformed input. Width/height positivity is enforced by the core
/// ([`pdfce_core::text_edit::AddTextError::InvalidBox`]); this only checks the
/// shape and finiteness so a typo fails before any document work.
fn parse_box_quad(s: &str) -> Option<(f64, f64, f64, f64)> {
    let mut it = s.split(',');
    let x: f64 = it.next()?.trim().parse().ok()?;
    let y: f64 = it.next()?.trim().parse().ok()?;
    let w: f64 = it.next()?.trim().parse().ok()?;
    let h: f64 = it.next()?.trim().parse().ok()?;
    if it.next().is_some() {
        return None; // too many components
    }
    if [x, y, w, h].iter().all(|v| v.is_finite()) {
        Some((x, y, w, h))
    } else {
        None
    }
}

/// Parse `"r,g,b"` into three `f64` components (each finite; clamped to
/// `0..=1` by the core), or `None` on any malformed input.
fn parse_rgb_triple(s: &str) -> Option<(f64, f64, f64)> {
    let mut it = s.split(',');
    let r: f64 = it.next()?.trim().parse().ok()?;
    let g: f64 = it.next()?.trim().parse().ok()?;
    let b: f64 = it.next()?.trim().parse().ok()?;
    if it.next().is_some() {
        return None; // too many components
    }
    if [r, g, b].iter().all(|v| v.is_finite()) {
        Some((r, g, b))
    } else {
        None
    }
}

/// `reflow`: Pass 15.1 within-block reflow surgery.
///
/// Recognises the block model on `--page` (with first-line-indent splitting
/// relaxed, matching `inspect --reflow-preview`), re-wraps the paragraph
/// `--block` under the requested width/alignment/leading via the 14.1
/// advance-preserving machinery, and saves INCREMENTALLY — only the block's
/// own content-stream object changes. Every gate (a composite/CJK block, a
/// rotated/skewed or shared/non-contiguous block, a missing-provenance or
/// bad-index/width condition) is a clean, named non-zero exit
/// ([`exit::EDIT_REFUSED`]), never a crash. All disclosures (derived-layout,
/// justify, page-overflow-emitted-not-clipped, tagged-stale, incremental/
/// prior-text) are surfaced verbatim.
#[allow(clippy::too_many_arguments)]
fn cmd_reflow(
    input: &Path,
    page: usize,
    block: usize,
    width: Option<f64>,
    align: Option<&str>,
    leading: Option<f64>,
    output: &Path,
) -> u8 {
    use pdfce_core::text_edit::{BlockAlignment, ReflowApplyError, ReflowRequest, apply_reflow};

    // Parse the alignment override up front so a typo fails cleanly before any
    // document work (the R27 fail-clean posture; identical to the preview
    // path's parse).
    let align_override = match align {
        None => None,
        Some(s) => match BlockAlignment::parse(s) {
            Some(a) => Some(a),
            None => {
                eprintln!(
                    "pdfce-cli: {}: --align {s}: expected left|right|center|justified",
                    input.display()
                );
                return exit::EDIT_REFUSED;
            }
        },
    };

    if page == 0 {
        eprintln!("pdfce-cli: --page is 1-based; 0 is not a valid page number");
        return exit::EDIT_REFUSED;
    }

    let source = match std::fs::read(input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };

    let req = ReflowRequest::new()
        .with_wrap_width_opt(width)
        .with_alignment_opt(align_override)
        .with_leading_opt(leading);

    let outcome = match apply_reflow(&doc, page - 1, block, &req) {
        Ok(o) => o,
        Err(err) => {
            eprintln!("pdfce-cli: reflow refused: {err}");
            // A refusal is a clean named non-zero; a save/runtime failure is a
            // distinct class. The `_` arm keeps this exhaustive as
            // `ReflowApplyError` grows (it is `#[non_exhaustive]`).
            return match err {
                ReflowApplyError::Refused(_)
                | ReflowApplyError::Preview(_)
                | ReflowApplyError::NoProvenance
                | ReflowApplyError::Unsupported(_)
                | ReflowApplyError::PageIndex(_)
                | ReflowApplyError::Encrypted => exit::EDIT_REFUSED,
                ReflowApplyError::Write(_) => exit::SAVE_REFUSED,
                ReflowApplyError::Extract(_)
                | ReflowApplyError::Content(_)
                | ReflowApplyError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::RUNTIME_ERROR,
            };
        }
    };

    if let Err(err) = std::fs::write(output, &outcome.bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }

    let report = &outcome.report;
    println!("reflow {} -> {}", input.display(), output.display());
    println!(
        "  page={page} block={block} align={} lines_before={} lines_after={} \
justified_lines={} height_delta={:.1}",
        report.alignment.as_str(),
        report.lines_before,
        report.lines_after,
        report.justified_lines,
        report.height_delta,
    );
    println!(
        "  base_font={} glyph_source={} content_object={}",
        report.base_font,
        match report.glyph_source {
            pdfce_core::text_edit::EditGlyphSource::Embedded => "Embedded",
            _ => "NonEmbedded",
        },
        report.content_object,
    );
    if let Some(ov) = report.overflow {
        println!(
            "  overflow: past_bottom={:.1}pt lines_outside={} (EMITTED off-page, not clipped)",
            ov.past_bottom_pt, ov.lines_outside
        );
    }
    if let Some(mcid) = report.tagged_mcid {
        println!("  tagged_mcid={mcid}");
    }
    println!("  disclosures:");
    for d in &report.disclosures {
        println!("    - {d}");
    }
    exit::SUCCESS
}

/// Arguments for [`cmd_format_text`], grouped to stay under the clippy
/// `too_many_arguments` bound (same pattern as [`EditTextArgs`]).
struct FormatTextArgs<'a> {
    input: &'a Path,
    output: &'a Path,
    /// 1-based page number.
    page: usize,
    find: &'a str,
    set_size: Option<f64>,
    /// `MODEL:comps` as passed on the command line, e.g. `rgb:1,0,0`.
    set_color: Option<&'a str>,
    /// A target font resource key or `/BaseFont`.
    set_font: Option<&'a str>,
    /// `--char-spacing` as passed, e.g. `0.5`, `0.5pt`, `20em`.
    char_spacing: Option<&'a str>,
    /// `--word-spacing` as passed, e.g. `2`, `2pt`, `200em` (Pass 19.4).
    word_spacing: Option<&'a str>,
    /// `--h-scale` percentage (100 = normal).
    h_scale: Option<f64>,
    /// The baseline toggle, already resolved from the three exclusive flags.
    script: Option<pdfce_core::text_edit::ScriptPosition>,
    /// `--rise` as passed, e.g. `3.25`, `3.25pt`, `280em` (Pass 19.2).
    rise: Option<&'a str>,
    /// The synthetic styles asked for, already folded from the two flags.
    /// [`StyleSynthesis::None`] means none were, which is the default and
    /// the only state in which nothing is synthesized (R90).
    synthetic: pdfce_core::text_edit::StyleSynthesis,
    pin: bool,
    font_dirs: &'a [PathBuf],
}

/// Parse a text-space metric argument into a [`MetricSpec`]
/// (`0.5` / `0.5pt` → absolute; `20em` → 20 thousandths of an em).
///
/// `flag` is the option's own spelling (`--char-spacing`, `--word-spacing`,
/// `--rise`), used only so the error message names the flag the operator
/// actually typed instead of whichever one happened to be implemented
/// first. It is a parameter rather than three near-copies of this function
/// precisely because the grammar must not drift between the flags: `Tc`,
/// `Tw` and `Ts` are all in unscaled text-space units (§9.3 Table 105's
/// closing note) and all governed by R89's Absolute/Relative discrimination,
/// so there is exactly one set of suffixes to learn.
///
/// The `em` suffix means **‰ of an em**, not ems — the typographic tracking
/// convention, and the same unit space `TJ`'s own adjustments live in
/// (§9.4.3). That is a genuine trap, so it is spelled out in `--help`, in
/// the error text below, and in the save report's disclosure. It is also
/// self-refuting in practice: a `Tc` of 20 *ems* would be a 240 pt gap
/// between every pair of glyphs at 12 pt.
///
/// Returns a human-readable error string (surfaced on stderr) rather than
/// panicking, exactly as [`parse_set_color`] does.
fn parse_text_metric(flag: &str, spec: &str) -> Result<pdfce_core::text_edit::MetricSpec, String> {
    use pdfce_core::text_edit::MetricSpec;
    let raw = spec.trim();
    let (number, relative) = match raw {
        r if r.len() > 2 && r.to_ascii_lowercase().ends_with("em") => (&r[..r.len() - 2], true),
        r if r.len() > 2 && r.to_ascii_lowercase().ends_with("pt") => (&r[..r.len() - 2], false),
        r => (r, false),
    };
    let value: f64 = number.trim().parse().map_err(|_| {
        format!(
            "{flag} {spec:?}: expected a number optionally suffixed `pt` (absolute, \
             unscaled text-space units) or `em` (RELATIVE — thousandths of an em, the tracking \
             unit; `20em` is 20/1000 em, NOT 20 ems)"
        )
    })?;
    if !value.is_finite() {
        return Err(format!("{flag} {spec:?}: not a finite number"));
    }
    Ok(if relative {
        MetricSpec::Relative(value)
    } else {
        MetricSpec::Absolute(value)
    })
}

/// Parse a `--set-color MODEL:C,..` argument into a [`NewFill`]
/// (`rgb:1,0,0`, `cmyk:0,1,1,0`, `gray:0.5`). Returns a human-readable
/// error string (surfaced on stderr) rather than panicking on bad input.
fn parse_set_color(spec: &str) -> Result<pdfce_core::text_edit::NewFill, String> {
    use pdfce_core::text_edit::{FillModel, NewFill};
    let (model_str, comps_str) = spec
        .split_once(':')
        .ok_or_else(|| format!("--set-color {spec:?}: expected MODEL:comps, e.g. rgb:1,0,0"))?;
    let model = match model_str.trim().to_ascii_lowercase().as_str() {
        "rgb" => FillModel::Rgb,
        "cmyk" => FillModel::Cmyk,
        "gray" | "grey" => FillModel::Gray,
        other => {
            return Err(format!(
                "--set-color: unknown model {other:?} (expected rgb, cmyk, or gray)"
            ));
        }
    };
    let mut comps = Vec::new();
    for part in comps_str.split(',') {
        let v: f64 = part
            .trim()
            .parse()
            .map_err(|_| format!("--set-color: {part:?} is not a number"))?;
        comps.push(v);
    }
    NewFill::new(model, comps).map_err(|e| e.to_string())
}

/// `format-text`: Pass 14.2 in-place formatting (size / fill colour /
/// font-family-style).
///
/// Locates `--find` within one show operator on `--page`, applies the
/// requested formatting via the shared advance-preserving surgery, relayouts
/// the line (reflow by default; `--pin` compensates), and saves
/// INCREMENTALLY. Every gate — a coverage refusal on a family change, a
/// missing target font, an invalid colour, an unresolvable (outlined) run —
/// is a clean, named non-zero exit ([`exit::EDIT_REFUSED`]), never a crash.
/// All disclosures (three-trust-level, incremental/prior-state, colour
/// narrowing, tagged-stale, relayout overflow) are surfaced verbatim.
fn cmd_format_text(args: &FormatTextArgs<'_>) -> u8 {
    use pdfce_core::text_edit::{
        EditGlyphSource, FollowerDisposition, FontSelector, FormatError, FormatOptions,
        FormatRequest,
    };

    // The shell owns font discovery (R61): `--font-dir` supplies operator
    // faces for a NON-embedded target's preview/trust level (decision 012).
    let (font_env, supplied_registered, font_notes) = build_font_environment(args.font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }

    // Parse the colour up front so a bad spec fails before any file I/O.
    let fill = match args.set_color {
        Some(spec) => match parse_set_color(spec) {
            Ok(f) => Some(f),
            Err(msg) => {
                eprintln!("pdfce-cli: {msg}");
                return exit::EDIT_REFUSED;
            }
        },
        None => None,
    };
    // Likewise the character-spacing spec — a bad unit suffix must fail
    // before the input file is even opened.
    // …and the three text-space metric specs, which share ONE parser
    // because they share one unit model: `Tc`, `Tw` and `Ts` are all in
    // unscaled text-space units and all governed by R89's
    // Absolute/Relative discrimination (§9.3 Table 105's closing note).
    // One parser, one set of suffixes to learn, no chance of three
    // spellings drifting apart.
    let metric = |flag: &str, spec: Option<&str>| match spec {
        Some(s) => match parse_text_metric(flag, s) {
            Ok(m) => Ok(Some(m)),
            Err(msg) => {
                eprintln!("pdfce-cli: {msg}");
                Err(exit::EDIT_REFUSED)
            }
        },
        None => Ok(None),
    };
    let char_spacing = match metric("--char-spacing", args.char_spacing) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let word_spacing = match metric("--word-spacing", args.word_spacing) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let rise = match metric("--rise", args.rise) {
        Ok(v) => v,
        Err(code) => return code,
    };

    let source = match std::fs::read(args.input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return exit_code_for_doc(&err);
        }
    };

    if args.page == 0 {
        eprintln!("pdfce-cli: --page is 1-based; 0 is not a valid page number");
        return exit::EDIT_REFUSED;
    }

    let mut req = FormatRequest::new(args.page - 1, args.find);
    if let Some(size) = args.set_size {
        req = req.size(size);
    }
    if let Some(f) = fill {
        req = req.fill(f);
    }
    if let Some(name) = args.set_font {
        req = req.font(FontSelector::new(name));
    }
    if let Some(spec) = char_spacing {
        req = req.char_spacing(spec);
    }
    if let Some(spec) = word_spacing {
        req = req.word_spacing(spec);
    }
    if let Some(pct) = args.h_scale {
        req = req.h_scale(pct);
    }
    if let Some(pos) = args.script {
        req = req.script(pos);
    }
    if let Some(spec) = rise {
        req = req.rise(spec);
    }
    // Passing the request through even when it is `None` would be harmless,
    // but doing it explicitly keeps "nothing was asked for" and "nothing was
    // applied" the same statement (R90: never silent, never a default).
    if !args.synthetic.is_none() {
        req = req.synthetic(args.synthetic);
    }
    let opts = FormatOptions::default().with_disposition(if args.pin {
        FollowerDisposition::Pin
    } else {
        FollowerDisposition::Reflow
    });

    let outcome = match pdfce_core::text_edit::set_format(&doc, &req, &opts) {
        Ok(o) => o,
        Err(err) => {
            eprintln!("pdfce-cli: format-text refused: {err}");
            return match err {
                FormatError::Refused(_)
                | FormatError::CoverageFailure(_)
                | FormatError::NoOp
                | FormatError::BadColor(_)
                | FormatError::TargetFontMissing(_)
                | FormatError::NoMatch(_)
                | FormatError::Unsupported(_)
                | FormatError::PageIndex(_)
                | FormatError::AmbientUnrestorable(_)
                | FormatError::BadHorizScale(_)
                | FormatError::WordSpacingComposite { .. }
                | FormatError::ConflictingRise
                | FormatError::RealFaceAvailable { .. }
                | FormatError::ShearUnsupported(_)
                | FormatError::Encrypted => exit::EDIT_REFUSED,
                FormatError::Write(_) => exit::SAVE_REFUSED,
                FormatError::Content(_) | FormatError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::RUNTIME_ERROR,
            };
        }
    };

    if let Err(err) = std::fs::write(args.output, &outcome.bytes) {
        eprintln!("pdfce-cli: {}: {err}", args.output.display());
        return exit::IO_ERROR;
    }

    let report = &outcome.report;
    println!(
        "format-text {} -> {}",
        args.input.display(),
        args.output.display()
    );
    println!("  page={} find={:?}", args.page, args.find);
    // The formatting summary — one field per operation, all present so the
    // line is diffable even when an operation was not requested.
    let size_str = report
        .size_change
        .map_or_else(|| "none".to_owned(), |(o, n)| format!("{o}->{n}"));
    let color_str = report.fill_space.unwrap_or("none");
    let font_str = report
        .font_change
        .as_ref()
        .map_or_else(|| "none".to_owned(), |(o, n)| format!("{o}->{n}"));
    println!("  set_size={size_str} set_color={color_str} set_font={font_str}");
    // Pass 19.1's three controls, printed as `ambient->emitted` so the line
    // is diffable and so the RATIOS pdfce chose are visible by value rather
    // than buried in the code (rule 4).
    let tc_str = report
        .char_spacing_change
        .map_or_else(|| "none".to_owned(), |(o, n)| format!("{o}->{n}"));
    let tz_str = report
        .h_scale_change
        .map_or_else(|| "none".to_owned(), |(o, n)| format!("{o}%->{n}%"));
    let script_str = report.script.map_or_else(
        || "none".to_owned(),
        |p| {
            let rise = report
                .rise_change
                .map_or_else(|| "0".to_owned(), |(_, n)| format!("{n}"));
            let size = report
                .script_size
                .map_or_else(|| "unchanged".to_owned(), |(b, e)| format!("{b}->{e}"));
            format!("{}(Ts={rise} Tf={size})", p.label())
        },
    );
    // Pass 19.4. `word_spacing` prints `ambient->emitted` like its siblings
    // AND the number of code-32s it reaches, because "how many spaces did
    // that touch" is the single question the control's §9.3.3 scope makes
    // load-bearing — a script that widened one gap and moved four is a
    // silent surprise otherwise. A count of 0 prints as 0.
    let tw_str = report.word_spacing_change.map_or_else(
        || "none".to_owned(),
        |(o, n)| {
            let spaces = report
                .word_spacing_affected_codes
                .map_or_else(String::new, |c| format!(" spaces={c}"));
            format!("{o}->{n}{spaces}")
        },
    );
    println!("  char_spacing={tc_str} word_spacing={tw_str} h_scale={tz_str} script={script_str}");
    // Pass 19.2. `rise` is printed whenever the baseline moved — by the
    // free-form control OR by the toggle — because "where is the baseline
    // now" is one question, not two. `synthesis` prints the mechanism's own
    // numbers (stroke width, shear, and the Trise x tan(theta) displacement)
    // so nothing pdfce chose is invisible to the operator (rule 4).
    let rise_str = report
        .rise_change
        .map_or_else(|| "none".to_owned(), |(o, n)| format!("{o}->{n}"));
    let synth_str = if report.synthesis.is_none() {
        "none".to_owned()
    } else {
        let bold = report
            .synthetic_bold_width
            .map_or_else(String::new, |w| format!(" stroke_w={w}"));
        let ital = report.synthetic_italic.map_or_else(String::new, |(t, o)| {
            format!(" shear_tan={t} rise_offset={o}")
        });
        format!("{}{bold}{ital}", report.synthesis)
    };
    println!("  rise={rise_str} synthesis={synth_str}");
    if !report.restore_narrowed.is_empty() {
        let names: Vec<String> = report
            .restore_narrowed
            .iter()
            .map(ToString::to_string)
            .collect();
        println!("  restore_narrowed={}", names.join(","));
    }
    if report.justify_slack_invalidated {
        println!("  justify_slack_invalidated=1");
    }
    println!(
        "  base_font={} content_object={} advance_delta={:.3} followers_repositioned={} fill_narrowed={}",
        report.base_font,
        report.content_object,
        report.advance_delta,
        report.followers_repositioned,
        u8::from(report.fill_narrowed),
    );

    // Refine the core's Embedded/NonEmbedded into the three decision-012
    // trust levels (identical to `edit-text`, via the shared classifier).
    let trust = match report.glyph_source {
        EditGlyphSource::Embedded => "Embedded",
        EditGlyphSource::NonEmbedded => match font_env.classify_nonembedded(&report.base_font) {
            pdfce_render::GlyphSource::Supplied => "Supplied",
            _ => "Bundled",
        },
        _ => "unknown",
    };
    println!(
        "  glyph_source={trust} subset={} supplied_registered={}",
        report.subset, supplied_registered
    );
    if let Some(mcid) = report.tagged_mcid {
        println!("  tagged_mcid={mcid}");
    }
    println!("  disclosures:");
    for d in &report.disclosures {
        println!("    - {d}");
    }
    exit::SUCCESS
}

/// `unembed-font` — remove embedded font programs, dry-run by default.
///
/// # Why the dry run is the default
///
/// `print` requires `--send` before paper moves; this requires `--apply`
/// before bytes move, for the same reason and one more. Unembedding is not
/// reversible from the output file: the program is gone from it, and the
/// only copy of the original is the input the operator still has. A default
/// that writes would make "I wanted to see what it would do" and "do it"
/// the same command.
///
/// The dry run runs the **whole** operation — the inventory, the plan, the
/// sharing census, the PDF/A detection — and prints exactly what `--apply`
/// would print, minus the save. Nothing is estimated.
///
/// # Why every refusal is printed, and Acrobat prints none
///
/// Acrobat refuses a font whose character codes are glyph indices into its
/// own embedded program by leaving it out of the unembed list, with no
/// reason shown anywhere (sourced to a former Adobe Principal Scientist in
/// `Acrobat_Features/optimize__font_unembedding.md`). A shorter list is not
/// actionable. Refusal is also the *majority* path here — 52 % of embedded
/// fonts across a 400-file corpus — so a command that quietly did less than
/// asked would be the normal experience of using it.
///
/// # Why two byte figures are printed and not one
///
/// `reclaim_on_full` is what a full rewrite drops. `reclaim_now` is what
/// this save actually drops, which for the default incremental mode is
/// **zero** — §7.5.6's update section is appended, so the freed program's
/// bytes are still in the prior revision and the output is larger than the
/// input. An operator whose whole goal is a smaller file is exactly the
/// operator most likely to read one number and stop, so both are on the
/// line and the difference is stated on stderr when it bites.
fn cmd_unembed_font(args: &UnembedArgs<'_>) -> u8 {
    use pdfce_core::edit::EditError;
    use pdfce_core::font_unembed::{PdfaClaim, UnembedRequest, UnembedSelection};

    if args.apply && args.output.is_none() {
        eprintln!("pdfce-cli: --apply needs --output <PATH>; a dry run needs neither");
        return exit::EDIT_REFUSED;
    }

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let selection = if args.all_removable {
        UnembedSelection::AllRemovable
    } else {
        UnembedSelection::Named(args.fonts.to_vec())
    };
    // Built through the constructors rather than a struct literal:
    // `UnembedRequest` is `#[non_exhaustive]`, which is the API-guidelines
    // posture for a request type that will grow options.
    let request = match selection {
        UnembedSelection::AllRemovable => UnembedRequest::all_removable(),
        UnembedSelection::Named(names) => UnembedRequest::named(names),
        _ => UnembedRequest::all_removable(),
    };
    let request = if args.keep_subset_tag {
        request.keeping_subset_tag()
    } else {
        request
    };

    // The plan is computed ONCE and printed before anything is decided, so
    // the dry run and the apply are looking at the same evidence.
    let plan = session.unembed_preview(&request);

    println!("unembed-font {}", args.input.display());
    for t in &plan.targets {
        let name = t.base_font.as_deref().unwrap_or("-");
        let rename = t
            .rename
            .as_deref()
            .map_or_else(|| "unchanged".to_owned(), |n| format!("{n:?}"));
        let shared = if t.program_shared_with.is_empty() {
            String::new()
        } else {
            let ids: Vec<String> = t
                .program_shared_with
                .iter()
                .map(|id| format!("{}", id.num))
                .collect();
            format!(" program_shared_with={}", ids.join(","))
        };
        println!(
            "  unembed name={name:?} obj={} key={} bytes={} freed={} rename={rename} \
charset_removed={} cidset_removed={} pages={}{shared}",
            t.id.num,
            t.program_key.label(),
            t.stored_bytes,
            u32::from(t.program_freed),
            u32::from(t.char_set_removed),
            u32::from(t.cid_set_removed),
            pdfce_core::fontinfo::format_page_ranges(&t.pages),
        );
    }
    // ★ The disclosure Acrobat does not make. Every refused font, by name,
    // on stdout with the rest of the report — not hidden on stderr and not
    // omitted, because a font that is missing from both lists is the exact
    // silence this command exists to break.
    for b in &plan.blocked {
        let name = b.base_font.as_deref().unwrap_or("-");
        let obj =
            b.id.map_or_else(|| "direct".to_owned(), |id| format!("{}", id.num));
        println!(
            "  refused name={name:?} obj={obj} bytes={} verdict={}",
            b.stored_bytes,
            b.blocker.token(),
        );
        println!("    reason: {}", b.blocker.reason());
    }
    for name in &plan.unmatched {
        println!("  unmatched {name:?}");
    }

    let reclaim_on_full = plan.bytes_reclaimable();
    let reclaim_now = if matches!(args.mode, SaveMode::Full) {
        reclaim_on_full
    } else {
        0
    };
    println!(
        "  fonts={} refused={} unmatched={} reclaim_on_full={reclaim_on_full} \
reclaim_now={reclaim_now} pdfa={} mode={} applied={}",
        plan.targets.len(),
        plan.blocked.len(),
        plan.unmatched.len(),
        plan.pdfa.token(),
        args.mode.name(),
        u32::from(args.apply),
    );

    // ★ Appearance change, stated as a fact and not a risk, whether or not
    // this run writes anything. It is the consequence an operator is least
    // likely to have thought about and the one they cannot see in a report.
    if !plan.targets.is_empty() {
        eprintln!(
            "pdfce-cli: {}: the pages using these fonts WILL LOOK DIFFERENT. Each glyph keeps its \
exact advance (/Widths is preserved), but the substituted face's own shapes and widths are not \
those numbers, so letters sit differently inside correctly-placed cells.",
            args.input.display()
        );
    }
    if plan.renames_any() {
        eprintln!(
            "pdfce-cli: {}: the six-letter subset tag is being removed from /BaseFont and \
/FontName (ISO 32000-1 §9.6.4, Table 122), because a name like ABCDEF+Arial matches no installed \
font once the program is gone. Pass --keep-subset-tag to leave both alone.",
            args.input.display()
        );
    }
    if !matches!(args.mode, SaveMode::Full) && reclaim_on_full > 0 {
        eprintln!(
            "pdfce-cli: {}: an incremental save RECLAIMS NOTHING — ISO 32000-1 §7.5.6's update \
section is appended, so the removed program's {reclaim_on_full} byte(s) stay in the prior \
revision and the output is LARGER than the input. Use --mode full to drop them.",
            args.input.display()
        );
    }
    for t in &plan.targets {
        if t.program_freed {
            continue;
        }
        eprintln!(
            "pdfce-cli: {}: {:?}'s font program is also reached by {} other font(s) that are NOT \
being unembedded, so the program object stays in the file. This font is unembedded; its bytes \
are not recovered.",
            args.input.display(),
            t.base_font.as_deref().unwrap_or("-"),
            t.program_shared_with.len(),
        );
    }

    // PDF/A: refused before anything is written unless acknowledged. Unlike
    // redaction's residuals — which are only knowable after the removal —
    // this is knowable in advance, so the operator gets the choice rather
    // than the news.
    if let PdfaClaim::Identified { part, conformance } = &plan.pdfa {
        let level = format!(
            "PDF/A-{}{}",
            part.as_deref().unwrap_or("?"),
            conformance.as_deref().unwrap_or("")
        );
        eprintln!(
            "pdfce-cli: {}: this document identifies itself as {level} (XMP pdfaid). EVERY part \
of ISO 19005 requires fonts to be embedded, so unembedding breaks that conformance, and pdfce \
does not remove or correct the claim for you.",
            args.input.display()
        );
        if args.apply && !args.acknowledge_pdfa {
            eprintln!(
                "pdfce-cli: refusing to write: pass --acknowledge-pdfa to proceed anyway. \
Nothing has been changed."
            );
            return exit::EDIT_REFUSED;
        }
    } else if matches!(plan.pdfa, PdfaClaim::MetadataUnreadable) {
        eprintln!(
            "pdfce-cli: {}: this document's XMP metadata could not be read, so pdfce could NOT \
check whether it claims PDF/A conformance. That is not the same as finding no claim.",
            args.input.display()
        );
    }

    if !plan.unmatched.is_empty() {
        eprintln!(
            "pdfce-cli: {}: {} --font name(s) matched no font in this document. Run `list-fonts` \
to see the names it actually carries.",
            args.input.display(),
            plan.unmatched.len()
        );
        return exit::EDIT_REFUSED;
    }

    if !args.apply {
        if plan.targets.is_empty() {
            eprintln!(
                "pdfce-cli: {}: DRY RUN — nothing would be unembedded. Every refusal is printed \
above with its reason.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
        eprintln!(
            "pdfce-cli: {}: DRY RUN — no file was written. Re-run with --apply --output <PATH> \
to perform this.",
            args.input.display()
        );
        return exit::SUCCESS;
    }

    let applied = match session.unembed_fonts(&request) {
        Ok(applied) => applied,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return match err {
                EditError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::EDIT_REFUSED,
            };
        }
    };
    // The plan the operator read and the plan that ran are the same value,
    // produced by the same function. Saying so costs one comparison and
    // makes a future divergence a test failure rather than a surprise.
    debug_assert_eq!(applied.targets.len(), plan.targets.len());

    // A signed document: disclosed, never silently broken. `impact_of` is
    // asked AFTER the edit and immediately before the save, because §11.1
    // makes the dirty set a save-time diff — the answer is not knowable at
    // edit time.
    let impact = session.signature_impact_of_save(match args.mode {
        SaveMode::Incremental => CoreSaveMode::Incremental,
        SaveMode::Full => CoreSaveMode::FullRewrite,
    });
    println!("  signature_impact={impact:?}");

    let Some(output) = args.output else {
        eprintln!("pdfce-cli: --apply needs --output <PATH>");
        return exit::EDIT_REFUSED;
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "  wrote {} objects={} verbatim={} reserialized={} appended={} out_bytes={} \
in_bytes={} undo_verified={} undo_identical={}",
        output.display(),
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.bytes_appended,
        r.bytes_written,
        source.len(),
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// `embed-font` — add the font programs a document is missing, dry-run by
/// default.
///
/// # Why a dry run is the default here too, when nothing is destroyed
///
/// `unembed-font` defaults to a dry run because it removes something the
/// output cannot get back. This command removes nothing, so that argument
/// does not transfer — and the default is the same anyway, for a different
/// reason.
///
/// **Embedding is an inference.** pdfce is choosing font programs the
/// document did not carry, and on a typical run some of those choices are
/// stand-ins rather than the face the file names. The operator has to be
/// able to see WHICH before it becomes document state (project rule 4), and
/// a command that wrote on the first invocation would make "show me what you
/// would pick" and "pick it" the same act. It is also the shape `print`
/// (`--send`) and `unembed-font` (`--apply`) already established, and a
/// third font command with a different default would be the surprise.
///
/// # What the report says, and why each column is on it
///
/// Per resolved font: the face chosen, the file it came from, and
/// `match=exact|alias|bundled` — the disclosure rule 4 requires. Per refused
/// font: its name and a reason that says what would satisfy it. Then, on the
/// summary line, **`not_embedded_after`** — the number the operator is
/// actually trying to drive to zero. A report that showed only what pdfce
/// managed to do would read as success over a file a print service will
/// still reject.
fn cmd_embed_font(args: &EmbedArgs<'_>) -> u8 {
    use pdfce_core::edit::EditError;
    use pdfce_core::font_embed_missing::{EmbedRequest, EmbedSelection, FontMatch, SuppliedFont};
    use pdfce_core::fontinfo::Program;
    use pdfce_render::font::EmbedMatch;

    if args.apply && args.output.is_none() {
        eprintln!("pdfce-cli: --apply needs --output <PATH>; a dry run needs neither");
        return exit::EDIT_REFUSED;
    }

    // The SHELL owns the filesystem: the environment is built here and
    // `pdfce-core` is handed bytes (project rule 2). Reuses `--font-dir`'s
    // one walker rather than a second one (R171).
    let (font_env, supplied_registered, font_notes) = build_font_environment(args.font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }
    if supplied_registered == 0 && !args.use_bundled_fonts {
        eprintln!(
            "pdfce-cli: no font folder supplied any usable face. Pass --font-dir <DIR> pointing \
at a folder of font files (on Windows, C:\\Windows\\Fonts), or --use-bundled-fonts to offer \
pdfce's own standard-14 substitutes."
        );
    }

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // Resolve a donor for every font the document is missing. The inventory
    // is phase A's, consumed rather than re-derived.
    let inventory = pdfce_core::fontinfo::inventory(&session.view());
    let selection = if args.all_missing {
        EmbedSelection::AllMissing
    } else {
        EmbedSelection::Named(args.fonts.to_vec())
    };
    let mut request = match &selection {
        EmbedSelection::Named(names) => EmbedRequest::named(names.clone()),
        _ => EmbedRequest::all_missing(),
    };
    let mut resolutions: Vec<(String, String, String, FontMatch)> = Vec::new();
    for record in &inventory.fonts {
        if !matches!(record.program, Program::NotEmbedded) {
            continue;
        }
        let Some(base_font) = record.base_font.as_deref() else {
            continue;
        };
        let Some(donor) = font_env.resolve_for_embedding(base_font, args.use_bundled_fonts) else {
            continue;
        };
        let matched = match donor.quality {
            EmbedMatch::Exact => FontMatch::Exact,
            EmbedMatch::Alias => FontMatch::Alias,
            EmbedMatch::Bundled => FontMatch::Bundled,
        };
        // A bundled face has no path; the source string says so in words
        // rather than printing an empty field.
        let source_label = if matches!(matched, FontMatch::Bundled) {
            format!("bundled: {}", donor.face_name)
        } else {
            format!("--font-dir face {:?}", donor.face_name)
        };
        resolutions.push((
            base_font.to_owned(),
            donor.face_name.clone(),
            source_label.clone(),
            matched,
        ));
        request = request.with_font(
            base_font,
            SuppliedFont::new(
                donor.data.bytes().to_vec(),
                donor.face_name.clone(),
                source_label,
                matched,
            ),
        );
    }

    // Computed ONCE and printed before anything is decided, so the dry run
    // and the apply are looking at the same evidence.
    let plan = session.embed_preview(&request);

    println!("embed-font {}", args.input.display());
    println!(
        "  supplied_registered={supplied_registered} resolved={} bundled_allowed={}",
        resolutions.len(),
        u32::from(args.use_bundled_fonts),
    );
    for t in &plan.targets {
        let name = t.base_font.as_deref().unwrap_or("-");
        println!(
            "  embed name={name:?} obj={} shape={} key={} subtype={} format={} face={:?} \
match={} bytes={} redeclared={} widths={} encoding={} descriptor={} rename={} pages={}",
            t.id.num,
            t.shape.token(),
            t.program_key.label(),
            t.stream_subtype.unwrap_or("-"),
            t.format.token(),
            t.face_name,
            t.matched.token(),
            t.program_bytes,
            u32::from(t.redeclared_truetype),
            t.widths_written,
            u32::from(t.encoding_written),
            u32::from(t.descriptor_written),
            t.rename.as_deref().unwrap_or("unchanged"),
            pdfce_core::fontinfo::format_page_ranges(&t.pages),
        );
        println!("    source: {}", t.source);
    }
    // Every refused font, by name, on stdout with the rest of the report —
    // the same disclosure posture `unembed-font` takes, and for the same
    // reason: a font missing from both lists is a silence the operator
    // cannot act on.
    for b in &plan.blocked {
        // "Already embedded" is not a refusal an operator needs a paragraph
        // about; it is the answer to "why is this row not in the list".
        let name = b.base_font.as_deref().unwrap_or("-");
        let obj =
            b.id.map_or_else(|| "direct".to_owned(), |id| format!("{}", id.num));
        println!(
            "  refused name={name:?} obj={obj} reason={}",
            b.blocker.token()
        );
        if b.blocker.token() != "already-embedded" {
            println!("    reason: {}", b.blocker.reason());
        }
    }
    for name in &plan.unmatched {
        println!("  unmatched {name:?}");
    }

    let substitutes = plan
        .targets
        .iter()
        .filter(|t| t.matched.is_substitute())
        .count();
    println!(
        "  fonts={} exact={} substitute={} refused={} unmatched={} bytes_added_max={} \
not_embedded_before={} not_embedded_after={} pdfa={} mode={} applied={}",
        plan.targets.len(),
        plan.targets.len() - substitutes,
        substitutes,
        plan.blocked.len(),
        plan.unmatched.len(),
        plan.bytes_added_uncompressed(),
        plan.missing_before,
        plan.missing_after(),
        plan.pdfa.token(),
        args.mode.name(),
        u32::from(args.apply),
    );

    // ★ The two disclosures rule 4 requires, stated as facts rather than
    // implied by the report's shape.
    if !plan.targets.is_empty() {
        eprintln!(
            "pdfce-cli: {}: character POSITIONS do not change — a PDF spaces text from its own \
/Widths array, which is preserved or written from the standard metrics a reader was already \
using. The LETTERFORMS will differ wherever the face embedded is not the one the document \
names.",
            args.input.display()
        );
    }
    if substitutes > 0 {
        eprintln!(
            "pdfce-cli: {}: {substitutes} of these use a STAND-IN face, not the one the document \
names. Each is printed above with match=alias or match=bundled and the face actually used.",
            args.input.display()
        );
    }
    if plan.redeclares_any() {
        eprintln!(
            "pdfce-cli: {}: one or more fonts are being re-declared from a PostScript font to a \
TrueType font, because the face supplied for them carries TrueType outlines and ISO 32000-1 \
§9.9 Table 126 admits no other way to attach one. The character mapping is written out \
explicitly at the same time, so the text is unchanged.",
            args.input.display()
        );
    }
    if plan.missing_after() > 0 {
        eprintln!(
            "pdfce-cli: {}: {} font(s) will STILL have no embedded program. Every one is listed \
above with its reason. A service that requires embedded fonts will still reject this file.",
            args.input.display(),
            plan.missing_after()
        );
    }

    if !plan.unmatched.is_empty() {
        eprintln!(
            "pdfce-cli: {}: {} --font name(s) matched no font in this document. Run `list-fonts` \
to see the names it actually carries.",
            args.input.display(),
            plan.unmatched.len()
        );
        return exit::EDIT_REFUSED;
    }

    if !args.apply {
        if plan.targets.is_empty() {
            eprintln!(
                "pdfce-cli: {}: DRY RUN — nothing would be embedded. Every refusal is printed \
above with its reason.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
        eprintln!(
            "pdfce-cli: {}: DRY RUN — no file was written. Re-run with --apply --output <PATH> \
to perform this.",
            args.input.display()
        );
        return exit::SUCCESS;
    }

    let applied = match session.embed_fonts(&request) {
        Ok(applied) => applied,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.input.display());
            return match err {
                EditError::PageTree(_) => exit::RUNTIME_ERROR,
                _ => exit::EDIT_REFUSED,
            };
        }
    };
    // The plan the operator read and the plan that ran are the same value,
    // produced by the same function.
    debug_assert_eq!(applied.targets.len(), plan.targets.len());

    let impact = session.signature_impact_of_save(match args.mode {
        SaveMode::Incremental => CoreSaveMode::Incremental,
        SaveMode::Full => CoreSaveMode::FullRewrite,
    });
    println!("  signature_impact={impact:?}");

    let Some(output) = args.output else {
        eprintln!("pdfce-cli: --apply needs --output <PATH>");
        return exit::EDIT_REFUSED;
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "  wrote {} objects={} verbatim={} reserialized={} appended={} out_bytes={} \
in_bytes={} undo_verified={} undo_identical={}",
        output.display(),
        r.objects_written,
        r.objects_verbatim,
        r.objects_reserialized,
        r.bytes_appended,
        r.bytes_written,
        source.len(),
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// The parsed `embed-font` flags, gathered for the same reason
/// [`UnembedArgs`] is.
struct EmbedArgs<'a> {
    input: &'a Path,
    fonts: &'a [String],
    all_missing: bool,
    font_dirs: &'a [PathBuf],
    use_bundled_fonts: bool,
    apply: bool,
    output: Option<&'a Path>,
    mode: SaveMode,
    verify_undo: bool,
}

/// The parsed `unembed-font` flags, gathered so the implementation takes one
/// parameter rather than nine — `clippy::too_many_arguments` is a real
/// readability signal here and not a formality.
struct UnembedArgs<'a> {
    input: &'a Path,
    fonts: &'a [String],
    all_removable: bool,
    apply: bool,
    output: Option<&'a Path>,
    mode: SaveMode,
    verify_undo: bool,
    keep_subset_tag: bool,
    acknowledge_pdfa: bool,
}

fn cmd_redact_apply(input: &Path, output: &Path, acknowledge_residuals: bool) -> u8 {
    use pdfce_core::redact::{self, RedactError};
    use pdfce_core::writer::SaveOptions;

    let source = match std::fs::read(input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };

    let (bytes, report) = match redact::apply_redactions(&doc, &SaveOptions::identity()) {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("pdfce-cli: redaction refused: {err}");
            return match err {
                RedactError::ImageRegion { .. }
                | RedactError::NothingToApply
                | RedactError::Encrypted => exit::EDIT_REFUSED,
                RedactError::Write(_) => exit::SAVE_REFUSED,
                _ => exit::RUNTIME_ERROR,
            };
        }
    };

    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }

    // The REDACTION REPORT — exactly what was removed and what was left.
    println!("redact-apply {} -> {}", input.display(), output.display());
    println!(
        "  pages_redacted={} marks_applied={} glyphs_removed={} show_operators_edited={}",
        report.pages_redacted,
        report.marks_applied,
        report.glyphs_removed,
        report.show_operators_edited,
    );
    println!(
        "  content_streams_rewritten={} annotations_removed={} info_strings_scrubbed={}",
        report.content_streams_rewritten, report.annotations_removed, report.info_strings_scrubbed,
    );
    println!(
        "  containers_decomposed={} objects_promoted={} estimated_width_fonts={} out_bytes={}",
        report.containers_decomposed,
        report.objects_promoted,
        report.estimated_width_fonts,
        bytes.len(),
    );
    println!("  carriers (diligence sweep, ISO 32000-1 §12.5.6.23):");
    for c in &report.carriers {
        println!(
            "    {:<24} present={} action={}",
            c.carrier,
            u32::from(c.present),
            c.action.as_str()
        );
    }
    if !report.notes.is_empty() {
        println!("  notes:");
        for note in &report.notes {
            println!("    - {note}");
        }
    }

    // The refusal-acknowledgement gate.
    if report.has_disclosed_residuals() && !acknowledge_residuals {
        eprintln!(
            "pdfce-cli: the covered content WAS removed, but one or more diligence carriers could \
             not be scrubbed and were DISCLOSED above (see action=DISCLOSED_NOT_SCRUBBED). Review \
             them, then re-run with --acknowledge-residuals to exit 0."
        );
        return exit::REDACTION_RESIDUALS;
    }
    exit::SUCCESS
}

/// `list-redactions`: report the `/Redact` marks awaiting apply, from the
/// document's own annotations (never a session counter).
fn cmd_list_redactions(input: &Path) -> u8 {
    let source = match std::fs::read(input) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::IO_ERROR;
        }
    };
    let doc = match open_document_bytes(source) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let count = pdfce_core::redact::count_redaction_marks(&doc);
    println!(
        "list-redactions {}: {count} unapplied /Redact mark(s)",
        input.display()
    );
    if count > 0 {
        eprintln!(
            "pdfce-cli: WARNING — this document carries {count} UNAPPLIED redaction mark(s); its \
             content is NOT yet redacted. Run `redact-apply` before sharing it."
        );
    }
    exit::SUCCESS
}

/// Build a [`MarkupSpec`](pdfce_core::annot_author::MarkupSpec) from the
/// parsed `annotate` flags, or a human-readable error naming the missing
/// or malformed geometry.
fn build_markup_spec(
    args: &AnnotateArgs<'_>,
) -> Result<pdfce_core::annot_author::MarkupSpec, String> {
    use pdfce_core::annot_author::{Color, LineEnding, MarkupSpec, Quad, TextMarkupKind};

    // Per-subtype default mark colour (pdfce's own defaults; the Acrobat
    // RAG marks most of these a GAP — highlight yellow is the sourced one).
    let default_color = match args.kind {
        AnnotKindArg::Highlight => Color::Rgb(1.0, 1.0, 0.0),
        _ => Color::Rgb(1.0, 0.0, 0.0),
    };
    let color = match args.color {
        Some(h) => parse_color(h)?,
        None => default_color,
    };
    let interior = match args.fill {
        Some(h) => Some(parse_color(h)?),
        None => None,
    };

    match args.kind {
        AnnotKindArg::Square | AnnotKindArg::Circle => {
            let r = rect_from(args.rect.ok_or("this subtype needs --rect x0,y0,x1,y1")?)?;
            if matches!(args.kind, AnnotKindArg::Square) {
                Ok(MarkupSpec::Square {
                    rect: r,
                    border: Some(color),
                    interior,
                    border_width: args.width,
                })
            } else {
                Ok(MarkupSpec::Circle {
                    rect: r,
                    border: Some(color),
                    interior,
                    border_width: args.width,
                })
            }
        }
        AnnotKindArg::Line => {
            let f = parse_floats(args.line.ok_or("line needs --line x0,y0,x1,y1")?)?;
            let [x0, y0, x1, y1] = <[f64; 4]>::try_from(f)
                .map_err(|_| "--line needs exactly four numbers".to_owned())?;
            Ok(MarkupSpec::Line {
                start: (x0, y0),
                end: (x1, y1),
                color,
                width: args.width,
                endings: (LineEnding::OpenArrow, LineEnding::OpenArrow),
            })
        }
        AnnotKindArg::Ink => {
            let strokes = parse_strokes(args.strokes.ok_or("ink needs --strokes")?)?;
            Ok(MarkupSpec::Ink {
                strokes,
                color,
                width: args.width,
            })
        }
        AnnotKindArg::Polygon | AnnotKindArg::Polyline => {
            let verts = parse_points(args.points.ok_or("this subtype needs --points")?)?;
            if matches!(args.kind, AnnotKindArg::Polygon) {
                Ok(MarkupSpec::Polygon {
                    vertices: verts,
                    border: Some(color),
                    interior,
                    width: args.width,
                })
            } else {
                Ok(MarkupSpec::PolyLine {
                    vertices: verts,
                    color,
                    width: args.width,
                })
            }
        }
        AnnotKindArg::Highlight
        | AnnotKindArg::Underline
        | AnnotKindArg::Strikeout
        | AnnotKindArg::Squiggly => {
            let quads = match (args.quads, args.rect) {
                (Some(q), _) => parse_quads(q)?,
                (None, Some(r)) => vec![Quad::from_rect(rect_from(r)?)],
                (None, None) => {
                    return Err("text markup needs --quads or --rect".to_owned());
                }
            };
            let kind = match args.kind {
                AnnotKindArg::Highlight => TextMarkupKind::Highlight,
                AnnotKindArg::Underline => TextMarkupKind::Underline,
                AnnotKindArg::Strikeout => TextMarkupKind::StrikeOut,
                _ => TextMarkupKind::Squiggly,
            };
            Ok(MarkupSpec::TextMarkup { kind, quads, color })
        }
        // The Pass-6.2 text-bearing subtypes take the variable-text path
        // (build_text_annot_spec), never this geometric one.
        AnnotKindArg::Freetext | AnnotKindArg::Text | AnnotKindArg::Stamp => {
            Err("internal: text subtype routed to the geometric path".to_owned())
        }
    }
}

/// Build a [`TextAnnotSpec`](pdfce_core::annot_author::TextAnnotSpec) from
/// the parsed `annotate` flags for a Pass-6.2 text-bearing subtype.
fn build_text_annot_spec(
    args: &AnnotateArgs<'_>,
) -> Result<pdfce_core::annot_author::TextAnnotSpec, String> {
    use pdfce_core::annot_author::{Color, TextAnnotSpec};
    use pdfce_core::page_tree::Rect;
    use pdfce_core::vartext::TextColor;

    let font = resolve_latin_std14(args.font)?;
    match args.kind {
        AnnotKindArg::Freetext => {
            let rect = rect_from(args.rect.ok_or("freetext needs --rect x0,y0,x1,y1")?)?;
            let text = args.text.ok_or("freetext needs --text \"…\"")?.to_owned();
            let color = match args.color {
                Some(h) => TextColor::from(parse_color(h)?),
                None => TextColor::Gray(0.0),
            };
            // --fill doubles as the optional FreeText border colour.
            let border = match args.fill {
                Some(h) => Some(parse_color(h)?),
                None => None,
            };
            Ok(TextAnnotSpec::FreeText {
                rect,
                text,
                font,
                font_size: args.size,
                color,
                quadding: args.quad.to_quadding(),
                multiline: args.multiline,
                border,
                border_width: args.width,
            })
        }
        AnnotKindArg::Text => {
            let rect = match args.rect {
                Some(r) => rect_from(r)?,
                None => Rect::from_corners(72.0, 72.0, 96.0, 96.0),
            };
            let color = match args.color {
                Some(h) => parse_color(h)?,
                None => Color::Rgb(1.0, 0.92, 0.30), // pdfce's own note yellow
            };
            Ok(TextAnnotSpec::Sticky {
                rect,
                icon: args.icon.to_icon(),
                contents: args.text.unwrap_or_default().to_owned(),
                color,
                open: false,
            })
        }
        AnnotKindArg::Stamp => {
            let rect = rect_from(args.rect.ok_or("stamp needs --rect x0,y0,x1,y1")?)?;
            let color = match args.color {
                Some(h) => parse_color(h)?,
                None => Color::Rgb(0.80, 0.10, 0.10), // pdfce's own stamp red
            };
            Ok(TextAnnotSpec::Stamp {
                rect,
                name: args.stamp_name.to_stamp_name(),
                label: args.text.map(str::to_owned),
                color,
            })
        }
        // The geometric subtypes never reach here (is_text_bearing gates).
        _ => Err("internal: non-text subtype routed to the text path".to_owned()),
    }
}

/// Resolve a `--font` value to a **Latin** standard-14 face, rejecting the
/// two symbolic fonts (they carry no `WinAnsi` encoding, so pdfce's Latin
/// variable-text generator cannot lay text out in them) and any
/// non-standard-14 name (pdfce authors only program-free standard-14 text
/// appearances — §9.6.2.1).
fn resolve_latin_std14(name: &str) -> Result<pdfce_core::fontdata::Std14, String> {
    use pdfce_core::fontdata::{Std14, std14_by_base_font};
    match std14_by_base_font(name) {
        Some(Std14::Symbol | Std14::ZapfDingbats) => Err(format!(
            "{name} is a symbolic font; text annotations need a Latin standard-14 font \
             (Helvetica, Times-Roman, Courier, and their Bold/Italic variants)"
        )),
        Some(f) => Ok(f),
        None => Err(format!(
            "{name:?} is not a standard-14 font name (e.g. Helvetica, Helvetica-Bold, \
             Times-Roman, Times-Italic, Courier)"
        )),
    }
}

/// Parse a whitespace/comma-separated list of decimal numbers.
fn parse_floats(s: &str) -> Result<Vec<f64>, String> {
    s.split([',', ' ', '\t', '\n', '\r'])
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>().map_err(|_| format!("not a number: {t}")))
        .collect()
}

/// Parse `x0,y0,x1,y1` into a normalized [`Rect`](pdfce_core::page_tree::Rect).
fn rect_from(s: &str) -> Result<pdfce_core::page_tree::Rect, String> {
    let f = parse_floats(s)?;
    let [x0, y0, x1, y1] =
        <[f64; 4]>::try_from(f).map_err(|_| "a rectangle needs exactly four numbers".to_owned())?;
    Ok(pdfce_core::page_tree::Rect::from_corners(x0, y0, x1, y1))
}

/// Parse `x,y x,y …` into a point list.
fn parse_points(s: &str) -> Result<Vec<(f64, f64)>, String> {
    let f = parse_floats(s)?;
    if f.len() % 2 != 0 {
        return Err("points must be an even count of numbers (x,y pairs)".to_owned());
    }
    Ok(f.chunks_exact(2).map(|c| (c[0], c[1])).collect())
}

/// Parse `x,y … | x,y …` into ink strokes (`|` separates strokes).
fn parse_strokes(s: &str) -> Result<Vec<Vec<(f64, f64)>>, String> {
    s.split('|')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(parse_points)
        .collect()
}

/// Parse `x1,y1,…,x8,y8 ; …` into text-markup quads (Z-order UL,UR,LL,LR).
fn parse_quads(s: &str) -> Result<Vec<pdfce_core::annot_author::Quad>, String> {
    use pdfce_core::annot_author::Quad;
    s.split(';')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(|g| {
            let f = parse_floats(g)?;
            let a = <[f64; 8]>::try_from(f)
                .map_err(|_| "each quad needs exactly eight numbers".to_owned())?;
            Ok(Quad {
                ul: (a[0], a[1]),
                ur: (a[2], a[3]),
                ll: (a[4], a[5]),
                lr: (a[6], a[7]),
            })
        })
        .collect()
}

/// Parse an `RRGGBB` (optionally `#`-prefixed) hex colour into a device
/// RGB [`Color`](pdfce_core::annot_author::Color).
fn parse_color(hex: &str) -> Result<pdfce_core::annot_author::Color, String> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return Err(format!("colour must be RRGGBB hex, got {hex:?}"));
    }
    let component = |i: usize| -> Result<f64, String> {
        u8::from_str_radix(h.get(i..i + 2).unwrap_or(""), 16)
            .map(|v| f64::from(v) / 255.0)
            .map_err(|_| format!("colour must be RRGGBB hex, got {hex:?}"))
    };
    Ok(pdfce_core::annot_author::Color::Rgb(
        component(0)?,
        component(2)?,
        component(4)?,
    ))
}

// ---------------------------------------------------------------------------
// Text extraction (Pass 4)
// ---------------------------------------------------------------------------

/// Implement `pdfce-cli extract-text`.
///
/// ## Where the two output channels go, and why
///
/// The subcommand has two things to say — the document's text, and the
/// counters describing how much of that text is actually the document's
/// — and mixing them on one stream would make either one unparseable.
/// So:
///
/// | invocation | stdout | stderr | file |
/// |---|---|---|---|
/// | `extract-text f.pdf` | the text | the result line | — |
/// | `extract-text f.pdf -o t.txt` | the result line | — | the text |
/// | `extract-text f.pdf --json` | the JSON | — | — |
/// | `extract-text f.pdf --json -o t.json` | the result line | — | the JSON |
///
/// The rule underneath: **the result line goes to stdout unless stdout
/// is carrying the payload**, in which case it goes to stderr. A shell
/// pipeline (`extract-text f.pdf | grep …`) gets clean text and still
/// sees the honesty line on the terminal; a script that redirects with
/// `-o` gets the counters on stdout where it can read them.
///
/// The result line follows the R5 stable-line contract used by every
/// other subcommand: `key=value`, fixed order, **appended to and never
/// reordered**.
fn cmd_extract_text(
    input: &Path,
    pages_spec: &str,
    output: Option<&Path>,
    json: bool,
    include_artifacts: bool,
) -> u8 {
    use pdfce_core::text_extract::{self, ExtractOptions};

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let page_list = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let indices = match parse_pages(pages_spec, page_list.len()) {
        Ok(indices) => indices,
        Err(message) => {
            eprintln!("pdfce-cli: {}: --pages {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };

    // The operator's persisted word-gap ratio. Extraction heuristics are
    // the one place where a document that reads fine to a human can
    // extract wrong, so this is a knob worth honouring rather than a
    // constant worth defending.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let options = ExtractOptions::default()
        .with_artifacts(include_artifacts)
        .with_word_gap_ratio(settings.word_gap_ratio)
        // The two EXTRACT-radius R169 knobs. Both move character offsets,
        // so both move what a text search and a text-based redaction
        // match (R35) — which is why they are honoured here rather than
        // left to a hard-coded constant nobody can see.
        .with_unmappable_code(settings.unmappable_code)
        .with_actual_text(settings.actual_text);
    let extracted = match text_extract::extract_pages(&doc, &indices, &options) {
        Ok(extracted) => extracted,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    let payload = if json {
        extraction_json(input, &extracted)
    } else {
        // A trailing newline on the text form: without it a shell prompt
        // lands mid-line after the last extracted word, and a `cat` of
        // the `-o` file runs into the next command. The JSON form already
        // ends with one.
        let mut text = extracted.plain_text();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text
    };

    // Deliver the payload.
    match output {
        Some(path) => {
            if let Err(err) = std::fs::write(path, payload.as_bytes()) {
                eprintln!("pdfce-cli: {}: {err}", path.display());
                return exit::IO_ERROR;
            }
        }
        None => print!("{payload}"),
    }

    // The stable result line, on whichever stream is free.
    let d = &extracted.diagnostics;
    let line = format!(
        "extracted {} pages={} chars={} codes={} \
via_tounicode={} via_encoding={} via_cid={} via_extension={} failed={} \
sourced_pct={:.1} spaces_derived={} lines_derived={} \
actual_text={} artifacts={} reversed={} identity_no_tounicode={} \
ucs2_missing={} predefined_cmaps_missing={} tagged={} suspects={} \
struct_tree={} forms={} rtl_runs={} invisible={} unreadable_pages={} \
contents_unresolved={}",
        input.display(),
        extracted.pages.len(),
        extracted.plain_text().chars().count(),
        d.codes_total,
        d.via_to_unicode,
        d.via_encoding_agl,
        d.via_cid_collection,
        d.via_glyph_name_extension,
        d.ladder_failures,
        d.sourced_fraction().unwrap_or(0.0) * 100.0,
        d.spaces_derived,
        d.lines_derived,
        d.actual_text_applied,
        d.artifact_sequences,
        d.reversed_chars_sequences,
        d.identity_fonts_without_to_unicode,
        d.ucs2_cmaps_unavailable,
        d.predefined_cmaps_unavailable,
        d.tagged,
        d.suspects,
        d.struct_tree_present,
        d.forms_executed,
        d.rtl_runs,
        d.invisible_glyphs,
        d.pages_unreadable,
        // Appended after every pre-existing key (the stable-line
        // contract's append-never-reorder rule): content streams the
        // pages named but the file does not contain, so their text is
        // missing from this extraction rather than absent from the
        // document.
        d.contents_unresolved,
    );
    if output.is_some() || json {
        // stdout is free (the payload went to a file), or the payload is
        // JSON that already carries everything — either way stdout is
        // the right home for the machine-readable line. The `--json`
        // to-stdout case is the one exception: there the JSON *is* the
        // payload on stdout, so the line goes to stderr.
        if json && output.is_none() {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    } else {
        eprintln!("{line}");
    }

    // Named diagnostics on stderr, always — they are the whole point of
    // "fuzzy, never sneaky" and a run that hid them would be the thing
    // rule 4 exists to prevent.
    for note in &d.notes {
        eprintln!("pdfce-cli: {note}");
    }

    exit::SUCCESS
}

/// Serialize an extraction to JSON.
///
/// Hand-rolled rather than reaching for `serde_json`, deliberately: this
/// is the only JSON surface in the whole workspace, the schema is fixed
/// here, and `docs/LEGAL.md` §6 makes every added dependency a decision
/// with a license classification and a `THIRD_PARTY_LICENSES.md`
/// regeneration behind it. Sixty lines of string building is the cheaper
/// side of that trade. Escaping goes through [`json_escape`], which is
/// the only part that can be got wrong.
///
/// ## Schema
///
/// ```jsonc
/// {
///   "input": "...",
///   "include_artifacts": false,
///   "diagnostics": { /* every TextDiagnostics counter, flat */ },
///   "notes": ["text: ..."],
///   "pages": [
///     { "page": 1,                       // 1-based
///       "runs": [
///         { "origin": "glyphs",          // glyphs | actual_text |
///                                        // derived_word_space |
///                                        // derived_line_break
///           "sourced": true,             // did this come from the FILE?
///           "text": "Hello",
///           "artifact": "pagination",    // omitted when not an artifact
///           "mcid": 0,                   // omitted when absent
///           "bbox": [llx, lly, urx, ury],
///           "glyphs": [
///             { "code": 72, "rung": "encoding_agl", "sourced": true,
///               "start": 0, "len": 1,
///               "x": 72.0, "y": 700.0, "advance": 17.3, "size": 24.0,
///               "invisible": false }
///           ] } ] } ]
/// }
/// ```
///
/// The two `sourced` booleans are the schema's reason to exist. A
/// consumer that filters runs on `sourced == true` gets exactly the
/// characters the document provides; one that additionally filters
/// glyphs on `sourced == true` drops the U+FFFD the ladder could not
/// resolve.
fn extraction_json(input: &Path, extracted: &pdfce_core::text_extract::ExtractedText) -> String {
    let d = &extracted.diagnostics;
    let mut out = String::with_capacity(4096);
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"input\": \"{}\",\n",
        json_escape(&input.display().to_string())
    ));
    out.push_str(&format!(
        "  \"include_artifacts\": {},\n",
        extracted.includes_artifacts()
    ));

    out.push_str("  \"diagnostics\": {\n");
    let counters: [(&str, u64); 20] = [
        ("codes_total", d.codes_total),
        ("via_to_unicode", d.via_to_unicode),
        ("via_encoding_agl", d.via_encoding_agl),
        ("via_cid_collection", d.via_cid_collection),
        ("via_glyph_name_extension", d.via_glyph_name_extension),
        ("ladder_failures", d.ladder_failures),
        (
            "identity_fonts_without_to_unicode",
            d.identity_fonts_without_to_unicode,
        ),
        ("ucs2_cmaps_unavailable", d.ucs2_cmaps_unavailable),
        (
            "predefined_cmaps_unavailable",
            d.predefined_cmaps_unavailable,
        ),
        ("actual_text_applied", d.actual_text_applied),
        ("actual_text_suppressions", d.actual_text_suppressions),
        ("alt_entries", d.alt_entries),
        ("expansion_entries", d.expansion_entries),
        ("artifact_sequences", d.artifact_sequences),
        ("artifact_chars", d.artifact_chars),
        ("reversed_chars_sequences", d.reversed_chars_sequences),
        ("spaces_derived", d.spaces_derived),
        ("lines_derived", d.lines_derived),
        ("rtl_runs", d.rtl_runs),
        ("invisible_glyphs", d.invisible_glyphs),
    ];
    for (name, value) in counters {
        out.push_str(&format!("    \"{name}\": {value},\n"));
    }
    out.push_str(&format!("    \"tagged\": {},\n", d.tagged));
    out.push_str(&format!("    \"suspects\": {},\n", d.suspects));
    out.push_str(&format!(
        "    \"struct_tree_present\": {},\n",
        d.struct_tree_present
    ));
    out.push_str(&format!(
        "    \"tag_suspect_sequences\": {},\n",
        d.tag_suspect_sequences
    ));
    out.push_str(&format!("    \"forms_executed\": {},\n", d.forms_executed));
    out.push_str(&format!(
        "    \"form_depth_overflows\": {},\n",
        d.form_depth_overflows
    ));
    out.push_str(&format!(
        "    \"pages_unreadable\": {},\n",
        d.pages_unreadable
    ));
    out.push_str(&format!(
        "    \"fonts_with_estimated_widths\": {},\n",
        d.fonts_with_estimated_widths
    ));
    out.push_str(&format!(
        "    \"sourced_fraction\": {:.6}\n",
        d.sourced_fraction().unwrap_or(0.0)
    ));
    out.push_str("  },\n");

    out.push_str("  \"notes\": [");
    for (i, note) in d.notes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\n    \"{}\"", json_escape(note)));
    }
    if d.notes.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("\n  ],\n");
    }

    out.push_str("  \"pages\": [");
    for (page_index, page) in extracted.pages.iter().enumerate() {
        if page_index > 0 {
            out.push(',');
        }
        out.push_str("\n    {\n");
        out.push_str(&format!("      \"page\": {},\n", page.page_index + 1));
        out.push_str("      \"runs\": [");
        for (run_index, run) in page.runs.iter().enumerate() {
            if run_index > 0 {
                out.push(',');
            }
            out.push_str("\n        {");
            out.push_str(&format!("\"origin\": \"{}\", ", run.origin.as_str()));
            out.push_str(&format!("\"sourced\": {}, ", run.is_sourced()));
            out.push_str(&format!("\"text\": \"{}\"", json_escape(&run.text)));
            if let Some(artifact) = &run.artifact {
                out.push_str(&format!(
                    ", \"artifact\": \"{}\"",
                    json_escape(artifact.as_str())
                ));
            }
            if let Some(mcid) = run.mcid {
                out.push_str(&format!(", \"mcid\": {mcid}"));
            }
            if let Some(b) = run.bbox {
                out.push_str(&format!(
                    ", \"bbox\": [{:.2}, {:.2}, {:.2}, {:.2}]",
                    b.llx, b.lly, b.urx, b.ury
                ));
            }
            if !run.glyphs.is_empty() {
                out.push_str(", \"glyphs\": [");
                for (i, g) in run.glyphs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!(
                        "{{\"code\": {}, \"rung\": \"{}\", \"sourced\": {}, \
\"start\": {}, \"len\": {}, \"x\": {:.2}, \"y\": {:.2}, \"advance\": {:.2}, \
\"size\": {:.2}, \"invisible\": {}}}",
                        g.code,
                        g.rung.as_str(),
                        g.rung.is_sourced(),
                        g.text_start,
                        g.text_len,
                        g.x,
                        g.y,
                        g.advance,
                        g.size,
                        g.invisible
                    ));
                }
                out.push(']');
            }
            out.push('}');
        }
        out.push_str(if page.runs.is_empty() {
            "]\n"
        } else {
            "\n      ]\n"
        });
        out.push_str("    }");
    }
    out.push_str(if extracted.pages.is_empty() {
        "]\n"
    } else {
        "\n  ]\n"
    });
    out.push_str("}\n");
    out
}

/// Escape a string for a JSON string literal (RFC 8259 §7).
///
/// The seven named escapes, plus `\u00XX` for every other C0 control.
/// Non-ASCII characters pass through as UTF-8, which RFC 8259 permits
/// and which keeps extracted CJK and accented text readable in the
/// output instead of turning it into a wall of `\uXXXX`.
///
/// DEL (U+007F) is deliberately **not** escaped: RFC 8259 requires
/// escaping only U+0000–U+001F, and escaping more would be a silent
/// divergence from the format for no benefit.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// inspect --text-blocks (Pass 14.0): editable text model + block recognition
// ---------------------------------------------------------------------------

/// Recognise and dump a document's editable text-block structure — the
/// READ-ONLY first slice of the Acrobat-style text-editing subsystem
/// (decision 014, Pass 14.0).
///
/// For each requested page it extracts text WITH provenance capture, builds
/// the derived Run→Line→Column→Block model
/// ([`pdfce_core::text_edit::EditableTextModel`]), and reports the
/// recognised structure with every inference COUNTED — the whole hierarchy
/// is derived (§14.8, S1-S9), so disclosing the counts is the point (rule
/// 4). Nothing is written.
///
/// Output shape mirrors the rest of the CLI's two-half contract: a single
/// stable, locale-invariant summary line, plus a detailed report (text or,
/// with `--json`, a machine document). The summary line's home follows the
/// same rule as `extract-text`: it goes to stdout, except when the JSON
/// payload already occupies stdout, in which case it goes to stderr.
fn cmd_inspect_text_blocks(input: &Path, pages_spec: &str, json: bool) -> u8 {
    use pdfce_core::text_edit::{BlockRecognitionOptions, EditableTextModel};
    use pdfce_core::text_extract::{self, ExtractOptions};

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let page_list = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let indices = match parse_pages(pages_spec, page_list.len()) {
        Ok(indices) => indices,
        Err(message) => {
            eprintln!("pdfce-cli: {}: --pages {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };

    // Provenance is captured so the dump can disclose the surgery substrate
    // (operator span, font, size, fill colour) each recognised line rests
    // on. It costs only the pages actually inspected (R20 spirit).
    // The operator's persisted word-gap ratio. Extraction heuristics are
    // the one place where a document that reads fine to a human can
    // extract wrong, so this is a knob worth honouring rather than a
    // constant worth defending.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let options = ExtractOptions::default()
        .with_provenance(true)
        .with_word_gap_ratio(settings.word_gap_ratio)
        // As the `extract-text` path: `TX-A1` and `AT-A1` both change the
        // characters this subcommand reports, so both are the operator's.
        .with_unmappable_code(settings.unmappable_code)
        .with_actual_text(settings.actual_text);
    let recog = BlockRecognitionOptions::default();

    // Accumulators for the stable summary line.
    let mut total_lines = 0u64;
    let mut total_blocks = 0u64;
    let mut total_glyphs = 0u64;
    let mut columns_max = 0usize;
    let mut atomic_runs = 0u64;
    let mut artifact_runs = 0u64;
    let mut multi_column_pages = 0u64;

    let mut report = String::new();
    let mut notes: Vec<String> = Vec::new();

    if json {
        report.push_str("{\n");
        report.push_str(&format!(
            "  \"input\": \"{}\",\n",
            json_escape(&input.display().to_string())
        ));
        report.push_str("  \"pages\": [");
    }

    for (emitted, &index) in indices.iter().enumerate() {
        let page = match text_extract::extract_page(&doc, &page_list[index], index, &options) {
            Ok(page) => page,
            Err(err) => {
                eprintln!("pdfce-cli: {}: page {}: {err}", input.display(), index + 1);
                return exit::RUNTIME_ERROR;
            }
        };
        let model = EditableTextModel::recognize(&page, &recog);
        let d = model.diagnostics();

        total_lines += d.lines_recognized;
        total_blocks += d.blocks_recognized;
        total_glyphs += d.glyphs_clustered;
        columns_max = columns_max.max(model.columns());
        atomic_runs += d.atomic_runs;
        artifact_runs += d.artifact_runs_skipped;
        if d.is_multi_column() {
            multi_column_pages += 1;
        }
        for note in &d.notes {
            if !notes.contains(note) {
                notes.push(note.clone());
            }
        }

        if json {
            if emitted > 0 {
                report.push(',');
            }
            append_page_json(&mut report, &model);
        } else {
            append_page_report(&mut report, &model);
        }
    }

    if json {
        report.push_str(if indices.is_empty() { "]\n" } else { "\n  ]\n" });
        report.push_str("}\n");
    }

    let summary = format!(
        "text-blocks {}: pages={} lines={} blocks={} columns_max={} glyphs={} \
atomic_runs={} artifact_runs={} multi_column_pages={}",
        input.display(),
        indices.len(),
        total_lines,
        total_blocks,
        columns_max,
        total_glyphs,
        atomic_runs,
        artifact_runs,
        multi_column_pages,
    );

    if json {
        // JSON payload owns stdout; the summary goes to stderr so a caller
        // capturing stdout gets clean JSON.
        print!("{report}");
        eprintln!("{summary}");
    } else {
        // Text mode: the stable summary line first (parseable), then the
        // human report, both on stdout.
        println!("{summary}");
        print!("{report}");
    }

    // The derived-structure disclosures always go to stderr, so they can
    // never be mistaken for sourced content (rule 4).
    for note in &notes {
        eprintln!("pdfce-cli: {note}");
    }

    exit::SUCCESS
}

/// Append one page's human-readable block report to `out`.
fn append_page_report(out: &mut String, model: &pdfce_core::text_edit::EditableTextModel<'_>) {
    let page_number = model.sourced_view().page_index + 1;
    let d = model.diagnostics();
    out.push_str(&format!(
        "page {}: columns={} lines={} blocks={} glyphs={} multi_column={}\n",
        page_number,
        model.columns(),
        d.lines_recognized,
        d.blocks_recognized,
        d.glyphs_clustered,
        u8::from(d.is_multi_column()),
    ));
    out.push_str(&format!(
        "  derived: paragraph_breaks_leading={} paragraph_breaks_indent={} \
lines_split_baseline={} atomic_runs={} artifact_runs={}\n",
        d.paragraph_breaks_by_leading,
        d.paragraph_breaks_by_indent,
        d.lines_split_by_baseline,
        d.atomic_runs,
        d.artifact_runs_skipped,
    ));
    for (bi, block) in model.blocks().iter().enumerate() {
        out.push_str(&format!(
            "  block {bi}: column={} kind={} lines={} bbox=[{:.1} {:.1} {:.1} {:.1}]\n",
            block.column,
            block_kind_str(block.kind),
            block.line_indices.len(),
            block.bbox.llx,
            block.bbox.lly,
            block.bbox.urx,
            block.bbox.ury,
        ));
        for &li in &block.line_indices {
            if let Some(line) = model.lines().get(li) {
                out.push_str(&format!("    | {}\n", model.line_text(line)));
            }
        }
    }
}

/// Append one page's block structure to a JSON array being built in `out`.
fn append_page_json(out: &mut String, model: &pdfce_core::text_edit::EditableTextModel<'_>) {
    let d = model.diagnostics();
    out.push_str("\n    {\n");
    out.push_str(&format!(
        "      \"page\": {},\n",
        model.sourced_view().page_index + 1
    ));
    out.push_str(&format!("      \"columns\": {},\n", model.columns()));
    out.push_str("      \"diagnostics\": {");
    let counters: [(&str, u64); 8] = [
        ("lines_recognized", d.lines_recognized),
        ("columns_recognized", d.columns_recognized),
        ("blocks_recognized", d.blocks_recognized),
        ("glyphs_clustered", d.glyphs_clustered),
        ("paragraph_breaks_by_leading", d.paragraph_breaks_by_leading),
        ("paragraph_breaks_by_indent", d.paragraph_breaks_by_indent),
        ("lines_split_by_baseline", d.lines_split_by_baseline),
        ("atomic_runs", d.atomic_runs),
    ];
    for (i, (name, value)) in counters.iter().enumerate() {
        out.push_str(&format!(
            "{}\"{name}\": {value}",
            if i > 0 { ", " } else { "" }
        ));
    }
    out.push_str(&format!(
        ", \"artifact_runs_skipped\": {}, \"multi_column\": {}}},\n",
        d.artifact_runs_skipped,
        d.is_multi_column(),
    ));
    out.push_str("      \"blocks\": [");
    for (bi, block) in model.blocks().iter().enumerate() {
        if bi > 0 {
            out.push(',');
        }
        out.push_str("\n        {");
        out.push_str(&format!("\"kind\": \"{}\", ", block_kind_str(block.kind)));
        out.push_str(&format!("\"column\": {}, ", block.column));
        out.push_str(&format!(
            "\"bbox\": [{:.2}, {:.2}, {:.2}, {:.2}], ",
            block.bbox.llx, block.bbox.lly, block.bbox.urx, block.bbox.ury
        ));
        out.push_str(&format!(
            "\"text\": \"{}\", ",
            json_escape(&model.block_text(block))
        ));
        out.push_str("\"lines\": [");
        for (i, &li) in block.line_indices.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if let Some(line) = model.lines().get(li) {
                append_line_json(out, model, line);
            }
        }
        out.push_str("]}");
    }
    out.push_str(if model.blocks().is_empty() {
        "]\n"
    } else {
        "\n      ]\n"
    });
    out.push_str("    }");
}

/// Append one line's JSON, with the representative provenance of its first
/// glyph (the surgery substrate the later Pass builds on).
fn append_line_json(
    out: &mut String,
    model: &pdfce_core::text_edit::EditableTextModel<'_>,
    line: &pdfce_core::text_edit::Line,
) {
    out.push('{');
    out.push_str(&format!("\"baseline_y\": {:.2}, ", line.baseline_y));
    out.push_str(&format!("\"size\": {:.2}, ", line.size));
    out.push_str(&format!(
        "\"bbox\": [{:.2}, {:.2}, {:.2}, {:.2}], ",
        line.bbox.llx, line.bbox.lly, line.bbox.urx, line.bbox.ury
    ));
    out.push_str(&format!("\"glyph_count\": {}, ", line.glyphs.len()));
    out.push_str(&format!(
        "\"text\": \"{}\"",
        json_escape(&model.line_text(line))
    ));
    // Representative provenance: the first glyph's, when captured.
    if let Some(&gref) = line.glyphs.first()
        && let Some(prov) = model.provenance(gref)
    {
        out.push_str(", \"provenance\": {");
        out.push_str(&format!(
            "\"content_stream\": \"{}\", ",
            content_stream_str(prov.content_stream)
        ));
        out.push_str(&format!(
            "\"operator_span\": [{}, {}], ",
            prov.operator_span.start,
            prov.operator_span.end()
        ));
        match &prov.font_resource {
            Some(name) => out.push_str(&format!(
                "\"font_resource\": \"{}\", ",
                json_escape(&String::from_utf8_lossy(name))
            )),
            None => out.push_str("\"font_resource\": null, "),
        }
        out.push_str(&format!("\"tf_size\": {:.2}, ", prov.tf_size));
        out.push_str(&format!(
            "\"fill_color\": {}}}",
            fill_color_json(prov.fill_color.as_ref())
        ));
    }
    out.push('}');
}

// ---------------------------------------------------------------------------
// inspect --reflow-preview (Pass 15.0): READ-ONLY within-block reflow preview
// ---------------------------------------------------------------------------

/// Compute and dump a READ-ONLY within-block reflow preview for one block —
/// the FF-A engine's first slice (decision 015, Pass 15.0). Nothing is
/// written: no content-stream mutation, no `EditSession` command, no save
/// (that is Pass 15.1). This subcommand exists to *demonstrate* the derived
/// preview a UI/surgery Pass will consume.
///
/// It extracts the selected page WITH provenance (so words are measured by
/// their real §9.4.4 advances), recognises the block model with
/// **first-line-indent paragraph splitting relaxed** — a right/centre/
/// justified paragraph has ragged left edges that the default indent rule
/// would fragment into single-line blocks, and reflow needs the WHOLE
/// paragraph — then previews the requested block through
/// [`pdfce_core::text_edit::ReflowEngine`]. The report shows the detected
/// alignment, the greedy re-wrap's new break points and per-line origins,
/// the new block box, and every disclosure (the disclosures go to stderr so
/// they can never be mistaken for sourced content, rule 4).
#[allow(clippy::too_many_arguments)]
fn cmd_inspect_reflow_preview(
    input: &Path,
    pages_spec: &str,
    block_index: usize,
    width: Option<f64>,
    align: Option<&str>,
    leading: Option<f64>,
    json: bool,
) -> u8 {
    use pdfce_core::text_edit::{
        BlockAlignment, EditableTextModel, ReflowEngine, ReflowError, ReflowRequest,
    };
    use pdfce_core::text_extract::{self, ExtractOptions};

    // Parse the alignment override up front, so a typo fails cleanly before
    // any document work (the R27 fail-clean posture).
    let align_override = match align {
        None => None,
        Some(s) => match BlockAlignment::parse(s) {
            Some(a) => Some(a),
            None => {
                eprintln!(
                    "pdfce-cli: {}: --align {s}: expected left|right|center|justified",
                    input.display()
                );
                return exit::EDIT_REFUSED;
            }
        },
    };

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    let page_list = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let indices = match parse_pages(pages_spec, page_list.len()) {
        Ok(indices) => indices,
        Err(message) => {
            eprintln!("pdfce-cli: {}: --pages {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };
    // Reflow previews exactly one block on one page; take the first page of
    // the selection (a batch pipeline over many blocks scripts the loop).
    let Some(&page_index) = indices.first() else {
        eprintln!("pdfce-cli: {}: --pages selected no page", input.display());
        return exit::EDIT_REFUSED;
    };
    let Some(page) = page_list.get(page_index) else {
        eprintln!("pdfce-cli: {}: page out of range", input.display());
        return exit::EDIT_REFUSED;
    };

    let options = ExtractOptions::default().with_provenance(true);
    let extracted = match text_extract::extract_page(&doc, page, page_index, &options) {
        Ok(page) => page,
        Err(err) => {
            eprintln!(
                "pdfce-cli: {}: page {}: {err}",
                input.display(),
                page_index + 1
            );
            return exit::RUNTIME_ERROR;
        }
    };

    // Relaxed indent recognition via the ONE source of truth in pdfce-core
    // (Pass 15.2 §0.3): the CLI, the reflow engine/apply path, and the GUI
    // all recognise paragraphs with this identical config.
    let recog = pdfce_core::text_edit::reflow_recognition_options();
    let model = EditableTextModel::recognize(&extracted, &recog);
    let engine = ReflowEngine::new(&model);

    let req = ReflowRequest::new()
        .with_wrap_width_opt(width)
        .with_alignment_opt(align_override)
        .with_leading_opt(leading)
        .with_page_cropbox(page.crop_box);
    let preview = match engine.preview(block_index, &req) {
        Ok(preview) => preview,
        Err(err) => {
            eprintln!("pdfce-cli: {}: reflow: {err}", input.display());
            // Every reflow error is an operator error (bad selector / width /
            // an empty block) — refused, not a corrupt-file runtime error.
            // The `_` arm keeps this exhaustive as `ReflowError` grows
            // (it is `#[non_exhaustive]`).
            return match err {
                ReflowError::BlockIndexOutOfRange(..)
                | ReflowError::EmptyBlock(_)
                | ReflowError::BadWidth(_) => exit::EDIT_REFUSED,
                _ => exit::EDIT_REFUSED,
            };
        }
    };

    let summary = format!(
        "reflow-preview {}: page={} block={} align={} align_source={} width={:.1} leading={:.1} \
lines_before={} lines_after={} words={} overflowing_words={} \
new_bbox=[{:.1},{:.1},{:.1},{:.1}] height_delta={:.1} overflow={}",
        input.display(),
        page_index + 1,
        block_index,
        preview.alignment.alignment.as_str(),
        alignment_source_str(preview.alignment.source),
        preview.wrap_width,
        preview.leading,
        preview.lines_before,
        preview.lines_after,
        preview.diagnostics.words,
        preview.diagnostics.overflowing_words,
        preview.new_bbox.llx,
        preview.new_bbox.lly,
        preview.new_bbox.urx,
        preview.new_bbox.ury,
        preview.height_delta(),
        u8::from(preview.overflow.is_some()),
    );

    let report = if json {
        reflow_preview_json(input, page_index, block_index, &preview)
    } else {
        reflow_preview_report(&preview)
    };

    if json {
        print!("{report}");
        eprintln!("{summary}");
    } else {
        println!("{summary}");
        print!("{report}");
    }

    // Every disclosure goes to stderr, never mistakable for sourced content
    // (rule 4).
    for note in &preview.diagnostics.disclosures {
        eprintln!("pdfce-cli: {note}");
    }

    exit::SUCCESS
}

/// The stable keyword for an [`pdfce_core::text_edit::AlignmentSource`].
fn alignment_source_str(source: pdfce_core::text_edit::AlignmentSource) -> &'static str {
    use pdfce_core::text_edit::AlignmentSource;
    match source {
        AlignmentSource::Detected => "detected",
        AlignmentSource::SingleLineDefault => "single_line_default",
        AlignmentSource::AmbiguousDefault => "ambiguous_default",
        AlignmentSource::Overridden => "overridden",
        _ => "unknown",
    }
}

/// The human-readable reflow-preview report (text mode).
fn reflow_preview_report(preview: &pdfce_core::text_edit::ReflowPreview) -> String {
    let a = &preview.alignment;
    let mut out = String::new();
    out.push_str(&format!(
        "detected: alignment={} source={} left_ragged={:.1} right_ragged={:.1} \
mid_ragged={:.1} tol={:.1}\n",
        a.alignment.as_str(),
        alignment_source_str(a.source),
        a.left_ragged_pt,
        a.right_ragged_pt,
        a.mid_ragged_pt,
        a.tolerance_pt,
    ));
    out.push_str(&format!(
        "box: old=[{:.1} {:.1} {:.1} {:.1}] new=[{:.1} {:.1} {:.1} {:.1}] height_delta={:.1}\n",
        preview.old_bbox.llx,
        preview.old_bbox.lly,
        preview.old_bbox.urx,
        preview.old_bbox.ury,
        preview.new_bbox.llx,
        preview.new_bbox.lly,
        preview.new_bbox.urx,
        preview.new_bbox.ury,
        preview.height_delta(),
    ));
    out.push_str(&format!(
        "lines: before={} after={} words={} space_width={:.2}{} leading={:.2}{}\n",
        preview.lines_before,
        preview.lines_after,
        preview.diagnostics.words,
        preview.diagnostics.space_width_pt,
        if preview.diagnostics.space_width_estimated {
            "(est)"
        } else {
            ""
        },
        preview.diagnostics.leading_pt,
        if preview.diagnostics.leading_estimated {
            "(est)"
        } else {
            ""
        },
    ));
    for (i, line) in preview.lines.iter().enumerate() {
        let slack = match line.justified_slack {
            Some(s) => format!("{s:.1}"),
            None => "-".to_string(),
        };
        out.push_str(&format!(
            "  L{i}: words=[{},{}) x={:.1} baseline={:.1} natural={:.1} gaps={} slack={} \
overflow={} | {}\n",
            line.words.start,
            line.words.end,
            line.origin_x,
            line.baseline_y,
            line.natural_width,
            line.gap_count,
            slack,
            u8::from(line.is_overflowing_word),
            line.text,
        ));
    }
    if let Some(ov) = preview.overflow {
        out.push_str(&format!(
            "overflow: past_bottom={:.1} lines_outside={}\n",
            ov.past_bottom_pt, ov.lines_outside
        ));
    }
    out
}

/// The reflow-preview as a JSON document (`--json`).
fn reflow_preview_json(
    input: &Path,
    page_index: usize,
    block_index: usize,
    preview: &pdfce_core::text_edit::ReflowPreview,
) -> String {
    let a = &preview.alignment;
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"input\": \"{}\",\n",
        json_escape(&input.display().to_string())
    ));
    out.push_str(&format!("  \"page\": {},\n", page_index + 1));
    out.push_str(&format!("  \"block\": {block_index},\n"));
    out.push_str("  \"alignment\": {");
    out.push_str(&format!("\"value\": \"{}\", ", a.alignment.as_str()));
    out.push_str(&format!(
        "\"source\": \"{}\", ",
        alignment_source_str(a.source)
    ));
    out.push_str(&format!(
        "\"left_ragged\": {:.2}, \"right_ragged\": {:.2}, \"mid_ragged\": {:.2}, \"tolerance\": {:.2}}},\n",
        a.left_ragged_pt, a.right_ragged_pt, a.mid_ragged_pt, a.tolerance_pt
    ));
    out.push_str(&format!("  \"wrap_width\": {:.2},\n", preview.wrap_width));
    out.push_str(&format!("  \"leading\": {:.2},\n", preview.leading));
    out.push_str(&format!(
        "  \"lines_before\": {}, \"lines_after\": {},\n",
        preview.lines_before, preview.lines_after
    ));
    out.push_str(&format!(
        "  \"old_bbox\": [{:.2}, {:.2}, {:.2}, {:.2}],\n",
        preview.old_bbox.llx, preview.old_bbox.lly, preview.old_bbox.urx, preview.old_bbox.ury
    ));
    out.push_str(&format!(
        "  \"new_bbox\": [{:.2}, {:.2}, {:.2}, {:.2}],\n",
        preview.new_bbox.llx, preview.new_bbox.lly, preview.new_bbox.urx, preview.new_bbox.ury
    ));
    out.push_str(&format!(
        "  \"height_delta\": {:.2},\n",
        preview.height_delta()
    ));
    // Lines.
    out.push_str("  \"lines\": [");
    for (i, line) in preview.lines.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    {");
        out.push_str(&format!(
            "\"words\": [{}, {}], ",
            line.words.start, line.words.end
        ));
        out.push_str(&format!("\"origin_x\": {:.2}, ", line.origin_x));
        out.push_str(&format!("\"baseline_y\": {:.2}, ", line.baseline_y));
        out.push_str(&format!("\"natural_width\": {:.2}, ", line.natural_width));
        out.push_str(&format!("\"gap_count\": {}, ", line.gap_count));
        out.push_str(&format!(
            "\"is_overflowing_word\": {}, ",
            line.is_overflowing_word
        ));
        match line.justified_slack {
            Some(s) => out.push_str(&format!("\"justified_slack\": {s:.2}, ")),
            None => out.push_str("\"justified_slack\": null, "),
        }
        out.push_str(&format!("\"text\": \"{}\"}}", json_escape(&line.text)));
    }
    out.push_str(if preview.lines.is_empty() {
        "],\n"
    } else {
        "\n  ],\n"
    });
    // Overflow.
    match preview.overflow {
        Some(ov) => out.push_str(&format!(
            "  \"overflow\": {{\"past_bottom\": {:.2}, \"lines_outside\": {}}},\n",
            ov.past_bottom_pt, ov.lines_outside
        )),
        None => out.push_str("  \"overflow\": null,\n"),
    }
    // Diagnostics.
    let d = &preview.diagnostics;
    out.push_str("  \"diagnostics\": {");
    out.push_str(&format!("\"words\": {}, ", d.words));
    out.push_str(&format!("\"overflowing_words\": {}, ", d.overflowing_words));
    out.push_str(&format!(
        "\"space_width\": {:.2}, \"space_width_estimated\": {}, ",
        d.space_width_pt, d.space_width_estimated
    ));
    out.push_str(&format!(
        "\"leading\": {:.2}, \"leading_estimated\": {}, ",
        d.leading_pt, d.leading_estimated
    ));
    out.push_str("\"disclosures\": [");
    for (i, note) in d.disclosures.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("\"{}\"", json_escape(note)));
    }
    out.push_str("]}\n");
    out.push_str("}\n");
    out
}

/// A stable identifier for a [`pdfce_core::text_edit::BlockKind`].
fn block_kind_str(kind: pdfce_core::text_edit::BlockKind) -> &'static str {
    use pdfce_core::text_edit::BlockKind;
    match kind {
        BlockKind::Paragraph => "paragraph",
        _ => "other",
    }
}

/// A stable identifier for a [`pdfce_core::text_extract::ContentStreamRef`].
fn content_stream_str(stream: pdfce_core::text_extract::ContentStreamRef) -> String {
    use pdfce_core::text_extract::ContentStreamRef;
    match stream {
        ContentStreamRef::Page => "page".to_string(),
        ContentStreamRef::Form { object } => format!("form:{object}"),
        _ => "unknown".to_string(),
    }
}

/// A JSON value for a fill colour: a tagged string, or `null` for the
/// §8.6.8 default (unset) colour.
fn fill_color_json(color: Option<&pdfce_core::text_extract::TextColor>) -> String {
    use pdfce_core::text_extract::TextColor;
    match color {
        None => "null".to_string(),
        Some(TextColor::Gray(g)) => format!("\"gray:{g:.3}\""),
        Some(TextColor::Rgb(r, g, b)) => format!("\"rgb:{r:.3},{g:.3},{b:.3}\""),
        Some(TextColor::Cmyk(c, m, y, k)) => format!("\"cmyk:{c:.3},{m:.3},{y:.3},{k:.3}\""),
        Some(TextColor::Other) => "\"other\"".to_string(),
        Some(_) => "\"unknown\"".to_string(),
    }
}

/// Emit the standard "not implemented yet" message for a stub subcommand
/// and return [`exit::UNIMPLEMENTED`].
fn unimplemented_stub(name: &str) -> u8 {
    eprintln!(
        "pdfce-cli: `{name}` is not implemented yet — it ships in a later Pass \
(see docs/ROADMAP.md). This is a Pass 0 scaffold stub."
    );
    exit::UNIMPLEMENTED
}

// ---------------------------------------------------------------------------
// Structural page operations (Pass 3.2)
// ---------------------------------------------------------------------------

/// Parse a 1-based page specification into 0-based indices.
///
/// Accepted: `all`, a comma-separated list of single pages (`3`) and
/// inclusive ranges (`3-7`), with optional surrounding whitespace. A
/// descending range (`7-3`) is accepted and expands **descending**, so
/// `--order 3-1` is a legitimate way to reverse three pages.
///
/// ## Why this refuses instead of skipping
///
/// A page number past the end of the document, a zero, or an
/// unparseable token is an **error**, not something to drop quietly. A
/// batch script that asks for pages 1-50 of a 30-page file has made a
/// mistake, and silently handing back 30 pages is how that mistake ships
/// to a thousand documents. This is the CLI half of the R27 fail-clean
/// posture.
///
/// `count` is the document's page count, used both to bound the numbers
/// and to expand `all`.
fn parse_pages(spec: &str, count: usize) -> Result<Vec<usize>, String> {
    let trimmed = spec.trim();
    if trimmed.eq_ignore_ascii_case("all") {
        return Ok((0..count).collect());
    }
    let mut out: Vec<usize> = Vec::new();
    for token in trimmed.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let (first, last) = match token.split_once('-') {
            Some((a, b)) => (parse_page_number(a, count)?, parse_page_number(b, count)?),
            None => {
                let single = parse_page_number(token, count)?;
                (single, single)
            }
        };
        if first <= last {
            out.extend(first..=last);
        } else {
            // Descending on purpose: `--order 3-1` reverses.
            out.extend((last..=first).rev());
        }
    }
    if out.is_empty() {
        return Err("the page specification selected no pages".to_owned());
    }
    Ok(out)
}

/// Parse one 1-based page number into a 0-based index, bounded by
/// `count`.
fn parse_page_number(token: &str, count: usize) -> Result<usize, String> {
    let token = token.trim();
    let number: usize = token
        .parse()
        .map_err(|_| format!("`{token}` is not a page number"))?;
    if number == 0 {
        return Err("page numbers are 1-based, so 0 is not a page".to_owned());
    }
    if number > count {
        return Err(format!(
            "page {number} is past the end of the document, which has {count} page(s)"
        ));
    }
    Ok(number - 1)
}

/// The `signature=` token on a stdout line.
///
/// Lives in the **narrative** half of the line, alongside `mode=`,
/// because it is a name rather than a non-negative integer and the
/// metrics half's contract is `key=<integer>`.
const fn signature_token(impact: SignatureImpact) -> &'static str {
    match impact {
        SignatureImpact::None => "none",
        // Named for the fact it establishes (§12.8.2.2.2's stage 1), NOT
        // for validity. See `pdfce_core::signature`.
        SignatureImpact::ByteRangePreserved => "byte-range-preserved",
        SignatureImpact::Invalidated => "invalidated",
        // `SignatureImpact` is #[non_exhaustive]; an unmapped future
        // verdict must not silently print as "none".
        _ => "unknown",
    }
}

/// Print the honest expansion of a signature verdict to stderr.
///
/// The `ByteRangePreserved` wording is the load-bearing one, and it is
/// deliberately not reassuring. `iso32000__s__12.8.md`'s VALIDATION MODEL
/// puts it plainly: *"Reporting stage-1 success as 'the signature is
/// still valid' is the specific error this section exists to prevent."*
/// So the message states what was preserved, states what was not
/// established, and stops.
fn report_signature(input: &Path, impact: SignatureImpact) {
    match impact {
        SignatureImpact::None => {}
        SignatureImpact::ByteRangePreserved => eprintln!(
            "pdfce-cli: {}: this document is signed. The save appends a revision, so each \
signature's signed byte range is preserved (ISO 32000-1 §12.8.1 NOTE 1) — but that is only the \
FIRST of two validation stages. Whether the changes are ones the signer permitted (§12.8.2.2.2) \
is a separate question pdfce does not answer here, and a validator may still report the document \
as altered since signing.",
            input.display()
        ),
        SignatureImpact::Invalidated => eprintln!(
            "pdfce-cli: {}: this document is signed, and this save INVALIDATES that signature. \
Keep a copy of the original if the signature matters.",
            input.display()
        ),
        _ => eprintln!(
            "pdfce-cli: {}: this document is signed and pdfce cannot classify this save's effect \
on it. Treat the signature as suspect.",
            input.display()
        ),
    }
}

/// Report an [`AssembleReport`]'s honesty counters to stderr.
///
/// Every one of these is something the operator **cannot see by looking
/// at the output file**, which is the test for whether it belongs on
/// stderr rather than only in the machine line.
fn report_assemble(output: &Path, report: &pdfce_core::pageops::AssembleReport) {
    if report.dangling_references > 0 {
        eprintln!(
            "pdfce-cli: {}: {} reference(s) pointed at a page that was not copied and were \
dropped — links and destinations that led outside the selection now lead nowhere. The \
annotations themselves were kept.",
            output.display(),
            report.dangling_references
        );
    }
    if report.outline_items_dropped > 0 {
        eprintln!(
            "pdfce-cli: {}: {} bookmark(s) were dropped because their destination page was not \
copied; {} were carried and repointed.",
            output.display(),
            report.outline_items_dropped,
            report.outline_items_kept
        );
    }
    if report.form_fields_renamed > 0 {
        eprintln!(
            "pdfce-cli: {}: {} form field(s) were renamed with a Doc<N>_ prefix because their \
names collided across sources. Without this, same-named fields become one logical field and \
typing in either fills both.",
            output.display(),
            report.form_fields_renamed
        );
    }
    if report.form_fields_dropped > 0 {
        eprintln!(
            "pdfce-cli: {}: {} form field(s) were dropped because their widgets straddle the \
copied/not-copied boundary. pdfce does not copy half a field — a field is identified by name \
across the whole document, so a partial copy would apply a value to controls that no longer exist.",
            output.display(),
            report.form_fields_dropped
        );
    }
    if report.named_destinations_dropped > 0 {
        eprintln!(
            "pdfce-cli: {}: {} named destination(s) were not carried. Bookmarks that used them \
were rewritten to explicit destinations; links inside the copied pages that used them by name \
will not resolve.",
            output.display(),
            report.named_destinations_dropped
        );
    }
    if report.page_labels_stale {
        eprintln!(
            "pdfce-cli: {}: this document has a page-label tree (/PageLabels) and it was carried across unchanged, so its numbering is now stale for the pages after the insertion point. Acrobat leaves it stale too; pdfce says so.",
            output.display()
        );
    }
    if report.page_labels_dropped {
        eprintln!(
            "pdfce-cli: {}: the source's page-label tree (/PageLabels) was NOT carried. Its \
numbering describes a different set of pages, so carrying it would produce labels that are \
confidently wrong.",
            output.display()
        );
    }
    if report.struct_tree_dropped {
        eprintln!(
            "pdfce-cli: {}: the source's tagged-PDF structure tree (/StructTreeRoot) was NOT \
carried. Subsetting a structure tree to a page selection is not implemented; copying it whole \
would leave dangling references and a file that claims to be tagged but is not.",
            output.display()
        );
    }
    report_separations(output, &report.separations);
}

/// Report anything the settings load wants the operator to know.
///
/// Goes to stderr, never changes the exit code, and is silent on the
/// normal case (no file, or a file read cleanly). The settings store's
/// fail-soft contract is that a configuration problem must not stop the
/// work — but a typo at a known line number is exactly the thing a
/// command-line operator can fix in ten seconds if told, and never notices
/// if not.
fn report_settings(report: &pdfce_core::settings::LoadReport) {
    use pdfce_core::settings::{SettingNote, StoreKind};

    if report.location.kind == StoreKind::PlatformFallback
        && let Some(dir) = report.location.directory()
    {
        eprintln!(
            "pdfce-cli: settings are being read from {} because pdfce's own folder is not writable; they do not travel with the program folder.",
            dir.display()
        );
    }
    for note in &report.notes {
        // Spelled out rather than `{:?}`-printed. A `Debug` dump is a
        // developer's view of a struct; the operator's question is "what
        // did I get wrong and on which line", and the answer has to be a
        // sentence they can act on without reading pdfce's source.
        let line = match note {
            SettingNote::Unreadable { path, reason } => format!(
                "the settings file at {} could not be read ({reason}); defaults are in use",
                path.display()
            ),
            SettingNote::UnknownKey { key, line } => format!(
                "line {line}: \"{key}\" is not a setting pdfce knows. It was left in the file, not removed"
            ),
            SettingNote::BadValue {
                key,
                value,
                line,
                using,
            } => format!(
                "line {line}: \"{value}\" is not a value \"{key}\" accepts, so \"{using}\" is being used instead; every other setting in the file still applies"
            ),
            SettingNote::Clamped {
                key,
                value,
                line,
                using,
            } => format!(
                "line {line}: \"{key} = {value}\" is outside the accepted range, so {using} is being used"
            ),
            SettingNote::Malformed { line } => format!(
                "line {line} is not a setting (it needs the form: name = value) and was skipped"
            ),
            SettingNote::Duplicate { key, line } => {
                format!("\"{key}\" is set more than once; the one on line {line} is in effect")
            }
            // `SettingNote` is `#[non_exhaustive]`: a note a future pdfce
            // adds must still reach the operator, even unspelled.
            _ => "something in the settings file was not applied as written".to_owned(),
        };
        eprintln!("pdfce-cli: settings: {line}.");
    }
}

/// Render `/DeviceColorant` byte strings as a readable list.
///
/// Table 364 types the value as "name **or** string", so it arrives as
/// bytes with no declared encoding. Lossy UTF-8 is right for a diagnostic
/// line: a colorant is almost always ASCII (`Cyan`, `PANTONE 485 C`), and
/// a mangled byte should still print something an operator can match
/// against their plate list rather than suppressing the whole message.
fn colorant_list(colorants: &[Vec<u8>]) -> String {
    colorants
        .iter()
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect::<Vec<String>>()
        .join(", ")
}

/// Disclose what an operation did to preseparated page sets (§14.11.4).
///
/// Shared by the document producers and the in-place editors because the
/// operator's question is the same either way: *which plates do I still
/// have?* Silent on the overwhelmingly common non-preseparated document.
fn report_separations(output: &Path, impact: &pdfce_core::pageops::SeparationImpact) {
    if impact.sets_split > 0 {
        // The sentence must match the POLICY rather than assume one. The
        // first version of this said arrays had been "rewritten" under
        // every policy, which is false for `Discard` — that removes the
        // dictionary outright. A true count with a false sentence
        // attached is a worse disclosure than no sentence, because it
        // gets believed.
        let did = match impact.policy {
            pdfce_core::pageops::SeparationPolicy::Discard => {
                "surviving page(s) had their /SeparationInfo removed entirely, so they are now \
ordinary pages carrying no record of which plate they were"
            }
            _ => {
                "surviving page(s) had their /SeparationInfo /Pages array rewritten to name only \
the plates that are still here"
            }
        };
        eprintln!(
            "pdfce-cli: {}: this is a PRESEPARATED document (ISO 32000-1 §14.11.4) — several \
page objects are one logical page, one per printing plate. The selection split {} set(s), so \
{} {did}. Removed: {}. Kept: {}.",
            output.display(),
            impact.sets_split,
            impact.pages_changed,
            colorant_list(&impact.colorants_removed),
            colorant_list(&impact.colorants_kept),
        );
    }
    if impact.malformed > 0 {
        eprintln!(
            "pdfce-cli: {}: {} page(s) carry a /SeparationInfo with no usable /Pages array, \
which §14.11.4 Table 364 makes REQUIRED. They were already non-conforming on arrival and were \
left exactly as they were — repairing one would mean guessing which pages belonged to the set.",
            output.display(),
            impact.malformed
        );
    }
}

/// The `separations=` metrics fragment shared by both metric tails.
///
/// Colorant names are deliberately absent from the machine line: they are
/// unescaped operator-supplied bytes and the metrics format is
/// whitespace-delimited `key=value`. The counts are the machine-readable
/// part; the names go to the stderr prose above, where they can be
/// arbitrary.
fn separation_metrics(impact: &pdfce_core::pageops::SeparationImpact) -> String {
    format!(
        "sep_sets_split={} sep_pages_changed={} sep_malformed={}",
        impact.sets_split, impact.pages_changed, impact.malformed
    )
}

/// The metrics tail every document-producing subcommand shares.
fn assemble_metrics(report: &pdfce_core::pageops::AssembleReport, out_bytes: usize) -> String {
    format!(
        "pages={} objects={} dangling={} outline_kept={} outline_dropped={} \
fields_renamed={} fields_dropped={} dests_dropped={} labels_dropped={} labels_stale={} \
struct_tree_dropped={} ocg_carried={} {} out_bytes={out_bytes}",
        report.pages,
        report.objects_copied,
        report.dangling_references,
        report.outline_items_kept,
        report.outline_items_dropped,
        report.form_fields_renamed,
        report.form_fields_dropped,
        report.named_destinations_dropped,
        u32::from(report.page_labels_dropped),
        u32::from(report.page_labels_stale),
        u32::from(report.struct_tree_dropped),
        u32::from(report.optional_content_carried),
        separation_metrics(&report.separations),
    )
}

/// The metrics tail every in-place editing subcommand shares.
fn edit_metrics(outcome: &EditOutcome) -> String {
    format!(
        "changed={} objects={} verbatim={} reserialized={} promoted={} deleted={} \
appended={} out_bytes={} undo_verified={} undo_identical={} delinearized={}",
        outcome.changed,
        outcome.report.objects_written,
        outcome.report.objects_verbatim,
        outcome.report.objects_reserialized,
        outcome.report.promoted.len(),
        outcome.report.objects_deleted,
        outcome.report.bytes_appended,
        outcome.report.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
        u32::from(outcome.report.delinearized),
    )
}

/// Map a [`PageOpError`] to an exit code and print it.
///
/// [`exit::EDIT_REFUSED`] for every named refusal: the documents were
/// readable and pdfce declined the operation as asked, which a batch
/// script must be able to tell apart from a broken file.
fn report_page_op_error(err: &PageOpError) -> u8 {
    eprintln!("pdfce-cli: {err}");
    exit::EDIT_REFUSED
}

/// Load a document for a read-only structural operation.
fn open_for_read(path: &Path) -> Result<Document, u8> {
    open_document(path).map_err(|err| {
        eprintln!("pdfce-cli: {}: {err}", path.display());
        exit_code_for_doc(&err)
    })
}

// =====================================================================
// Pass 12.M2 dimensioning subcommands (decision 011 §2.3/§2.4)
// =====================================================================

/// Parse a `x,y x,y ...` (space/`;`-separated) point list into page-space
/// points. `None` on any malformed token or an empty list.
fn parse_dim_points(s: &str) -> Option<Vec<pdfce_core::vector::Point>> {
    let mut out = Vec::new();
    for tok in s.split([' ', ';', '\t', '\n']).filter(|t| !t.is_empty()) {
        let (x, y) = tok.split_once(',')?;
        out.push(pdfce_core::vector::Point::new(
            x.trim().parse().ok()?,
            y.trim().parse().ok()?,
        ));
    }
    (!out.is_empty()).then_some(out)
}

/// Parse an `N:M` ratio into `(paper, real)`. `None` if malformed.
fn parse_ratio(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(':')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// Borrowed argument bundle for [`cmd_dimension_add`] (clippy arg-count).
struct DimensionAddArgs<'a> {
    input: &'a Path,
    page: u32,
    kind: DimKindArg,
    points: &'a str,
    group: u32,
    constraint: ConstraintArg,
    offset: f64,
    text_along: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `dimension-add` — author a scaled dimension additively (Pass 12.M2).
fn cmd_dimension_add(args: &DimensionAddArgs<'_>) -> u8 {
    use pdfce_core::dimension::{DimensionKind, GroupId, fit_circle_taubin};
    let &DimensionAddArgs {
        input,
        page,
        kind,
        points,
        group,
        constraint,
        offset,
        text_along,
        output,
        mode,
        verify_undo,
    } = args;

    let Some(pts) = parse_dim_points(points) else {
        eprintln!(
            "pdfce-cli: {}: --points must be `x,y x,y ...` in points",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };
    let dk = match kind {
        DimKindArg::Linear => {
            let [a, b, ..] = pts.as_slice() else {
                eprintln!(
                    "pdfce-cli: {}: a linear dimension needs at least two points",
                    input.display()
                );
                return exit::EDIT_REFUSED;
            };
            DimensionKind::Linear {
                // Pass 27.1: the placement the operator asked for. Defaults to
                // 0.0/0.0 — the dimension line through the first picked point,
                // text centred — which is what the GUI's own neutral placement
                // produces, so the two surfaces still author identical bytes
                // for identical inputs.
                offset,
                text_along,
                a: *a,
                b: *b,
                constraint: constraint.to_core(),
            }
        }
        DimKindArg::Radius | DimKindArg::Diameter => {
            let Some(fit) = fit_circle_taubin(&pts) else {
                eprintln!(
                    "pdfce-cli: {}: need at least 3 non-collinear points to fit a circle",
                    input.display()
                );
                return exit::EDIT_REFUSED;
            };
            DimensionKind::Circular {
                fit,
                show_diameter: matches!(kind, DimKindArg::Diameter),
            }
        }
    };

    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let Some(page_index) = page.checked_sub(1).map(|i| i as usize) else {
        eprintln!(
            "pdfce-cli: {}: --page is 1-based; 0 is not a page",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };
    let (annot_id, dim_id) = match session.add_dimension(page_index, GroupId(group), dk) {
        Ok(v) => v,
        Err(err) => return report_edit_error(input, &err),
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "dimension-add {} page {page} kind={} group={group} mode={} -> {}; \
annot={annot_id} dim={} changed={} objects={} verbatim={} appended={} out_bytes={} \
undo_verified={} undo_identical={}",
        input.display(),
        kind.token(),
        mode.name(),
        output.display(),
        dim_id.0,
        outcome.changed,
        r.objects_written,
        r.objects_verbatim,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `dimension-list` — inventory the stored dimension model (read-only).
/// The drafting standard, as a CLI value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum StandardArg {
    /// ANSI/ASME practice — broken dimension line, horizontal text, point
    /// decimal marker. pdfce's default.
    Ansi,
    /// ISO 129-1 practice — unbroken line, value above and aligned, comma
    /// decimal marker.
    Iso,
}

impl StandardArg {
    fn to_core(self) -> pdfce_core::dimension::DimStandard {
        match self {
            Self::Ansi => pdfce_core::dimension::DimStandard::Ansi,
            Self::Iso => pdfce_core::dimension::DimStandard::Iso,
        }
    }
}

/// `group-set-standard` — set a group's drafting standard, regenerating every
/// member (Pass 27.2).
///
/// ## Contract
///
/// - Emits one `group-set-standard …` line naming the group, the standard and
///   the MEMBER COUNT regenerated, then defers the exit code to
///   [`finish_edit`]. The count is reported because this changes the SHAPE of
///   every member, which is a larger visible change than a scale edit.
/// - An unknown group is refused before any mutation.
fn cmd_group_set_standard(
    input: &Path,
    group: u32,
    standard: StandardArg,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let members = match session
        .set_group_standard(pdfce_core::dimension::GroupId(group), standard.to_core())
    {
        Ok(n) => n,
        Err(err) => return report_edit_error(input, &err),
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "group-set-standard {} group={group} standard={} members_regenerated={members} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        if matches!(standard, StandardArg::Iso) {
            "iso"
        } else {
            "ansi"
        },
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// Grouped arguments for `dimension-offset` (clippy arg-count).
struct DimensionOffsetArgs<'a> {
    input: &'a Path,
    dimension: u32,
    offset: f64,
    text_along: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `dimension-offset` — set a ce dimension's placement (Pass 27.1).
///
/// ## Contract
///
/// - Emits one `dimension-offset …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - The measured value is unchanged by construction: this writes only the
///   placement fields, which the value function does not read.
/// - A circular target, or an unknown id, is refused through
///   [`report_edit_error`] before any mutation — the same message and exit
///   code the GUI surfaces.
fn cmd_dimension_offset(args: &DimensionOffsetArgs<'_>) -> u8 {
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    if let Err(err) = session.place_dimension(
        pdfce_core::dimension::DimensionId(args.dimension),
        args.offset,
        args.text_along,
    ) {
        return report_edit_error(args.input, &err);
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "dimension-offset {} dimension={} offset={} text_along={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.dimension,
        args.offset,
        args.text_along,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Borrowed argument bundle for [`cmd_add_text_field`] (clippy arg-count).
struct AddTextFieldArgs<'a> {
    input: &'a Path,
    name: &'a str,
    page: usize,
    rect: &'a str,
    value: Option<&'a str>,
    max_len: Option<i64>,
    tooltip: Option<&'a str>,
    no_tooltip: bool,
    multiline: bool,
    read_only: bool,
    required: bool,
    password: bool,
    comb: bool,
    border: BorderArg,
    border_width: f64,
    visibility: VisibilityArg,
    output: &'a Path,
    mode: SaveMode,
    defaults_from: Option<&'a str>,
    verify_undo: bool,
}

/// Borrowed argument bundle for [`cmd_add_check_box`] (clippy arg-count).
struct AddCheckBoxArgs<'a> {
    input: &'a Path,
    name: &'a str,
    page: usize,
    rect: &'a str,
    on_state: &'a str,
    checked: bool,
    tooltip: Option<&'a str>,
    no_tooltip: bool,
    read_only: bool,
    required: bool,
    output: &'a Path,
    mode: SaveMode,
    defaults_from: Option<&'a str>,
    verify_undo: bool,
    border: BorderArg,
    border_width: f64,
    visibility: VisibilityArg,
}

/// Borrowed argument bundle for [`cmd_add_choice_field`] (clippy arg-count).
struct AddChoiceFieldArgs<'a> {
    input: &'a Path,
    name: &'a str,
    page: usize,
    rect: &'a str,
    options: &'a [String],
    combo: bool,
    editable: bool,
    multi_select: bool,
    sort: bool,
    tooltip: Option<&'a str>,
    no_tooltip: bool,
    read_only: bool,
    required: bool,
    output: &'a Path,
    mode: SaveMode,
    defaults_from: Option<&'a str>,
    verify_undo: bool,
    border: BorderArg,
    border_width: f64,
    visibility: VisibilityArg,
}

/// Borrowed argument bundle for [`cmd_add_push_button`] (clippy arg-count).
struct AddPushButtonArgs<'a> {
    input: &'a Path,
    name: &'a str,
    page: usize,
    rect: &'a str,
    caption: &'a str,
    tooltip: Option<&'a str>,
    no_tooltip: bool,
    read_only: bool,
    output: &'a Path,
    mode: SaveMode,
    defaults_from: Option<&'a str>,
    verify_undo: bool,
    border: BorderArg,
    border_width: f64,
    visibility: VisibilityArg,
}

/// Borrowed argument bundle for [`cmd_add_radio_button`] (clippy arg-count).
struct AddRadioButtonArgs<'a> {
    input: &'a Path,
    name: &'a str,
    page: usize,
    rect: &'a str,
    export_value: &'a str,
    selected: bool,
    tooltip: Option<&'a str>,
    no_tooltip: bool,
    no_toggle_to_off: bool,
    radios_in_unison: bool,
    read_only: bool,
    required: bool,
    output: &'a Path,
    mode: SaveMode,
    defaults_from: Option<&'a str>,
    verify_undo: bool,
    border: BorderArg,
    border_width: f64,
    visibility: VisibilityArg,
}

/// Parse `--page` (1-based) and `--rect` (`llx,lly,urx,ury`), the two
/// arguments every field-authoring subcommand takes in the same form.
///
/// Shared so the three subcommands cannot disagree about whether `--page` is
/// 1-based — which is the kind of divergence that produces a field on the
/// wrong page rather than an error.
fn parse_page_and_rect(
    input: &Path,
    page: usize,
    rect: &str,
) -> Result<(usize, pdfce_core::page_tree::Rect), u8> {
    let Some(page_index) = page.checked_sub(1) else {
        eprintln!(
            "pdfce-cli: {}: --page is 1-based; 0 is not a page",
            input.display()
        );
        return Err(exit::EDIT_REFUSED);
    };
    let parts: Vec<f64> = rect
        .split(',')
        .filter_map(|t| t.trim().parse::<f64>().ok())
        .collect();
    let [llx, lly, urx, ury] = parts[..] else {
        eprintln!(
            "pdfce-cli: {}: --rect needs four numbers as LLX,LLY,URX,URY",
            input.display()
        );
        return Err(exit::EDIT_REFUSED);
    };
    Ok((
        page_index,
        pdfce_core::page_tree::Rect { llx, lly, urx, ury },
    ))
}

/// `add-check-box` — author a new check box.
///
/// ## Contract
///
/// - Emits one `add-check-box …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - Every refusal — an `Off` on-state, XFA present, a name already used by
///   a different field type, a degenerate rectangle, an empty name, a page
///   out of range — goes through [`report_edit_error`] BEFORE any mutation.
/// - `--page` is 1-BASED here and 0-based in the core call.
fn cmd_add_check_box(args: &AddCheckBoxArgs<'_>) -> u8 {
    let (page_index, rect) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec = pdfce_core::edit::NewCheckBox::new(page_index, args.name, rect)
        .with_on_state(args.on_state)
        .checked(args.checked)
        .with_flags(args.read_only, args.required)
        .with_border(args.border.into(), args.border_width)
        .with_visibility(args.visibility.into());
    // R105: exactly one of the two must have been chosen. `clap`'s
    // `conflicts_with` rules out BOTH; only "neither" can reach here, and it
    // is refused rather than defaulted.
    spec = match (args.tooltip, args.no_tooltip) {
        (Some(t), _) => spec.with_tooltip(t),
        (None, true) => spec.declining_tooltip(),
        (None, false) => {
            eprintln!(
                "pdfce-cli: {}: decide about the accessibility name — pass --tooltip <text>, or --no-tooltip to decline it. It is what a screen reader announces for this field, so it is never defaulted silently.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
    };

    // Applied to the SPEC before authoring, so everything downstream
    // — the merge check, the appearance build, the undo entry — sees
    // one fully-formed request rather than a partially-defaulted one.
    let defaults = match read_defaults(&session, args.input, args.defaults_from) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let applied = defaults
        .map(|d| spec.apply_defaults(&d))
        .unwrap_or_default();
    let authored = match session.add_check_box(&spec) {
        Ok(o) => o,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let field_id = authored.field_id;
    // Folded into the SAME disclosure struct the core produced, not
    // reported alongside it: one channel, so `any()` still answers for
    // everything and a caller gating on it cannot miss half the facts.
    let mut disclosures = authored.disclosures;
    disclosures.defaults_type_mismatch = applied.type_mismatch;
    disclosures.defaults_on_state_ambiguous = applied.on_state_ambiguous;
    report_field_disclosures(args.name, disclosures);
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "add-check-box {} name={:?} page={} rect={},{},{},{} on_state={:?} checked={} field={} {} merged={} tagged={} struct_tabs={} tooltip_declined={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.name,
        args.page,
        rect.llx,
        rect.lly,
        rect.urx,
        rect.ury,
        args.on_state,
        u32::from(args.checked),
        field_id.num,
        field_id.generation,
        u32::from(authored.merged),
        u32::from(authored.disclosures.tagged_document),
        u32::from(authored.disclosures.structure_tab_order),
        u32::from(authored.disclosures.tooltip_declined),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}
/// `rename-field` — change a field's partial name `/T` (decision 020's F6).
///
/// The disclosure this exists to carry is `descendants_renamed`. Renaming a
/// grouping node re-derives every descendant's fully-qualified name without
/// writing to one of them (§12.7.3.2), so the operator's one-field request
/// can rename a subtree. Saying so is rule 4; leaving it to be discovered
/// when an FDF stops matching is not.
fn cmd_rename_field(
    input: &Path,
    name: &str,
    to: &str,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let rename = match session.rename_field(name, to) {
        Ok(r) => r,
        Err(err) => return report_edit_error(input, &err),
    };

    // In prose, for the person about to wonder why six fields moved. The
    // machine-readable count is on the result line below.
    if rename.descendants_renamed > 0 {
        eprintln!(
            "pdfce-cli: field {name:?}: {} field(s) beneath it now have different fully-qualified names, because §12.7.3.2 builds those names from this one — no object of theirs was written, but anything naming them (FDF, JavaScript, submit mappings) no longer matches",
            rename.descendants_renamed
        );
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "rename-field {} from={:?} to={:?} descendants_renamed={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        rename.from,
        rename.to,
        rename.descendants_renamed,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `move-widget` — translate one widget annotation's `/Rect`.
///
/// The disclosure this verb owes the operator is `siblings_left_behind`: a
/// field with widgets on three pages looks like ONE thing to someone who
/// asked to move "the signature box", and moving one while silently leaving
/// two behind is the kind of partial result that reads as a bug an hour
/// later. It is printed in prose when it is non-zero, and always on the
/// machine-readable line.
fn cmd_move_widget(
    input: &Path,
    name: &str,
    index: usize,
    dx: f64,
    dy: f64,
    output: &Path,
    mode: SaveMode,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let moved = match session.move_widget(name, index, dx, dy) {
        Ok(m) => m,
        Err(err) => return report_edit_error(input, &err),
    };

    if moved.siblings_left_behind > 0 {
        eprintln!(
            "pdfce-cli: field {name:?}: moved widget {index} only — {} other widget(s) of this field stayed where they were. A field's widgets are separate appearances and can sit on different pages; move each one you want moved",
            moved.siblings_left_behind
        );
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        false,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "move-widget {} name={name:?} index={index} dx={dx} dy={dy} from=[{} {} {} {}] to=[{} {} {} {}] siblings_left_behind={} mode={} -> {}; changed={} objects={} appended={} out_bytes={}",
        input.display(),
        moved.from.llx,
        moved.from.lly,
        moved.from.urx,
        moved.from.ury,
        moved.to.llx,
        moved.to.lly,
        moved.to.urx,
        moved.to.ury,
        moved.siblings_left_behind,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
    );
    finish_edit(input, &outcome)
}

/// `delete-annotation` — the general annotation-deletion verb (Pass 38.5).
///
/// ## Resolving `--page`/`--index` to an object, and why NOT an object number
///
/// The core verb takes an [`ObjId`](pdfce_core::object::ObjId), because
/// object identity is the only handle that stays correct while a session
/// mutates. A CLI cannot use that: the operator's source of truth is
/// `list-annotations`' own output, which prints `page=` and `index=` and
/// deliberately does **not** print object numbers. So the pair is resolved
/// here, against the SAME `page_annotations` walk `list-annotations` uses,
/// which is what makes "list it, then delete that index" reliable rather
/// than approximately right.
///
/// Both out-of-range cases are refused with the count that was actually
/// there — an index past the end says how many annotations the page has,
/// because "index 4 is out of range" without the bound sends the operator
/// back to re-run the list command for a number this process already knew.
///
/// ## Contract
///
/// - One `delete-annotation …` line carrying `subtype=`, `route=`,
///   `popup_removed=`, `parent_popup_cleared=`, `replies_orphaned=`,
///   `group_promoted=` and `ap_removed=`, then the exit code from
///   [`finish_edit`].
/// - **Every non-obvious consequence is ALSO printed in prose to stderr**,
///   for the same reason `delete-field` prints its `selection_cleared`
///   disclosure twice: the machine-readable field is for a script, the
///   sentence is for the person who will otherwise wonder an hour later why
///   three other comments changed.
/// - Refusals — no such page or index, a `/Widget` target, an encrypted
///   document, a certification at `/P` below 3 — go through
///   [`report_edit_error`] or their own message BEFORE any mutation.
fn cmd_delete_annotation(
    input: &Path,
    page: usize,
    index: usize,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    if page == 0 {
        eprintln!(
            "pdfce-cli: {}: --page is 1-based; 0 is not a page",
            input.display()
        );
        return exit::RUNTIME_ERROR;
    }
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // Resolved inside a block so the session borrow ends before the mutable
    // call below.
    let annot_id = {
        let slots = match session.page_slots() {
            Ok(slots) => slots,
            Err(err) => {
                eprintln!("pdfce-cli: {}: {err}", input.display());
                return exit::RUNTIME_ERROR;
            }
        };
        let Some(slot) = slots.get(page - 1) else {
            eprintln!(
                "pdfce-cli: {}: no page {page} — the document has {} page(s)",
                input.display(),
                slots.len()
            );
            return exit::RUNTIME_ERROR;
        };
        let annots = pdfce_core::annot::page_annotations(&session.graph(), slot.id);
        let Some(annot) = annots.get(index) else {
            eprintln!(
                "pdfce-cli: {}: page {page} has no annotation at index {index} — it has {} (indices 0..{})",
                input.display(),
                annots.len(),
                annots.len().saturating_sub(1)
            );
            return exit::RUNTIME_ERROR;
        };
        // An annotation reached as a DIRECT dictionary inside `/Annots` has no
        // object identity to delete. Malformed (Table 164 dictionaries are
        // indirect objects) and refused by name rather than silently skipped.
        let Some(id) = annot.id else {
            eprintln!(
                "pdfce-cli: {}: page {page} index {index} is a direct dictionary inside /Annots, not an indirect object — it has no identity to delete, and rewriting the array around it would be a repair this command does not perform",
                input.display()
            );
            return exit::EDIT_REFUSED;
        };
        id
    };

    let gone = match session.delete_annotation(annot_id) {
        Ok(gone) => gone,
        Err(err) => return report_edit_error(input, &err),
    };

    // The prose half of the disclosures. Ordered worst-first: the group
    // promotion changes what a reader SEES on other annotations, which is
    // the one an operator is least likely to predict.
    if gone.group_members_promoted > 0 {
        eprintln!(
            "pdfce-cli: {}: {} other annotation(s) were subordinates of the one you deleted (/RT /Group). While it existed, a conforming reader was required to IGNORE their own author and note text and display its instead — so those now become visible. Their /IRT link was removed; nothing else about them changed.",
            input.display(),
            gone.group_members_promoted
        );
    }
    if gone.replies_orphaned > 0 {
        eprintln!(
            "pdfce-cli: {}: {} repl(ies) pointed at the annotation you deleted. They were KEPT — they are separate annotations with their own text — and their now-dangling /IRT was removed, so each is a standalone comment. Deleting a whole thread means deleting each member.",
            input.display(),
            gone.replies_orphaned
        );
    }
    if gone.popup_removed {
        eprintln!(
            "pdfce-cli: {}: its /Popup window was deleted with it — ISO 32000-1 12.5.6.14 says a pop-up \"shall not appear alone\", so leaving it would be non-conforming, not merely untidy.",
            input.display()
        );
    }
    if gone.parent_popup_cleared {
        eprintln!(
            "pdfce-cli: {}: you deleted a /Popup window; its parent annotation was kept (deleting a window does not delete the comment it belongs to) and its now-dangling /Popup key was removed.",
            input.display()
        );
    }
    // DELETE IS NOT REDACT, and this is the one warning that must not be
    // conditional on anything. ISO 32000-1 Annex H.7.3: "although the two
    // objects have been deleted, they are still present in the file" — an
    // incremental save APPENDS a free-list entry, it does not go back and
    // overwrite the bytes. So the note text of a deleted comment is still
    // recoverable from the saved file with a hex editor.
    //
    // An operator deleting a comment because it was confidential is exactly
    // the person who will not think to ask, and the redaction feature two
    // subcommands away is the one that actually removes bytes. Printed on
    // every incremental delete, with the remedy, rather than left to a
    // manual page.
    if matches!(mode, SaveMode::Incremental) {
        eprintln!(
            "pdfce-cli: {}: note — an incremental save APPENDS the deletion; the annotation's bytes, including its note text, are still present in the output file and recoverable (ISO 32000-1 Annex H.7.3). Deleting is not redacting. Pass --mode full if the content must not survive in the file — but note that a full rewrite destroys every existing signature.",
            input.display()
        );
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    let route = match gone.route {
        pdfce_core::edit::AnnotationDeletionRoute::General => "general",
        pdfce_core::edit::AnnotationDeletionRoute::RedactionMark => "redaction-mark",
        pdfce_core::edit::AnnotationDeletionRoute::Dimension => "dimension",
        // `AnnotationDeletionRoute` is #[non_exhaustive]: a future route must
        // print an honest unknown rather than be mapped to the wrong verb.
        _ => "other",
    };
    println!(
        "delete-annotation {} page={page} index={index} subtype={} route={route} popup_removed={} parent_popup_cleared={} replies_orphaned={} group_promoted={} ap_removed={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        sanitize_token(&gone.subtype),
        u32::from(gone.popup_removed),
        u32::from(gone.parent_popup_cleared),
        gone.replies_orphaned,
        gone.group_members_promoted,
        gone.appearance_streams_removed,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `delete-field` and `delete-widget` — the two deletion verbs, which differ
/// only in whether an index was given.
///
/// ## Why ONE function behind two subcommands
///
/// They are the same operation with a different scope, and §3.6.3 makes the
/// last-member case of `delete-widget` *become* `delete-field` — so a second
/// implementation would be two code paths that have to agree about what
/// "gone" means, which is the kind of agreement that quietly lapses. The
/// subcommands stay separate at the surface because `--index` is meaningless
/// for one of them and mandatory for the other, and an optional index whose
/// absence silently means "delete everything" is a footgun.
///
/// ## Contract
///
/// - Emits one `delete-field …` / `delete-widget …` line carrying
///   `widgets_removed=`, `field_removed=`, `selection_cleared=` and
///   `emptied_parents=`, then defers the exit code to [`finish_edit`].
/// - `selection_cleared=1` is §3.6.3's required disclosure and is ALSO
///   printed in prose to stderr — a value the operator set, silently
///   discarded, is exactly what rule 4 forbids.
/// - Refusals — no such field, an index past the end, an encrypted or
///   certified document — go through [`report_edit_error`] before any
///   mutation.
fn cmd_delete_form_field(
    input: &Path,
    name: &str,
    index: Option<usize>,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let outcome_of_delete = match index {
        Some(i) => session.delete_widget(name, i),
        None => session.delete_field(name),
    };
    let deletion = match outcome_of_delete {
        Ok(d) => d,
        Err(err) => return report_edit_error(input, &err),
    };

    // §3.6.3's disclosure, in prose. The machine-readable field below is for
    // scripts; this is for the person who is about to wonder why the form
    // came back blank.
    if deletion.selection_cleared {
        eprintln!(
            "pdfce-cli: field {name:?}: the widget you deleted held this field's selected value, which no remaining widget can display — the selection has been cleared to Off"
        );
    }
    if deletion.emptied_parents > 0 {
        eprintln!(
            "pdfce-cli: field {name:?}: {} grouping node(s) were left with no fields beneath them and were removed as well — a named node owning nothing still occupies its slot in the field-name space",
            deletion.emptied_parents
        );
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    let verb = if index.is_some() {
        "delete-widget"
    } else {
        "delete-field"
    };
    println!(
        "{verb} {} name={:?} index={} widgets_removed={} field_removed={} selection_cleared={} emptied_parents={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        name,
        index.map_or_else(|| "-".to_owned(), |i| i.to_string()),
        deletion.widgets_removed,
        u32::from(deletion.field_removed),
        u32::from(deletion.selection_cleared),
        deletion.emptied_parents,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `delete-field-group` — remove a grouping node and its whole subtree.
///
/// # Why this command refuses to run without `--yes`
///
/// Every other delete in this CLI removes the thing you named. This one
/// removes the thing you named **and every field beneath it**, and the
/// operator typing the command cannot see that set — a subtree is exactly
/// the shape whose contents are invisible from its name. `Personal` might
/// be one field or forty.
///
/// So the default is the listing: resolve the node, print the terminals by
/// name, write nothing, exit `0`. `--yes` is the second, deliberate act.
/// This is rule 4's disclosure obligation in the shape a CLI can honour —
/// there is no canvas to show the affected fields on, so the names are the
/// disclosure, and a flag is the confirmation.
///
/// Exiting `0` from the dry run is deliberate: nothing failed. A non-zero
/// exit would make a scripted preview indistinguishable from a refusal, and
/// the whole point is that previewing is a normal, expected thing to do.
///
/// # Why the terminals are listed on stdout, not stderr
///
/// The opposite of `fill-field`'s conversion note. That note is an aside
/// about an operation you asked for; this listing **is** the output of the
/// dry run — the answer to the question the invocation asked. A script
/// capturing stdout wants it.
fn cmd_delete_field_group(
    input: &Path,
    name: &str,
    output: &Path,
    yes: bool,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // The preview runs the same gates as the deletion, so a dry run that
    // succeeds is a promise the real run can keep.
    let preview = match session.field_group_deletion_preview(name) {
        Ok(p) => p,
        Err(err) => return report_edit_error(input, &err),
    };

    if !yes {
        // The listing IS the output. One line per terminal so the set is
        // greppable and diffable, then a summary that says what else goes.
        // Terminals AND the grouping nodes, both by name. The nodes matter
        // to the operator for a reason the count cannot convey: each one is
        // a NAME being freed, and a name that comes back is a name a later
        // `add-*-field` can take. "nodes=2" does not tell them `Personal`
        // is about to become available again.
        for t in &preview.terminals {
            println!("would-delete field={t:?}");
        }
        for n in &preview.nodes {
            println!("would-delete group={n:?}");
        }
        println!(
            "delete-field-group {} name={:?} DRY-RUN terminals={} widgets={} nodes={} — nothing written; pass --yes to delete",
            input.display(),
            name,
            preview.terminals.len(),
            preview.widgets_removed,
            preview.nodes_removed,
        );
        return exit::SUCCESS;
    }

    let deletion = match session.delete_field_group(name) {
        Ok(d) => d,
        Err(err) => return report_edit_error(input, &err),
    };

    // Named even on the real run. The operator may have passed `--yes`
    // straight away, and a destructive act should say what it destroyed
    // whether or not it was previewed first.
    for t in &deletion.terminals {
        println!("deleted field={t:?}");
    }

    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "delete-field-group {} name={:?} terminals={} widgets_removed={} nodes_removed={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        name,
        deletion.terminals.len(),
        deletion.widgets_removed,
        deletion.nodes_removed,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `add-radio-button` — author ONE member of a radio group.
///
/// ## Contract
///
/// - Emits one `add-radio-button …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - **One invocation adds one MEMBER.** Repeating the verb with the same
///   `--name` and a different `--export-value` is how a group is built; the
///   `merged=` field in the output line says which happened, so a script can
///   tell "created the group" from "joined it" without re-reading the file.
/// - Refusals — an `Off` export value, a duplicate export value in a group
///   that is not `--radios-in-unison`, a positional-`/Opt` group pdfce cannot
///   extend, a name already used by a different field type or KIND, plus
///   every structural refusal the sibling authoring verbs share — go through
///   [`report_edit_error`] BEFORE any mutation.
/// - `--page` is 1-BASED here and 0-based in the core call.
fn cmd_add_radio_button(args: &AddRadioButtonArgs<'_>) -> u8 {
    let (page_index, rect) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec =
        pdfce_core::edit::NewRadioButton::new(page_index, args.name, rect, args.export_value)
            .selected(args.selected)
            .with_group_flags(args.no_toggle_to_off, args.radios_in_unison)
            .with_flags(args.read_only, args.required)
            .with_border(args.border.into(), args.border_width)
            .with_visibility(args.visibility.into());
    // R105, exactly as the sibling verbs: `clap`'s `conflicts_with` rules out
    // BOTH being passed, so only "neither" can reach here, and it is refused
    // rather than defaulted.
    spec = match (args.tooltip, args.no_tooltip) {
        (Some(t), _) => spec.with_tooltip(t),
        (None, true) => spec.declining_tooltip(),
        (None, false) => {
            eprintln!(
                "pdfce-cli: {}: decide about the accessibility name — pass --tooltip <text>, or --no-tooltip to decline it. It is what a screen reader announces for this field, so it is never defaulted silently.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
    };

    // Applied to the SPEC before authoring, so everything downstream
    // — the merge check, the appearance build, the undo entry — sees
    // one fully-formed request rather than a partially-defaulted one.
    let defaults = match read_defaults(&session, args.input, args.defaults_from) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let applied = defaults
        .map(|d| spec.apply_defaults(&d))
        .unwrap_or_default();
    let authored = match session.add_radio_button(&spec) {
        Ok(o) => o,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let field_id = authored.field_id;
    // Folded into the SAME disclosure struct the core produced, not
    // reported alongside it: one channel, so `any()` still answers for
    // everything and a caller gating on it cannot miss half the facts.
    let mut disclosures = authored.disclosures;
    disclosures.defaults_type_mismatch = applied.type_mismatch;
    disclosures.defaults_on_state_ambiguous = applied.on_state_ambiguous;
    report_field_disclosures(args.name, disclosures);
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "add-radio-button {} name={:?} page={} rect={},{},{},{} export_value={:?} selected={} field={} {} merged={} tagged={} struct_tabs={} tooltip_declined={} flags_ignored={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.name,
        args.page,
        rect.llx,
        rect.lly,
        rect.urx,
        rect.ury,
        args.export_value,
        u32::from(args.selected),
        field_id.num,
        field_id.generation,
        u32::from(authored.merged),
        u32::from(authored.disclosures.tagged_document),
        u32::from(authored.disclosures.structure_tab_order),
        u32::from(authored.disclosures.tooltip_declined),
        u32::from(authored.disclosures.group_flags_ignored),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// `add-choice-field` — author a new list box or drop-down.
///
/// ## Contract
///
/// - Emits one `add-choice-field …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - `--option EXPORT=LABEL` splits the submitted value from the displayed
///   one; `--option LABEL` makes them the same. **The first `=` splits**, so
///   a label may contain `=` and an export value may not — the export value
///   is form data and the label is prose, and prose is where an `=` actually
///   turns up.
/// - Refusals — no options, `--editable` without `--combo`, a duplicated
///   export value, plus every structural refusal the other authoring
///   subcommands share — go through [`report_edit_error`] before any
///   mutation.
fn cmd_add_choice_field(args: &AddChoiceFieldArgs<'_>) -> u8 {
    let (page_index, rect) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let options: Vec<pdfce_core::edit::ChoiceOption> = args
        .options
        .iter()
        .map(|raw| match raw.split_once('=') {
            Some((export, display)) => pdfce_core::edit::ChoiceOption::new(export, display),
            None => pdfce_core::edit::ChoiceOption::plain(raw),
        })
        .collect();

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec = pdfce_core::edit::NewChoiceField::new(page_index, args.name, rect, options)
        .multi_select(args.multi_select)
        .sorted(args.sort)
        .with_flags(args.read_only, args.required)
        .with_border(args.border.into(), args.border_width)
        .with_visibility(args.visibility.into());
    if args.combo {
        spec = spec.as_combo(args.editable);
    } else {
        // Carried through UNCHANGED rather than silently cleared, so the core
        // refuses `--editable` without `--combo` instead of the CLI quietly
        // dropping a flag the operator asked for.
        spec.editable = args.editable;
    }
    // R105: exactly one of the two must have been chosen. `clap`'s
    // `conflicts_with` rules out BOTH; only "neither" can reach here, and it
    // is refused rather than defaulted.
    spec = match (args.tooltip, args.no_tooltip) {
        (Some(t), _) => spec.with_tooltip(t),
        (None, true) => spec.declining_tooltip(),
        (None, false) => {
            eprintln!(
                "pdfce-cli: {}: decide about the accessibility name — pass --tooltip <text>, or --no-tooltip to decline it. It is what a screen reader announces for this field, so it is never defaulted silently.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
    };

    // Applied to the SPEC before authoring, so everything downstream
    // — the merge check, the appearance build, the undo entry — sees
    // one fully-formed request rather than a partially-defaulted one.
    let defaults = match read_defaults(&session, args.input, args.defaults_from) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let applied = defaults
        .map(|d| spec.apply_defaults(&d))
        .unwrap_or_default();
    let authored = match session.add_choice_field(&spec) {
        Ok(outcome) => outcome,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let field_id = authored.field_id;
    // R4 + decision 020 §3.4.3/§3.5.3: everything pdfce knows and the
    // operator cannot see is said at the moment it happens, not left to be
    // discovered later.
    // Folded into the SAME disclosure struct the core produced, not
    // reported alongside it: one channel, so `any()` still answers for
    // everything and a caller gating on it cannot miss half the facts.
    let mut disclosures = authored.disclosures;
    disclosures.defaults_type_mismatch = applied.type_mismatch;
    disclosures.defaults_on_state_ambiguous = applied.on_state_ambiguous;
    report_field_disclosures(args.name, disclosures);
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "add-choice-field {} name={:?} page={} rect={},{},{},{} options={} no_options={} combo={} editable={} multi_select={} sort={} field={} {} merged={} tagged={} struct_tabs={} tooltip_declined={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.name,
        args.page,
        rect.llx,
        rect.lly,
        rect.urx,
        rect.ury,
        // The SPEC's count, not the argument count. `--defaults-from` can
        // fill this list, and a summary reading `options=0` beside a file
        // carrying three of them is the shape where a wrong number sits next
        // to a right one and nobody notices.
        spec.options.len(),
        u32::from(authored.disclosures.has_no_options),
        u32::from(args.combo),
        u32::from(args.editable),
        u32::from(args.multi_select),
        u32::from(args.sort),
        field_id.num,
        field_id.generation,
        u32::from(authored.merged),
        u32::from(authored.disclosures.tagged_document),
        u32::from(authored.disclosures.structure_tab_order),
        u32::from(authored.disclosures.tooltip_declined),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// `add-push-button` — author a new push button (§12.7.4.2.2).
///
/// ## Contract
///
/// - Emits one `add-push-button …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - Every refusal — XFA present, a name already used by a different field
///   type or by a grouping node, a degenerate rectangle, an empty name, a
///   page out of range, an undecided accessibility name — goes through
///   [`report_edit_error`] (or the R105 branch below) BEFORE any mutation.
/// - `--page` is 1-BASED here and 0-based in the core call.
/// - **`inert=1` on every successful run.** The machine-readable line
///   carries the fact as a field and not only as a stderr sentence, so a
///   script that captures stdout and discards stderr still learns that the
///   button it just made does nothing. This is the one creation verb whose
///   success has a caveat that is true 100% of the time, and a caveat only
///   ever delivered on the human channel is one that automation cannot see.
fn cmd_add_push_button(args: &AddPushButtonArgs<'_>) -> u8 {
    let (page_index, rect) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec = pdfce_core::edit::NewPushButton::new(page_index, args.name, rect, args.caption)
        .with_flags(args.read_only)
        .with_border(args.border.into(), args.border_width)
        .with_visibility(args.visibility.into());
    // R105: exactly one of the two must have been chosen. `clap`'s
    // `conflicts_with` rules out BOTH; only "neither" can reach here, and it
    // is refused rather than defaulted.
    spec = match (args.tooltip, args.no_tooltip) {
        (Some(t), _) => spec.with_tooltip(t),
        (None, true) => spec.declining_tooltip(),
        (None, false) => {
            eprintln!(
                "pdfce-cli: {}: decide about the accessibility name — pass --tooltip <text>, or --no-tooltip to decline it. It is what a screen reader announces for this field, so it is never defaulted silently.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
    };

    // Applied to the SPEC before authoring, so everything downstream sees one
    // fully-formed request rather than a partially-defaulted one — and so the
    // empty-caption disclosure is computed against the caption that actually
    // lands, not against the argument.
    let defaults = match read_defaults(&session, args.input, args.defaults_from) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let applied = defaults
        .map(|d| spec.apply_defaults(&d))
        .unwrap_or_default();
    let authored = match session.add_push_button(&spec) {
        Ok(outcome) => outcome,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let field_id = authored.field_id;
    // Folded into the SAME disclosure struct the core produced, not reported
    // alongside it: one channel, so `any()` still answers for everything.
    let mut disclosures = authored.disclosures;
    disclosures.defaults_type_mismatch = applied.type_mismatch;
    disclosures.defaults_on_state_ambiguous = applied.on_state_ambiguous;
    report_field_disclosures(args.name, disclosures);
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "add-push-button {} name={:?} page={} rect={},{},{},{} caption={:?} no_caption={} inert={} read_only={} field={} {} merged={} tagged={} struct_tabs={} tooltip_declined={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.name,
        args.page,
        rect.llx,
        rect.lly,
        rect.urx,
        rect.ury,
        // The SPEC's caption, not the argument. `--defaults-from` can fill
        // it, and a summary printing the empty argument beside a file
        // carrying a copied caption is the wrong-number-next-to-a-right-one
        // shape the choice verb's `options=` count already had once.
        spec.caption,
        u32::from(authored.disclosures.push_button_no_caption),
        u32::from(authored.disclosures.push_button_inert),
        u32::from(args.read_only),
        field_id.num,
        field_id.generation,
        u32::from(authored.merged),
        u32::from(authored.disclosures.tagged_document),
        u32::from(authored.disclosures.structure_tab_order),
        u32::from(authored.disclosures.tooltip_declined),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Print every disclosure a field-creation call owes the operator.
///
/// # Why this is one function rather than three copies
///
/// Three of these are things pdfce KNOWS and the operator cannot see by
/// looking at the result: a tagged document whose tag tree the new field is
/// absent from, a page whose structure tab order gives the new field no tab
/// position at all, and a declined accessibility name. None is an error —
/// each is a true statement about a document created exactly as asked — and
/// none is discoverable after the fact.
///
/// Copied per verb, the third copy is where one of them goes missing, and it
/// would go missing SILENTLY: a disclosure that is never printed looks
/// exactly like a disclosure that did not apply.
/// Read a `--defaults-from` template, or `None` when the flag was absent.
///
/// Separated from the four creation commands so the lookup, the
/// field-not-found refusal and the "flag absent" case are decided once. A
/// template that does not exist is an ERROR, not an empty default: the
/// operator named a field, and silently proceeding with nothing copied would
/// be indistinguishable from a successful copy of a field that has no
/// copyable properties.
fn read_defaults(
    session: &pdfce_core::edit::EditSession,
    input: &Path,
    from: Option<&str>,
) -> Result<Option<pdfce_core::edit::FieldDefaults>, u8> {
    match from {
        None => Ok(None),
        Some(name) => match session.field_defaults(name) {
            Ok(defaults) => Ok(Some(defaults)),
            Err(err) => Err(report_edit_error(input, &err)),
        },
    }
}

fn report_field_disclosures(name: &str, d: pdfce_core::edit::FieldAuthorDisclosures) {
    if d.tooltip_declined {
        eprintln!(
            "pdfce-cli: field {name:?}: no accessibility name (tooltip) was set, as requested — screen readers will announce the field's name instead"
        );
    }
    if d.tagged_document {
        eprintln!(
            "pdfce-cli: field {name:?}: this document is tagged (/StructTreeRoot), and the new field is NOT in its structure tree — pdfce does not write structure elements"
        );
    }
    if d.structure_tab_order {
        eprintln!(
            "pdfce-cli: field {name:?}: this page uses structure tab order (/Tabs /S) and the new field is untagged, so its tab position is UNDEFINED — not last. Set an explicit tab order, or use row/column order for this page."
        );
    }
    if d.has_no_options {
        eprintln!(
            "pdfce-cli: field {name:?}: this choice field has no options and cannot be filled until options are added"
        );
    }
    if d.group_flags_ignored {
        eprintln!(
            "pdfce-cli: field {name:?}: this member joined an EXISTING radio group, so the group's own --no-toggle-to-off / --radios-in-unison settings apply and the ones passed here were ignored. Those flags live on the field, so honouring them now would have changed how the members already in the group behave."
        );
    }
    if d.defaults_type_mismatch {
        eprintln!(
            "pdfce-cli: field {name:?}: --defaults-from copied NOTHING. Every property the field types share is a yes/no flag, and those are never copied (a --flag cannot express 'off'), so the only copyable properties are type-specific: --max-len for text, the option list for choice, the on-state for a check box, the caption for a push button. A radio template has nothing to copy at all."
        );
    }
    if d.defaults_on_state_ambiguous {
        eprintln!(
            "pdfce-cli: field {name:?}: the --defaults-from check box has widgets with DIFFERENT on-state names, and the first one was used. A check box normally uses one on-state everywhere it appears, so a template that does not is worth a look."
        );
    }
    if d.push_button_inert {
        eprintln!(
            "pdfce-cli: field {name:?}: this push button has NO ACTION and does nothing when clicked. pdfce recognises and preserves actions but never authors one, so what was created is a valid, inert button — a placeholder to be wired up elsewhere, not a working submit or reset."
        );
    }
    if d.push_button_no_caption {
        eprintln!(
            "pdfce-cli: field {name:?}: this push button has an EMPTY caption and will render as a blank plate. Pass --caption <text> if that was not intended."
        );
    }
}

/// Arguments for [`cmd_add_image`], grouped so the dispatcher stays one
/// expression and `clippy::too_many_arguments` is satisfied honestly rather
/// than by an `#[allow]`.
struct AddImageArgs<'a> {
    input: &'a Path,
    image: &'a Path,
    page: usize,
    rect: &'a str,
    stretch: bool,
    natural: bool,
    compression: pdfce_core::image_import::ImageCompression,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `add-image` — place a raster image on a page (§8.9.5).
///
/// ## Contract
///
/// - Emits one `add-image …` line on stdout carrying **every** disclosure as
///   a field, then defers the exit code to [`finish_edit`].
/// - Emits the prose form of the same disclosures on stderr.
/// - Every refusal — an unreadable file, an unsupported format, an
///   unsupported sub-feature, a degenerate rectangle, a page out of range, a
///   certified document — is reported BEFORE any mutation.
/// - `--page` is 1-BASED here and 0-based in the core call, matching every
///   other page-taking subcommand.
///
/// ## Why both channels carry the same facts
///
/// This is the `add-push-button` precedent generalised. A disclosure
/// delivered only as an English sentence on stderr is invisible to a script
/// that captures stdout and discards stderr — and image placement has
/// several disclosures a batch job genuinely needs to act on: that a JPEG
/// was embedded verbatim (`verbatim=1`) or a BMP re-compressed
/// (`recompressed=`), that a soft mask was written that pdfce's own renderer
/// will not show (`smask_not_previewed=1`), that an image is being enlarged
/// past its own resolution (`low_res=1`), or that a CMYK JPEG's polarity is
/// undeclared (`cmyk_polarity_unverifiable=1`, R30). Those belong in the
/// machine-readable line, not only in the human one.
fn cmd_add_image(args: &AddImageArgs<'_>) -> u8 {
    use pdfce_core::edit::NewImage;
    use pdfce_core::image_import;

    let (page_index, requested) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    // Read and parse the image BEFORE opening the PDF: a bad image is by far
    // the likelier mistake, and diagnosing it without having parsed a
    // possibly-large document first is both faster and clearer.
    let bytes = match std::fs::read(args.image) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.image.display());
            return exit::IO_ERROR;
        }
    };
    let options = image_import::ImportOptions::new().with_compression(args.compression);
    let img = match image_import::import_with(&bytes, &options) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", args.image.display());
            if let image_import::ImageImportError::Unsupported { feature } = &err {
                eprintln!(
                    "pdfce-cli: the unsupported feature is {feature}. pdfce places \
                     {}; re-saving the file without that feature is usually enough.",
                    image_import::SUPPORTED_FORMATS
                );
            }
            return exit::EDIT_REFUSED;
        }
    };

    // `--natural` keeps the rectangle's lower-left corner and replaces its
    // SIZE. Applied here rather than in the core so the core's placement
    // contract stays "the caller's rectangle, fitted" with no metadata-driven
    // branch inside it.
    let rect = if args.natural {
        let (w, h) = img.natural_size_pt();
        pdfce_core::page_tree::Rect {
            llx: requested.llx,
            lly: requested.lly,
            urx: requested.llx + w,
            ury: requested.lly + h,
        }
    } else {
        requested
    };

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec = NewImage::new(page_index, rect, &img);
    if args.stretch || args.natural {
        // `--natural` implies an exact fit: the rectangle WAS computed from
        // the image's own aspect ratio, so "contain" would be a no-op that
        // could still round its way into a spurious letterboxed= report.
        spec = spec.stretching();
    }
    let placed = match session.add_image(&spec) {
        Ok(outcome) => outcome,
        Err(err) => return report_edit_error(args.input, &err),
    };
    report_image_disclosures(args.image, &placed);

    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };

    let d = &placed.disclosures;
    let p = placed.placed_rect;
    let (px_w, px_h) = img.display_size_px();
    let r = &outcome.report;
    println!(
        "add-image {} image={} format={} page={} pixels={}x{} bpc={} colorspace={} \
         filter={} verbatim={} rect={},{},{},{} placed={:.3},{:.3},{:.3},{:.3} fit={} \
         image_obj={} {} smask={} name={} dpi={} dpi_source={} eff_dpi={:.1},{:.1} \
         compression_requested={} compression_applied={} quality={} \
         source_bytes={} stored_bytes={} lossless_from_lossy={} jpeg_from_lossy={} \
         letterboxed={} distorted={} low_res={} recompressed={} smask_written={} \
         transparency_not_previewed={} colour_key={} profile_dropped={} \
         cmyk_polarity_unverifiable={} progressive={} exif_orientation={} \
         bmp_padding_ignored={} version_needed={} tagged={} mode={} -> {}; \
         changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.image.display(),
        img.format.name(),
        args.page,
        px_w,
        px_h,
        img.bits_per_component,
        colorspace_name(&img.color_space),
        filter_name(img.filter),
        u32::from(d.recompressed.is_none()),
        requested.llx,
        requested.lly,
        requested.urx,
        requested.ury,
        p.llx,
        p.lly,
        p.urx,
        p.ury,
        if args.stretch || args.natural {
            "stretch"
        } else {
            "contain"
        },
        placed.image_id.num,
        placed.image_id.generation,
        placed.soft_mask_id.map_or_else(
            || "-".to_owned(),
            |id| format!("{} {}", id.num, id.generation)
        ),
        String::from_utf8_lossy(&placed.resource_name),
        img.dpi
            .map_or_else(|| "-".to_owned(), |(x, y)| format!("{x:.1},{y:.1}")),
        dpi_source_key(img.notes.dpi_source),
        d.effective_dpi.0,
        d.effective_dpi.1,
        d.requested_compression.key(),
        d.applied_compression.key(),
        d.jpeg_quality
            .map_or_else(|| "-".to_owned(), |q| q.to_string()),
        d.source_bytes,
        d.stored_bytes,
        u32::from(d.lossless_from_lossy),
        u32::from(d.jpeg_from_lossy),
        u32::from(d.letterboxed),
        u32::from(d.aspect_distorted),
        u32::from(d.below_screen_resolution),
        d.recompressed.map_or("-", |r| r.key()),
        u32::from(d.soft_mask_written),
        u32::from(d.transparency_not_previewed),
        u32::from(d.colour_key_mask_written),
        u32::from(d.colour_profile_dropped),
        u32::from(d.cmyk_polarity_unverifiable),
        u32::from(d.progressive_jpeg),
        d.exif_orientation_applied
            .map_or_else(|| "-".to_owned(), |v| v.to_string()),
        u32::from(d.bmp_fourth_byte_ignored),
        d.version_ahead_of_document.map_or_else(
            || "-".to_owned(),
            |(f, doc)| {
                let (major, minor) = f.since();
                format!("{major}.{minor}(doc {doc})")
            }
        ),
        u32::from(d.tagged_document),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Why a compression policy was substituted, in one clause.
///
/// The substitution itself is reported unconditionally; this supplies the
/// *reason*, which is the part an operator can act on. Both directions are
/// real: a passthrough that could not happen, and a lossless re-encode that
/// turned out to be unnecessary.
fn compression_substitution_reason(d: &pdfce_core::edit::ImageAuthorDisclosures) -> &'static str {
    use pdfce_core::image_import::RecompressReason;
    match d.recompressed {
        Some(RecompressReason::NoCompressedSource) => {
            "This format stores no compressed bytes to pass through, so pdfce compressed it losslessly on the way in — the pixels are unchanged."
        }
        Some(RecompressReason::AlphaSplit) => {
            "This image's alpha channel is interleaved with its colour channels and PDF carries opacity in a separate image, so the samples had to be split and re-compressed — losslessly; the pixels are unchanged."
        }
        None => {
            "The source was already stored losslessly, so its own bytes were kept unchanged rather than re-compressed — strictly better than what was asked for."
        }
        // `LosslessRequested` and `JpegRequested` are policies the operator
        // CHOSE, so a "substitution" involving them is not a substitution at
        // all — their own dedicated notes below say what the choice cost.
        Some(_) => "See the note above for why.",
    }
}

/// The `/ColorSpace` an imported image landed in, as a stable key.
fn colorspace_name(cs: &pdfce_core::image_import::ImportColorSpace) -> &'static str {
    use pdfce_core::image_import::ImportColorSpace;
    match cs {
        ImportColorSpace::DeviceGray => "DeviceGray",
        ImportColorSpace::DeviceRgb => "DeviceRGB",
        ImportColorSpace::DeviceCmyk => "DeviceCMYK",
        ImportColorSpace::Indexed { .. } => "Indexed",
        // `ImportColorSpace` is `#[non_exhaustive]`, so a wildcard is
        // required. It reports "?" rather than guessing a plausible name:
        // a new colour space reaching this line means the CLI's vocabulary
        // is out of date, and a wrong-but-plausible label would hide that
        // from the script reading the field.
        _ => "?",
    }
}

/// The `/Filter` (and predictor, where there is one) as a stable key.
fn filter_name(f: pdfce_core::image_import::ImportFilter) -> &'static str {
    use pdfce_core::image_import::ImportFilter;
    match f {
        ImportFilter::DctDecode => "DCTDecode",
        ImportFilter::Flate => "FlateDecode",
        ImportFilter::FlatePngPredictor { .. } => "FlateDecode+Predictor15",
        // See `colorspace_name` for why the wildcard reports "?".
        _ => "?",
    }
}

/// Where a declared resolution came from, as a stable key. `assumed` is not
/// a claim the file made — it is pdfce saying it had nothing to go on.
fn dpi_source_key(s: pdfce_core::image_import::DpiSource) -> &'static str {
    use pdfce_core::image_import::DpiSource;
    match s {
        DpiSource::Assumed => "assumed-72",
        DpiSource::JfifDensity => "jfif",
        DpiSource::ExifResolution => "exif",
        DpiSource::PngPhys => "png-phys",
        DpiSource::BmpPelsPerMeter => "bmp-ppm",
        // See `colorspace_name` for why the wildcard reports "?".
        _ => "?",
    }
}

/// Print, on stderr, every disclosure an image placement owes the operator.
///
/// One function rather than a run of inline `eprintln!`s at the call site,
/// for the same reason [`report_field_disclosures`] is one: a disclosure
/// that is never printed looks exactly like a disclosure that did not apply,
/// so the set has to live in one place where it can be read as a set.
fn report_image_disclosures(image: &Path, outcome: &pdfce_core::edit::ImageAuthorOutcome) {
    let d = &outcome.disclosures;
    let name = image.display();
    if d.requested_compression != d.applied_compression {
        eprintln!(
            "pdfce-cli: {name}: --compression {} was asked for and {} was used. {}",
            d.requested_compression.key(),
            d.applied_compression.key(),
            compression_substitution_reason(d),
        );
    }
    if d.lossless_from_lossy {
        eprintln!(
            "pdfce-cli: {name}: this is a LOSSY source stored losslessly, as asked. That preserves exactly the pixels it decodes to — compression artefacts included — and RECOVERS NOTHING that was already lost, while typically multiplying the stored size several-fold. If the goal was a better picture rather than a stable one to edit, --compression passthrough is the cheaper answer."
        );
    }
    // The lossy re-encode, stated in two sentences rather than one, because
    // "your bytes changed" and "your picture got worse, twice over" are
    // different facts and only the second is unrecoverable.
    if d.recompressed == Some(pdfce_core::image_import::RecompressReason::JpegRequested) {
        let q = d
            .jpeg_quality
            .map_or_else(|| "?".to_owned(), |q| q.to_string());
        eprintln!(
            "pdfce-cli: {name}: re-encoded as JPEG at quality {q}, as asked — {} bytes in, {} bytes stored. This is LOSSY: the stored picture is not the one you handed over, and no later step can recover the difference.",
            d.source_bytes, d.stored_bytes
        );
        if d.jpeg_from_lossy {
            eprintln!(
                "pdfce-cli: {name}: that source was ALREADY a JPEG, so this is a SECOND lossy pass. The DCT has now run twice, and the second pass quantises the first pass's ringing and blocking INTO the picture rather than smoothing them out — the damage compounds, it is invisible at editing zoom, and raising --quality does not undo it. If a lossless original exists, place that instead."
            );
        }
    }
    if d.letterboxed {
        let p = outcome.placed_rect;
        eprintln!(
            "pdfce-cli: {name}: the image kept its shape and was CENTRED in the rectangle, so it landed at {:.2},{:.2},{:.2},{:.2} rather than filling it. Pass --stretch to fill the rectangle exactly.",
            p.llx, p.lly, p.urx, p.ury
        );
    }
    if d.aspect_distorted {
        eprintln!(
            "pdfce-cli: {name}: --stretch was used and the rectangle's shape differs from the image's, so the image is DISTORTED — as asked."
        );
    }
    if d.below_screen_resolution {
        eprintln!(
            "pdfce-cli: {name}: at this size the image works out to {:.0} × {:.0} dpi, below one image pixel per point — it is being enlarged past its own resolution and will look soft in print.",
            d.effective_dpi.0, d.effective_dpi.1
        );
    }
    match d.recompressed {
        None => {}
        Some(pdfce_core::image_import::RecompressReason::AlphaSplit) => eprintln!(
            "pdfce-cli: {name}: this image's alpha channel is interleaved with its colour channels, and PDF carries opacity in a SEPARATE image, so the samples were split and re-compressed. The pixels are unchanged — the re-compression is lossless — but the embedded bytes are no longer the file's own."
        ),
        Some(pdfce_core::image_import::RecompressReason::NoCompressedSource) => eprintln!(
            "pdfce-cli: {name}: a BMP is uncompressed, so pdfce compressed it on the way in. The pixels are unchanged and the result is far smaller, but the embedded bytes are no longer the file's own."
        ),
        // Deliberately silent: the operator ASKED for this one, and the
        // `lossless_from_lossy` disclosure above already says what it cost.
        // A second sentence restating the same fact is how a disclosure set
        // trains people to skim.
        Some(pdfce_core::image_import::RecompressReason::LosslessRequested) => {}
        // Likewise: asked for, and already reported above with the size
        // change and the generation-loss warning it earns.
        Some(pdfce_core::image_import::RecompressReason::JpegRequested) => {}
        // `RecompressReason` is `#[non_exhaustive]`. A future reason must
        // still SAY that a re-compression happened, even before this CLI
        // learns to explain it — silence is the one unacceptable answer.
        Some(other) => eprintln!(
            "pdfce-cli: {name}: pdfce decoded and re-compressed this image ({}). The pixels are unchanged, but the embedded bytes are no longer the file's own.",
            other.key()
        ),
    }
    if d.soft_mask_written {
        eprintln!(
            "pdfce-cli: {name}: the transparency was written as a soft mask (/SMask), not flattened against white — the page shows through, as it should."
        );
    }
    // The `transparency_not_previewed` note is GONE — `pdfce-render` now
    // composites both `/SMask` and colour-key `/Mask`, so the field is
    // retired to a constant `false` in core and there is nothing to say.
    // The stdout field survives (a stable-line key is a contract) and
    // reports `0`; only the prose is removed, because prose that fires when
    // nothing is wrong is what teaches an operator to stop reading it.
    if d.colour_key_mask_written {
        eprintln!(
            "pdfce-cli: {name}: the image declared one fully-transparent colour, written as a colour-key /Mask. The image data itself was embedded unchanged."
        );
    }
    if d.colour_profile_dropped {
        eprintln!(
            "pdfce-cli: {name}: this file carries embedded colour-management data (an ICC profile or a gamma/chromaticity claim) that pdfce did NOT carry over — the image was placed in a device colour space, so colours may shift slightly."
        );
    }
    if d.cmyk_polarity_unverifiable {
        eprintln!(
            "pdfce-cli: {name}: this is a four-component (CMYK) JPEG whose stored polarity NOTHING in the file declares. pdfce embedded it unchanged and wrote no /Decode array, which is what every production PDF engine does. If it appears as a photographic negative, that is why — and the fix belongs in the source file, not here."
        );
    }
    if d.progressive_jpeg {
        eprintln!(
            "pdfce-cli: {name}: this is a progressive JPEG. It is legal inside a PDF from version 1.3 and was embedded unchanged, but it decodes more slowly and uses more memory than a baseline one. Re-save it as baseline if the document will be opened often."
        );
    }
    if let Some(o) = d.exif_orientation_applied {
        eprintln!(
            "pdfce-cli: {name}: this image carries EXIF orientation {o}, which pdfce applied by turning the PLACEMENT rather than re-encoding the pixels — so it appears the right way up at no cost in quality."
        );
    }
    if d.bmp_fourth_byte_ignored {
        eprintln!(
            "pdfce-cli: {name}: this 32-bit BMP's fourth byte per pixel was ignored. A BI_RGB bitmap has no alpha channel — that byte is padding, and treating it as opacity would have made the image invisible."
        );
    }
    if let Some((feature, doc)) = d.version_ahead_of_document {
        let (major, minor) = feature.since();
        eprintln!(
            "pdfce-cli: {name}: this image uses {} (PDF {major}.{minor}) in a document that declares PDF {doc}. pdfce did NOT rewrite the document's version header — that is a structural change you did not ask for. Readers are overwhelmingly version-tolerant, so the image will almost certainly display.",
            feature.name()
        );
    }
    if d.tagged_document {
        eprintln!(
            "pdfce-cli: {name}: this document is tagged (/StructTreeRoot) and the new image is NOT in its structure tree, so it has no alternate text and assistive technology cannot describe it. pdfce does not write structure elements."
        );
    }
}

/// `add-text-field` — author a new text form field.
///
/// ## Contract
///
/// - Emits one `add-text-field …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - Every refusal — XFA present, a name already used by a different field
///   type, a degenerate rectangle, an empty name, a page out of range —
///   goes through [`report_edit_error`] BEFORE any mutation, with the same
///   message and exit code the GUI will surface.
/// - `--page` is 1-BASED here and 0-based in the core call, matching every
///   other page-taking subcommand in this CLI.
/// - Every disclosure the core reports is printed by
///   [`report_field_disclosures`], so a fact stated by one authoring verb
///   cannot be silently dropped by another.
fn cmd_add_text_field(args: &AddTextFieldArgs<'_>) -> u8 {
    let (page_index, rect) = match parse_page_and_rect(args.input, args.page, args.rect) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let (llx, lly, urx, ury) = (rect.llx, rect.lly, rect.urx, rect.ury);

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };

    let mut spec = pdfce_core::edit::NewTextField::new(page_index, args.name, rect)
        .with_password(args.password)
        .with_comb(args.comb)
        .with_border(args.border.into(), args.border_width)
        .with_visibility(args.visibility.into())
        .with_flags(args.multiline, args.read_only, args.required);
    if let Some(v) = args.value {
        spec = spec.with_value(v);
    }
    if let Some(m) = args.max_len {
        spec = spec.with_max_len(m);
    }
    // R105: exactly one of the two must have been chosen. `clap`'s
    // `conflicts_with` rules out BOTH; only "neither" can reach here, and it
    // is refused rather than defaulted.
    spec = match (args.tooltip, args.no_tooltip) {
        (Some(t), _) => spec.with_tooltip(t),
        (None, true) => spec.declining_tooltip(),
        (None, false) => {
            eprintln!(
                "pdfce-cli: {}: decide about the accessibility name — pass --tooltip <text>, or --no-tooltip to decline it. It is what a screen reader announces for this field, so it is never defaulted silently.",
                args.input.display()
            );
            return exit::EDIT_REFUSED;
        }
    };

    // Applied to the SPEC before authoring, so everything downstream
    // — the merge check, the appearance build, the undo entry — sees
    // one fully-formed request rather than a partially-defaulted one.
    let defaults = match read_defaults(&session, args.input, args.defaults_from) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let applied = defaults
        .map(|d| spec.apply_defaults(&d))
        .unwrap_or_default();
    let authored = match session.add_text_field(&spec) {
        Ok(o) => o,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let field_id = authored.field_id;
    // Folded into the SAME disclosure struct the core produced, not
    // reported alongside it: one channel, so `any()` still answers for
    // everything and a caller gating on it cannot miss half the facts.
    let mut disclosures = authored.disclosures;
    disclosures.defaults_type_mismatch = applied.type_mismatch;
    disclosures.defaults_on_state_ambiguous = applied.on_state_ambiguous;
    report_field_disclosures(args.name, disclosures);
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "add-text-field {} name={:?} page={} rect={},{},{},{} field={} {} merged={} tagged={} struct_tabs={} tooltip_declined={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.name,
        args.page,
        llx,
        lly,
        urx,
        ury,
        field_id.num,
        field_id.generation,
        u32::from(authored.merged),
        u32::from(authored.disclosures.tagged_document),
        u32::from(authored.disclosures.structure_tab_order),
        u32::from(authored.disclosures.tooltip_declined),
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// `dimension-display` — switch a placed circular ce dimension between the
/// radius and the diameter reading (Pass 34.2).
///
/// ## Contract
///
/// - Emits one `dimension-display …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - An unknown id, **or a LINEAR ce dimension**, is refused through
///   [`report_edit_error`] before any mutation, with the same message and exit
///   code the GUI surfaces. The linear refusal is the interesting one: it is
///   how a script learns it aimed the verb at the wrong ce dimension rather
///   than writing a file in which nothing changed.
/// - Six parameters rather than a borrowed args struct: the sibling
///   `dimension-offset` needed one to stay under clippy's arg-count ceiling
///   because it carries two extra `f64`s; this one has the same shape as
///   `dimension-delete`, which takes them plainly.
fn cmd_dimension_display(
    input: &Path,
    dimension: u32,
    show: DisplayReading,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    if let Err(err) = session.set_dimension_display(
        pdfce_core::dimension::DimensionId(dimension),
        show.show_diameter(),
    ) {
        return report_edit_error(input, &err);
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "dimension-display {} dimension={dimension} show={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        show.token(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `dimension-delete` — remove one ce dimension and every trace of it.
///
/// ## Contract
///
/// - Emits one `dimension-delete …` line with the usual save-report fields,
///   then defers the exit code to [`finish_edit`].
/// - An unknown id is refused through [`report_edit_error`] before any
///   mutation, with the same message and exit code the GUI surfaces.
fn cmd_dimension_delete(
    input: &Path,
    dimension: u32,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    if let Err(err) = session.delete_dimension(pdfce_core::dimension::DimensionId(dimension)) {
        return report_edit_error(input, &err);
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "dimension-delete {} dimension={dimension} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

fn cmd_dimension_list(input: &Path) -> u8 {
    use pdfce_core::dimension::{DimensionKind, ScaleState};

    let doc = match open_for_read(input) {
        Ok(doc) => doc,
        Err(code) => return code,
    };
    let session = pdfce_core::edit::EditSession::new(doc);
    let model = session.dimension_model();
    println!(
        "dimension-list {} groups={} dimensions={}",
        input.display(),
        model.groups().len(),
        model.dimensions().len()
    );
    for g in model.groups() {
        let scale = match g.scale {
            ScaleState::NeverSet => "no-scale".to_owned(),
            ScaleState::OneToOne => "1:1".to_owned(),
            ScaleState::Calibrated { scale } => format!("{scale} {}/pt", g.unit().abbrev()),
        };
        println!(
            "  group {} \"{}\" unit={} scale={scale} visible={} members={}",
            g.id.0,
            g.name,
            g.unit().token(),
            g.visible,
            model.member_count(g.id),
        );
    }
    for d in model.dimensions() {
        let value = model.display(d.id).map_or_else(String::new, |m| m.text);
        let kind = match d.kind {
            DimensionKind::Linear { .. } => "linear",
            DimensionKind::Circular {
                show_diameter: true,
                ..
            } => "diameter",
            DimensionKind::Circular { .. } => "radius",
        };
        // The placement, for a linear dimension. Printed because it is
        // otherwise invisible from the CLI — an operator scripting
        // `subpath-delete`-style batch work cannot see WHERE a ce dimension
        // sits, only what it says, and `dimension-offset` below needs the
        // current values to adjust from.
        let placement = match d.kind {
            DimensionKind::Linear {
                offset, text_along, ..
            } => format!(" offset={offset} text_along={text_along}"),
            DimensionKind::Circular { .. } => String::new(),
        };
        println!(
            "  dim {} group={} kind={kind} value=\"{value}\"{placement}",
            d.id.0, d.group.0
        );
    }
    exit::SUCCESS
}

/// `group-add` — create a named dimension group (Pass 12.M2).
fn cmd_group_add(
    input: &Path,
    name: &str,
    unit_str: &str,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let Some(unit) = pdfce_core::dimension::Unit::parse(unit_str) else {
        eprintln!(
            "pdfce-cli: {}: unknown --unit `{unit_str}` (mm|cm|m|in|ft|ft-in)",
            input.display()
        );
        return exit::EDIT_REFUSED;
    };
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let group = match session.add_dimension_group(name, unit) {
        Ok(id) => id,
        Err(err) => return report_edit_error(input, &err),
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "group-add {} name=\"{name}\" unit={} mode={} -> {}; group={} changed={} objects={} \
appended={} out_bytes={}",
        input.display(),
        unit.token(),
        mode.name(),
        output.display(),
        group.0,
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
    );
    finish_edit(input, &outcome)
}

/// Borrowed argument bundle for [`cmd_group_set_scale`] (clippy arg-count).
struct GroupSetScaleArgs<'a> {
    input: &'a Path,
    group: u32,
    real_length: Option<&'a str>,
    drawn: Option<f64>,
    ratio: Option<&'a str>,
    unit: &'a str,
    one_to_one: bool,
    precision: Option<u32>,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `group-set-scale` — set a group's scale + units and regenerate members.
fn cmd_group_set_scale(args: &GroupSetScaleArgs<'_>) -> u8 {
    use pdfce_core::dimension::{
        GroupId, NumberFormat, ScaleEntry, ScaleState, Unit, parse_length, preview_group_scale,
    };

    let Some(unit) = Unit::parse(args.unit) else {
        eprintln!(
            "pdfce-cli: {}: unknown --unit `{}` (mm|cm|m|in|ft|ft-in)",
            args.input.display(),
            args.unit
        );
        return exit::EDIT_REFUSED;
    };
    let format = match args.precision {
        Some(p) if unit == Unit::FeetInches => NumberFormat::feet_inches(p, false),
        Some(p) => NumberFormat::decimal(unit, p),
        None => unit.default_format(),
    };

    let scale = if args.one_to_one {
        ScaleState::OneToOne
    } else if let Some(ratio) = args.ratio {
        let Some((paper, real)) = parse_ratio(ratio) else {
            eprintln!("pdfce-cli: {}: --ratio must be `N:M`", args.input.display());
            return exit::EDIT_REFUSED;
        };
        match preview_group_scale(ScaleEntry::Ratio {
            paper,
            real,
            basis: unit,
        }) {
            Some(p) => ScaleState::Calibrated { scale: p.scale },
            None => {
                eprintln!("pdfce-cli: {}: invalid ratio", args.input.display());
                return exit::EDIT_REFUSED;
            }
        }
    } else if let (Some(real_text), Some(drawn)) = (args.real_length, args.drawn) {
        // Parsed with the SAME function the GUI field uses, so `55 5/8"` means
        // one thing in this product rather than two. A second, CLI-local
        // number parser would be a duplicated predicate and would drift (R92)
        // — and it would drift silently, because both would keep accepting
        // plain decimals long after they disagreed about fractions.
        let parsed = match parse_length(real_text, unit) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("pdfce-cli: --real-length: {e}");
                return exit::EDIT_REFUSED;
            }
        };
        // A unit named in the text wins over `--unit`, matching the GUI: the
        // operator said inches by typing `"`, and making them repeat it in a
        // flag would be asking the same question twice.
        let unit = if parsed.unit_from_text {
            parsed.unit
        } else {
            unit
        };
        match preview_group_scale(ScaleEntry::RealLength {
            drawn_pdf_length: drawn,
            real_length: parsed.value,
            unit,
        }) {
            Some(p) => ScaleState::Calibrated { scale: p.scale },
            None => {
                eprintln!(
                    "pdfce-cli: {}: --drawn must be a positive length",
                    args.input.display()
                );
                return exit::EDIT_REFUSED;
            }
        }
    } else {
        eprintln!(
            "pdfce-cli: {}: give --one-to-one, --ratio N:M, or --real-length L --drawn D",
            args.input.display()
        );
        return exit::EDIT_REFUSED;
    };

    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let members = match session.set_group_scale(GroupId(args.group), scale, format) {
        Ok(n) => n,
        Err(err) => return report_edit_error(args.input, &err),
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "group-set-scale {} group={} mode={} -> {}; members_regenerated={members} \
changed={} objects={} appended={} out_bytes={}",
        args.input.display(),
        args.group,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
    );
    finish_edit(args.input, &outcome)
}

/// `layer-toggle` — show/hide a dimension group's optional-content layer.
fn cmd_layer_toggle(
    input: &Path,
    group: u32,
    hide: bool,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    use pdfce_core::dimension::GroupId;

    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let visible = match session.toggle_dimension_layer(GroupId(group), !hide) {
        Ok(v) => v,
        Err(err) => return report_edit_error(input, &err),
    };
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "layer-toggle {} group={group} mode={} -> {}; visible={visible} changed={} \
objects={} appended={} out_bytes={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
    );
    finish_edit(input, &outcome)
}

// =====================================================================
// `object-list` — paint-order object inventory + headless hit-test
// =====================================================================
//
// WHY this subcommand exists: `object-move`, `object-delete` and
// `node-move` all address an object by its 0-based paint-order index, and
// before this there was NO way — CLI or GUI — to discover that index. The
// editing subcommands' own help text pointed at "`object-list`-style
// tooling" that did not exist, so the three edits were effectively
// unusable outside a debugger. This closes that gap.
//
// It is deliberately read-only and deliberately thin: every number it
// prints comes from `pdfce_core::vector::decompose_page` and
// `pdfce_core::vector::hit_test_point` — the SAME two functions the GUI's
// `ObjectModelProvider` calls — so the listing cannot drift from what the
// edits address or from what a click in the GUI selects. Re-deriving
// either here would recreate exactly the "two decompositions quietly
// diverge" failure decision 011 Z2 warns against.

/// Default `--tolerance` for `object-list --hit`, in page-space points.
///
/// Chosen to equal `pdfce-gui`'s `object_provider::FALLBACK_SELECT_TOLERANCE`
/// (3.0), which is the canvas-space catch radius a click falls back to. At
/// 100% zoom the canvas is distance-preserving against page space (the
/// `page_device_geometry` scale-1.0 map is a pure rotation + Y-flip +
/// translation), so 3.0 pt here reproduces the GUI's 100%-zoom behaviour.
/// The GUI's *live* tolerance additionally scales as `1 / zoom` to hold the
/// on-screen radius constant; the CLI has no zoom, so it takes the value
/// literally and the operator overrides it when reproducing a zoomed click.
const HIT_TOLERANCE_PT: f64 = 3.0;

/// A stable one-token name for a path's paint disposition (ISO 32000-1
/// §8.5.3): what actually marks the page, which is also what decides how
/// [`pdfce_core::vector::hit_test_point`] tests it — a filled path is hit
/// by its interior under its winding rule, a stroke-only path only by
/// proximity to its outline, and a `n` no-op/clip path only within the bare
/// tolerance. Printing it makes an otherwise-baffling hit-test result
/// ("I clicked inside it and missed") self-explaining.
fn paint_token(style: pdfce_core::vector::PaintStyle) -> &'static str {
    use pdfce_core::vector::FillRule;
    match (style.fill, style.stroke) {
        (Some(FillRule::NonZero), true) => "fill-nonzero+stroke",
        (Some(FillRule::NonZero), false) => "fill-nonzero",
        (Some(FillRule::EvenOdd), true) => "fill-evenodd+stroke",
        (Some(FillRule::EvenOdd), false) => "fill-evenodd",
        (None, true) => "stroke",
        // An `n` path: constructed, painted by nothing (a clip or a
        // discarded path). Still selectable, but only precisely.
        (None, false) => "none",
    }
}

/// How a text object's bbox was built, as a stable token — the CLI half of
/// ui-spec §E.3's requirement that a box's *provenance* be recoverable
/// wherever the box is shown.
///
/// `approximate=1` alone cannot answer the question a script (or an
/// operator diagnosing a missed click) actually has, because it is `1` for
/// every text object. These four tokens can:
///
/// | Token | Meaning |
/// |---|---|
/// | `font-metrics` | Advances from the font's own width table, height from its `/FontDescriptor`. The box is where a conforming reader lays the run out. |
/// | `metric-advances-nominal-height` | Advances real; no ascent/descent available, so the height is a nominal em (the Type 3 case). |
/// | `estimated-advances` | The font carried no width source at all, so the advances are estimated from metrically-similar Helvetica (§9.6.2.2 does not permit such a font; real files ship them). |
/// | `em-box` | No font resolved for at least one show operator: that part of the box is the legacy square around the run's START position, which reaches into blank paper before the text and stops short of its end. |
fn bounds_basis_token(basis: pdfce_core::vector::TextBoundsBasis) -> &'static str {
    use pdfce_core::vector::TextBoundsBasis;
    match basis {
        TextBoundsBasis::FontMetrics => "font-metrics",
        TextBoundsBasis::MetricAdvancesNominalHeight => "metric-advances-nominal-height",
        TextBoundsBasis::EstimatedAdvances => "estimated-advances",
        TextBoundsBasis::EmBox => "em-box",
    }
}

/// A page-space [`Bounds`](pdfce_core::vector::Bounds) as the stable
/// `minx,miny,maxx,maxy` token, or `none` for a box that enclosed no finite
/// point.
///
/// A **zero-width or zero-height box is NOT `none`** — a horizontal rule or
/// a vertical rule legitimately has one degenerate axis (`min.y == max.y`),
/// and `Bounds::is_empty` is `min > max`, not `min == max`. Reporting such a
/// box as `none` would have made exactly the thin geometry this tool exists
/// to find look unlocatable.
/// Coordinates are printed at **four decimal places with trailing zeros
/// trimmed**, so `50.0` prints as `50` (as it always has) and a
/// metrics-derived text edge at `70.46000272035599` prints as `70.46`
/// rather than as seventeen digits of `f32`-widening artefact. Four
/// decimals is 1/10 000 of a PDF point — four orders of magnitude finer
/// than the hit-test tolerance that consumes these numbers, so the
/// rounding cannot change any answer this tool gives.
fn bbox_token(b: pdfce_core::vector::Bounds) -> String {
    if b.is_empty() {
        "none".to_owned()
    } else {
        format!(
            "{},{},{},{}",
            coord_token(b.min.x),
            coord_token(b.min.y),
            coord_token(b.max.x),
            coord_token(b.max.y)
        )
    }
}

/// One page-space coordinate, at four decimal places with trailing zeros
/// (and a trailing `.`) trimmed — see [`bbox_token`].
fn coord_token(v: f64) -> String {
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    // `-0` is the one output the trim can produce that reads as a defect
    // rather than as a number; it is a real f64 value, and `0` is the same
    // point.
    if trimmed == "-0" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Quote a decoded string as a single `key="…"` token, so a value
/// containing spaces cannot be mistaken for the next field.
///
/// Every other field on an `object …` line is `key=value` with no quoting,
/// because every other value is a number or a fixed token. A text preview is
/// neither: it can contain spaces, quotes, backslashes, newlines and
/// arbitrary Unicode. The escaping is therefore stated exactly, so a script
/// can reverse it without guessing:
///
/// - `\` → `\\`, `"` → `\"` (the two characters that would otherwise break
///   the token's own delimiters);
/// - any character below U+0020, plus U+007F → `\xNN` with two lowercase hex
///   digits (a literal newline inside a line-oriented format is not
///   recoverable at all, and an invisible control character in a value a
///   human reads is worse than an escape they can see);
/// - everything else passes through as UTF-8, including non-ASCII text —
///   the CLI's output is UTF-8 and mangling `é` into an escape would make
///   the common non-English case unreadable for no safety gain.
fn quoted_token(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A text object's `text=` field: a quoted preview, or one of two bare
/// tokens that mean genuinely different things.
///
/// `none` and `undecodable` are unquoted, which is what makes them
/// unambiguous against a quoted string — a document really could contain the
/// literal text `undecodable`, and it would print as `text="undecodable"`.
///
/// The three answers, from
/// [`TextPreview`](pdfce_core::vector::TextPreview):
///
/// | Field | Meaning |
/// |---|---|
/// | `text="…"` | Decoded. `lossy=1` on the same line if some codes still failed and are shown as U+FFFD. |
/// | `text=undecodable` | Codes were shown and **not one** could be mapped — ISO 32000-1 §9.10.2's failure clause for every code (the `Identity-H`-without-`/ToUnicode` case). A document fact, not a pdfce limitation. |
/// | `text=none` | Nothing was shown, or no font resolver was in scope. |
fn text_preview_fields(preview: &pdfce_core::vector::TextPreview) -> String {
    use pdfce_core::vector::TextPreview;
    match preview {
        TextPreview::Decoded {
            text,
            truncated,
            lossy,
        } => format!(
            "text={} truncated={} lossy={}",
            quoted_token(text),
            u32::from(*truncated),
            u32::from(*lossy),
        ),
        TextPreview::Undecodable => "text=undecodable truncated=0 lossy=1".to_owned(),
        TextPreview::Unavailable | TextPreview::Empty => "text=none truncated=0 lossy=0".to_owned(),
    }
}

/// A text object's `font=`/`size=` fields.
///
/// `font=` is the **typeface** (`/BaseFont`, §9.6.2.1 Table 111) when the
/// font dictionary resolves, since that is what identifies the object to a
/// human; `resource=` always carries the `Tf` name (`F1`), which is the
/// handle a script or a later edit needs. Both, because they answer
/// different questions and neither substitutes for the other.
///
/// `size=` is the `Tf` size **operand as the file states it** — a text-space
/// quantity, not scaled by `Tm`/`cm`. See
/// [`TextFont::size`](pdfce_core::vector::TextFont::size) for why folding
/// the matrices in would be a confident number that disagrees with the
/// content stream.
fn font_fields(font: Option<&pdfce_core::vector::TextFont>) -> String {
    match font {
        None => "font=none resource=none size=none".to_owned(),
        Some(f) => format!(
            "font={} resource={} size={}",
            f.base_font
                .as_deref()
                .map_or_else(|| "none".to_owned(), quoted_token),
            quoted_token(&f.resource),
            f.size,
        ),
    }
}

/// One `object …` line's kind + kind-specific detail fields, for the object
/// at paint-order `index`.
///
/// Kinds are `path` / `text` / `image` / `form`. `image` and `form` are the
/// same [`VectorObject::Image`](pdfce_core::vector::VectorObject) arm
/// discriminated by its [`ImageSource`](pdfce_core::vector::ImageSource):
/// a Form XObject is reported separately because it is a *container* whose
/// contents were flattened into this same paint-order list, which materially
/// changes what deleting it does.
fn object_detail(obj: &pdfce_core::vector::VectorObject) -> (&'static str, String) {
    use pdfce_core::vector::{ImageSource, VectorObject};
    match obj {
        VectorObject::Path(p) => {
            // `anchors` is the count `node-move --node` indexes into: every
            // subpath's start plus each segment endpoint, in decomposition
            // order. Equal to `vector::anchor_count` by construction (that
            // function's own doc comment), derived here from the geometry so
            // no second content-stream walk is needed for a listing.
            let anchors: usize = p.subpaths.iter().map(|sp| sp.anchors().count()).sum();
            let closed = p.subpaths.iter().filter(|sp| sp.closed).count();
            (
                "path",
                format!(
                    "subpaths={} anchors={anchors} closed={closed} paint={} line_width={}",
                    p.subpaths.len(),
                    paint_token(p.style),
                    p.line_width,
                ),
            )
        }
        // `approximate=1` means the bbox is not measured glyph ink. It is
        // `1` for every text object and always will be until pdfce reads
        // glyph outlines, so on its own it does not distinguish the good
        // case from the bad one — which is what `bounds=` is for.
        //
        // The `text=`/`font=`/`resource=`/`size=` fields are the CLI half of
        // ui-spec §B.4 #1 (rule 11): the GUI's object row and this line
        // describe one object from one `decompose_page` walk, so a script
        // and an operator looking at the same file read the same facts.
        VectorObject::Text(t) => (
            "text",
            format!(
                // `runs=` is the headless oracle for per-run hit-testing.
                // `bounds=` reports the ENCLOSING rectangle, which for a
                // producer that puts many labels in one BT..ET can span the
                // whole page while the ink covers almost none of it — so the
                // bounds field alone cannot tell an operator whether
                // selection will behave. The run count can: `runs=0` means
                // selection falls back to that enclosing box, `runs=N` means
                // it tests N real extents.
                "approximate={} bounds={} runs={} {} {}",
                u32::from(t.approximate),
                bounds_basis_token(t.bounds_basis),
                t.runs.len(),
                font_fields(t.font.as_ref()),
                text_preview_fields(&t.preview),
            ),
        ),
        VectorObject::Image(i) => {
            let kind = match i.source {
                ImageSource::Form => "form",
                ImageSource::Inline | ImageSource::XObject => "image",
            };
            let source = match i.source {
                ImageSource::Inline => "inline",
                ImageSource::XObject => "xobject",
                ImageSource::Form => "form",
            };
            // `pixels=WxH` is the SAMPLE count from `/Width`/`/Height`
            // (§8.9.5 Table 89) — not a size on the page, which is what
            // `bbox=` on the same line already gives. The pair is what lets
            // a script compute effective placed resolution. `none` for a
            // form XObject (no samples) and for a malformed image.
            let pixels = i
                .pixel_size
                .map_or_else(|| "none".to_owned(), |(w, h)| format!("{w}x{h}"));
            (kind, format!("source={source} pixels={pixels}"))
        }
    }
}

/// Parse a `--hit X,Y` operand into a page-space point.
///
/// Deliberately strict — a silently-misparsed coordinate would report a
/// confident wrong answer about which object a click selects, which is worse
/// than a refusal (rule 4: fuzzy, never sneaky). `None` on anything but
/// exactly two finite comma-separated numbers.
fn parse_hit_point(s: &str) -> Option<pdfce_core::vector::Point> {
    let (x, y) = s.split_once(',')?;
    let x: f64 = x.trim().parse().ok()?;
    let y: f64 = y.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then(|| pdfce_core::vector::Point::new(x, y))
}

/// Grouped arguments for `object-list` — a struct to keep the handler under
/// clippy's `too_many_arguments` bar, like the editing subcommands.
struct ObjectListArgs<'a> {
    input: &'a Path,
    page_number: u32,
    hit: Option<&'a str>,
    all_hits: bool,
    enter: Option<usize>,
    tolerance: f64,
}

/// `object-list` — inventory one page's vector objects in paint order, and
/// optionally answer a headless hit-test query (read-only).
///
/// ## Contract
///
/// - Emits one `object page=… index=… kind=… bbox=… …` line per object, in
///   paint order (index 0 painted first, so the LAST line is topmost).
/// - Emits a `hit …` line iff `--hit` was supplied.
/// - Emits one `hit-candidate page=… ordinal=… index=… kind=…` line per
///   object under the point, front-most first (`ordinal=0` IS the `hit`
///   line's object), iff `--hit` **and** `--all-hits` were supplied. The
///   prefix is `hit-candidate`, deliberately not `hit`, so a script already
///   matching `^hit ` keeps matching exactly one line.
/// - Emits an `object-list …` summary line last.
/// - Exit `SUCCESS` (0) on a readable page — including when the page has no
///   objects, and including when `--hit` MISSES. A miss is a valid answer,
///   not a failure; scripts read the `index=` field (`none` on a miss)
///   rather than the exit code.
/// - Exit `RUNTIME_ERROR` (1) for an out-of-range/zero `--page`, a
///   malformed `--hit`, an unreadable page tree, or content that will not
///   tokenize. Exit `IO_ERROR`/`NOT_A_PDF` per [`exit_code_for_doc`] for a
///   file that will not load.
///
/// ## Why the hit-test is here and not reimplemented
///
/// It calls [`pdfce_core::vector::hit_test_point`] on the model
/// [`pdfce_core::vector::decompose_page`] returned — byte for byte the path
/// `pdfce-gui`'s `ObjectModelProvider::hit_test` takes after it converts the
/// pointer out of canvas space. That makes this subcommand a *diagnostic
/// oracle* for GUI selection: if `--hit` reports an index headlessly and a
/// click at the corresponding screen position does not select, the defect is
/// in the GUI's input/coordinate path, not in core's geometry.
///
/// `--all-hits` extends that oracle role to the one GUI behaviour a topmost
/// query cannot explain: click-through cycling. It calls
/// [`pdfce_core::vector::hit_test_point_all`], which is the same function
/// the GUI provider's `hit_test_all` calls and whose head is, by
/// construction, `hit_test_point`'s answer — so `ordinal=0` always names the
/// same object as the `hit` line, and the rest of the list is exactly what
/// repeated Alt+clicks walk through.
fn cmd_object_list(args: ObjectListArgs<'_>) -> u8 {
    use pdfce_core::vector::{
        Matrix, decompose_page, hit_test_point_all, hit_test_subpaths, subpath_bounds,
    };

    let ObjectListArgs {
        input,
        page_number,
        hit,
        all_hits,
        enter,
        tolerance,
    } = args;

    // Validate the query operands BEFORE loading the document: a typo
    // should fail immediately and identically whether or not the file
    // happens to be readable, and — critically — before any `object` rows
    // are printed, so a refusal never leaves half an answer on stdout.
    let hit_point = match hit {
        None => None,
        Some(raw) => match parse_hit_point(raw) {
            Some(p) => Some(p),
            None => {
                eprintln!(
                    "pdfce-cli: {}: malformed --hit `{raw}` (expected `X,Y` in PDF user space, \
e.g. `--hit 200,200`)",
                    input.display()
                );
                return exit::RUNTIME_ERROR;
            }
        },
    };
    // `--tolerance` is parsed by clap as a bare f64, so `nan` and negatives
    // both arrive intact. Either would make EVERY query a miss, which reads
    // as "hit-testing is broken" rather than "you passed nonsense" — refuse
    // by name instead (rule 4: fuzzy, never sneaky).
    if hit_point.is_some() && (!tolerance.is_finite() || tolerance < 0.0) {
        eprintln!(
            "pdfce-cli: {}: --tolerance must be a finite, non-negative number of points \
(got `{tolerance}`)",
            input.display()
        );
        return exit::RUNTIME_ERROR;
    }

    let doc = match open_for_read(input) {
        Ok(doc) => doc,
        Err(code) => return code,
    };
    let pages = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    // 1-based → 0-based, matching every other `--page` subcommand.
    // `checked_sub` absorbs `--page 0` without wrapping; `get` absorbs
    // past-the-end. Both land on one message, as `render-page` does.
    let Some(page) = page_number
        .checked_sub(1)
        .and_then(|i| pages.get(i as usize))
    else {
        eprintln!(
            "pdfce-cli: {}: page {page_number} is out of range (document has {} page(s), \
numbered 1..={})",
            input.display(),
            pages.len(),
            pages.len()
        );
        return exit::RUNTIME_ERROR;
    };

    // `Matrix::IDENTITY` is the initial CTM the GUI provider also passes, so
    // the coordinates printed here are the page space every other page-space
    // operand in this CLI uses.
    let model = match decompose_page(&doc.view(), page, Matrix::IDENTITY) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("pdfce-cli: {}: page {page_number}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    let (mut paths, mut text, mut images, mut forms) = (0usize, 0usize, 0usize, 0usize);
    for (index, obj) in model.objects.iter().enumerate() {
        let (kind, detail) = object_detail(obj);
        match kind {
            "path" => paths += 1,
            "text" => text += 1,
            "form" => forms += 1,
            _ => images += 1,
        }
        println!(
            "object page={page_number} index={index} kind={kind} bbox={} {detail}",
            bbox_token(obj.page_bbox()),
        );
    }

    if let Some(point) = hit_point {
        // The tolerance is passed through verbatim so the operator can
        // reproduce any zoom's catch radius; a non-finite or negative value
        // would make every query a miss, which reads as "hit-testing is
        // broken", so refuse it by name instead.
        if !tolerance.is_finite() || tolerance < 0.0 {
            eprintln!(
                "pdfce-cli: {}: --tolerance must be a finite, non-negative number of points",
                input.display()
            );
            return exit::RUNTIME_ERROR;
        }
        // ONE query answers both lines. `hit_test_point` is defined as this
        // list's head (see `pdfce_core::vector::hit`), so calling it as well
        // would be a second scan that could only ever agree — and a second
        // scan that CAN disagree is exactly the divergence decision 011 §Z2
        // names. `candidates=` on the `hit` line is therefore always
        // consistent with the `hit-candidate` lines below it.
        let candidates = hit_test_point_all(&model, point, tolerance);
        let kind_of = |i: usize| -> String {
            model
                .objects
                .get(i)
                .map_or("none", |obj| object_detail(obj).0)
                .to_owned()
        };
        let (index, kind) = match candidates.first() {
            Some(&i) => (i.to_string(), kind_of(i)),
            None => ("none".to_owned(), "none".to_owned()),
        };
        println!(
            "hit page={page_number} at={},{} tolerance={tolerance} index={index} kind={kind} \
candidates={}",
            point.x,
            point.y,
            candidates.len(),
        );
        if all_hits {
            // Front-most first: `ordinal=0` is the object the `hit` line
            // names and the object a plain click selects; each higher
            // ordinal is one more Alt+click down the stack, wrapping back to
            // 0 after the last.
            for (ordinal, &i) in candidates.iter().enumerate() {
                println!(
                    "hit-candidate page={page_number} at={},{} ordinal={ordinal} index={i} \
kind={} bbox={}",
                    point.x,
                    point.y,
                    kind_of(i),
                    model
                        .objects
                        .get(i)
                        .map_or_else(|| "none".to_owned(), |o| bbox_token(o.page_bbox())),
                );
            }
        }
    }

    // The level BELOW the object: which subpath of an entered object the same
    // point lands on. Printed after the `hit`/`hit-candidate` lines because it
    // refines them — a reader sees which object was named, then which of its
    // parts. Silent without `--hit`, and silent for a non-path or out-of-range
    // `--enter`, so a script may pass `--enter` unconditionally.
    if let (Some(point), Some(index)) = (hit_point, enter) {
        for (ordinal, sp) in hit_test_subpaths(&model, index, point, tolerance)
            .into_iter()
            .enumerate()
        {
            println!(
                "subpath-hit page={page_number} object={index} ordinal={ordinal} subpath={sp} \
bbox={}",
                subpath_bounds(&model, index, sp).map_or_else(
                    || "none".to_owned(),
                    |b| format!("{},{},{},{}", b.min.x, b.min.y, b.max.x, b.max.y)
                ),
            );
        }
    }

    let d = &model.diagnostics;
    println!(
        "object-list {} page={page_number} objects={} paths={paths} text={text} images={images} \
forms={forms} dropped_objects={} dropped_nodes={}",
        input.display(),
        model.objects.len(),
        d.objects_dropped,
        d.nodes_dropped,
    );
    exit::SUCCESS
}

/// Grouped arguments for `object-move` (Pass 9c-min) — a struct to keep the
/// handler under clippy's `too_many_arguments` bar, like the other editing
/// subcommands.
struct ObjectMoveArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    dx: f64,
    dy: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `object-move` — translate a vector object's construction operands by a
/// page-space `(dx, dy)` via content-stream surgery (Pass 9c-min, decision
/// 011 §2.5). Only the edited content stream changes (R46/§5.7).
fn cmd_object_move(args: &ObjectMoveArgs<'_>) -> u8 {
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.move_object(page_index, args.object, args.dx, args.dy) {
        Err(err) => return report_edit_error(args.input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "object-move {} page {} object={} dx={} dy={} mode={} -> {}; changed={} objects={} \
appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.dx,
        args.dy,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// `object-delete` — remove a vector object's construction + painting
/// operators from the content stream via surgery (Pass 9c-min). NOT
/// redaction (it removes a drawing object, not covered content for
/// security).
fn cmd_object_delete(
    input: &Path,
    page: u32,
    object: usize,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let page_index = (page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    if let Err(err) = session.delete_object(page_index, object) {
        return report_edit_error(input, &err);
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "object-delete {} page {page} object={object} mode={} -> {}; changed={} objects={} \
appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// Grouped arguments for `subpath-move` (Pass 28.0).
struct SubpathMoveArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    subpath: usize,
    dx: f64,
    dy: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `subpath-move` — translate ONE subpath of a path object.
fn cmd_subpath_move(args: &SubpathMoveArgs<'_>) -> u8 {
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.move_subpath(page_index, args.object, args.subpath, args.dx, args.dy) {
        Err(err) => return report_edit_error(args.input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "subpath-move {} page {} object={} subpath={} dx={} dy={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.subpath,
        args.dx,
        args.dy,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Grouped arguments for `subpath-delete` (Pass 25.2) — a struct to keep the
/// handler under clippy's `too_many_arguments` bar, like its siblings.
struct SubpathDeleteArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    subpath: usize,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `subpath-delete` — remove ONE subpath of a path object via content-stream
/// surgery, leaving the object's other subpaths byte-verbatim.
///
/// ## Contract
///
/// - Emits one `subpath-delete …` line naming the page, object, subpath and
///   the usual save-report fields, then defers the exit code to
///   [`finish_edit`] like every other editing subcommand.
/// - Every refusal — clipping path, structure mismatch, out-of-range index,
///   non-path object — happens before any mutation and is reported through
///   [`report_edit_error`], so the refusal vocabulary and exit codes are the
///   same ones the GUI surfaces. The operator gets the same answer whichever
///   shell they came through, which is the point of having one core.
fn cmd_subpath_delete(args: &SubpathDeleteArgs<'_>) -> u8 {
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.delete_subpath(page_index, args.object, args.subpath) {
        Err(err) => return report_edit_error(args.input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "subpath-delete {} page {} object={} subpath={} mode={} -> {}; changed={} objects={} \
appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.subpath,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Grouped arguments for `node-delete` (Pass 36.1).
struct NodeDeleteArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    node: usize,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `node-delete` — remove ONE anchor of a path object via content-stream
/// surgery, leaving every other object and every sibling subpath
/// byte-verbatim (Pass 36.1).
///
/// ## Contract
///
/// - Emits one `node-delete …` line naming the page, object, node and the
///   usual save-report fields, then defers the exit code to [`finish_edit`]
///   like every other editing subcommand.
/// - Disclosures — currently "a curve went with the point" — go to **stderr**
///   via [`report_disclosures`], so a script's stdout record stays
///   machine-parseable while the operator-facing consequence is still stated.
/// - Every refusal happens before any mutation and is reported through
///   [`report_edit_error`], so the refusal vocabulary and exit codes match the
///   GUI's exactly. Same core, same answer, whichever shell asked.
fn cmd_node_delete(args: &NodeDeleteArgs<'_>) -> u8 {
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.delete_node(page_index, args.object, args.node) {
        Err(err) => return report_edit_error(args.input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "node-delete {} page {} object={} node={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.node,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Grouped arguments for `node-move` (Pass 9c-min).
struct NodeMoveArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    node: usize,
    x: f64,
    y: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// Parse one `--move NODE,X,Y` triple.
///
/// # Why a custom parser rather than three repeated flags
///
/// The alternative — `--node 0 --x 200 --y 80 --node 1 --x 280 --y 80` —
/// relies on three independent repeated lists staying the same length and
/// in the same order. A dropped value silently pairs every anchor with the
/// wrong point from there on, and the command succeeds. Keeping the three
/// numbers in ONE token makes that unrepresentable.
///
/// # Errors
///
/// A message naming the offending token and what was expected. Negative
/// coordinates are legal (the flag carries `allow_hyphen_values`), so
/// `1,-5,-5` parses; a negative NODE does not, because an anchor index is
/// a position in a list.
fn parse_node_move(token: &str) -> Result<(usize, pdfce_core::vector::Point), String> {
    let parts: Vec<&str> = token.split(',').collect();
    let [n, x, y] = parts[..] else {
        return Err(format!(
            "--move {token:?}: expected NODE,X,Y (three comma-separated values), got {} \
             value(s)",
            parts.len()
        ));
    };
    let node: usize = n
        .trim()
        .parse()
        .map_err(|_| format!("--move {token:?}: {n:?} is not a 0-based anchor index"))?;
    let x: f64 = x
        .trim()
        .parse()
        .map_err(|_| format!("--move {token:?}: {x:?} is not a number"))?;
    let y: f64 = y
        .trim()
        .parse()
        .map_err(|_| format!("--move {token:?}: {y:?} is not a number"))?;
    Ok((node, pdfce_core::vector::Point::new(x, y)))
}

/// `nodes-move` — move several anchors of one path object as ONE surgery
/// (`Pass 23.3`).
///
/// ## Contract
///
/// - One `nodes-move …` line carrying `object=`, `nodes=` (how many were
///   moved) and the usual save-report fields, then the exit code from
///   [`finish_edit`].
/// - Disclosures to **stderr**, like `node-move`'s, so the stdout record
///   stays a fixed shape. De-duplicated by core: three rewritten rectangles
///   say so once.
/// - Every refusal — a malformed `--move` token, no anchors, a duplicated
///   anchor, an out-of-range index — happens **before** any mutation, and
///   the argument parsing is done up front for the same reason: a batch
///   whose fourth token is malformed must not apply its first three.
fn cmd_nodes_move(
    input: &Path,
    page: u32,
    object: usize,
    moves: &[String],
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    // ALL tokens parsed before the document is even opened. A partial parse
    // followed by a partial edit is the failure this ordering removes.
    let mut parsed = Vec::with_capacity(moves.len());
    for token in moves {
        match parse_node_move(token) {
            Ok(m) => parsed.push(m),
            Err(msg) => {
                eprintln!("pdfce-cli: {}: {msg}", input.display());
                return exit::RUNTIME_ERROR;
            }
        }
    }

    let page_index = (page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.move_nodes(page_index, object, &parsed) {
        Err(err) => return report_edit_error(input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "nodes-move {} page {} object={} nodes={} mode={} -> {}; changed={} objects={} \
appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        page.max(1),
        object,
        parsed.len(),
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `export-dxf` — a page's vector geometry as CAD-importable DXF.
///
/// ## Contract
///
/// - One `export-dxf …` line carrying `entities=`, the per-kind counts, and
///   what was skipped, then `0` on success.
/// - **What did NOT make it into the file goes to stderr in prose.** A
///   drawing that is half annotation exports as geometry alone, and an
///   operator who is not told opens it in SOLIDWORKS and concludes the
///   export lost things at random. That sentence is needed BEFORE the file
///   is opened, not after.
/// - **Read-only on the input.** An `EditSession` IS constructed — it is
///   the only route to the `/PieceInfo` dimension sidecar, which is where
///   the drawing's calibration lives — but nothing is mutated and no save
///   path is reachable from here. The session is a reader in this function
///   and the absence of any `save` call is what makes that true.
///
/// ## Scale: derived when not given, and REFUSED when ambiguous
///
/// `--scale` is optional. Omitted, the page's ce dimensions are consulted
/// (`suggest_scale`) and the three outcomes are handled differently on
/// purpose:
///
/// - **Calibrated** — the derived figure is printed BEFORE the file is
///   written, naming the group it came from. That is rule 4: an inference
///   is disclosed, not applied silently.
/// - **Uncalibrated** — falls back to paper scale with the loud warning
///   this command already carried. pdfce genuinely does not know, and
///   saying so is the honest answer.
/// - **Conflicting** — the export is REFUSED and every candidate listed.
///   A sheet with a 1:1 plan and a 1:5 detail is an ordinary drawing and
///   DXF carries one scale; choosing either would export half the sheet
///   wrong by a factor of five, and it would look entirely plausible.
///   `--scale` resolves it, which is what the refusal says.
///
/// The inference is scoped to **the pages being exported**, not to the
/// document. It shipped document-wide, which was wrong in both directions
/// on a multi-page sheet set: an unambiguous page-1 export could be
/// refused because page 3 held a 1:5 detail, and — the half that actually
/// damages metal — a page 1 with no calibration of its own would be
/// exported at page 3's scale with nothing on screen or in the output
/// looking odd. `dimension_groups_on_page` resolves each ce dimension's
/// owning page through its annotation's `/P`, and only those groups get a
/// vote.
///
/// ## Two modes, and why multi-page shares ONE scale
///
/// - `--page N -o file.dxf` — one page, one file.
/// - `--pages <spec> --output-dir <dir>` — one DXF per page, named
///   `<stem>_p<n>.dxf` zero-padded to the widest page number in the run.
///   Identical naming to the GUI's multi-page export, so a batch script
///   and an operator produce interchangeable output (project rule 11).
///
/// The scale is inferred from the union of every selected page's ce
/// dimension groups and applied to all of them, because `--scale` is one
/// value and a run that silently used a different scale per file would be
/// the plausible-wrong-answer failure this whole feature exists to close.
/// The consequence is deliberate: **pages at different scales are a
/// refusal**, reported exactly like two disagreeing groups on one page,
/// with the same two remedies (separate runs, or an explicit `--scale`).
/// Everything `export-dxf` was asked for.
///
/// A struct rather than nine positional parameters: the flag set grew past
/// the point where a call site reads as documentation, and
/// `clippy::too_many_arguments` is the lint that says so. Field names at
/// the call site also make the two mutually-exclusive destination flags
/// legible — `output: None, output_dir: Some(..)` states the mode, where a
/// pair of bare `Option`s in argument position would not.
struct ExportDxfArgs<'a> {
    input: &'a Path,
    /// 1-based, single-page mode. Ignored when `pages` is set (clap makes
    /// them mutually exclusive).
    page: u32,
    /// A `parse_pages` spec — multi-page mode.
    pages: Option<&'a str>,
    /// Single-page destination file.
    output: Option<&'a Path>,
    /// Multi-page destination directory.
    output_dir: Option<&'a Path>,
    units: DxfUnitArg,
    /// Explicit override; `None` means "derive it from the ce dimensions".
    scale: Option<f64>,
    fit_arcs: bool,
    /// Whether page text becomes `TEXT` entities.
    text: bool,
}

fn cmd_export_dxf(args: ExportDxfArgs<'_>) -> u8 {
    use pdfce_core::export::dxf::{
        DxfOptions, DxfOutcome, DxfScaleSuggestion, DxfText, DxfUnits, suggest_scale_for_groups,
        write_dxf,
    };

    let ExportDxfArgs {
        input,
        page,
        pages: pages_spec,
        output,
        output_dir,
        units,
        scale,
        fit_arcs,
        text,
    } = args;

    if let Some(s) = scale
        && (!s.is_finite() || s <= 0.0)
    {
        eprintln!(
            "pdfce-cli: {}: --scale must be a positive number; {s} would collapse or mirror the drawing",
            input.display()
        );
        return exit::RUNTIME_ERROR;
    }
    // Clap enforces that `--output` and `--output-dir` are mutually
    // exclusive and that `--output-dir` requires `--pages`. What it cannot
    // express is that ONE of them must be present, because which one is
    // legal depends on the other flag — so that is checked here, with a
    // message naming the flag the operator actually wants rather than a
    // generic "required argument missing".
    if output.is_none() && output_dir.is_none() {
        eprintln!(
            "pdfce-cli: {}: nowhere to write — pass --output <file.dxf> for a single page, or --output-dir <dir> with --pages",
            input.display()
        );
        return exit::RUNTIME_ERROR;
    }
    if pages_spec.is_some() && output_dir.is_none() {
        eprintln!(
            "pdfce-cli: {}: --pages writes one DXF per page and needs --output-dir <dir>; --output names a single file",
            input.display()
        );
        return exit::RUNTIME_ERROR;
    }

    let doc = match open_document(input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit_code_for_doc(&err);
        }
    };
    // Read-only: see the contract above. This exists solely to reach the
    // `/PieceInfo` sidecar the drawing's calibration lives in.
    let session = pdfce_core::edit::EditSession::new(doc);
    let doc = &session;
    let page_list = match doc.pages() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };

    // ---- which pages ----
    //
    // `parse_pages` is the established spec parser and REFUSES an
    // out-of-range page rather than dropping it (see its own docs on why
    // silently handing back 30 pages of a requested 50 is how a mistake
    // ships to a thousand documents). The single-page path keeps its own
    // bounds message, which names the page the operator typed.
    let indices: Vec<usize> = match pages_spec {
        Some(spec) => match parse_pages(spec, page_list.len()) {
            Ok(v) => v,
            Err(message) => {
                eprintln!("pdfce-cli: {}: {message}", input.display());
                return exit::RUNTIME_ERROR;
            }
        },
        None => {
            let index = (page.max(1) - 1) as usize;
            if page_list.get(index).is_none() {
                eprintln!(
                    "pdfce-cli: {}: no page {page} — the document has {} page(s)",
                    input.display(),
                    page_list.len()
                );
                return exit::RUNTIME_ERROR;
            }
            vec![index]
        }
    };

    // ---- decompose every page BEFORE writing anything ----
    //
    // All-or-nothing on purpose. A run that wrote four files and then died
    // on page five would leave a directory an operator has to reconcile by
    // hand against a page list, and the usual cause of a decompose failure
    // (a malformed content stream) is a property of the document rather
    // than of the moment, so retrying gains nothing.
    let mut models = Vec::with_capacity(indices.len());
    for index in &indices {
        let Some(target) = page_list.get(*index) else {
            eprintln!(
                "pdfce-cli: {}: no page {} — the document has {} page(s)",
                input.display(),
                index + 1,
                page_list.len()
            );
            return exit::RUNTIME_ERROR;
        };
        match pdfce_core::vector::decompose_page(
            &doc.view(),
            target,
            pdfce_core::vector::Matrix::IDENTITY,
        ) {
            Ok(m) => models.push(m),
            Err(err) => {
                eprintln!(
                    "pdfce-cli: {}: page {}: {err} — nothing was written",
                    input.display(),
                    index + 1
                );
                return exit::RUNTIME_ERROR;
            }
        }
    }

    // ---- resolve the drawing scale (see this function's contract) ----
    //
    // Done AFTER decomposition so a page that cannot be read fails on that
    // rather than on a scale question the operator would then have answered
    // for nothing.
    //
    // Scoped to the SELECTED pages' groups, deduplicated: a group present
    // on two pages must not vote twice, or it would inflate `agreeing` and
    // read as corroboration of itself.
    let mut groups: Vec<pdfce_core::dimension::GroupId> = Vec::new();
    for index in &indices {
        for id in doc.dimension_groups_on_page(*index) {
            if !groups.contains(&id) {
                groups.push(id);
            }
        }
    }
    let multi = indices.len() > 1;
    let scope = if multi { "these pages'" } else { "this page's" };
    let suggestion = suggest_scale_for_groups(&doc.dimension_model(), &groups);
    // Whether pdfce chose this number or the operator did — read before
    // `scale` is shadowed. See the paper-scale disclosure at the end.
    let scale_was_derived = scale.is_none();
    let scale = match scale {
        Some(explicit) => explicit,
        None => match &suggestion {
            DxfScaleSuggestion::Calibrated {
                scale,
                group,
                agreeing,
                ..
            } => {
                // Rule 4: the inference is stated BEFORE the file is
                // written, naming its source, so the operator can see what
                // pdfce concluded and re-run with --scale if it is wrong.
                eprintln!(
                    "pdfce-cli: {}: using scale {scale} derived from the ce dimension group {group:?}{} — the drawing is calibrated, so this export is at REAL size, not paper size. Pass --scale to override.",
                    input.display(),
                    if *agreeing > 1 {
                        format!(" (and {} other calibrated group(s) agree)", agreeing - 1)
                    } else {
                        String::new()
                    }
                );
                *scale
            }
            DxfScaleSuggestion::Uncalibrated => 1.0,
            DxfScaleSuggestion::Conflicting { candidates } => {
                eprintln!(
                    "pdfce-cli: {}: REFUSED — {scope} ce dimension groups disagree about the scale, and a DXF carries only one. Nothing was written.",
                    input.display()
                );
                for c in candidates {
                    eprintln!("    {:?} says scale {}", c.group, c.scale);
                }
                eprintln!(
                    "  A 1:1 plan and a 1:5 detail on one sheet is an ordinary drawing, so pdfce will not pick for you: choosing wrong exports part of the sheet at the wrong size and the result looks entirely plausible. Pass --scale <n> to say which, or export the views separately."
                );
                return exit::RUNTIME_ERROR;
            }
        },
    };

    let opts = DxfOptions {
        units: match units {
            DxfUnitArg::In => DxfUnits::Inches,
            DxfUnitArg::Mm => DxfUnits::Millimetres,
        },
        scale,
        fit_arcs,
        text: if text {
            DxfText::Entities
        } else {
            DxfText::Omit
        },
        ..DxfOptions::default()
    };

    // Zero-padded to the LARGEST page number in the run, not to the
    // document's page count: exporting pages 8-10 of a 400-page file
    // should not produce `_p008`.
    let width = indices
        .iter()
        .map(|i| (i + 1).to_string().len())
        .max()
        .unwrap_or(1);
    let stem = input
        .file_stem()
        .map_or_else(|| "export".to_owned(), |s| s.to_string_lossy().into_owned());

    let mut total = DxfOutcome::default();
    for (index, model) in indices.iter().zip(&models) {
        let (dxf, out) = write_dxf(model, &opts);
        let path = match output_dir {
            Some(dir) => dir.join(format!("{stem}_p{:0width$}.dxf", index + 1)),
            // Unreachable: the guard above rejects both-absent, and clap
            // rejects both-present. Handled rather than unwrapped so a
            // future flag change cannot turn this into a panic.
            None => match output {
                Some(path) => path.to_path_buf(),
                None => return exit::RUNTIME_ERROR,
            },
        };
        if let Err(err) = std::fs::write(&path, dxf.as_bytes()) {
            eprintln!("pdfce-cli: {}: {err}", path.display());
            return exit::IO_ERROR;
        }
        let entities = out.polylines + out.circles + out.arcs + out.splines + out.text_entities;
        println!(
            "export-dxf {} page {} -> {}; entities={entities} polylines={} circles={} arcs={} splines={} text={} unreadable_text={} skipped_text={} skipped_images={} units={} scale={} fit_arcs={}",
            input.display(),
            index + 1,
            path.display(),
            out.polylines,
            out.circles,
            out.arcs,
            out.splines,
            out.text_entities,
            out.unreadable_text,
            out.skipped_text,
            out.skipped_images,
            match units {
                DxfUnitArg::In => "in",
                DxfUnitArg::Mm => "mm",
            },
            scale,
            u32::from(fit_arcs),
        );
        total.polylines += out.polylines;
        total.circles += out.circles;
        total.arcs += out.arcs;
        total.splines += out.splines;
        total.skipped_text += out.skipped_text;
        total.skipped_images += out.skipped_images;
        total.text_entities += out.text_entities;
        total.unreadable_text += out.unreadable_text;
    }

    // ---- the disclosures, in prose, on stderr ----
    //
    // SUMMED across the run and emitted ONCE. Per-file would be the
    // obvious choice and is the wrong one: a forty-page batch would print
    // forty near-identical paragraphs, and a disclosure repeated forty
    // times is one an operator scrolls past — which is the same
    // learned-past failure the paper-scale gating below was written to
    // avoid, arriving through volume instead of through wording. The
    // per-page machine-readable line above already carries each page's
    // own counts for anything that needs them.
    if total.skipped_text > 0 {
        eprintln!(
            "pdfce-cli: {}: {} text object(s) were NOT exported — this DXF carries geometry only, so any dimensions, labels and notes on the drawing are absent from it. Their outlines are not there either; the text was never converted to curves.",
            input.display(),
            total.skipped_text
        );
    }
    if total.unreadable_text > 0 {
        eprintln!(
            "pdfce-cli: {}: {} text run(s) could NOT be read and are absent from the DXF — pdfce could not map their character codes to characters (a font with no /ToUnicode, typically). This is different from --no-text: these are labels you can see on the page that pdfce cannot transcribe, so the DXF is missing text you will expect to find in it.",
            input.display(),
            total.unreadable_text
        );
    }
    if total.skipped_images > 0 {
        eprintln!(
            "pdfce-cli: {}: {} image(s) were NOT exported — DXF has no raster entity in the subset pdfce writes, so a scanned or rendered region of the page is simply missing rather than blank.",
            input.display(),
            total.skipped_images
        );
    }
    // Gated on THREE things, and each rules out a different way of telling
    // the operator something they already know:
    //
    //   * the scale is 1 — there is nothing to warn about otherwise;
    //   * the pages are UNCALIBRATED — a group calibrated to an explicit
    //     1:1 is a real answer the operator gave, and warning them that
    //     pdfce might not know the scale when it demonstrably does is the
    //     shape of disclosure that gets learned past and then ignored when
    //     it matters;
    //   * ★ the 1 was DERIVED, not typed. This third clause was missing
    //     and it produced a genuinely absurd message: `--scale 1` on an
    //     uncalibrated drawing printed "pdfce does not know what scale the
    //     drawing is at … pass --scale 2 for 1:2, and so on" — instructing
    //     the operator to do the thing they had just done. It is the same
    //     objection as the second clause arriving from the other side: an
    //     explicit `--scale 1` is the operator answering, exactly as an
    //     explicit 1:1 calibration is. Found by running the command rather
    //     than by reading it.
    if scale_was_derived
        && (scale - 1.0).abs() < f64::EPSILON
        && matches!(suggestion, DxfScaleSuggestion::Uncalibrated)
    {
        eprintln!(
            "pdfce-cli: {}: exported at PAPER scale. Nothing on {} is calibrated, so pdfce does not know what scale the drawing is at — if it is a scaled view, a 1:2 detail say, the geometry is that fraction of real size and will look entirely plausible. Either measure a known feature in the GUI (the scale then comes across automatically) or pass --scale 2 for 1:2, and so on.",
            input.display(),
            if multi { "these pages" } else { "this page" }
        );
    }

    exit::SUCCESS
}

/// `text-run-delete` — remove one show operator from a text object
/// (`Pass 32.0`).
///
/// ## Contract
///
/// - One `text-run-delete …` line with the usual save-report fields, then
///   the exit code from [`finish_edit`].
/// - Refusals — an out-of-range run, and the §9.4.2 guard when the next run
///   inherits its position — go through [`report_edit_error`] before any
///   mutation. The guard's message names its own remedy.
fn cmd_text_run_delete(
    input: &Path,
    page: u32,
    object: usize,
    run: usize,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let page_index = (page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.delete_text_run(page_index, object, run) {
        Err(err) => return report_edit_error(input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "text-run-delete {} page {} object={} run={} mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        input.display(),
        page.max(1),
        object,
        run,
        mode.name(),
        output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(input, &outcome)
}

/// `node-move` — move one anchor to a page-space point via surgery
/// (Pass 9c-min, decision 011 §2.5).
///
/// An `re` rectangle corner and the implicit reused start of an `h`-reopened
/// subpath have no operand of their own; both are handled by materializing one
/// (Pass 30.0) and both DISCLOSE that they did, on stderr so a script's stdout
/// record stays machine-parseable.
fn cmd_node_move(args: &NodeMoveArgs<'_>) -> u8 {
    use pdfce_core::vector::Point;
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.move_node(
        page_index,
        args.object,
        args.node,
        Point::new(args.x, args.y),
    ) {
        Err(err) => return report_edit_error(args.input, &err),
        // stderr, not stdout: the stdout line is a fixed-shape record other
        // tools parse, and a variable-length prose block in the middle of it
        // would break them.
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "node-move {} page {} object={} node={} to=({},{}) mode={} -> {}; changed={} objects={} \
appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.node,
        args.x,
        args.y,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// Grouped arguments for `handle-move` (Pass 30.1).
struct HandleMoveArgs<'a> {
    input: &'a Path,
    page: u32,
    object: usize,
    node: usize,
    side: HandleArg,
    x: f64,
    y: f64,
    output: &'a Path,
    mode: SaveMode,
    verify_undo: bool,
}

/// `handle-move` — move one Bézier control point, leaving its on-curve node
/// where it is (Pass 30.1).
///
/// The operation that changes a curve's SHAPE; `node-move` can only move the
/// points a curve passes through. A `v`/`y` segment whose requested handle is
/// implied by another point is re-spelled as `c`, disclosed on stderr so the
/// stdout record stays machine-parseable.
fn cmd_handle_move(args: &HandleMoveArgs<'_>) -> u8 {
    use pdfce_core::vector::Point;
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    match session.move_handle(
        page_index,
        args.object,
        args.node,
        args.side.to_core(),
        Point::new(args.x, args.y),
    ) {
        Err(err) => return report_edit_error(args.input, &err),
        Ok(disclosures) => report_disclosures(&disclosures),
    }
    let outcome = match save_edited(
        &mut session,
        &source,
        args.output,
        args.mode,
        ProducerArg::Preserve,
        args.verify_undo,
    ) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    let r = &outcome.report;
    println!(
        "handle-move {} page {} object={} node={} side={} to=({},{}) mode={} -> {}; changed={} objects={} appended={} out_bytes={} undo_verified={} undo_identical={}",
        args.input.display(),
        args.page,
        args.object,
        args.node,
        args.side.token(),
        args.x,
        args.y,
        args.mode.name(),
        args.output.display(),
        outcome.changed,
        r.objects_written,
        r.bytes_appended,
        r.bytes_written,
        u32::from(outcome.undo_verified),
        u32::from(outcome.undo_identical),
    );
    finish_edit(args.input, &outcome)
}

/// The file-name stem a `{stem}` placeholder and a per-source bookmark
/// both want.
fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned())
}

/// Implement `pdfce-cli extract-pages`.
fn cmd_extract_pages(input: &Path, pages: &str, output: &Path) -> u8 {
    let doc = match open_for_read(input) {
        Ok(doc) => doc,
        Err(code) => return code,
    };
    let count = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(pages, count) {
        Ok(selected) => selected,
        Err(message) => {
            eprintln!("pdfce-cli: {}: {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };

    let view = DocumentView::new(&doc, doc.bytes(), doc.version());
    // The operator's persisted §14.11.4 policy. Loaded here rather than
    // taken as a default, because a setting no front end passes is a
    // setting that does nothing.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let (bytes, report) =
        match pdfce_core::pageops::extract_with(&view, &selected, settings.separations) {
            Ok(pair) => pair,
            Err(err) => return report_page_op_error(&err),
        };
    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }
    println!(
        "extract-pages {} -> {}; {}",
        input.display(),
        output.display(),
        assemble_metrics(&report, bytes.len())
    );
    report_assemble(output, &report);
    exit::SUCCESS
}

/// Implement `pdfce-cli merge`.
fn cmd_merge(inputs: &[PathBuf], output: &Path, bookmarks: bool) -> u8 {
    if inputs.len() < 2 {
        eprintln!(
            "pdfce-cli: merge needs at least two input PDFs; got {}.",
            inputs.len()
        );
        return exit::EDIT_REFUSED;
    }
    let mut docs = Vec::with_capacity(inputs.len());
    for path in inputs {
        match open_for_read(path) {
            Ok(doc) => docs.push(doc),
            Err(code) => return code,
        }
    }
    let views: Vec<DocumentView<'_>> = docs
        .iter()
        .map(|doc| DocumentView::new(doc, doc.bytes(), doc.version()))
        .collect();
    // Titles are PDF text strings (§7.9.2), encoded by the engine's own
    // encoder rather than assembled here — pdfce-core owns the format,
    // the CLI owns only the choice of what to call each source.
    let titles: Vec<Vec<u8>> = if bookmarks {
        inputs
            .iter()
            .map(|path| pdfce_core::edit::encode_text_string(&stem_of(path)))
            .collect()
    } else {
        Vec::new()
    };

    let (bytes, report) = match pdfce_core::pageops::merge(&views, &titles) {
        Ok(pair) => pair,
        Err(err) => return report_page_op_error(&err),
    };
    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }
    println!(
        "merge {} files -> {}; {}",
        inputs.len(),
        output.display(),
        assemble_metrics(&report, bytes.len())
    );
    report_assemble(output, &report);
    exit::SUCCESS
}

/// Implement `pdfce-cli insert-pages`.
fn cmd_insert_pages(
    input: &Path,
    source: &Path,
    source_pages: &str,
    before: Option<usize>,
    after: Option<usize>,
    output: &Path,
) -> u8 {
    let (target_doc, source_doc) = match (open_for_read(input), open_for_read(source)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(code), _) | (_, Err(code)) => return code,
    };
    let source_count = match pdfce_core::page_tree::pages(&source_doc) {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", source.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(source_pages, source_count) {
        Ok(selected) => selected,
        Err(message) => {
            eprintln!("pdfce-cli: {}: {message}", source.display());
            return exit::EDIT_REFUSED;
        }
    };
    // 1-based on the command line, 0-based in the engine; the conversion
    // happens exactly here. `--before 0` means "at the very start", which
    // is why it saturates rather than erroring.
    let position = match (before, after) {
        (Some(page), _) => InsertPosition::Before(page.saturating_sub(1)),
        (None, Some(page)) => InsertPosition::After(page.saturating_sub(1)),
        (None, None) => InsertPosition::End,
    };

    let target_view = DocumentView::new(&target_doc, target_doc.bytes(), target_doc.version());
    let source_view = DocumentView::new(&source_doc, source_doc.bytes(), source_doc.version());
    let (bytes, report) =
        match pdfce_core::pageops::insert(&target_view, &source_view, &selected, position) {
            Ok(pair) => pair,
            Err(err) => return report_page_op_error(&err),
        };
    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }
    println!(
        "insert-pages {} + {} -> {}; {}",
        input.display(),
        source.display(),
        output.display(),
        assemble_metrics(&report, bytes.len())
    );
    report_assemble(output, &report);
    exit::SUCCESS
}

/// Implement `pdfce-cli split`.
#[allow(clippy::too_many_arguments)] // one parameter per documented flag
fn cmd_split(
    input: &Path,
    out_dir: &Path,
    every: usize,
    after: Option<&str>,
    bookmarks: bool,
    name_template: &str,
    force: bool,
) -> u8 {
    let doc = match open_for_read(input) {
        Ok(doc) => doc,
        Err(code) => return code,
    };
    let count = match pdfce_core::page_tree::pages(&doc) {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let criterion = if bookmarks {
        SplitCriterion::TopLevelBookmarks
    } else if let Some(spec) = after {
        match parse_pages(spec, count) {
            Ok(points) => SplitCriterion::AfterPages(points),
            Err(message) => {
                eprintln!("pdfce-cli: {}: {message}", input.display());
                return exit::EDIT_REFUSED;
            }
        }
    } else {
        SplitCriterion::EveryN(every)
    };

    let view = DocumentView::new(&doc, doc.bytes(), doc.version());
    let stem = stem_of(input);
    // The operator's persisted §14.11.4 policy. Loaded here rather than
    // taken as a default, because a setting no front end passes is a
    // setting that does nothing.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let parts = match pdfce_core::pageops::split_with(
        &view,
        &criterion,
        name_template,
        &stem,
        settings.separations,
    ) {
        Ok(parts) => parts,
        Err(err) => return report_page_op_error(&err),
    };

    if let Err(err) = std::fs::create_dir_all(out_dir) {
        eprintln!("pdfce-cli: {}: {err}", out_dir.display());
        return exit::IO_ERROR;
    }
    // Collision check BEFORE anything is written: a split that overwrites
    // half a folder and then fails is worse than one that refuses.
    if !force {
        let existing: Vec<String> = parts
            .iter()
            .map(|(part, _, _)| part.name.clone())
            .filter(|name| out_dir.join(name).exists())
            .collect();
        if !existing.is_empty() {
            eprintln!(
                "pdfce-cli: {}: {} output name(s) already exist there ({}). Nothing was written; \
pass --force to overwrite.",
                out_dir.display(),
                existing.len(),
                existing.join(", ")
            );
            return exit::EDIT_REFUSED;
        }
    }

    let mut pages_written = 0usize;
    let mut bytes_written = 0usize;
    for (part, bytes, report) in &parts {
        let path = out_dir.join(&part.name);
        if let Err(err) = std::fs::write(&path, bytes) {
            eprintln!("pdfce-cli: {}: {err}", path.display());
            return exit::IO_ERROR;
        }
        pages_written += report.pages;
        bytes_written += bytes.len();
    }
    println!(
        "split {} -> {}; parts={} pages={} out_bytes={bytes_written}",
        input.display(),
        out_dir.display(),
        parts.len(),
        pages_written
    );
    if let Some((_, _, first)) = parts.first() {
        // The carryover disclosures are the same for every part, so they
        // are printed once rather than N times.
        report_assemble(out_dir, first);
    }
    exit::SUCCESS
}

/// Implement `pdfce-cli delete-pages`.
fn cmd_delete_pages(
    input: &Path,
    pages: &str,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let count = match session.pages() {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(pages, count) {
        Ok(selected) => selected,
        Err(message) => {
            eprintln!("pdfce-cli: {}: {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };
    // The operator's persisted §14.11.4 policy. Loaded here rather than
    // taken as a default, because a setting no front end passes is a
    // setting that does nothing.
    let (settings, settings_report) =
        pdfce_core::settings::Settings::load(pdfce_core::settings::resolve_store());
    report_settings(&settings_report);
    let outcome = match session.delete_pages_with(&selected, settings.separations) {
        Ok(outcome) => outcome,
        Err(err) => return report_edit_error(input, &err),
    };
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(saved) => saved,
        Err(code) => return code,
    };

    println!(
        "delete-pages {} mode={} signature={} -> {}; pages_removed={} objects_freed={} \
dangling_bookmarks={} dangling_links={} dangling_dests={} page_labels_stale={} {} {}",
        input.display(),
        mode.name(),
        signature_token(outcome.signature),
        output.display(),
        outcome.pages_removed,
        outcome.objects_freed,
        outcome.dangling.outline_items,
        outcome.dangling.links,
        outcome.dangling.named_destinations,
        u32::from(outcome.dangling.page_labels_stale),
        separation_metrics(&outcome.separations),
        edit_metrics(&saved)
    );

    // The two honesty disclosures the UI spec makes mandatory, in their
    // command-line form.
    if !outcome.dangling.is_empty() {
        eprintln!(
            "pdfce-cli: {}: {} bookmark(s), {} link(s) and {} named destination(s) pointed at a \
removed page and now point nowhere. pdfce reports them and does not repair them — repointing one \
at whatever page now occupies that index would be pdfce deciding what the author meant.",
            input.display(),
            outcome.dangling.outline_items,
            outcome.dangling.links,
            outcome.dangling.named_destinations
        );
    }
    if outcome.dangling.page_labels_stale {
        eprintln!(
            "pdfce-cli: {}: this document has a page-label tree (/PageLabels). Deleting pages \
does not adjust it, so its numbering is now stale.",
            input.display()
        );
    }
    // The one class above that pdfce repairs rather than reports — see
    // `DeleteOutcome::separations` for why a structural invariant is
    // repairable where an authorial destination is not.
    report_separations(input, &outcome.separations);
    eprintln!(
        "pdfce-cli: {}: deletion removes pages from the DOCUMENT, not from the file's bytes. \
The previous revision can still contain them. This is not redaction.",
        output.display()
    );
    report_signature(input, outcome.signature);
    finish_edit(input, &saved)
}

/// Implement `pdfce-cli reorder-pages`.
fn cmd_reorder_pages(
    input: &Path,
    order: &str,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let count = match session.pages() {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let new_order = match parse_pages(order, count) {
        Ok(order) => order,
        Err(message) => {
            eprintln!("pdfce-cli: {}: {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };
    if let Err(err) = session.reorder_pages(&new_order) {
        return report_edit_error(input, &err);
    }
    let impact = session.signature_impact_of_save(match mode {
        SaveMode::Incremental => CoreSaveMode::Incremental,
        SaveMode::Full => CoreSaveMode::FullRewrite,
    });
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(saved) => saved,
        Err(code) => return code,
    };
    println!(
        "reorder-pages {} mode={} signature={} -> {}; pages={} {}",
        input.display(),
        mode.name(),
        signature_token(impact),
        output.display(),
        count,
        edit_metrics(&saved)
    );
    report_signature(input, impact);
    finish_edit(input, &saved)
}

/// Implement `pdfce-cli rotate`.
fn cmd_rotate(
    input: &Path,
    degrees: i32,
    pages: &str,
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
) -> u8 {
    let (source, mut session) = match open_for_edit(input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let count = match session.pages() {
        Ok(pages) => pages.len(),
        Err(err) => {
            eprintln!("pdfce-cli: {}: {err}", input.display());
            return exit::RUNTIME_ERROR;
        }
    };
    let selected = match parse_pages(pages, count) {
        Ok(selected) => selected,
        Err(message) => {
            eprintln!("pdfce-cli: {}: {message}", input.display());
            return exit::EDIT_REFUSED;
        }
    };
    let rotated = match session.rotate_pages(&selected, degrees) {
        Ok(rotated) => rotated,
        Err(err) => return report_edit_error(input, &err),
    };
    let impact = session.signature_impact_of_save(match mode {
        SaveMode::Incremental => CoreSaveMode::Incremental,
        SaveMode::Full => CoreSaveMode::FullRewrite,
    });
    let saved = match save_edited(
        &mut session,
        &source,
        output,
        mode,
        ProducerArg::Preserve,
        verify_undo,
    ) {
        Ok(saved) => saved,
        Err(code) => return code,
    };
    println!(
        "rotate {} mode={} signature={} -> {}; rotate={} rotated={} {}",
        input.display(),
        mode.name(),
        signature_token(impact),
        output.display(),
        degrees,
        rotated,
        edit_metrics(&saved)
    );
    report_signature(input, impact);
    finish_edit(input, &saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `f` on a worker thread with a large (16 MiB) stack.
    ///
    /// clap builds and validates the command tree by **recursion**
    /// proportional to the subcommand/argument count, and this CLI's surface
    /// (dozens of subcommands, and growing — the Pass 9c-min `object-move`/
    /// `object-delete`/`node-move` additions among them) now needs more stack
    /// than a default Windows test thread's ~2 MiB to run `Command::debug_assert`
    /// and to walk `Cli::command()`. This is a scaling property of clap's
    /// recursion, not a bug in the CLI: **production is unaffected** — the real
    /// binary parses on the process main thread (8 MiB) and never calls
    /// `debug_assert`. The worker keeps the full validation without shrinking
    /// the CLI surface; a panic (a failed assertion) propagates through `join`.
    fn on_large_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(f)
            .expect("spawn the large-stack validation thread")
            .join()
            .expect("the large-stack validation thread panicked");
    }

    /// clap's own invariant check — catches malformed `#[command]`/`#[arg]`
    /// wiring (duplicate flags, bad defaults) at test time rather than on
    /// first run. Recommended by clap's docs for any derive-based CLI. Run on
    /// [`on_large_stack`] because `debug_assert` recurses over the whole tree.
    #[test]
    fn cli_definition_is_valid() {
        on_large_stack(|| {
            use clap::CommandFactory as _;
            Cli::command().debug_assert();
        });
    }

    #[test]
    fn io_error_maps_to_io_exit_code() {
        let err = PdfError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert_eq!(exit_code_for(&err), exit::IO_ERROR);
    }

    // decision 012: the --font-dir walk (shell-side, R61).

    #[test]
    fn font_dir_empty_gives_bundled_only_and_no_notes() {
        // No --font-dir → the deterministic default path is untouched:
        // zero registrations, zero notes (R63).
        let (_env, registered, notes) = build_font_environment(&[]);
        assert_eq!(registered, 0);
        assert!(notes.is_empty());
    }

    #[test]
    fn font_dir_registers_a_readable_face_under_its_filename_stem() {
        // A bundled Foxit CFF copied into a temp dir as `Calibri.cff`
        // must register under the stem `Calibri` (and its advertised
        // name(s)) so a document's non-embedded `Calibri` matches. This
        // exercises the real read → parse → face_names → insert_named
        // path without shipping any third-party font.
        let dir = std::env::temp_dir().join(format!("pdfce-fontdir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let face = pdfce_render::font::bundled::faces();
        let bytes = face
            .get(&pdfce_render::FallbackKey::Serif)
            .unwrap()
            .bytes()
            .to_vec();
        let path = dir.join("Calibri.cff");
        std::fs::write(&path, &bytes).unwrap();

        let (env, registered, notes) = build_font_environment(std::slice::from_ref(&dir));
        assert!(registered >= 1, "at least the stem registers: {notes:?}");
        assert!(
            env.named("Calibri").is_some(),
            "must match the filename stem: {notes:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn font_dir_skips_corrupt_file_without_error_and_notes_it() {
        // Acceptance: a corrupt/misnamed supplied file fails CLEAN — it
        // is skipped and noted, never fatal, and the bundled default is
        // still available for the render to fall back to.
        let dir = std::env::temp_dir().join(format!("pdfce-fontbad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Calibri.ttf"), b"this is not a font at all").unwrap();

        let (env, registered, notes) = build_font_environment(std::slice::from_ref(&dir));
        assert_eq!(registered, 0, "a corrupt file registers nothing");
        assert!(
            notes.iter().any(|n| n.contains("not a usable font")),
            "the skip must be disclosed: {notes:?}"
        );
        // The environment is still a usable bundled one.
        assert!(env.fallback(pdfce_render::FallbackKey::Sans).is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn font_dir_missing_directory_is_a_note_not_a_panic() {
        let missing = std::env::temp_dir().join("pdfce-does-not-exist-xyz-012");
        let (_env, registered, notes) = build_font_environment(&[missing]);
        assert_eq!(registered, 0);
        assert_eq!(notes.len(), 1, "one note for the unreadable dir");
    }

    #[test]
    fn header_errors_map_to_not_a_pdf() {
        assert_eq!(
            exit_code_for(&PdfError::MissingHeader { searched: 8 }),
            exit::NOT_A_PDF
        );
        assert_eq!(
            exit_code_for(&PdfError::MalformedVersion {
                found: "x.y".to_owned()
            }),
            exit::NOT_A_PDF
        );
    }

    #[test]
    fn exit_codes_are_distinct_and_avoid_claps_reserved_two() {
        // The exit-code table IS the scripting contract (module docs):
        // two failure modes sharing a code silently merges them for
        // every caller that branches on it, and `2` belongs to clap.
        let codes = [
            exit::SUCCESS,
            exit::RUNTIME_ERROR,
            exit::IO_ERROR,
            exit::NOT_A_PDF,
            exit::NOT_BYTE_IDENTICAL,
            exit::RELOAD_FAILED,
            exit::RASTER_DIFFERS,
            exit::SAVE_REFUSED,
            exit::EDIT_REFUSED,
            exit::UNIMPLEMENTED,
        ];
        let mut seen = codes;
        seen.sort_unstable();
        let before = seen.len();
        let mut dedup = seen.to_vec();
        dedup.dedup();
        assert_eq!(dedup.len(), before, "two exit codes collide: {codes:?}");
        assert!(!codes.contains(&2), "2 is reserved by clap");
    }

    #[test]
    fn round_trip_mode_names_match_the_stdout_contract() {
        // `mode=` is part of the stable stdout line, so these strings
        // are a compatibility surface, not cosmetics. They are also the
        // clap value names, which must stay in step.
        assert_eq!(mode_name(RoundTripMode::Incremental), "incremental");
        assert_eq!(mode_name(RoundTripMode::Full), "full");
        assert_eq!(mode_name(RoundTripMode::AppendIdentity), "append-identity");
    }

    #[test]
    fn save_mode_names_match_the_stdout_contract() {
        // `mode=` is part of the stable stdout line for both editing
        // subcommands, so these strings are a compatibility surface.
        assert_eq!(SaveMode::Incremental.name(), "incremental");
        assert_eq!(SaveMode::Full.name(), "full");
    }

    #[test]
    fn editing_subcommands_default_to_the_signature_safe_save_mode() {
        // Incremental is the default because it is the only mode that
        // preserves an existing signature's byte range (§12.8.1 NOTE 1)
        // and the only one that leaves prior revisions recoverable. A
        // default of `full` would silently destroy signatures on every
        // batch edit.
        on_large_stack(|| {
            use clap::CommandFactory as _;
            let cmd = Cli::command();
            for name in ["set-info", "rotate-page"] {
                let sub = cmd
                    .get_subcommands()
                    .find(|c| c.get_name() == name)
                    .unwrap_or_else(|| panic!("{name} subcommand is missing"));
                let mode = sub
                    .get_arguments()
                    .find(|a| a.get_id() == "mode")
                    .unwrap_or_else(|| panic!("{name} has no --mode flag"));
                assert_eq!(
                    mode.get_default_values()
                        .first()
                        .map(|v| v.to_string_lossy().into_owned()),
                    Some("incremental".to_owned()),
                    "{name}"
                );
            }
        });
    }

    #[test]
    fn info_field_args_map_one_to_one_onto_the_core_enum() {
        // A CLI enum that silently mapped two flags onto one field
        // would make `--clear` unpredictable; the round trip pins it.
        use pdfce_core::edit::InfoField;
        let pairs = [
            (InfoFieldArg::Title, InfoField::Title),
            (InfoFieldArg::Author, InfoField::Author),
            (InfoFieldArg::Subject, InfoField::Subject),
            (InfoFieldArg::Keywords, InfoField::Keywords),
        ];
        for (arg, expected) in pairs {
            assert_eq!(InfoField::from(arg), expected);
        }
        assert_eq!(pairs.len(), InfoField::all().len());
    }

    #[test]
    fn round_trip_defaults_are_the_verification_safe_ones() {
        // The CLI's `--producer` default is `preserve`, deliberately
        // NOT pdfce-core's `Set`: this subcommand's job is verification,
        // and a stamped /Producer is a byte change that would make the
        // per-object identity check fail for one object by design.
        on_large_stack(|| {
            use clap::CommandFactory as _;
            let cmd = Cli::command();
            let rt = cmd
                .get_subcommands()
                .find(|c| c.get_name() == "round-trip")
                .expect("round-trip subcommand is missing");
            let producer = rt
                .get_arguments()
                .find(|a| a.get_id() == "producer")
                .expect("--producer flag is missing");
            assert_eq!(
                producer
                    .get_default_values()
                    .first()
                    .map(|v| v.to_string_lossy().into_owned()),
                Some("preserve".to_owned())
            );
            let mode = rt
                .get_arguments()
                .find(|a| a.get_id() == "mode")
                .expect("--mode flag is missing");
            assert_eq!(
                mode.get_default_values()
                    .first()
                    .map(|v| v.to_string_lossy().into_owned()),
                Some("incremental".to_owned())
            );
        });
    }
}
