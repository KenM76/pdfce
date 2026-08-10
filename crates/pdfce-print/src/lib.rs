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
//! # Its own crate, and why not `pdfce-core`
//!
//! This began as a module inside `pdfce-cli`. It moved when the GUI
//! needed it, and it moved OUT rather than DOWN.
//!
//! `pdfce-core` and `pdfce-render` must not gain a platform dependency:
//! that is the invariant (`ARCHITECTURE.md` §3) which keeps the eventual
//! web/WASM fork a shell-crate swap instead of a rewrite, and a print
//! spooler is about as platform-bound as code gets. Putting it in either
//! would trade a load-bearing property for the convenience of one fewer
//! manifest.
//!
//! The alternative — a copy in each shell — fails for the ordinary
//! reason: two copies of page-placement arithmetic drift, and the
//! symptom is a GUI print that lands differently from a CLI print of the
//! same document, which nobody would look for.
//!
//! So: one crate, two shells, and `windows` confined to the only place
//! in the workspace that talks to a spooler.
//!
//! # ★ Spooling is an irreversible outward-facing act
//!
//! Printing consumes paper, occupies a device other people may share,
//! and cannot be undone. Nothing in this crate starts a job as a side
//! effect of anything else: [`spool`] is the only function that reaches
//! `StartDoc`, and it is reached only from a control an operator
//! deliberately clicked.
//!
//! [`DryRun::Yes`] exists so that the whole path — device context,
//! `DEVMODE`, capability query, placement, rasterisation, the per-page
//! loop — can be exercised and verified without a sheet of paper moving.
//! That is not a testing convenience bolted on afterwards; it is how this
//! code was developed, because the machine it was written on has one
//! printer and its owner was sitting at it.
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

// NOTE: this module is NOT wholly `cfg(windows)`. The page-placement
// math below is pure geometry with no platform dependency, and it is the
// part most worth unit-testing — so it compiles and its tests run on the
// Linux and macOS CI jobs too. Only the spooler-facing half is gated.

#[cfg(windows)]
use std::fmt;

/// One printer the system knows about.
#[cfg(windows)]
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
#[cfg(windows)]
#[derive(Debug, Clone)]
pub enum PrintError {
    /// `EnumPrinters` failed. Carries the Win32 error code, because
    /// "could not list printers" without one is unactionable.
    Enumerate(u32),
    /// A printer name did not resolve to a device. Carries the name,
    /// because the overwhelmingly common cause is a typo and a generic
    /// failure leaves the operator nothing to compare against.
    OpenDevice(String),
    /// The driver reported a resolution of zero, which is malformed.
    /// Named rather than worked around: dividing by it would produce
    /// infinities that reach the placement math and emerge as a blank
    /// page with no explanation.
    NoResolution(String),
    /// `CreateDC` returned no device context for a printer that
    /// enumerated. Distinct from [`PrintError::OpenDevice`]: the name
    /// resolved and the DEVICE still refused, which usually means a
    /// driver problem rather than a typo, and sends the operator
    /// somewhere different.
    DeviceContext {
        /// The printer that refused.
        printer: String,
    },
    /// `StartDoc` failed. **No job exists**, so nothing is queued and
    /// nothing needs cancelling.
    JobStart {
        /// The printer that refused the job.
        printer: String,
    },
    /// `StartPage` failed part-way through a job. The job is aborted.
    PageStart,
    /// `EndPage` failed part-way through a job. The job is aborted.
    PageEnd,
    /// `EndDoc` failed. The job may or may not have reached the device —
    /// stated as the uncertainty it is, because claiming either would be
    /// a guess about a queue this process no longer controls.
    JobEnd,
    /// `StretchDIBits` drew nothing.
    Blit,
    /// A page's pixel dimensions exceed what GDI accepts.
    PageTooLarge,
}

#[cfg(windows)]
impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enumerate(code) => write!(
                f,
                "the Windows print spooler could not be queried (error {code}) — \
                 the Print Spooler service may be stopped"
            ),
            Self::OpenDevice(name) => write!(
                f,
                "no printer named {name:?} — run `pdfce-cli list-printers` to see \
                 the names this machine knows"
            ),
            Self::DeviceContext { printer } => write!(
                f,
                "the printer {printer:?} was found but its driver would not open a device;                  this is usually a driver problem rather than a wrong name"
            ),
            Self::JobStart { printer } => write!(
                f,
                "{printer:?} refused the print job. Nothing was queued, so there is nothing                  to cancel"
            ),
            Self::PageStart => write!(f, "the printer refused a page; the job was cancelled"),
            Self::PageEnd => write!(f, "a page failed to finish; the job was cancelled"),
            Self::JobEnd => write!(
                f,
                "the job did not close cleanly. Some pages may already have reached the                  printer — check the queue rather than reprinting blind"
            ),
            Self::Blit => write!(f, "the page image could not be drawn to the printer"),
            Self::PageTooLarge => write!(
                f,
                "the page is too large in pixels for the print system; try a lower resolution"
            ),
            Self::NoResolution(name) => write!(
                f,
                "the driver for {name:?} reports a resolution of zero dots per inch, \
                 which pdfce cannot lay a page out against"
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
#[cfg(windows)]
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
#[cfg(windows)]
unsafe fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: the caller guarantees NUL-termination and validity.
    unsafe { p.to_string() }.unwrap_or_default()
}

/// Decode a UTF-16 buffer that may carry a trailing NUL.
#[cfg(windows)]
fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(buf.get(..end).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Printer capabilities — Windows, read-only, starts no job
// ---------------------------------------------------------------------------

/// What a printer can physically do with a sheet.
///
/// Every measurement is in **points** (1/72 inch), converted from the
/// device's own pixels here so nothing downstream has to know the DPI.
/// That conversion is the one place a printing bug hides most easily:
/// mixing device pixels and points silently produces output that is right
/// on one printer and wrong on the next.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrinterCaps {
    /// Horizontal resolution in dots per inch.
    pub dpi_x: u32,
    /// Vertical resolution.
    pub dpi_y: u32,
    /// The full sheet, in points.
    pub physical_pt: (f64, f64),
    /// The area the hardware can actually mark, in points.
    ///
    /// Always smaller than [`Self::physical_pt`]. Fitting a page to the
    /// PHYSICAL size instead of this one produces a page whose edges the
    /// hardware crops — which looks exactly like a pdfce bug and is not
    /// one.
    pub printable_pt: (f64, f64),
    /// Where the printable area begins relative to the sheet corner, in
    /// points. Needed because GDI's drawing origin is the printable
    /// corner, not the paper corner.
    pub offset_pt: (f64, f64),
}

/// Query a printer's capabilities.
///
/// Opens an information device context, reads it, and closes it. **It
/// starts no print job** — `CreateDC` on a printer is a read of the
/// driver's configuration, not a spool operation, so this is safe to run
/// on a machine somebody is using.
///
/// # Errors
///
/// [`PrintError::OpenDevice`] when the printer name does not resolve.
/// The most common cause is a typo, so the error names the string that
/// failed rather than reporting a generic failure.
#[cfg(windows)]
pub fn printer_caps(name: &str) -> Result<PrinterCaps, PrintError> {
    use windows::Win32::Graphics::Gdi::{
        CreateDCW, DeleteDC, GetDeviceCaps, HORZRES, LOGPIXELSX, LOGPIXELSY, PHYSICALHEIGHT,
        PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, VERTRES,
    };
    use windows::core::HSTRING;

    let wide = HSTRING::from(name);
    // SAFETY: `wide` outlives the call. A null return is the documented
    // failure signal, checked immediately below.
    let hdc = unsafe { CreateDCW(None, &wide, None, None) };
    if hdc.is_invalid() {
        return Err(PrintError::OpenDevice(name.to_owned()));
    }

    // SAFETY: `hdc` is a valid DC until `DeleteDC` below.
    let caps = unsafe {
        let dpi_x = GetDeviceCaps(Some(hdc), LOGPIXELSX);
        let dpi_y = GetDeviceCaps(Some(hdc), LOGPIXELSY);
        // Guard the divisors before any conversion. A driver reporting
        // zero DPI is malformed, and dividing by it would produce
        // infinities that reach the placement math and turn into a blank
        // page nobody can explain.
        if dpi_x <= 0 || dpi_y <= 0 {
            let _ = DeleteDC(hdc);
            return Err(PrintError::NoResolution(name.to_owned()));
        }
        let px_to_pt_x = |px: i32| f64::from(px) * 72.0 / f64::from(dpi_x);
        let px_to_pt_y = |px: i32| f64::from(px) * 72.0 / f64::from(dpi_y);
        let c = PrinterCaps {
            dpi_x: dpi_x.unsigned_abs(),
            dpi_y: dpi_y.unsigned_abs(),
            physical_pt: (
                px_to_pt_x(GetDeviceCaps(Some(hdc), PHYSICALWIDTH)),
                px_to_pt_y(GetDeviceCaps(Some(hdc), PHYSICALHEIGHT)),
            ),
            printable_pt: (
                px_to_pt_x(GetDeviceCaps(Some(hdc), HORZRES)),
                px_to_pt_y(GetDeviceCaps(Some(hdc), VERTRES)),
            ),
            offset_pt: (
                px_to_pt_x(GetDeviceCaps(Some(hdc), PHYSICALOFFSETX)),
                px_to_pt_y(GetDeviceCaps(Some(hdc), PHYSICALOFFSETY)),
            ),
        };
        let _ = DeleteDC(hdc);
        c
    };
    Ok(caps)
}

// ---------------------------------------------------------------------------
// Page placement — pure geometry, no platform dependency
// ---------------------------------------------------------------------------

/// How a page is sized onto the sheet.
///
/// The four modes Acrobat Reader offers, and they are genuinely four:
/// **Fit and ShrinkOversized are not the same operation**
/// (`Acrobat_Features/printing__scaling_modes.md`). Fit scales in both
/// directions — a small page is ENLARGED to fill the sheet.
/// ShrinkOversized only ever reduces. Treating them as one, which is the
/// natural simplification, silently blows a business card up to A4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleMode {
    /// Scale to fill the printable area, up or down, preserving aspect.
    /// Reader's default.
    Fit,
    /// 1 PDF point = 1/72 inch on paper, whatever that costs.
    ActualSize,
    /// Like [`Self::ActualSize`], except a page too large for the sheet
    /// is reduced to fit. Never enlarges.
    ShrinkOversized,
    /// An explicit multiplier, where `1.0` is actual size. Reader accepts
    /// a free-form 1–1000%, not a set of presets.
    Custom(f64),
}

/// Where and how big a page lands on the sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Multiplier from PDF points to paper points.
    pub scale: f64,
    /// Offset within the printable area, in paper points, to centre the
    /// page.
    pub offset_x_pt: f64,
    /// Vertical offset, same units.
    pub offset_y_pt: f64,
    /// **The scaled page does not fit and will lose content off the
    /// edges.**
    ///
    /// Acrobat's documented behaviour here is to clip SILENTLY — a page
    /// wider than the paper simply loses its margins with no warning
    /// (`printing__scaling_modes.md`, recorded as a still-open Acrobat
    /// weakness). pdfce reports it instead, which is the operator's
    /// standing ruling applied: parity is a floor, and losing content
    /// without saying so is not a behaviour worth matching.
    pub clipped: bool,
}

/// Compute where a page lands on a sheet.
///
/// All inputs and outputs are in **points** (1/72 inch), including the
/// paper measurements — the caller converts from device pixels using the
/// printer's own DPI, so this function never sees a device unit and
/// therefore cannot be wrong about one.
///
/// `printable` is the PRINTABLE area, not the physical sheet. Every
/// printer has an unprintable margin it cannot reach, and fitting to the
/// physical size instead produces a page whose edges are cropped by the
/// hardware — which looks exactly like a pdfce bug and is not one.
#[must_use]
pub fn place_page(page: (f64, f64), printable: (f64, f64), mode: ScaleMode) -> Placement {
    let (pw, ph) = page;
    let (aw, ah) = printable;
    // A degenerate page or sheet has no meaningful placement. Returning
    // scale 1.0 rather than dividing by zero: the caller gets something
    // renderable, and `clipped` tells the truth about it.
    if pw <= 0.0 || ph <= 0.0 || aw <= 0.0 || ah <= 0.0 {
        return Placement {
            scale: 1.0,
            offset_x_pt: 0.0,
            offset_y_pt: 0.0,
            clipped: true,
        };
    }

    let fit = (aw / pw).min(ah / ph);
    let scale = match mode {
        ScaleMode::Fit => fit,
        ScaleMode::ActualSize => 1.0,
        // `min(1.0)` is the whole difference from Fit, and the reason
        // both modes exist.
        ScaleMode::ShrinkOversized => fit.min(1.0),
        // A non-finite or non-positive multiplier is a caller error that
        // must not become a non-finite scale downstream; fall back to
        // actual size rather than propagate a NaN into device
        // coordinates, where it would silently produce nothing on paper.
        ScaleMode::Custom(m) if m.is_finite() && m > 0.0 => m,
        ScaleMode::Custom(_) => 1.0,
    };

    let w = pw * scale;
    let h = ph * scale;
    // A hair of tolerance: floating-point `fit` can land a whisker over
    // the boundary and report a clip nobody could see on paper.
    const EPS: f64 = 0.5;
    Placement {
        scale,
        // Centred. Clamped at zero so an oversized page starts at the
        // edge of the printable area rather than at a negative offset,
        // which would push MORE of it off the sheet than necessary.
        offset_x_pt: ((aw - w) / 2.0).max(0.0),
        offset_y_pt: ((ah - h) / 2.0).max(0.0),
        clipped: w > aw + EPS || h > ah + EPS,
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
    use super::{
        Collate, DeviceGeometry, JobSpec, PageSubset, Placement, ScaleMode, job_resolution,
        place_page, plan_job,
    };

    /// A4 in points.
    const A4: (f64, f64) = (595.0, 842.0);
    /// A Letter sheet's printable area with a typical 1/4-inch hardware
    /// margin all round.
    const LETTER_PRINTABLE: (f64, f64) = (612.0 - 36.0, 792.0 - 36.0);

    /// Fit ENLARGES a small page; ShrinkOversized refuses to.
    ///
    /// The single assertion that keeps the two modes distinct. Collapsing
    /// them is the natural simplification, and it silently blows a
    /// business card up to fill a Letter sheet.
    #[test]
    fn fit_enlarges_where_shrink_oversized_refuses_to() {
        let card = (252.0, 144.0);
        let fit = place_page(card, LETTER_PRINTABLE, ScaleMode::Fit);
        let shrink = place_page(card, LETTER_PRINTABLE, ScaleMode::ShrinkOversized);
        assert!(fit.scale > 1.0, "Fit must enlarge: {fit:?}");
        assert!(
            (shrink.scale - 1.0).abs() < f64::EPSILON,
            "ShrinkOversized must never enlarge: {shrink:?}"
        );
    }

    /// The two modes agree when the page is too big — which is the only
    /// case where Shrink has anything to do.
    #[test]
    fn the_two_modes_agree_on_an_oversized_page() {
        let big = (1190.0, 1684.0);
        let fit = place_page(big, LETTER_PRINTABLE, ScaleMode::Fit);
        let shrink = place_page(big, LETTER_PRINTABLE, ScaleMode::ShrinkOversized);
        assert!(fit.scale < 1.0);
        assert!((fit.scale - shrink.scale).abs() < 1e-12);
        assert!(!fit.clipped, "a fitted page must not clip: {fit:?}");
    }

    /// Actual size CLIPS an oversized page, and says so.
    ///
    /// Acrobat clips here silently. Reporting it is the deliberate
    /// divergence: an operator who is about to lose the right-hand column
    /// of a drawing should learn it before the paper comes out.
    #[test]
    fn actual_size_reports_the_clip_acrobat_stays_quiet_about() {
        let big = (1190.0, 1684.0);
        let p = place_page(big, LETTER_PRINTABLE, ScaleMode::ActualSize);
        assert!((p.scale - 1.0).abs() < f64::EPSILON);
        assert!(p.clipped, "content WILL be lost and must be reported");
        // Offsets clamp at zero: an oversized page starts at the printable
        // edge rather than at a negative offset, which would throw away
        // more of it than the paper requires.
        assert_eq!(p.offset_x_pt, 0.0);
        assert_eq!(p.offset_y_pt, 0.0);
    }

    /// A page that fits is centred in the printable area.
    #[test]
    fn a_fitting_page_is_centred() {
        let small = (288.0, 396.0);
        let p = place_page(small, LETTER_PRINTABLE, ScaleMode::ActualSize);
        assert!(!p.clipped);
        assert!((p.offset_x_pt - (LETTER_PRINTABLE.0 - 288.0) / 2.0).abs() < 1e-9);
        assert!((p.offset_y_pt - (LETTER_PRINTABLE.1 - 396.0) / 2.0).abs() < 1e-9);
    }

    /// A custom multiplier is honoured, and a nonsense one degrades to
    /// actual size rather than poisoning device coordinates with a NaN.
    #[test]
    fn a_custom_scale_is_honoured_and_a_nonsense_one_is_not_propagated() {
        let p = place_page(A4, LETTER_PRINTABLE, ScaleMode::Custom(0.5));
        assert!((p.scale - 0.5).abs() < f64::EPSILON);
        for bad in [f64::NAN, f64::INFINITY, 0.0, -2.0] {
            let q: Placement = place_page(A4, LETTER_PRINTABLE, ScaleMode::Custom(bad));
            assert!(q.scale.is_finite() && q.scale > 0.0, "bad={bad} gave {q:?}");
        }
    }

    /// A degenerate page or sheet yields something renderable rather than
    /// a division by zero — and admits it is not right.
    #[test]
    fn degenerate_input_does_not_produce_a_non_finite_scale() {
        for (page, sheet) in [
            ((0.0, 100.0), LETTER_PRINTABLE),
            ((100.0, 0.0), LETTER_PRINTABLE),
            (A4, (0.0, 100.0)),
            (A4, (100.0, -5.0)),
        ] {
            let p = place_page(page, sheet, ScaleMode::Fit);
            assert!(p.scale.is_finite() && p.scale > 0.0, "{p:?}");
            assert!(p.clipped, "a degenerate placement must not claim to fit");
        }
    }

    // ---- job planning ----

    /// Capabilities standing in for a 600-DPI Letter printer with a
    /// quarter-inch unprintable margin all round.
    fn letter_600() -> DeviceGeometry {
        DeviceGeometry {
            dpi: (600, 600),
            printable_pt: (576.0, 756.0),
        }
    }

    fn spec(pages: Vec<usize>, mode: ScaleMode, max_dpi: u32) -> JobSpec {
        JobSpec {
            pages,
            mode,
            max_dpi,
            subset: PageSubset::All,
            reverse: false,
            copies: 1,
            collate: Collate::Collated,
        }
    }

    /// **The render scale already carries the print scale.**
    ///
    /// This is the property that keeps a printed line as sharp as the
    /// same line on screen: the pixels handed to GDI are the size they
    /// will occupy on paper, so the blit is a copy rather than a
    /// resample. If this ever becomes plain `dpi / 72`, output softens
    /// everywhere and nothing else fails.
    #[test]
    fn the_render_scale_folds_in_the_placement_scale() {
        let caps = letter_600();
        // A Letter page shrunk to the printable area: 576/612 ≈ 0.941.
        let plans = plan_job(
            &caps,
            &[(612.0, 792.0)],
            &spec(vec![0], ScaleMode::Fit, 600),
        );
        let p = plans.first().expect("one page planned");
        assert!(p.placement.scale < 1.0, "Fit shrinks a full-bleed page");
        let expected = (600.0 / 72.0) * p.placement.scale;
        assert!(
            (p.render_scale - expected).abs() < 1e-9,
            "render_scale must be dpi/72 × placement.scale, not dpi/72"
        );
    }

    /// The cap binds, is reported, and changes the render scale with it.
    #[test]
    fn the_dpi_cap_binds_and_is_disclosed() {
        let caps = letter_600();
        let res = job_resolution(&caps, &spec(vec![0], ScaleMode::ActualSize, 300));
        assert_eq!(res.dpi, 300);
        assert_eq!(res.device_dpi, 600);
        assert!(res.capped, "300 < 600, so the operator is told");

        let uncapped = job_resolution(&caps, &spec(vec![0], ScaleMode::ActualSize, 1200));
        assert_eq!(uncapped.dpi, 600, "the cap never RAISES beyond the device");
        assert!(!uncapped.capped);
    }

    /// ★ **An asymmetric device renders at its SMALLER axis.**
    ///
    /// 600×300 is real on plotters. Rendering at 600 for a device that
    /// can only place 300 dots vertically makes the driver resample —
    /// which undoes the entire reason for rendering at device
    /// resolution, silently, and on exactly the machines whose output
    /// people care most about.
    #[test]
    fn an_asymmetric_device_renders_at_its_smaller_axis() {
        let caps = DeviceGeometry {
            dpi: (600, 300),
            ..letter_600()
        };
        assert_eq!(
            job_resolution(&caps, &spec(vec![0], ScaleMode::ActualSize, 2400)).dpi,
            300
        );
    }

    /// **A stale page index is skipped, not fatal.**
    ///
    /// A page range is operator input and can name a page a since-edited
    /// document no longer has. Refusing the whole job because one index
    /// is stale is worse than printing what exists and reporting the
    /// count — the operator wanted paper, and nine of ten pages is
    /// recoverable where zero is not.
    #[test]
    fn an_out_of_range_page_is_skipped_rather_than_failing_the_job() {
        let caps = letter_600();
        let sizes = [(612.0, 792.0), (612.0, 792.0)];
        let plans = plan_job(&caps, &sizes, &spec(vec![0, 7, 1], ScaleMode::Fit, 300));
        assert_eq!(plans.len(), 2, "two real pages survive");
        assert_eq!(plans[0].index, 0);
        assert_eq!(plans[1].index, 1, "and the order given is preserved");
    }

    /// The page ORDER in the spec is the print order, including
    /// duplicates and reversals — the shells build ranges, and reverse
    /// order is an option Acrobat offers.
    #[test]
    fn the_planned_order_is_the_requested_order() {
        let caps = letter_600();
        let sizes = [(612.0, 792.0); 3];
        let plans = plan_job(&caps, &sizes, &spec(vec![2, 0, 2], ScaleMode::Fit, 300));
        assert_eq!(
            plans.iter().map(|p| p.index).collect::<Vec<_>>(),
            vec![2, 0, 2]
        );
    }

    /// Mixed page sizes each get their own placement — a document with a
    /// landscape drawing among portrait pages must not scale them all to
    /// the first page's factor.
    #[test]
    fn each_page_is_placed_on_its_own_size() {
        let caps = letter_600();
        let sizes = [(612.0, 792.0), (792.0, 612.0)];
        let plans = plan_job(&caps, &sizes, &spec(vec![0, 1], ScaleMode::Fit, 300));
        assert!(
            (plans[0].placement.scale - plans[1].placement.scale).abs() > 1e-6,
            "a portrait and a landscape page cannot share a fit scale"
        );
    }

    // ---- page sequencing: subset, reverse, copies, collate ----

    fn seq(
        pages: Vec<usize>,
        subset: PageSubset,
        reverse: bool,
        copies: u16,
        collate: Collate,
    ) -> Vec<usize> {
        JobSpec {
            pages,
            mode: ScaleMode::Fit,
            max_dpi: 300,
            subset,
            reverse,
            copies,
            collate,
        }
        .sequence()
    }

    /// ★ **Odd/even is by DOCUMENT page number, not position in the
    /// range.**
    ///
    /// "Pages 2-9, odd" means the pages numbered 3, 5, 7, 9 — what is
    /// printed on the paper — not the first, third and fifth entries of
    /// the range, which would be 2, 4, 6.
    ///
    /// Both readings produce a plausible page count, and one produces
    /// entirely the wrong sheets. That is why this has a test rather
    /// than a comment.
    #[test]
    fn odd_and_even_are_by_document_page_number() {
        // Zero-based 1..=8 is document pages 2..=9.
        let range: Vec<usize> = (1..=8).collect();
        assert_eq!(
            seq(range.clone(), PageSubset::Odd, false, 1, Collate::Collated),
            vec![2, 4, 6, 8],
            "document pages 3,5,7,9"
        );
        assert_eq!(
            seq(range, PageSubset::Even, false, 1, Collate::Collated),
            vec![1, 3, 5, 7],
            "document pages 2,4,6,8"
        );
    }

    /// ★ **Subset is applied BEFORE reverse.**
    ///
    /// "Even pages, reversed" is the even pages in reverse order.
    /// Reversing first and then taking every other entry yields a
    /// different SET — on an even-length range it yields the odd pages.
    #[test]
    fn the_subset_is_taken_before_the_reverse() {
        let range: Vec<usize> = (0..4).collect(); // document pages 1..=4
        assert_eq!(
            seq(range, PageSubset::Even, true, 1, Collate::Collated),
            vec![3, 1],
            "document pages 4 then 2 — not pages 3 and 1"
        );
    }

    /// Collated repeats the whole sequence; uncollated repeats each page.
    #[test]
    fn collation_decides_where_the_copies_go() {
        let range = vec![0, 1, 2];
        assert_eq!(
            seq(range.clone(), PageSubset::All, false, 2, Collate::Collated),
            vec![0, 1, 2, 0, 1, 2]
        );
        assert_eq!(
            seq(range, PageSubset::All, false, 2, Collate::Uncollated),
            vec![0, 0, 1, 1, 2, 2]
        );
    }

    /// **Copies multiply the FINISHED sequence.**
    ///
    /// If copies were applied before the subset, the filter would run
    /// over duplicated pages and collation would have nothing left to
    /// mean. Pinned with all three options at once, because the order of
    /// operations is the only place a defect can hide in code this
    /// short.
    #[test]
    fn copies_apply_to_the_sequence_after_subset_and_reverse() {
        let range: Vec<usize> = (0..4).collect();
        assert_eq!(
            seq(range, PageSubset::Odd, true, 2, Collate::Collated),
            vec![2, 0, 2, 0],
            "odd document pages 1,3 -> reversed 3,1 -> twice"
        );
    }

    /// Zero copies prints once. A job of nothing is never what was
    /// meant, and erroring would be a dialog fault for a value no UI
    /// should have produced.
    #[test]
    fn zero_copies_is_treated_as_one() {
        assert_eq!(
            seq(vec![0, 1], PageSubset::All, false, 0, Collate::Collated),
            vec![0, 1]
        );
    }
}

// ---------------------------------------------------------------------------
// Job planning — the arithmetic both shells share
// ---------------------------------------------------------------------------

/// What to print, in the caller's terms.
///
/// # Why planning is separate from rendering
///
/// Both shells need the same answer to "at what scale, and where on the
/// sheet, does page N land, and what resolution should it be rendered
/// at" — and that arithmetic is the part that drifts when it is written
/// twice. The symptom of drift here is a GUI print landing differently
/// from a CLI print of the same document at the same settings, which
/// nobody would think to compare.
///
/// So the arithmetic lives here and the RENDERING stays in the shells.
/// That keeps this crate free of `pdfce-render` — see the crate docs on
/// why a printing crate that also rendered would need the whole render
/// stack to be testable, when the failures worth testing here (a wrong
/// `DEVMODE`, an upside-down DIB, a job left open) have nothing to do
/// with PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct JobSpec {
    /// Zero-based page indices, in the order they should print.
    pub pages: Vec<usize>,
    /// How each page is sized onto the sheet.
    pub mode: ScaleMode,
    /// Upper bound on rendering resolution, in DPI.
    ///
    /// A MEMORY bound, not a quality preference: an A4 page at 600 DPI is
    /// 4960×7016 px, about 139 MB at RGBA for one page. Whoever sets it
    /// is choosing a number the operator did not, so both shells disclose
    /// it when it binds (rule 4).
    pub max_dpi: u32,
    /// Odd/even filtering, applied over [`Self::pages`].
    pub subset: PageSubset,
    /// Print the sequence back to front.
    pub reverse: bool,
    /// How many copies. Zero is treated as one — a job of nothing is
    /// never what an operator meant, and refusing it would be a dialog
    /// error for a value no UI should have allowed.
    pub copies: u16,
    /// Copy ordering.
    pub collate: Collate,
}

/// Which of the selected pages actually print (Acrobat's odd/even
/// subset filter).
///
/// Applied AFTER the range, and composing with it rather than replacing
/// it — "pages 1-10, even only" is a thing an operator asks for, and a
/// design where the subset replaced the range would make that
/// unexpressible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageSubset {
    /// Every page in the range.
    #[default]
    All,
    /// Odd pages by their 1-based DOCUMENT number, not their position in
    /// the range.
    ///
    /// This distinction is the whole reason the field is documented: an
    /// operator printing "2-9, odd" means document pages 3, 5, 7, 9 —
    /// the numbers printed on the paper — not the first, third and fifth
    /// entries of the range. Getting it wrong produces a plausible page
    /// count and the wrong sheets, which is the hardest kind of wrong to
    /// notice.
    Odd,
    /// Even pages, by document number, same reasoning.
    Even,
}

/// How multiple copies are ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Collate {
    /// 1,2,3, 1,2,3 — whole documents, in order.
    #[default]
    Collated,
    /// 1,1, 2,2, 3,3 — all copies of each page together.
    ///
    /// Faster on most hardware because the page is rasterised once per
    /// position rather than once per copy, and the order a stapler
    /// wants.
    Uncollated,
}

impl JobSpec {
    /// Expand `pages` into the actual print sequence: subset filtered,
    /// reversed if asked, then multiplied by copies in the chosen order.
    ///
    /// # Order of operations, and why it is this one
    ///
    /// Subset, then reverse, then copies. Each step is defined on the
    /// result of the previous, and the order is not arbitrary:
    ///
    /// - **Subset before reverse**, because "even pages, reversed" means
    ///   the even pages in reverse order. Reversing first and then taking
    ///   every other entry would yield a different set entirely — odd
    ///   pages, on an even-length range.
    /// - **Copies last**, because a copy is a copy of the finished
    ///   sequence. Multiplying first would let the subset filter run over
    ///   duplicated pages, and `Collate` would have nothing left to mean.
    ///
    /// Written down because all three steps are one-liners and the ORDER
    /// is the only place a defect can hide.
    #[must_use]
    pub fn sequence(&self) -> Vec<usize> {
        let mut seq: Vec<usize> = self
            .pages
            .iter()
            .copied()
            .filter(|&i| match self.subset {
                PageSubset::All => true,
                // `i` is zero-based; the operator's page number is `i+1`.
                PageSubset::Odd => (i + 1) % 2 == 1,
                PageSubset::Even => (i + 1) % 2 == 0,
            })
            .collect();
        if self.reverse {
            seq.reverse();
        }
        let copies = self.copies.max(1);
        match self.collate {
            Collate::Collated => seq
                .iter()
                .copied()
                .cycle()
                .take(seq.len() * copies as usize)
                .collect(),
            Collate::Uncollated => seq
                .iter()
                .flat_map(|&i| std::iter::repeat_n(i, copies as usize))
                .collect(),
        }
    }
}

/// Where one page lands, and how big to render it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PagePlan {
    /// The page this describes, as given in [`JobSpec::pages`].
    pub index: usize,
    /// Placement on the sheet.
    pub placement: Placement,
    /// The scale to rasterise at, in device pixels per PDF point.
    ///
    /// # It already carries the print scale, deliberately
    ///
    /// This is `dpi / 72 × placement.scale`, so the pixels handed to the
    /// spooler are already the size they will occupy on paper and the
    /// blit is a 1:1 copy.
    ///
    /// The alternative — render at device resolution and let
    /// `StretchDIBits` scale — resamples twice, once in the renderer's
    /// own transform and once in GDI's, and the visible result is a
    /// printed line softer than the same line on screen. On a CAD
    /// drawing, whose value is thin lines, that is the difference the
    /// operator would notice first.
    pub render_scale: f64,
}

/// The resolution a job will render at, and whether the cap bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobResolution {
    /// The DPI actually used.
    pub dpi: u32,
    /// The device's own resolution, before the cap.
    pub device_dpi: u32,
    /// Whether [`JobSpec::max_dpi`] reduced it — the case that must be
    /// disclosed, because pdfce chose a number the operator did not.
    pub capped: bool,
}

impl JobResolution {
    /// Rough memory cost of ONE page at the DEVICE's resolution, in
    /// megabytes, for a US-Letter sheet at RGBA.
    ///
    /// Approximate on purpose, and the figure a disclosure quotes: an
    /// operator deciding whether to raise the cap needs an order of
    /// magnitude, not a precise number for a page size they may not be
    /// printing.
    #[must_use]
    pub const fn uncapped_page_mb(self) -> u64 {
        (self.device_dpi as u64 * self.device_dpi as u64 * 8 * 11 * 4) / 1_000_000
    }
}

/// The device geometry planning needs, with no platform type in it.
///
/// # Why not just take `PrinterCaps`
///
/// `PrinterCaps` is `cfg(windows)` — it is what a Win32 driver reported.
/// The planning arithmetic is pure geometry, and this module's own note
/// says that half stays un-gated so it compiles and TESTS on the Linux
/// and macOS CI jobs.
///
/// Taking `PrinterCaps` here would have quietly moved the most
/// test-worthy code in the crate behind a `cfg` that CI does not build —
/// the tests would still pass on Windows and simply stop existing
/// elsewhere, which is the kind of coverage loss nothing reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceGeometry {
    /// Resolution in dots per inch, horizontal and vertical.
    pub dpi: (u32, u32),
    /// The printable area in points — smaller than the sheet by the
    /// unprintable margins the driver reports.
    pub printable_pt: (f64, f64),
}

#[cfg(windows)]
impl From<&PrinterCaps> for DeviceGeometry {
    fn from(caps: &PrinterCaps) -> Self {
        Self {
            dpi: (caps.dpi_x, caps.dpi_y),
            printable_pt: caps.printable_pt,
        }
    }
}

/// Resolve the rendering resolution for a job.
#[must_use]
pub fn job_resolution(device: &DeviceGeometry, spec: &JobSpec) -> JobResolution {
    // The SMALLER axis, not an average: a device with asymmetric
    // resolution (600×300 is real on some plotters) must not be rendered
    // at a resolution one axis cannot reproduce, because the driver then
    // resamples and undoes the point of rendering at device resolution.
    let smaller = device.dpi.0.min(device.dpi.1);
    let dpi = smaller.min(spec.max_dpi);
    JobResolution {
        dpi,
        device_dpi: smaller,
        capped: dpi < smaller,
    }
}

/// Plan every page of a job.
///
/// `page_sizes` is indexed by the document's page order, in PDF points.
/// Indices in [`JobSpec::pages`] that fall outside it are SKIPPED rather
/// than erroring: a page range is operator input, and a job that refuses
/// wholesale because one index is stale is worse than one that prints
/// what it can and reports the count.
#[must_use]
pub fn plan_job(
    device: &DeviceGeometry,
    page_sizes: &[(f64, f64)],
    spec: &JobSpec,
) -> Vec<PagePlan> {
    let resolution = job_resolution(device, spec);
    spec.sequence()
        .into_iter()
        .filter_map(|index| {
            let size = *page_sizes.get(index)?;
            let placement = place_page(size, device.printable_pt, spec.mode);
            Some(PagePlan {
                index,
                placement,
                render_scale: (f64::from(resolution.dpi) / 72.0) * placement.scale,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Spooling (§ the irreversible half)
// ---------------------------------------------------------------------------

/// One page's pixels, ready to place on a sheet.
///
/// The caller rasterises. This crate does device setup, placement and
/// blitting, and knows nothing about PDF — which is why it does not
/// depend on `pdfce-render`.
///
/// That split is deliberate rather than incidental: a printing crate
/// that also rendered would need the whole render stack to be testable,
/// and the interesting failures here (a wrong `DEVMODE`, an upside-down
/// DIB, a job left open on an error path) have nothing to do with PDF.
#[derive(Debug, Clone)]
pub struct PageBitmap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8, row-major, top row first — the layout `tiny_skia::Pixmap`
    /// produces, so the caller hands over `pixmap.data().to_vec()`
    /// unchanged.
    pub rgba: Vec<u8>,
    /// Where this page lands on the sheet, from [`place_page`].
    pub placement: Placement,
    /// The page's size in PDF points, for the placement arithmetic.
    pub page_pt: (f64, f64),
}

/// Whether [`spool`] actually starts a print job.
///
/// # ★ Not a testing convenience — the development mode
///
/// [`DryRun::Yes`] performs every step except the four that reach the
/// spooler (`StartDoc`, `StartPage`, `EndPage`, `EndDoc`) and the blit.
/// It opens the device context, reads the real device's resolution and
/// printable area, computes placement for every page, and walks the
/// whole loop.
///
/// So the things that actually go wrong — a printer name that does not
/// resolve, a device that reports a printable area smaller than the
/// caller assumed, a page whose scaled size clips, an arithmetic slip in
/// the DIB header — all surface without a sheet of paper moving.
///
/// This exists because the machine this was written on has one printer
/// and its owner was sitting at it. That constraint produced a better
/// design than unlimited paper would have: the expensive, irreversible
/// step is isolated behind one flag rather than woven through the
/// function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRun {
    /// Do everything except start a job. Nothing prints.
    Yes,
    /// Start a real job on a real device. **Consumes paper.**
    No,
}

/// What a spool attempt did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolReport {
    /// Pages sent, or that would have been sent under [`DryRun::Yes`].
    pub pages: usize,
    /// Whether a job was actually started.
    pub printed: bool,
    /// The device's reported resolution.
    pub dpi: (i32, i32),
    /// Pages whose placement reported [`Placement::clipped`].
    ///
    /// Reported rather than refused: an operator may legitimately want a
    /// page cropped to the sheet, and Acrobat clips silently. pdfce
    /// clips and SAYS so — the operator's standing ruling that parity is
    /// a floor.
    pub clipped_pages: usize,
    /// The job's spooler ID, when one was started.
    pub job_id: Option<u32>,
}

/// Send pages to a printer — **the only function in pdfce that starts a
/// print job**.
///
/// # Errors
///
/// [`PrintError`] if the printer cannot be resolved, the device context
/// cannot be created, or the spooler rejects the job. A job that fails
/// part-way is ABORTED rather than left open (see the guard below), so a
/// half-finished document does not sit in the queue holding a device.
///
/// # Safety of the irreversible step
///
/// `StartDoc` is reached on exactly one code path, guarded by
/// [`DryRun::No`], and this function is called from exactly one place in
/// each shell — a control the operator clicked. Nothing here runs as a
/// side effect of rendering, previewing, saving or opening.
#[cfg(windows)]
pub fn spool(
    printer: &str,
    pages: &[PageBitmap],
    dry_run: DryRun,
    output: Option<&std::path::Path>,
) -> Result<SpoolReport, PrintError> {
    use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC};
    use windows::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
    use windows::core::PCWSTR;

    let caps = printer_caps(printer)?;
    let wide: Vec<u16> = printer.encode_utf16().chain(std::iter::once(0)).collect();

    // SAFETY: `wide` is NUL-terminated and outlives the call. A null DC
    // is the documented failure and is checked rather than assumed.
    let hdc = unsafe { CreateDCW(PCWSTR::null(), PCWSTR(wide.as_ptr()), PCWSTR::null(), None) };
    if hdc.is_invalid() {
        return Err(PrintError::DeviceContext {
            printer: printer.to_owned(),
        });
    }

    // Every early return past this point must delete the DC, and a job
    // opened must be ended. Rust has no `finally`, so the work happens in
    // a closure whose result is inspected AFTER the cleanup — which is
    // the shape that makes "the error path leaked a device context" and
    // "the error path left a job in the queue" both unrepresentable
    // rather than merely avoided.
    let mut report = SpoolReport {
        pages: 0,
        printed: false,
        dpi: (caps.dpi_x as i32, caps.dpi_y as i32),
        clipped_pages: pages.iter().filter(|p| p.placement.clipped).count(),
        job_id: None,
    };

    let outcome: Result<(), PrintError> = (|| {
        if dry_run == DryRun::Yes {
            // The dry run stops HERE, after the device has been opened
            // and interrogated for real. Everything above this line is
            // the part that fails in practice.
            report.pages = pages.len();
            return Ok(());
        }

        let doc_name: Vec<u16> = "pdfce document\0".encode_utf16().collect();
        // `lpszOutput` redirects the job to a FILE instead of the port.
        //
        // This is what makes a `PORTPROMPT:` driver — "Microsoft Print to
        // PDF" and most PDF writers — usable without a Save dialog
        // appearing. It is both a real capability ("print to file") and
        // the only way this code path can be verified by anything other
        // than a person watching a printer.
        //
        // The buffer is bound rather than built inline because the
        // `PCWSTR` must outlive the `DOCINFOW`: a pointer into a dropped
        // temporary is a dangling one, and nothing in the type system
        // catches it here.
        let out_wide: Option<Vec<u16>> = output.map(|p| {
            p.as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect()
        });
        let info = DOCINFOW {
            cbSize: i32::try_from(std::mem::size_of::<DOCINFOW>()).unwrap_or(0),
            lpszDocName: PCWSTR(doc_name.as_ptr()),
            lpszOutput: out_wide
                .as_ref()
                .map_or_else(PCWSTR::null, |w| PCWSTR(w.as_ptr())),
            ..Default::default()
        };
        // SAFETY: `hdc` is valid (checked above) and `info` outlives the
        // call. A non-positive return is the documented failure.
        let job = unsafe { StartDocW(hdc, &info) };
        if job <= 0 {
            return Err(PrintError::JobStart {
                printer: printer.to_owned(),
            });
        }
        report.printed = true;
        report.job_id = u32::try_from(job).ok();

        for page in pages {
            // SAFETY: valid DC, and the page loop always pairs
            // StartPage with EndPage — see the abort path below for the
            // case where it cannot.
            if unsafe { StartPage(hdc) } <= 0 {
                return Err(PrintError::PageStart);
            }
            blit_page(hdc, page, (caps.dpi_x as i32, caps.dpi_y as i32))?;
            if unsafe { EndPage(hdc) } <= 0 {
                return Err(PrintError::PageEnd);
            }
            report.pages += 1;
        }

        // SAFETY: valid DC with a job open.
        if unsafe { EndDoc(hdc) } <= 0 {
            return Err(PrintError::JobEnd);
        }
        Ok(())
    })();

    // A job that errored part-way is ABORTED, not left open. Windows
    // holds the device for an unfinished job, so a leaked one blocks
    // every other user of a shared printer until it times out — the
    // failure mode most likely to affect somebody who is not the
    // operator.
    if outcome.is_err() && report.printed {
        // SAFETY: valid DC with a job open. `AbortDoc` is the
        // documented cancel, and its result is deliberately ignored —
        // the error already being returned is the one that matters, and
        // a failure to abort cleanly changes nothing the caller can act
        // on.
        unsafe {
            let _ = AbortDoc(hdc);
        }
    }
    // SAFETY: valid DC, deleted exactly once on every path.
    unsafe {
        let _ = DeleteDC(hdc);
    }
    outcome.map(|()| report)
}

/// Blit one page's pixels onto the current page of `hdc`.
///
/// # The two conversions that are easy to get wrong
///
/// **Orientation.** A `BITMAPINFOHEADER` with a POSITIVE height is
/// bottom-up: Windows reads the first row in memory as the BOTTOM of the
/// image. The caller's buffer is top-down (that is what `tiny_skia`
/// produces), so the height is negated. Get this wrong and every page
/// prints upside down — which is obvious on paper and invisible in every
/// test that does not print.
///
/// **Channel order.** `BI_RGB` at 32bpp is B, G, R, X in memory, and the
/// caller's buffer is R, G, B, A. The swap happens here rather than
/// being asked of the caller, because the caller's layout is the
/// renderer's and this crate is the one that knows what GDI wants.
///
/// Alpha is DISCARDED, not composited: a printed page has no
/// transparency, and the renderer has already composited onto white.
#[cfg(windows)]
fn blit_page(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    page: &PageBitmap,
    dpi: (i32, i32),
) -> Result<(), PrintError> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, SRCCOPY, StretchDIBits,
    };

    let w = i32::try_from(page.width).map_err(|_| PrintError::PageTooLarge)?;
    let h = i32::try_from(page.height).map_err(|_| PrintError::PageTooLarge)?;

    // RGBA (caller) -> BGRX (GDI).
    let mut bgra = Vec::with_capacity(page.rgba.len());
    for px in page.rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[px[2], px[1], px[0], 0]);
    }

    let header = BITMAPINFOHEADER {
        biSize: u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
        biWidth: w,
        // NEGATIVE: top-down. See the fn docs.
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let info = BITMAPINFO {
        bmiHeader: header,
        ..Default::default()
    };

    // Points -> device pixels, at the device's own resolution. 72 points
    // to the inch is the PDF unit's definition, not a convention.
    let px_x = |pt: f64| (pt * f64::from(dpi.0) / 72.0).round() as i32;
    let px_y = |pt: f64| (pt * f64::from(dpi.1) / 72.0).round() as i32;

    let dest_w = px_x(page.page_pt.0 * page.placement.scale);
    let dest_h = px_y(page.page_pt.1 * page.placement.scale);
    let dest_x = px_x(page.placement.offset_x_pt);
    let dest_y = px_y(page.placement.offset_y_pt);

    // SAFETY: `hdc` is valid with a page open; `info` and `bgra` outlive
    // the call; the dimensions are derived from the buffer itself.
    let sent = unsafe {
        StretchDIBits(
            hdc,
            dest_x,
            dest_y,
            dest_w,
            dest_h,
            0,
            0,
            w,
            h,
            Some(bgra.as_ptr().cast()),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    if sent == 0 {
        return Err(PrintError::Blit);
    }
    Ok(())
}

/// A stub so callers compile on non-Windows without `cfg` at every call
/// site. Printing is a Windows capability in this release.
#[cfg(not(windows))]
pub fn spool(
    _printer: &str,
    _pages: &[PageBitmap],
    _dry_run: DryRun,
    _output: Option<&std::path::Path>,
) -> Result<SpoolReport, PrintError> {
    Err(PrintError::Unsupported)
}
