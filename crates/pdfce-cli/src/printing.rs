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
    use super::{Placement, ScaleMode, place_page};

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
}
