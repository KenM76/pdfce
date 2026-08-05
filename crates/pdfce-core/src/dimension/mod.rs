//! # `dimension` — the scaled measurement/dimensioning subsystem (Pass 12.M2)
//!
//! The headline capability of decision 011's first beta
//! (`docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
//! §2.3–2.4): linear + radius/diameter dimensions, named groups with per-group
//! scale + units, the tri-state scale, and the **hybrid storage** that makes a
//! dimensioned PDF self-contained and interoperable at once.
//!
//! ## Crate placement (GUI-core separation, binding invariant)
//!
//! Everything here is `pdfce-core` — **no GUI/windowing dependency** (no
//! egui/eframe/winit/wgpu, not even `tiny-skia`): the best-fit geometry, the
//! scale/units math, the group model, and the PDF-object storage authoring are
//! pure. The GUI (`pdfce-gui`) only supplies the tool UX; the render honouring
//! of the authored OCG layer is `pdfce-render`; the in-document wiring is
//! [`crate::edit`]. `cargo tree -p pdfce-core` stays egui/eframe/winit/wgpu/
//! glow-free (`docs/ARCHITECTURE.md` §3).
//!
//! ## The five pieces
//!
//! 1. [`fit`] — the **Taubin** best-fit circle (chosen for the short-arc
//!    regime where Kåsa is biased — proven by test), with a fit residual and
//!    an optional Gauss-Newton refine. ZERO new dependency (rule 13).
//! 2. [`units`] — the six unit modes (mm/cm/m/inch/decimal-ft/**ft-in**), the
//!    ISO §12.9 Table 263 number-format algorithm (incl. feet-inches), scale
//!    back-calculation, and the **tri-state** scale (never-set / explicit-1:1 /
//!    calibrated).
//! 3. [`group`] — the authoritative model: named groups (scale + units + OCG +
//!    membership) and immutable per-dimension geometry (the value model:
//!    geometry stored, displayed value derived).
//! 4. [`measure_dict`] — the portable §12.9 `/Measure` dict + §8.11 `/OCG` /
//!    `/OCProperties` optional-content builders (the reader-visible interop
//!    mirror + toggleable layer).
//! 5. [`author`] — the `/Line` + `/IT /LineDimension` annotation with a
//!    fully-baked `/AP` (leader + arrowheads + value label), additive
//!    (overlay-append, R46 zero-exception).
//! 6. [`sidecar`] — the authoritative §14.5 `/PieceInfo /pdfce /Private` model
//!    serialization (round-trips the whole [`group::DimensionModel`]).
//!
//! ## Hybrid storage (decision 011 §2.4 "binding answer 1")
//!
//! Three coordinated layers, authored together by [`crate::edit`]:
//! (a) native `/Line` + `/IT /LineDimension` annotations with baked `/AP`
//! ([`author`]); (b) a per-annotation `/Measure` scale mirror ([`measure_dict`])
//! — the interop projection; (c) the **authoritative** `/PieceInfo` sidecar
//! ([`sidecar`]) — pdfce's own model, its survival guaranteed by R34, not by
//! §14.5. On load, native-vs-sidecar disagreement ⇒ disclose + prefer the
//! sidecar. Each group's dimensions sit on a per-group `/OCG` layer honoured by
//! pdfce's render (authored-annotation `/OC` only) and any OCG-aware reader.

pub mod author;
pub mod fit;
pub mod group;
/// Parse a real-world length written the way a drawing writes it
/// (`55 5/8"`, `4'-7 1/2"`), for the scale-by-known-dimension workflow.
pub mod length_parse;
pub mod measure_dict;
pub mod sidecar;
pub mod units;

// Re-export the everyday surface at `crate::dimension::…`.
pub use author::{AUTHORED_ANNOT_KEYS, AUTHORED_MEASURE_KEY, AuthoredDimension, author_dimension};
pub use fit::{FitCircle, fit_circle_taubin, fit_circle_taubin_refined};
pub use group::{
    DEFAULT_GROUP_ID, DimensionId, DimensionKind, DimensionModel, DimensionRecord, Group, GroupId,
};
pub use length_parse::{LengthParseError, ParsedLength, parse_length};
pub use measure_dict::{build_measure_dict, build_ocg, build_ocproperties};
pub use sidecar::{SIDECAR_VERSION, deserialize_model, serialize_model, sidecar_version};
pub use units::{
    FractionMode, MeasurementDisplay, NO_SCALE_DISCLOSURE, NumberFormat, ScaleEntry, ScalePreview,
    ScaleState, Unit, format_measurement, preview_group_scale,
};
