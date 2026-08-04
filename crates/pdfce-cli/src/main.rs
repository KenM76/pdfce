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
//! pure-ASCII line to stdout** summarizing what it did. Two lines are
//! specified so far; both are part of the compatibility surface and are
//! versioned like any other public API (a change is breaking).
//!
//! ```text
//! inspect:      <input>: PDF <major>.<minor>
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
//!               supplied=<g> supplied_registered=<h> contents_unresolved=<i>
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
    /// **Drag a node** of a path object (Pass 9c-min, decision 011 §2.5):
    /// rewrite ONE anchor's coordinate pair to a page-space point via surgery
    /// (R46/§5.7). `--node` is the anchor's 0-based index in decomposition
    /// order (start, then each segment endpoint, across subpaths). An `re`
    /// rectangle corner and an implicit reopened-subpath start are refused —
    /// move the whole object instead.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ProducerArg {
    /// Write `/Producer (pdfce <version>)` into an existing `/Info`.
    Set,
    /// Leave `/Info` byte-untouched (R41's no-fingerprint posture).
    Preserve,
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
        } => cmd_render_page(&input, page, scale, &output, !no_annotations, &font_dirs),
        Command::ListAnnotations { input, pages } => cmd_list_annotations(&input, &pages),
        Command::ListFields {
            input,
            fillable_only,
        } => cmd_list_fields(&input, fillable_only),
        Command::FillField {
            input,
            sets,
            output,
            mode,
            verify_undo,
        } => cmd_fill_field(&input, &sets, &output, mode, verify_undo),
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
            output: &output,
            mode,
            verify_undo,
        }),
        Command::DimensionList { input } => cmd_dimension_list(&input),
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
    let full = std::fs::read(file)
        .ok()
        .and_then(|bytes| pdfce_core::document::Document::from_bytes(bytes).ok());

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
         stream-lengths-recovered={}",
        file.display(),
        report.reason,
        report.file_level_objects,
        report.objstm_objects,
        report.last_wins_collisions,
        report.trailer_source,
        report.offset_start,
        report.stream_lengths_recovered,
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
fn cmd_render_page(
    input: &Path,
    page_number: u32,
    scale: f32,
    output: &Path,
    annotations: bool,
    font_dirs: &[PathBuf],
) -> u8 {
    // Build the font environment from any `--font-dir` BEFORE loading the
    // document: the walk is pure shell-side I/O (R61), and a bad font dir
    // is a note, never a fatal error. With no `--font-dir` this is exactly
    // the bundled default and the deterministic path is untouched (R63).
    let (font_env, supplied_registered, font_notes) = build_font_environment(font_dirs);
    for note in &font_notes {
        eprintln!("pdfce-cli: font-dir: {note}");
    }

    let doc = match pdfce_core::document::Document::load(input) {
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
    let mut render_options = pdfce_render::RenderOptions::default().with_annotations(annotations);
    render_options.fonts = font_env;
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
contents_unresolved={}",
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
    );
    report_diagnostics(d);

    exit::SUCCESS
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
///       flags=0x<hex> widget=<0|1> disposition=<D> ap=<A>
/// list-annotations <input> pages=<N>; annots=<T> paint_ready=<P> no_ap=<Q> \
///       state_missing=<S> suppressed=<H> popup=<U> widget=<W> need_appearances=<0|1>
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
/// # Exit codes
///
/// `0` success; `3`/`4` unreadable / not-a-PDF; `1` for a structural
/// failure or an out-of-range `--pages` selection.
fn cmd_list_annotations(input: &Path, pages_spec: &str) -> u8 {
    let doc = match pdfce_core::document::Document::load(input) {
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
            println!(
                "annot page={} index={array_index} subtype={subtype} rect={rect} \
flags=0x{:X} widget={} disposition={disposition} ap={ap_shape}",
                page_index + 1,
                annot.flags.0,
                usize::from(annot.is_widget()),
            );
        }
    }

    println!(
        "list-annotations {} pages={}; annots={total} paint_ready={paint_ready} no_ap={no_ap} \
state_missing={state_missing} suppressed={suppressed} popup={popup} widget={widget} \
need_appearances={need_appearances}",
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
fn cmd_list_fields(input: &Path, fillable_only: bool) -> u8 {
    let doc = match pdfce_core::document::Document::load(input) {
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
        let name = if field.fully_qualified_name.is_empty() {
            "(unnamed)".to_owned()
        } else {
            sanitize_token(&field.fully_qualified_name)
        };
        let value = {
            let v = field.value.display_text();
            if v.is_empty() {
                "-".to_owned()
            } else {
                sanitize_token(&v)
            }
        };
        println!(
            "field name={name} type={ty} button={button} flags=0x{:X} value={value} \
widgets={} ap={} fillable={} readonly={} aa={}",
            field.flags.0,
            field.widgets.len(),
            u32::from(field.has_appearance()),
            u32::from(field.is_fillable()),
            u32::from(field.flags.read_only()),
            u32::from(field.has_additional_actions),
        );
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

/// `fill-field`: set form-field values and save (Pass 7).
///
/// Each `NAME=VALUE` is dispatched by the field's modelled type: text/choice
/// through [`EditSession::fill_text_field`], check-box/radio through
/// [`EditSession::set_button_state`]. All assignments land in one session
/// (so the save carries them as one incremental revision), then the shared
/// [`save_edited`]/[`finish_edit`] plumbing writes and reports.
fn cmd_fill_field(
    input: &Path,
    sets: &[String],
    output: &Path,
    mode: SaveMode,
    verify_undo: bool,
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

/// Print the fuzzy-never-sneaky disclosures a fill owes (an applied
/// auto-size, any unencodable characters) to stderr.
fn disclose_fill(name: &str, out: &pdfce_core::edit::FillOutcome) {
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
    let doc = match pdfce_core::document::Document::load(input) {
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
    };
    if let Err(err) = std::fs::write(output, &bytes) {
        eprintln!("pdfce-cli: {}: {err}", output.display());
        return exit::IO_ERROR;
    }
    let fmt = match format {
        DataFormat::Fdf => "fdf",
        DataFormat::Xfdf => "xfdf",
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
    // Detect FDF vs XFDF by content: an XFDF file starts (after whitespace)
    // with `<`; an FDF file carries the `%FDF` header / `/FDF` dictionary.
    let is_xml = data_bytes
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'<');
    let parsed = if is_xml {
        pdfce_core::fdf::FormData::parse_xfdf(&data_bytes)
    } else {
        pdfce_core::fdf::FormData::parse_fdf(&data_bytes)
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
    let outcome = match session.import_form_data(&data) {
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
/// stays splittable on spaces. Names cannot legally contain whitespace
/// (§7.3.5 uses `#20` for a space), so this only fires on pathological
/// input.
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
    let doc = match Document::from_bytes(source.clone()) {
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

    let saved = match mode {
        RoundTripMode::Incremental => {
            pdfce_core::writer::save_incremental(&doc, &DirtySet::empty(), &options)
        }
        RoundTripMode::Full => pdfce_core::writer::save_full(&doc, &DirtySet::empty(), &options),
        RoundTripMode::AppendIdentity => {
            // Every object of the base revision, re-emitted unchanged.
            let ids: Vec<_> = doc.objects().map(|io| io.id).collect();
            pdfce_core::writer::save_incremental(
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
    let reloaded = Document::from_bytes(bytes.clone());
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
promoted={}",
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
    let doc = pdfce_core::document::Document::from_bytes(source.clone()).map_err(|err| {
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

    let options = SaveOptions::default().with_producer(match producer {
        ProducerArg::Set => ProducerPolicy::Set,
        ProducerArg::Preserve => ProducerPolicy::Preserve,
    });
    let changed = session.dirty_set().len();

    let saved = match mode {
        SaveMode::Incremental => session.to_incremental_bytes(&SaveOptions::identity()),
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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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

/// Six uppercase ASCII letters derived from a face name, for the §9.6.4
/// subset prefix.
///
/// Deterministic rather than random. A random tag would be equally valid and
/// would make two otherwise identical runs produce different bytes, which
/// breaks byte-comparison — and byte-comparison is how this project proves
/// its round-trip invariant. Derived from the name so two different faces in
/// one document are unlikely to collide.
///
/// Not a hash with collision guarantees, and does not pretend to be: if two
/// faces ever do collide, the consequence is two subsets sharing a tag, which
/// consumers tolerate (the tag is a hint, not an identifier). Spending
/// entropy here to avoid a harmless collision would cost the determinism,
/// which is the property actually worth having.
fn subset_tag_for(name: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (0..6)
        .map(|i| {
            let k = (h >> (i * 8)) & 0xff;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the modulo keeps this inside the 26-letter alphabet"
            )]
            char::from(b'A' + (k % 26) as u8)
        })
        .collect()
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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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
        let tag = subset_tag_for(&stem);

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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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
    let doc = match pdfce_core::document::Document::from_bytes(source) {
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

    let doc = match pdfce_core::document::Document::load(input) {
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

    let options = ExtractOptions::default().with_artifacts(include_artifacts);
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

    let doc = match Document::load(input) {
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
    let options = ExtractOptions::default().with_provenance(true);
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

    let doc = match Document::load(input) {
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
}

/// The metrics tail every document-producing subcommand shares.
fn assemble_metrics(report: &pdfce_core::pageops::AssembleReport, out_bytes: usize) -> String {
    format!(
        "pages={} objects={} dangling={} outline_kept={} outline_dropped={} \
fields_renamed={} fields_dropped={} dests_dropped={} labels_dropped={} labels_stale={} \
struct_tree_dropped={} ocg_carried={} out_bytes={out_bytes}",
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
    Document::load(path).map_err(|err| {
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
        println!(
            "  dim {} group={} kind={kind} value=\"{value}\"",
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
    if let Err(err) = session.move_object(page_index, args.object, args.dx, args.dy) {
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
    if let Err(err) = session.delete_subpath(page_index, args.object, args.subpath) {
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

/// `node-move` — rewrite one anchor's coordinate pair to a page-space point
/// via surgery (Pass 9c-min, decision 011 §2.5). Refuses an `re` rectangle
/// corner / implicit reopened start by name.
fn cmd_node_move(args: &NodeMoveArgs<'_>) -> u8 {
    use pdfce_core::vector::Point;
    let page_index = (args.page.max(1) - 1) as usize;
    let (source, mut session) = match open_for_edit(args.input) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    if let Err(err) = session.move_node(
        page_index,
        args.object,
        args.node,
        Point::new(args.x, args.y),
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
    let (bytes, report) = match pdfce_core::pageops::extract(&view, &selected) {
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
    let parts = match pdfce_core::pageops::split(&view, &criterion, name_template, &stem) {
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
    let outcome = match session.delete_pages(&selected) {
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
dangling_bookmarks={} dangling_links={} dangling_dests={} page_labels_stale={} {}",
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
