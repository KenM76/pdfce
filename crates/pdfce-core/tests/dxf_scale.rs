//! Deriving a DXF export scale from the page's **ce dimensions**
//! (`Pass 52.2` substrate).
//!
//! ## What is being protected
//!
//! A PDF drawing is at *paper* scale. A 1:2 detail exported at face value
//! arrives at half real size, and — this is the part that makes it
//! dangerous rather than merely wrong — it **looks entirely plausible**.
//! Nothing about the resulting DXF says it is half size. The operator finds
//! out at the cutting table.
//!
//! Every generic PDF→DXF converter has this problem and none of them can
//! solve it, because the scale is not in the file. pdfce can, because it
//! already asked: the measure tool's *scale by known dimension* takes the
//! length the drawing itself prints for a feature and derives the factor.
//! [`suggest_scale`] is the bridge from that answer to
//! [`DxfOptions::scale`].
//!
//! ## The three cases, and why the third is the reason this is a test file
//!
//! `Uncalibrated` and `Calibrated` are the obvious two. `Conflicting` is
//! the one that would otherwise be handled by accident: a sheet carrying a
//! 1:1 plan **and** a 1:5 detail is an ordinary drawing, and DXF has one
//! scale. Silently taking the first group's answer exports half the sheet
//! wrong — and, again, plausibly.
//!
//! ## Unit-independence is load-bearing, not incidental
//!
//! `ScaleState::effective_scale` answers *"how many of the group's display
//! units is one PDF point?"*, which is a **different number** for a
//! millimetre group than for an inch group describing the same 1:1
//! drawing (0.3528 vs 0.01389). Comparing those raw would report a
//! conflict between two groups that agree perfectly. The division by the
//! unit's own baseline is what cancels the unit out, and
//! `two_groups_in_different_units_describing_one_scale_do_not_conflict`
//! is the assertion that it actually does.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::dimension::{DimensionModel, NumberFormat, ScaleState, Unit};
use pdfce_core::export::dxf::{DxfScaleSuggestion, DxfUnits, suggest_scale};

/// The relative slack the assertions allow. The conversions are exact in
/// binary for inches (1/72) but not for millimetres (25.4/72), so an exact
/// comparison would be testing float representation rather than the
/// arithmetic.
const EPS: f64 = 1e-12;

fn assert_close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= EPS * want.abs().max(1.0),
        "{what}: expected {want}, got {got}"
    );
}

// ---------------------------------------------------------------------------
// Uncalibrated
// ---------------------------------------------------------------------------

/// **A fresh document infers nothing — it does not infer 1.0.**
///
/// The distinction is the whole point. `Uncalibrated` makes the caller say
/// *"pdfce does not know the scale of this drawing"*; a `1.0` fallback
/// would let it say nothing at all and export at paper scale, which is the
/// failure this feature exists to prevent.
#[test]
fn a_document_with_no_calibrated_group_infers_nothing_rather_than_one() {
    let model = DimensionModel::new();
    assert_eq!(
        suggest_scale(&model),
        DxfScaleSuggestion::Uncalibrated,
        "the default group's scale is NeverSet, so there is nothing to infer"
    );
}

// ---------------------------------------------------------------------------
// Calibrated
// ---------------------------------------------------------------------------

/// **A group calibrated to a 1:2 view yields scale 2.0.**
///
/// 1 pt = 25.4/36 mm is twice the true-scale 25.4/72, i.e. the drawing
/// shows a feature at half its real size, so real-units-per-paper-unit is
/// 2.
#[test]
fn a_one_to_two_millimetre_group_yields_scale_two() {
    let mut model = DimensionModel::new();
    let g = model.add_group("Detail", Unit::Millimeter);
    model.set_group_scale(
        g,
        ScaleState::Calibrated { scale: 25.4 / 36.0 },
        NumberFormat::decimal(Unit::Millimeter, 2),
    );
    match suggest_scale(&model) {
        DxfScaleSuggestion::Calibrated {
            scale,
            units,
            group,
            agreeing,
        } => {
            assert_close(scale, 2.0, "a 1:2 view");
            assert_eq!(units, DxfUnits::Millimetres, "a millimetre group");
            assert_eq!(group, "Detail", "named so the disclosure can cite it");
            assert_eq!(agreeing, 1);
        }
        other => panic!("expected Calibrated, got {other:?}"),
    }
}

/// **An explicit 1:1 is a real answer, distinct from never-set.**
///
/// `ScaleState` is deliberately tri-state (ui-spec §4.3) precisely so a
/// deliberate full-size drawing is not confused with an uncalibrated one.
/// If that distinction were lost here, an operator who had explicitly
/// confirmed 1:1 would still be told pdfce did not know.
#[test]
fn an_explicit_one_to_one_group_is_calibrated_at_scale_one() {
    let mut model = DimensionModel::new();
    let g = model.add_group("Full size", Unit::Inch);
    model.set_group_scale(
        g,
        ScaleState::OneToOne,
        NumberFormat::decimal(Unit::Inch, 3),
    );
    match suggest_scale(&model) {
        DxfScaleSuggestion::Calibrated { scale, units, .. } => {
            assert_close(scale, 1.0, "an explicit 1:1");
            assert_eq!(units, DxfUnits::Inches);
        }
        other => panic!("expected Calibrated, got {other:?}"),
    }
}

/// **Two groups in DIFFERENT units describing the same scale agree.**
///
/// The assertion the unit-cancellation exists for. A millimetre group and
/// an inch group on the same 1:1 sheet have `effective_scale` values of
/// 25.4/72 ≈ 0.3528 and 1/72 ≈ 0.01389 — a factor of 25.4 apart. Compared
/// raw, that is a conflict, and the operator would be asked to resolve a
/// disagreement that does not exist.
#[test]
fn two_groups_in_different_units_describing_one_scale_do_not_conflict() {
    let mut model = DimensionModel::new();
    let mm = model.add_group("Plan (mm)", Unit::Millimeter);
    let inch = model.add_group("Plan (in)", Unit::Inch);
    model.set_group_scale(
        mm,
        ScaleState::OneToOne,
        NumberFormat::decimal(Unit::Millimeter, 1),
    );
    model.set_group_scale(
        inch,
        ScaleState::OneToOne,
        NumberFormat::decimal(Unit::Inch, 3),
    );
    match suggest_scale(&model) {
        DxfScaleSuggestion::Calibrated {
            scale, agreeing, ..
        } => {
            assert_close(scale, 1.0, "both describe full size");
            assert_eq!(
                agreeing, 2,
                "corroboration is counted; it is not a second answer"
            );
        }
        other => panic!(
            "two groups that agree must not be reported as a conflict; got {other:?} — \
             this is the unit-cancellation failing"
        ),
    }
}

// ---------------------------------------------------------------------------
// Conflicting
// ---------------------------------------------------------------------------

/// **A 1:1 plan and a 1:5 detail on one sheet is a conflict, not a pick.**
///
/// This is an ordinary drawing, not a malformed one, and DXF carries one
/// scale. Choosing either group silently exports the other half of the
/// sheet wrong by a factor of five — and the result looks fine.
#[test]
fn groups_calibrated_to_different_scales_conflict_rather_than_first_winning() {
    let mut model = DimensionModel::new();
    let plan = model.add_group("Plan", Unit::Millimeter);
    let detail = model.add_group("Detail 1:5", Unit::Millimeter);
    model.set_group_scale(
        plan,
        ScaleState::OneToOne,
        NumberFormat::decimal(Unit::Millimeter, 1),
    );
    model.set_group_scale(
        detail,
        ScaleState::Calibrated {
            scale: 5.0 * 25.4 / 72.0,
        },
        NumberFormat::decimal(Unit::Millimeter, 1),
    );
    match suggest_scale(&model) {
        DxfScaleSuggestion::Conflicting { candidates } => {
            assert_eq!(candidates.len(), 2, "both opinions must be reported");
            // Group order, so a caller's list does not reshuffle between
            // calls under the operator's cursor.
            assert_eq!(candidates[0].group, "Plan");
            assert_eq!(candidates[1].group, "Detail 1:5");
            assert_close(candidates[0].scale, 1.0, "the plan");
            assert_close(candidates[1].scale, 5.0, "the detail");
        }
        other => panic!("a disagreement must be surfaced, not resolved silently; got {other:?}"),
    }
}

/// **A never-set group alongside a calibrated one is not a conflict.**
///
/// `NeverSet` is the absence of an opinion, not a competing one. Treating
/// it as a candidate would make the default group — which every document
/// has, and which starts never-set — conflict with the first real
/// calibration the operator performs, i.e. it would fire on the single
/// most common case there is.
#[test]
fn a_never_set_group_abstains_instead_of_conflicting() {
    let mut model = DimensionModel::new();
    // `DimensionModel::new()` already carries a never-set "Default" group;
    // this adds a second so the abstention is not a one-off.
    let _quiet = model.add_group("Not calibrated", Unit::Meter);
    let g = model.add_group("Measured", Unit::Inch);
    model.set_group_scale(
        g,
        ScaleState::Calibrated { scale: 2.0 / 72.0 },
        NumberFormat::decimal(Unit::Inch, 3),
    );
    match suggest_scale(&model) {
        DxfScaleSuggestion::Calibrated {
            scale,
            agreeing,
            group,
            ..
        } => {
            assert_close(scale, 2.0, "the one group that has an opinion");
            assert_eq!(agreeing, 1, "the two never-set groups do not count");
            assert_eq!(group, "Measured");
        }
        other => panic!("expected Calibrated, got {other:?}"),
    }
}
