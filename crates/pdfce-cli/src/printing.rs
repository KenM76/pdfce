//! # Printing — Windows print-system access for `pdfce-cli`
//!
//! The Reader-parity sweep's largest gap. Acrobat Reader's most-used
//! function after viewing is printing, and pdfce had **no print code
//! anywhere in the workspace** before this module.
//!
//! ## Why this lives in the CLI crate and not in `pdfce-core`
//!
//! Printing is the one genuinely platform-bound capability pdfce needs.
//! `pdfce-core` and `pdfce-render` must never gain a platform or
//! windowing dependency — that invariant (project rule 2,
//! `ARCHITECTURE.md` §3) is what keeps the eventual web/WASM fork a
//! shell-crate swap rather than a rewrite, and a `windows` dependency in
//! core would end it as surely as an `egui` one.
//!
//! So the split is: **core rasterises, the shell spools.**
//! `pdfce_render::render_page` produces an RGBA pixmap from a page on any
//! platform; this module is the Windows-only half that hands those pixels
//! to a printer. The GUI will call the same code for the same reason.
//!
//! The whole module is `#[cfg(windows)]` at its use site, and the
//! `windows` crate is declared under `[target.'cfg(windows)'.dependencies]`
//! so the Linux and macOS CI jobs still compile this crate — a compile
//! signal that the codebase stays platform-clean (R10), never a support
//! claim (R9).
//!
//! ## Not a new dependency
//!
//! The `windows` crate was ALREADY in the workspace tree at 0.62, pulled
//! transitively by eframe/winit, MIT-OR-Apache-2.0, already listed in the
//! generated `THIRD_PARTY_LICENSES.md`. Verified with `cargo tree` before
//! adding rather than assumed — project rule 13 makes classifying a
//! dependency a precondition, and "it was already there" is a claim that
//! has to be checked like any other.
//!
//! ## What this module does NOT do yet, stated plainly
//!
//! **It does not spool a job.** Printing is an outward-facing side effect
//! on the operator's machine — it consumes paper, occupies a shared
//! device, and cannot be undone. This first slice is the read-only half:
//! enumerate what printers exist and report what pdfce would target. The
//! spooling half is written against a real printer only with the
//! operator's explicit go-ahead.
//!
//! ## The rendering approach, and how it differs from Reader
//!
//! Reader sends **vector and text natively to the print driver**, which
//! RIPs at print time; "Print as Image" is a separate, explicitly-invoked
//! fallback for driver bugs and damaged content
//! (`Acrobat_Features/printing__rendering_pipeline_and_resolution.md`).
//!
//! pdfce's planned first slice rasterises — i.e. it makes Reader's
//! *fallback* the default. That is an honest limitation, not a hidden
//! one: a raster print of a vector CAD drawing at 300 DPI is visibly
//! coarser than the driver's own RIP would produce, and an operator
//! printing a drawing needs to be told that rather than discovering it on
//! paper. Emitting vector to a GDI device context means a second
//! rendering backend targeting GDI primitives, which is a substantial
//! piece of work and a later slice.
//!
//! Memory is the constraint that decides the default resolution: an A4
//! page at 600 DPI is 4960×7016 px, which at RGBA is ~139 MB for one
//! page. At 300 DPI it is ~35 MB. So a cap exists, and when it binds it
//! is disclosed — pdfce chose a resolution the operator did not ask for,
//! which is exactly rule 4's territory.

#![cfg(windows)]

use std::fmt;

/// One printer the system knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printer {
    /// The printer's name, as the spooler reports it. This is the string
    /// a caller passes to `--printer`.
    pub name: String,
    /// The driver's name, for disambiguation. Two printers can share a
    /// human-readable name closely enough that an operator cannot tell
    /// which is which; the driver usually distinguishes them.
    pub driver: String,
    /// The port, for the same reason.
    pub port: String,
    /// Whether this is the system default.
    pub is_default: bool,
}

/// Why the print system could not be queried.
#[derive(Debug, Clone)]
pub enum PrintError {
    /// `EnumPrinters` failed. Carries the Win32 error code, because
    /// "could not list printers" without one is unactionable.
    Enumerate(u32),
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enumerate(code) => write!(
                f,
                "the Windows print spooler could not be queried (error {code}) — \
                 the Print Spooler service may be stopped"
            ),
        }
    }
}

/// List the printers this machine can reach.
///
/// # Why `PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS`
///
/// `LOCAL` alone misses network printers the user has connected to,
/// which on a workstation is usually *most* of them — an operator whose
/// office printer is absent from the list would reasonably conclude
/// pdfce cannot see it. `CONNECTIONS` adds exactly those.
///
/// # The two-call pattern is required, not defensive
///
/// `EnumPrinters` is called twice by design: the first call fails with
/// `ERROR_INSUFFICIENT_BUFFER` and reports the byte count needed, the
/// second fills it. There is no way to ask for the size alone, and
/// guessing a buffer size would either truncate the list silently or
/// waste memory on every call.
///
/// # Errors
///
/// [`PrintError::Enumerate`] when the spooler cannot be queried at all.
/// An empty list is NOT an error — a machine with no printers installed
/// is a normal machine, and reporting that as a failure would send a
/// caller looking for a fault that does not exist.
pub fn list_printers() -> Result<Vec<Printer>, PrintError> {
    use windows::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_INFO_2W,
    };

    // SAFETY: the two-call pattern below is the documented contract for
    // `EnumPrintersW`. The first call is expected to fail; its purpose is
    // to write the required byte count into `needed`.
    let mut needed: u32 = 0;
    let mut returned: u32 = 0;
    unsafe {
        // Deliberately ignoring the result: this call is EXPECTED to fail
        // with ERROR_INSUFFICIENT_BUFFER, and treating that as an error
        // would make the happy path unreachable.
        let _ = EnumPrintersW(
            PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
            None,
            2,
            None,
            &mut needed,
            &mut returned,
        );
    }
    if needed == 0 {
        // No printers at all. Not an error — see this function's docs.
        return Ok(Vec::new());
    }

    let mut buffer = vec![0u8; needed as usize];
    // SAFETY: `buffer` is `needed` bytes, which is the size the call above
    // asked for.
    unsafe {
        EnumPrintersW(
            PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
            None,
            2,
            Some(&mut buffer),
            &mut needed,
            &mut returned,
        )
    }
    .map_err(|e| PrintError::Enumerate(e.code().0.unsigned_abs()))?;

    // The default printer's name, for flagging. A failure here is not
    // fatal: not knowing which is default is a smaller loss than
    // reporting no printers at all, so it degrades to "none flagged".
    let default_name = {
        let mut len: u32 = 0;
        // SAFETY: same two-call pattern; the first call reports the length.
        unsafe {
            let _ = GetDefaultPrinterW(None, &mut len);
        }
        if len == 0 {
            String::new()
        } else {
            let mut buf = vec![0u16; len as usize];
            // SAFETY: `buf` holds `len` UTF-16 units, as just requested.
            // Returns BOOL, not Result — unlike `EnumPrintersW` in the same
            // module, which does return Result. The `windows` crate maps
            // each API to whatever its own signature is, so the two sit
            // side by side with different shapes.
            let ok = unsafe {
                GetDefaultPrinterW(Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut len)
            }
            .as_bool();
            if ok {
                utf16_to_string(&buf)
            } else {
                String::new()
            }
        }
    };

    let mut out = Vec::with_capacity(returned as usize);
    // SAFETY: the spooler wrote `returned` contiguous `PRINTER_INFO_2W`
    // records at the head of `buffer`; the pointers inside them point into
    // the same allocation, which outlives this loop.
    let infos = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<PRINTER_INFO_2W>(), returned as usize)
    };
    for info in infos {
        // SAFETY: these are NUL-terminated UTF-16 strings inside `buffer`.
        let name = unsafe { pwstr_to_string(info.pPrinterName) };
        let driver = unsafe { pwstr_to_string(info.pDriverName) };
        let port = unsafe { pwstr_to_string(info.pPortName) };
        if name.is_empty() {
            // A nameless printer cannot be targeted by `--printer`, so
            // listing it would offer something unusable (R83).
            continue;
        }
        out.push(Printer {
            is_default: !default_name.is_empty() && name == default_name,
            name,
            driver,
            port,
        });
    }
    Ok(out)
}

/// Decode a NUL-terminated wide string the spooler owns.
///
/// # Safety
///
/// `p` must be either null or a pointer to a NUL-terminated UTF-16 string
/// that remains valid for the duration of the call.
unsafe fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: the caller guarantees NUL-termination and validity.
    unsafe { p.to_string() }.unwrap_or_default()
}

/// Decode a UTF-16 buffer that may carry a trailing NUL.
fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(buf.get(..end).unwrap_or_default())
}
