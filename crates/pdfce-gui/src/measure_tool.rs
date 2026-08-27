//! # measure_tool — Pass 12.M2b on-canvas dimension-authoring state machines
//!
//! The **pure, GUI-free authoring-state logic** the three Pass 12.M2 measure
//! tools drive on the canvas (`docs/ui_specs/pass-12.M2-dimension-tools.md`,
//! decision 011 §2.3/§2.4). Pass 12.M2 shipped the dimensioning *engine* +
//! `pdfce-cli` (`c7c1744`); the GUI shipped the "Measure ▾" menu, the three
//! [`crate::canvas::CanvasTool`] variants, the 12.M1 snap-indicator
//! primitives, and a status overlay — but **not** the click-to-author
//! interaction. This module is that missing slice's testable heart: the
//! pick state machines, the circular fit-set, and the scale-dialog
//! back-calc plumbing, all expressed over `pdfce-core` types (never egui),
//! so every transition is unit-tested here without a live frame — the same
//! discipline that keeps [`crate::canvas`]/[`crate::viewer`] headlessly
//! testable while `main.rs` stays a thin, compile-and-launch-only shell.
//!
//! ## What this module owns vs. what the shipped engine owns (REUSE, never reimplement)
//!
//! This module contains **zero** dimension geometry, Taubin math, scale
//! arithmetic, or storage. Every load-bearing computation is a call into the
//! already-shipped `pdfce-core::dimension` / `pdfce-core::vector`:
//!
//! - [`constrained_second_point`] / [`measured_length`] (12.M1) — the H/V/
//!   aligned projection and the measured page-space length.
//! - [`fit_circle_taubin`] (12.M2) — the best-fit circle over a sample set.
//! - [`preview_group_scale`] (12.M2) — the scale back-calc for both entry
//!   paths.
//! - [`DimensionKind`] (12.M2) — the immutable geometry the GUI hands to
//!   `EditSession::add_dimension`, **byte-for-byte the same value the CLI's
//!   `dimension-add` builds** (`pdfce-cli` stores `Linear { a: *a, b: *b,
//!   constraint }` from its two raw `--points`, and `Circular { fit,
//!   show_diameter }` from `fit_circle_taubin(&pts)` — so this module stores
//!   the **raw** snapped picks, NOT the constrained projection, matching the
//!   CLI exactly; the constrained segment is a *display-only* preview,
//!   ui-spec §2.5). The equivalence tests [`tests::gui_linear_kind_equals_cli_
//!   linear_kind`] / [`tests::gui_circular_kind_equals_cli_circular_kind`] pin
//!   this: identical `DimensionKind` ⇒ identical `add_dimension` call ⇒
//!   identical additive `/Line`+`/Measure`+`/PieceInfo`+`/OCG` bytes (rule:
//!   same engine path).
//!
//! ## The three tools' state
//!
//! - [`LinearPick`] — the A→B two-click state machine (ui-spec §2.1),
//!   shared verbatim by [`ScalePick`]'s reference line (§4.1).
//! - [`CircularPick`] — the tool's OWN object pick-set (ui-spec §3.1, NOT
//!   `canvas_selection`), live-refit on every toggle (§3.2), with the
//!   display-only radius/diameter toggle (§3.4).
//! - [`ScalePick`] + [`ScaleEntryFields`] — draw a reference line, then the
//!   two co-equal scale-entry paths (real-length recommended, ratio) that
//!   back-calc through [`preview_group_scale`] (§4).
//! - [`GroupAction`] — the group-panel actions, each a mapping onto exactly
//!   one shipped `EditSession` command (§5.4: one undo step each).
//!
//! Everything is `pdfce-gui`-internal; `cargo tree -p pdfce-core` is
//! unaffected (this module is not in core), and it adds no dependency.

use pdfce_core::dimension::{
    DimStandard, DimensionKind, FitCircle, FractionMode, GroupId, LengthParseError, NumberFormat,
    ScaleEntry, ScalePreview, ScaleState, TwoLineAuthoring, TwoLinePlacement, TwoLineRefusal, Unit,
    author_from_two_lines, fit_circle_taubin, parse_length, preview_group_scale,
};
use pdfce_core::vector::linepick::{ParallelPolicy, PickedLine};
use pdfce_core::vector::{AxisConstraint, Point, constrained_second_point, measured_length};

// ---------------------------------------------------------------------------
// Linear pick — the A→B two-click state machine (ui-spec §2.1/§2.5)
// ---------------------------------------------------------------------------

/// The linear-dimension two-pick state machine (ui-spec §2.1): click A →
/// live preview to a constrained second point → click B commits.
///
/// [`Self::first`] being `None` means "awaiting point A"; `Some(a)` means "A
/// is set, the next commit is point B." [`Self::commit_point`] on the second
/// pick returns the authored [`DimensionKind::Linear`] and resets, so the
/// tool is immediately ready for the next dimension (the operator draws a
/// run of dimensions without re-entering the tool).
///
/// **Raw-second-point storage (byte-equivalence, module docs):** the authored
/// `b` is the RAW snapped second pick, exactly as the CLI stores it; the
/// constraint is recorded alongside so `measured_length`/`author_dimension`
/// apply the H/V projection at value/appearance time. The on-canvas preview
/// segment ([`Self::preview_segment`]) is the *constrained* line — display
/// only, so "what you see is what's measured" (ui-spec §2.5) without diverging
/// the stored geometry from the CLI's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearPick {
    /// Point A (page space) once picked; `None` while awaiting the first pick.
    pub first: Option<Point>,
    /// Point B, once picked — the tool is then in its PLACING state, waiting
    /// for the third click that decides where the dimension is drawn (Pass
    /// 27.1).
    ///
    /// # Why a third click
    ///
    /// The operator asked for SolidWorks behaviour, and SolidWorks dimensions
    /// in three: what, to what, and where. The third is not ceremony — it is
    /// the only chance to say how far off the drawing the dimension sits, and
    /// without it every dimension lands on top of the geometry it measures and
    /// has to be dragged off afterwards. pdfce committed on the second click
    /// with a zero standoff, which is exactly that.
    pub second: Option<Point>,
    /// Whether this pick needs the third, PLACING click.
    ///
    /// True for a real ce dimension, which has to be told where to sit. False
    /// for [`ScalePick`]'s reference line, which is a measurement aid that is
    /// never drawn as a dimension — asking the operator where to place
    /// something that is about to disappear would be ceremony with no meaning.
    /// One flag rather than two pick types, because every other transition is
    /// identical and duplicating them is how they drift (R92).
    pub place_after: bool,
    /// The H/V/aligned constraint the property-bar segmented control sets
    /// (ui-spec §2.5). Applied to the SECOND point's projection + measured
    /// length; the stored `b` remains the raw pick (module docs).
    pub constraint: AxisConstraint,
}

impl Default for LinearPick {
    fn default() -> Self {
        Self::new()
    }
}

impl LinearPick {
    /// A fresh pick, awaiting point A, free (aligned) constraint.
    #[must_use]
    pub fn new() -> Self {
        Self {
            first: None,
            second: None,
            place_after: true,
            constraint: AxisConstraint::Aligned,
        }
    }

    /// Register a committed (snapped) pick point `p` (page space). If A was
    /// not yet set, this sets A and returns `None` (awaiting B). If A was set,
    /// this authors a [`DimensionKind::Linear`] with the RAW `b = p` and the
    /// current constraint (module docs), resets to awaiting-A, and returns it.
    pub fn commit_point(&mut self, p: Point) -> Option<DimensionKind> {
        match (self.first, self.second) {
            (None, _) => {
                self.first = Some(p);
                None
            }
            // Second pick: what is being measured is now known, but not where
            // the dimension goes. Enter the placing state rather than commit —
            // unless this pick does not need placing (a scale reference line),
            // in which case the second click still commits, exactly as before.
            (Some(a), None) => {
                if self.place_after {
                    self.second = Some(p);
                    None
                } else {
                    self.first = None;
                    Some(DimensionKind::Linear {
                        a,
                        b: p,
                        constraint: self.constraint,
                        offset: 0.0,
                        text_along: 0.0,
                    })
                }
            }
            // Third pick: where. The pointer resolves into the dimension's own
            // frame — perpendicular is the standoff, parallel is where the
            // number sits along the line — the two components of the placement
            // point SolidWorks' own API takes.
            (Some(a), Some(b)) => {
                let kind = self.placing_kind(a, b, p);
                self.first = None;
                self.second = None;
                Some(kind)
            }
        }
    }

    /// The dimension a placing click at `p` would author. Shared by
    /// [`Self::commit_point`] and [`Self::placing_preview`] so what the
    /// operator SEES while placing is definitionally what commits (R85).
    fn placing_kind(&self, a: Point, b: Point, p: Point) -> DimensionKind {
        let probe = DimensionKind::Linear {
            a,
            b,
            constraint: self.constraint,
            offset: 0.0,
            text_along: 0.0,
        };
        let (offset, text_along) = probe.placement_from_point(p).unwrap_or((0.0, 0.0));
        DimensionKind::Linear {
            a,
            b,
            constraint: self.constraint,
            offset,
            text_along,
        }
    }

    /// While placing, the dimension exactly as it would commit if the operator
    /// clicked at `p` right now — for the live preview.
    ///
    /// `None` unless both points are picked.
    #[must_use]
    pub fn placing_preview(&self, p: Point) -> Option<DimensionKind> {
        let (a, b) = (self.first?, self.second?);
        Some(self.placing_kind(a, b, p))
    }

    /// Whether the tool is waiting for the placing click.
    #[must_use]
    pub fn is_placing(&self) -> bool {
        self.first.is_some() && self.second.is_some()
    }

    /// A pick for a **reference line** rather than a ce dimension: two clicks,
    /// no placing step. Used by [`ScalePick`].
    #[must_use]
    pub fn reference_line() -> Self {
        Self {
            place_after: false,
            ..Self::new()
        }
    }

    /// Discard the in-progress first pick (Escape stage 1 / Reject, ui-spec
    /// §1.3): stay in the tool, forget point A.
    pub fn clear(&mut self) {
        self.first = None;
        self.second = None;
    }

    /// Whether a first point is placed (the tool is mid-gesture — a
    /// discardable [`crate::canvas::GestureInterrupt::Discard`]).
    #[must_use]
    pub fn in_progress(&self) -> bool {
        self.first.is_some()
    }

    /// The CONSTRAINED display segment `(a, projected_b)` for the live preview
    /// line (ui-spec §2.5), given the current raw pointer `raw`, or `None`
    /// while awaiting A. Display only — the authored `b` is the raw pick.
    #[must_use]
    pub fn preview_segment(&self, raw: Point) -> Option<(Point, Point)> {
        self.first
            .map(|a| (a, constrained_second_point(a, raw, self.constraint)))
    }

    /// The measured page-space length from A to the raw pointer under the
    /// current constraint (ui-spec §2.6 live readout), or `None` while
    /// awaiting A. `measured_length` uses the RAW second point (projecting
    /// first gives the identical result — `snap.rs` module docs).
    #[must_use]
    pub fn measured(&self, raw: Point) -> Option<f64> {
        self.first.map(|a| measured_length(a, raw, self.constraint))
    }
}

// ---------------------------------------------------------------------------
// Two-line pick — select two lines, pdfce reads what they mean (`Pass 68.0`)
// ---------------------------------------------------------------------------

/// Which geometry [`crate::canvas::CanvasTool::MeasureLinear`]'s next pick
/// targets (ui-spec `pass-68.0` §1/§2.2).
///
/// A real change in what a click MEANS: [`Self::Points`] resolves any snap
/// candidate anywhere on the page, while [`Self::TwoLines`] calls
/// [`pdfce_core::vector::linepick::pick_line_in_page`] and requires landing on
/// straight, already-drawn geometry — refusing curves and misses rather than
/// inventing a point.
///
/// # Why this is a mode and not a fourth tool
///
/// Because the operator declares it explicitly, in a control that is visible
/// the whole time the tool is armed. That is the test pass-46 §1.2 already
/// applied to `MarkupKind`'s ten kinds: a click's meaning may vary by mode, so
/// long as it never turns on state the operator cannot see. The full argument,
/// including why the `AddText`-vs-sub-mode precedent does NOT apply, is on
/// [`crate::canvas::CanvasTool::MeasureLinear`].
///
/// Switching mode discards any in-progress pick first — free, because nothing
/// has committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinearPickMode {
    /// The original: two snapped POINT picks author the measurement.
    #[default]
    Points,
    /// Two LINE picks; pdfce reads what they mean and authors accordingly.
    TwoLines,
}

/// The two-line ce-dimension pick: click one line, click another, and pdfce
/// authors the ce dimension the geometry calls for — a LINEAR one between two
/// parallel lines, an ANGULAR one between two that meet.
///
/// The operator's request, verbatim (2026-08-12): *"dimensioning tool should
/// allow the selection of two lines. if those lines are parallel it makes a
/// linear dimension between them like SolidWorks would, if they are at an
/// angle it makes an angle dimension."*
///
/// # ★ Nothing here classifies anything
///
/// This struct holds two picks and a checkbox. Every geometric question — are
/// these parallel, which of the four angles did the operator mean, is the apex
/// virtual, is the pair collinear — is answered by
/// [`pdfce_core::dimension::author_from_two_lines`], which is the same
/// function `pdfce-cli`'s `dimension-add --kind two-lines` calls.
///
/// That is deliberate and load-bearing. A second classifier living at the
/// canvas is exactly how the two shells acquire the disagreement
/// `Settings::parallel_epsilon_degrees` was introduced to prevent: the setting
/// centralises the *threshold*, and duplicating the code that consumes it
/// would reintroduce the divergence one level above the value that stops it.
/// The GUI owes a gesture and a disclosure surface, not a second reading of
/// the geometry.
///
/// # ★ Why this is NOT stored in `MeasureState::pending`
///
/// `pending` looks like the obvious home — it is already documented as "the
/// linear tool's completed-but-not-yet-authored dimension". It is the wrong
/// home, and the reason is a shipped piece of machinery that would break
/// silently. `PdfceApp::committable_gesture` reads:
///
/// ```text
/// let measure = doc.active_tool() == Some(CanvasTool::MeasureLinear)
///     && doc.measure.as_ref().is_some_and(|s| s.pending.is_some());
/// ```
///
/// That is decision 031's commit-on-interrupt path: a completed *two-point*
/// pick is safe to auto-commit when something else interrupts the gesture,
/// because nothing about it is inferred — it is exactly what the operator
/// clicked. The same function deliberately EXCLUDES the circular tool, whose
/// best fit is inferred.
///
/// A two-line verdict is inferred in precisely the circular sense:
/// parallel-vs-angled, which of four angles, whether the apex is virtual. Put
/// it in `pending` and that `is_some()` check — which cannot tell an ordinary
/// pick from an inference, having only ever asked whether the field was
/// populated — would quietly make it interrupt-committable, reopening for this
/// gesture exactly the hazard decision 031 closed. A sibling field keeps the
/// existing, already-tested rule correct without teaching it a new distinction.
///
/// # ★ Why the verdict is DERIVED on every read instead of cached
///
/// The ui-spec proposed a `verdict` field recomputed "whenever `second` or
/// `force_parallel` changes". This implementation deliberately has no such
/// field, and the reason is not a preference — it is that the proposed write
/// list was already incomplete by one when it was written. **The epsilon can
/// change too**: `Settings::parallel_epsilon_degrees` has a slider in the
/// settings panel, and moving it re-reads the same two lines into a different
/// answer (pinned by
/// [`tests::changing_the_epsilon_setting_re_reads_the_same_pair`]). A cache
/// listing two of its three producers is the failure mode already recorded in
/// `D:\dev\rag\egui\a_derived_value_with_one_producer_cannot_drift_a_cached_copy_with_n_producers_will.md`
/// — found in this very codebase on `recovery_note`, and fixed in `149fd03` by
/// deleting the cache rather than adding the missing reset site.
///
/// The recomputation is a few dozen floating-point operations on two stored
/// segments, so the cache buys nothing and costs a synchronisation obligation.
/// [`Self::authoring`] re-derives on every call: there is nothing to keep in
/// sync, and the verdict the operator reads is definitionally the one that
/// will commit (R85).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TwoLinePick {
    /// The first picked line; `None` while awaiting it.
    pub first: Option<PickedLine>,
    /// The second picked line; `None` until the pair is complete.
    pub second: Option<PickedLine>,
    /// The operator's **"treat these two lines as parallel"** override.
    ///
    /// Requested directly (2026-08-12): *"When making or editing a dimension
    /// of this type, there should be a checkbox option to treat the two lines
    /// as parallel."* It exists because the automatic reading is a GUESS and a
    /// global threshold cannot be right for every pair in a drawing — two
    /// nominally-parallel edges can arrive 0.8° apart from an exporter's
    /// rounding, and the operator, looking at the part, knows which.
    ///
    /// Ticking it never fakes the measurement: the true angle survives in
    /// [`pdfce_core::dimension::TwoLineAuthoring::measured_angle_degrees`] so
    /// the disclosure can state what was overridden.
    ///
    /// **Survives [`Self::clear`]**, like `snap_master` and the active group.
    /// The first instinct is the opposite — it is an assertion about two
    /// specific lines, so carrying it to the next pair looks like applying a
    /// claim the operator never made about them. What settles it is the
    /// failure the override was invented to avoid, stated in `linepick.rs`'s
    /// own docs: without it, the only remedy would be *"to change a global
    /// setting to author one dimension and change it back, which is how a
    /// setting becomes a thing people fight."* Resetting per pair recreates
    /// exactly that friction at smaller scale, for the operator dimensioning a
    /// whole drawing out of a sloppy exporter. It is safe to persist because
    /// the verdict — including the word "forced" and the true angle — is on
    /// screen before any Accept.
    pub force_parallel: bool,
}

impl TwoLinePick {
    /// A fresh pick, awaiting the first line, with the override off.
    ///
    /// Off is the honest default: the override is the operator asserting
    /// something about the drawing that pdfce's own reading disagrees with, so
    /// it cannot be on until they say so.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Offer a picked line to the gesture. Returns `true` if it was taken.
    ///
    /// The three cases, and why each is what it is:
    ///
    /// - **Awaiting line A** — take it.
    /// - **Awaiting line B** — take it; the pair is now complete and the
    ///   verdict is on screen for review.
    /// - **Pair complete** — depends on the verdict, which is why this needs
    ///   `epsilon_degrees`:
    ///   - a valid verdict ⇒ **ignored**, matching `pending`'s own documented
    ///     "further picks are ignored, the operator is reviewing" rule. An
    ///     inference under review must not be replaced by a stray click.
    ///   - a REFUSED pair (collinear, or a degenerate line) ⇒ the new line
    ///     **replaces line B**. That is the natural "try a different second
    ///     line" recovery, and it needs no special case beyond reassigning
    ///     `second`. Line A is kept, so recovering costs one click rather
    ///     than two.
    ///
    /// Returning `false` for the ignored case lets the caller leave the
    /// disclosure untouched rather than re-announcing an unchanged verdict.
    pub fn offer_line(&mut self, line: PickedLine, epsilon_degrees: f64) -> bool {
        match (self.first, self.second) {
            (None, _) => {
                self.first = Some(line);
                true
            }
            (Some(_), None) => {
                self.second = Some(line);
                true
            }
            (Some(_), Some(_)) => {
                // Only a refused pair yields to a new pick.
                let refused = matches!(
                    self.authoring(epsilon_degrees, TwoLinePlacement::default()),
                    Some(Err(_))
                );
                if refused {
                    self.second = Some(line);
                }
                refused
            }
        }
    }

    /// What the current pair authors, re-derived from scratch.
    ///
    /// `None` while the pair is incomplete. `Some(Err(..))` when the pair
    /// cannot yield a ce dimension — collinear, or a zero-length line — which
    /// the caller is expected to disclose by name rather than swallow.
    ///
    /// `epsilon_degrees` comes from `Settings::parallel_epsilon_degrees` and is
    /// never a literal at the call site, so this tool and the CLI cannot
    /// disagree about when two lines count as parallel.
    #[must_use]
    pub fn authoring(
        &self,
        epsilon_degrees: f64,
        placement: TwoLinePlacement,
    ) -> Option<Result<TwoLineAuthoring, TwoLineRefusal>> {
        let (a, b) = (self.first?, self.second?);
        let mut policy = ParallelPolicy::from_setting(epsilon_degrees);
        if self.force_parallel {
            policy = policy.forcing_parallel();
        }
        Some(author_from_two_lines(&a, &b, policy, placement))
    }

    /// Discard the in-progress pair (Escape stage 1 / Reject), staying in the
    /// tool.
    ///
    /// [`Self::force_parallel`] deliberately SURVIVES, mirroring how
    /// `snap_master` and the active group survive
    /// [`MeasureState::clear_gesture`] — see that field's own docs for why.
    pub fn clear(&mut self) {
        self.first = None;
        self.second = None;
    }

    /// Whether any part of a pair is picked — a discardable gesture.
    #[must_use]
    pub fn in_progress(&self) -> bool {
        self.first.is_some() || self.second.is_some()
    }

    /// Whether both lines are picked and a verdict is on screen for review.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.first.is_some() && self.second.is_some()
    }
}

/// How many chords approximate a previewed dimension arc.
///
/// Twenty-four over the full turn is smooth at any zoom pdfce offers, and an
/// angular ce dimension's wedge is a fraction of that — so the drawn arc is
/// visually smooth while staying a handful of segments.
const ARC_PREVIEW_STEPS: usize = 24;

/// The page-space line segments that draw a ce dimension's live preview.
///
/// # Why this is a function and not two painter loops
///
/// Two places preview a ce dimension before it is committed: the two-line
/// authoring gesture (previewing what Accept would author) and the placement
/// drag (previewing where a release would put it). They must draw the SAME
/// shape for the same kind, or the operator sees one thing while authoring and
/// a different thing while adjusting it.
///
/// Keeping it here rather than in `main.rs` also makes it testable — the arc
/// decomposition in particular has a wrap-around case (a wedge crossing ±π)
/// that is easy to get wrong and invisible until an operator picks the two
/// arms that trigger it.
///
/// Returns page-space pairs; the caller supplies the projection to screen.
/// Empty for [`DimensionKind::Circular`], which the ce-dimension preview does
/// not draw (the circular tool outlines its source objects instead).
#[must_use]
pub fn dimension_preview_segments(kind: &DimensionKind) -> Vec<(Point, Point)> {
    match *kind {
        DimensionKind::Linear { .. } => kind
            .linear_geometry()
            .map(|(dim_a, dim_b, ext_a, ext_b)| {
                vec![(dim_a, dim_b), (ext_a, dim_a), (ext_b, dim_b)]
            })
            .unwrap_or_default(),
        // Every segment, plus the closing one when the shape closes. The
        // preview draws from the same vertex list the committed ce dimension
        // will be authored from, so what the operator sees while picking is
        // what gets baked — the property `measure::pick`'s own preview relies
        // on for the linear tool.
        DimensionKind::Perimeter {
            ref points, closed, ..
        } => {
            let mut out: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
            if closed && let (Some(first), Some(last)) = (points.first(), points.last()) {
                out.push((*last, *first));
            }
            out
        }
        DimensionKind::Angular {
            apex,
            dir_a,
            dir_b,
            radius,
            ..
        } => {
            let start = dir_a.y.atan2(dir_a.x);
            // Take the SHORT way round between the two arms. Without this fold
            // a wedge whose arms straddle the ±π discontinuity would sweep the
            // long way and draw a reflex arc — the correct angle, illustrated
            // by the wrong picture.
            let mut sweep = dir_b.y.atan2(dir_b.x) - start;
            while sweep > std::f64::consts::PI {
                sweep -= std::f64::consts::TAU;
            }
            while sweep < -std::f64::consts::PI {
                sweep += std::f64::consts::TAU;
            }
            let at = |ang: f64| {
                Point::new(
                    radius.mul_add(ang.cos(), apex.x),
                    radius.mul_add(ang.sin(), apex.y),
                )
            };
            let mut out = Vec::with_capacity(ARC_PREVIEW_STEPS + 2);
            for step in 0..ARC_PREVIEW_STEPS {
                #[allow(clippy::cast_precision_loss)]
                let (t0, t1) = (
                    step as f64 / ARC_PREVIEW_STEPS as f64,
                    (step + 1) as f64 / ARC_PREVIEW_STEPS as f64,
                );
                out.push((at(sweep.mul_add(t0, start)), at(sweep.mul_add(t1, start))));
            }
            // A leg from the apex out along each arm. These matter most when
            // the apex is VIRTUAL: they are what shows the operator where
            // pdfce decided the two lines would meet.
            out.push((apex, at(start)));
            out.push((apex, at(start + sweep)));
            out
        }
        DimensionKind::Circular { .. } => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Circular pick — the tool's OWN best-fit pick-set (ui-spec §3)
// ---------------------------------------------------------------------------

/// One object toggled into the circular fit set: its object index (into the
/// page's `PageObjects`, for outlining the source + toggle bookkeeping) and
/// its page-space anchor sample points (the fit input, ui-spec §3.2/§3.3).
#[derive(Debug, Clone, PartialEq)]
struct CircObject {
    index: usize,
    samples: Vec<Point>,
}

/// The radius/diameter tool's own pick-set (ui-spec §3.1: deliberately NOT
/// `canvas_selection` — a circle-fit attempt has no meaning as the
/// substrate's general object selection). Objects are toggle-added; the fit
/// re-runs live on every change; Accept authors a [`DimensionKind::Circular`].
///
/// The display-only [`Self::show_diameter`] toggle (ui-spec §3.4) picks
/// radius vs. diameter on the SAME fit — never a second fit (decision 011's
/// value model: `diameter = 2×radius`, the same stored geometry).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CircularPick {
    /// The toggled objects, in pick order, each with its sample points.
    objects: Vec<CircObject>,
    /// Display toggle: `true` ⇒ show the diameter, `false` ⇒ the radius
    /// (ui-spec §3.4). Purely a display choice on the same [`FitCircle`].
    pub show_diameter: bool,
}

impl CircularPick {
    /// A fresh, empty pick-set showing the radius.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle object `index` (with its page-space anchor `samples`) into or
    /// out of the fit set (ui-spec §3.1/§3.2: a plain click toggle-adds/
    /// removes). Adding a circle-like object's own anchors, or several small
    /// line-segment objects' anchors, both feed the SAME fit. Returns `true`
    /// if the object is now IN the set, `false` if it was toggled out.
    pub fn toggle_object(&mut self, index: usize, samples: Vec<Point>) -> bool {
        if let Some(pos) = self.objects.iter().position(|o| o.index == index) {
            self.objects.remove(pos);
            false
        } else {
            self.objects.push(CircObject { index, samples });
            true
        }
    }

    /// The object indices currently in the fit set (for outlining the picked
    /// sources on the canvas).
    pub fn object_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.objects.iter().map(|o| o.index)
    }

    /// How many objects are in the fit set (the disclosure's "from N objects").
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// All picked sample points, concatenated in pick order — the exact input
    /// to [`fit_circle_taubin`] (ui-spec §3.3). The same point set the CLI
    /// would pass via `--points`, so the fit (and thus the authored kind) is
    /// byte-identical to the CLI's for the same picks.
    #[must_use]
    pub fn samples(&self) -> Vec<Point> {
        self.objects
            .iter()
            .flat_map(|o| o.samples.iter().copied())
            .collect()
    }

    /// The live best-fit circle over the current pick-set, or `None` for a
    /// degenerate set (< 3 usable points / numerically singular — ui-spec §3.3
    /// / `fit_circle_taubin`). Re-run every frame the set changes; the preview
    /// draws this dashed with its residual surfaced (ui-spec §3.4).
    #[must_use]
    pub fn fit(&self) -> Option<FitCircle> {
        fit_circle_taubin(&self.samples())
    }

    /// The [`DimensionKind::Circular`] this pick-set authors on Accept, or
    /// `None` when the fit is degenerate (Accept is disabled — fuzzy-never-
    /// sneaky, never auto-applied). Reuses [`Self::fit`]; `show_diameter` is
    /// the display toggle only.
    #[must_use]
    pub fn author(&self) -> Option<DimensionKind> {
        self.fit().map(|fit| DimensionKind::Circular {
            fit,
            show_diameter: self.show_diameter,
        })
    }

    /// Discard the whole pick-set (Escape stage 1 / Reject, ui-spec §1.3):
    /// stay in the tool, forget the picked objects. Keeps the display toggle.
    pub fn clear(&mut self) {
        self.objects.clear();
    }

    /// Whether any object is picked (the tool is mid-gesture, discardable).
    #[must_use]
    pub fn in_progress(&self) -> bool {
        !self.objects.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Scale entry — the two co-equal back-calc paths (ui-spec §4.2/§4.5)
// ---------------------------------------------------------------------------

/// The scale-entry sub-panel's working fields (ui-spec §4.2), shared by the
/// [`ScalePick`] dialog and the group-panel inline editor (ui-spec §5.2: ONE
/// scale-entry UI in the whole app). Two co-equal paths, one clearly
/// recommended:
///
/// - **Real length (recommended, default):** the operator typed the drawn
///   reference line's real length + unit; back-calc `scale = real /
///   drawn_pdf_length` — needs a drawn line, so it is offered only where one
///   exists ([`ScalePick`]).
/// - **Direct ratio:** `paper : real` on a disclosed paper-unit basis;
///   needs no drawn line, so it is the path the group panel uses to set a
///   scale by typing alone (ui-spec §7.2 accessibility win).
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleEntryFields {
    /// `true` ⇒ the real-length path is selected (the recommended default);
    /// `false` ⇒ the direct-ratio path (ui-spec §4.2 `selectable_value`).
    pub use_real_length: bool,
    /// The typed real-world length for the real-length path, in [`Self::unit`].
    ///
    /// Derived from [`Self::real_length_text`] whenever that parses; it is the
    /// number the scale maths actually uses. Kept as the parsed value rather
    /// than re-parsing at commit time so that what the operator was SHOWN in
    /// the preview is definitionally what gets committed.
    pub real_length: f64,
    /// What the operator literally typed for the real length.
    ///
    /// A text field, not a numeric spinner, because the whole point of the
    /// scale-by-known-dimension workflow is to type the dimension as the
    /// drawing writes it — `55 5/8"`, `4'-7 1/2"`. A spinner forced the
    /// operator to convert to a decimal and pick a unit by hand, which is two
    /// chances to enter a number that is plausible and wrong. Parsed by
    /// [`pdfce_core::dimension::parse_length`].
    pub real_length_text: String,
    /// The unit the real length is typed in / the ratio resolves to (becomes
    /// the group's top unit).
    pub unit: Unit,
    /// The paper side of the direct ratio (`1` in `1:100`).
    pub ratio_paper: f64,
    /// The real side of the direct ratio (`100` in `1:100`).
    pub ratio_real: f64,
    /// The paper-unit basis for the ratio path (default [`Unit::Inch`]; PDF
    /// paper units are 1/72", disclosed — ui-spec §4.2).
    pub basis: Unit,
    /// How the fractional part of every label in this group is displayed
    /// (Pass 25.5).
    ///
    /// `None` means "whatever the unit's default is" — the behaviour before
    /// this field existed, and still the right answer for an operator who
    /// never opens the display controls. `Some` is an explicit choice that
    /// must survive a unit change, which is why it is stored rather than
    /// re-derived from the unit each time.
    ///
    /// Exists because the operator asked for it directly: *"also want to be
    /// able to choose the units and display type - rounding, fraction, etc."*
    /// The unit was already selectable; the display type was hardcoded to
    /// `Unit::default_format()` at commit, so a drawing dimensioned in inches
    /// always read `55.63"` and could never read `55 5/8"` — the notation the
    /// drawing itself uses.
    pub fraction: Option<FractionMode>,
}

impl Default for ScaleEntryFields {
    fn default() -> Self {
        Self {
            // Real-length is the recommended, pre-selected path (ui-spec §4.2).
            use_real_length: true,
            real_length: 1.0,
            real_length_text: "1".to_owned(),
            unit: Unit::Meter,
            ratio_paper: 1.0,
            ratio_real: 100.0,
            basis: Unit::Inch,
            fraction: None,
        }
    }
}

impl ScaleEntryFields {
    /// Re-read [`Self::real_length_text`], updating the parsed value and
    /// (when the text named one) the unit.
    ///
    /// Returns the parse error for display, or `None` when it parsed. Called
    /// on every keystroke, so the operator sees what pdfce understood while
    /// they are still looking at the field rather than after committing.
    ///
    /// # Why a failed parse leaves the previous value alone
    ///
    /// Mid-typing, `55 5/` is not a length. Zeroing the value on every
    /// intermediate keystroke would make the live scale preview flicker
    /// through garbage, and — worse — would leave a *stale* preview looking
    /// authoritative if the operator stopped typing at that moment. Instead
    /// the last good value is held and the error is shown, so the preview and
    /// the message never disagree about whether the input is usable.
    ///
    /// # Why the unit dropdown moves only when the text names a unit
    ///
    /// Typing `55 5/8"` says inches; the dropdown should follow, or the
    /// operator has to say the same thing twice. Typing a bare `55.625` says
    /// nothing about units, and moving the dropdown then would be the tool
    /// second-guessing a choice the operator already made.
    pub fn sync_real_length(&mut self) -> Option<LengthParseError> {
        match parse_length(&self.real_length_text, self.unit) {
            Ok(p) => {
                self.real_length = p.value;
                if p.unit_from_text {
                    self.unit = p.unit;
                }
                None
            }
            Err(e) => Some(e),
        }
    }

    /// Fields seeded for a group-panel editor where NO reference line was
    /// drawn: the ratio path is the only usable one (the real-length path
    /// needs a drawn length), so it is pre-selected (ui-spec §7.2).
    #[must_use]
    pub fn for_group_panel() -> Self {
        Self {
            use_real_length: false,
            ..Self::default()
        }
    }

    /// The [`ScaleEntry`] these fields describe, given the optional drawn
    /// reference length `drawn_pdf_length` (points). Chooses the real-length
    /// path only when it is selected AND a drawn length is available; else the
    /// ratio path (which needs no line). This is what routes the group-panel
    /// (no line) path to Ratio and the [`ScalePick`] (line drawn) path to
    /// whichever the operator picked.
    #[must_use]
    pub fn entry(&self, drawn_pdf_length: Option<f64>) -> ScaleEntry {
        match (self.use_real_length, drawn_pdf_length) {
            (true, Some(drawn)) => ScaleEntry::RealLength {
                drawn_pdf_length: drawn,
                real_length: self.real_length,
                unit: self.unit,
            },
            _ => ScaleEntry::Ratio {
                paper: self.ratio_paper,
                real: self.ratio_real,
                basis: self.basis,
            },
        }
    }

    /// The live scale preview (ui-spec §4.2 "→ scale = 25.0 ft / 42.3 pt"),
    /// via the shipped [`preview_group_scale`] — pure, no mutation. `None`
    /// for a degenerate entry (Accept then shows nothing to commit).
    #[must_use]
    pub fn preview(&self, drawn_pdf_length: Option<f64>) -> Option<ScalePreview> {
        preview_group_scale(self.entry(drawn_pdf_length))
    }

    /// The `(ScaleState, NumberFormat)` this entry commits as, for
    /// `EditSession::set_group_scale` (ui-spec §4.4 re-propagation). The
    /// back-calculated scale becomes [`ScaleState::Calibrated`]; the format is
    /// the entry unit's default (a calibrated group is never "1:1" or
    /// "never-set" — the tri-state's third state). `None` for a degenerate
    /// entry.
    #[must_use]
    pub fn commit(&self, drawn_pdf_length: Option<f64>) -> Option<(ScaleState, NumberFormat)> {
        let preview = self.preview(drawn_pdf_length)?;
        // An explicit display choice wins over the unit's default, and
        // survives a unit change — an operator who asked for eighths does not
        // want them silently reverted by switching from inches to feet.
        let format = match self.fraction {
            Some(fraction) => NumberFormat {
                unit: preview.unit,
                fraction,
                // The marker follows the group's standard, set by
                // `set_group_standard`, not by this dialog.
                decimal_marker: preview.unit.default_format().decimal_marker,
            },
            None => preview.unit.default_format(),
        };
        Some((
            ScaleState::Calibrated {
                scale: preview.scale,
            },
            format,
        ))
    }
}

/// The scale-dimension tool's state (ui-spec §4.1): draw a reference line with
/// the SAME [`LinearPick`] mechanic as a linear dimension, then — once both
/// points are picked — switch to the scale-entry dialog ([`Self::fields`])
/// keyed on the drawn line's length.
#[derive(Debug, Clone, PartialEq)]
pub struct ScalePick {
    /// The reference-line two-point pick (reused verbatim from the linear
    /// tool, ui-spec §4.1 — including H/V/aligned + snapping).
    pub line: LinearPick,
    /// The drawn reference line's measured length (points) once both points
    /// are picked — `Some` switches the property bar to the scale-entry
    /// dialog (ui-spec §4.1). `None` while still drawing the line.
    pub drawn_pdf_length: Option<f64>,
    /// The scale-entry dialog's fields (both paths available here, since a
    /// line was drawn).
    pub fields: ScaleEntryFields,
}

impl Default for ScalePick {
    fn default() -> Self {
        Self::new()
    }
}

impl ScalePick {
    /// A fresh scale pick, awaiting the reference line's first point.
    #[must_use]
    pub fn new() -> Self {
        Self {
            line: LinearPick::reference_line(),
            drawn_pdf_length: None,
            fields: ScaleEntryFields::default(),
        }
    }

    /// Register a committed (snapped) reference-line pick `p`. While the
    /// dialog is open ([`Self::drawn_pdf_length`] is `Some`) further picks are
    /// ignored (the operator is typing the scale, ui-spec §4.1). Otherwise the
    /// pick advances the line; when the line completes, the drawn length is
    /// recorded and the dialog opens. Returns `true` when the dialog just
    /// opened.
    pub fn commit_point(&mut self, p: Point) -> bool {
        if self.drawn_pdf_length.is_some() {
            return false;
        }
        if let Some(kind) = self.line.commit_point(p) {
            // The measured length of the just-drawn reference line, under its
            // own H/V/aligned constraint (kind.measured_points()).
            self.drawn_pdf_length = Some(kind.measured_points());
            true
        } else {
            false
        }
    }

    /// Whether the scale-entry dialog is open (both reference points picked).
    #[must_use]
    pub fn dialog_open(&self) -> bool {
        self.drawn_pdf_length.is_some()
    }

    /// The live scale preview for the current dialog fields + drawn length
    /// (ui-spec §4.2), or `None` if the dialog is closed or the entry is
    /// degenerate.
    #[must_use]
    pub fn preview(&self) -> Option<ScalePreview> {
        self.drawn_pdf_length
            .and_then(|_| self.fields.preview(self.drawn_pdf_length))
    }

    /// The `(ScaleState, NumberFormat)` an Accept commits (ui-spec §4.4),
    /// or `None` while the dialog is closed / the entry is degenerate.
    #[must_use]
    pub fn commit(&self) -> Option<(ScaleState, NumberFormat)> {
        self.drawn_pdf_length
            .and_then(|_| self.fields.commit(self.drawn_pdf_length))
    }

    /// Discard the whole gesture (Escape stage 1 / Reject, ui-spec §1.3):
    /// forget the reference line and close the dialog, keeping the operator's
    /// typed dialog values (so a mis-drawn line is cheap to redo).
    pub fn clear(&mut self) {
        self.line.clear();
        self.drawn_pdf_length = None;
    }

    /// Whether a gesture is in progress (a point picked or the dialog open —
    /// a discardable gesture).
    #[must_use]
    pub fn in_progress(&self) -> bool {
        self.line.in_progress() || self.drawn_pdf_length.is_some()
    }
}

// ---------------------------------------------------------------------------
// Group-panel actions — each maps to exactly one shipped EditSession command
// ---------------------------------------------------------------------------

/// A group-panel action (ui-spec §5), each mapping onto **exactly one** shipped
/// `EditSession` command so the operator's mental model ("I made a group," "I
/// hid a layer") is one `Ctrl+Z` (ui-spec §5.4). Only the operations the
/// shipped 12.M2 engine exposes on `EditSession` are represented — create
/// (`add_dimension_group`), set-scale/units/format (`set_group_scale`), and
/// toggle-layer (`toggle_dimension_layer`). (Selecting the active authoring
/// group is pure view state — the property bar mutates
/// [`MeasureState::group`] directly, no engine call, so it is not an action
/// here.) (Rename/delete are not in the shipped `EditSession` surface and
/// are deliberately NOT reimplemented in the GUI — that would push sidecar-
/// rewriting logic out of core; they are a named follow-up, not this slice.)
#[derive(Debug, Clone, PartialEq)]
pub enum GroupAction {
    /// Create a new named group with `unit`'s default format →
    /// `EditSession::add_dimension_group(name, unit)`.
    Create {
        /// The operator-typed group name.
        name: String,
        /// The group's initial display unit.
        unit: Unit,
    },
    /// Set a group's scale + number format →
    /// `EditSession::set_group_scale(group, scale, format)` (re-propagates
    /// every member's baked `/AP`, ui-spec §4.4).
    SetScale {
        /// The group to recalibrate.
        group: GroupId,
        /// The tri-state scale to store.
        scale: ScaleState,
        /// The number format (carries the display unit + precision).
        format: NumberFormat,
    },
    /// Set a group's drafting standard →
    /// `EditSession::set_group_standard(group, standard)` (Pass 27.2;
    /// regenerates every member's baked `/AP`, exactly as a scale change
    /// does — a group exists so its members agree).
    SetStandard {
        /// The group to restyle.
        group: GroupId,
        /// ANSI or ISO.
        standard: DimStandard,
    },
    /// Toggle a group's optional-content layer visibility →
    /// `EditSession::toggle_dimension_layer(group, visible)` (the default
    /// group is un-hideable — the engine enforces it, ui-spec §5.3).
    ToggleLayer {
        /// The group whose layer to show/hide.
        group: GroupId,
        /// The requested visibility.
        visible: bool,
    },
}

/// The outcome of resolving a click on the active snap candidate
/// ([`MeasureState::resolve_click`], ui-spec §2.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClickOutcome {
    /// A derived-centerline candidate was PROMOTED by this (first) click — the
    /// disclosure now asks the operator to click again to confirm. Nothing was
    /// committed (fuzzy-never-sneaky, rule 4).
    Promoted,
    /// The pick COMMITS at this point (a routine/raw pick, or the confirming
    /// second click on a promoted derived candidate) — the tool advances its
    /// state machine with this page-space point.
    Commit(Point),
}

// ---------------------------------------------------------------------------
// The container tool state, built on tool entry
// ---------------------------------------------------------------------------

/// The measure tools' per-page canvas state, built on tool entry and torn down
/// on exit (mirroring `TextEditState`/`AddTextState`, canvas.rs §0.1). Holds
/// the three tools' pick state, the shared snap controls + active group, and
/// the last Accept's disclosures — all pure session/view state, never written
/// to `EditSession` before an explicit Accept (ui-spec §7.3 crash-safety).
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureState {
    /// The page this state targets (staleness key — the gesture is cleared on
    /// page navigation while the tool stays active, ui-spec §1.3).
    pub page_index: usize,
    /// The active authoring group the next dimension joins (ui-spec §2.6 group
    /// picker). Defaults to the always-present default group.
    pub group: GroupId,
    /// The persistent "Snap to content" master toggle (ui-spec §2.4), default
    /// ON. Off ⇒ every pick is the raw pointer position.
    pub snap_master: bool,
    /// The Tab-cycle index into the current snap candidate list (ui-spec §2.4).
    /// Reset to 0 on each new gesture / candidate-list change.
    pub snap_cycle: usize,
    /// The derived-centerline candidate point PROMOTED by a first click but not
    /// yet confirmed (ui-spec §2.3.1's proportional two-click confirm: the
    /// first click on a derived — fuzzy-inference — candidate only promotes it;
    /// a second click on the SAME point confirms it as the pick, never an
    /// auto-apply, rule 4). `None` when no derived candidate is mid-confirm.
    pub derived_promoted: Option<Point>,
    /// The linear-dimension pick (`MeasureLinear`).
    pub linear: LinearPick,
    /// The circular best-fit pick-set (`MeasureCircular`).
    pub circular: CircularPick,
    /// The scale-dimension pick + dialog (`MeasureScale`).
    pub scale: ScalePick,
    /// Which geometry `MeasureLinear`'s picks target (`Pass 68.0`).
    ///
    /// A Tool Options setting rather than part of the gesture, so clearing a
    /// gesture never silently reverts it — the same status `snap_master` and
    /// [`Self::group`] have.
    pub linear_pick_mode: LinearPickMode,
    /// The two-line pick (`MeasureLinear` in [`LinearPickMode::TwoLines`]).
    ///
    /// A sibling field rather than a use of [`Self::pending`], deliberately —
    /// see [`TwoLinePick`]'s own docs for the `committable_gesture` hazard
    /// that choice avoids.
    pub two_lines: TwoLinePick,
    /// The linear tool's completed-but-not-yet-authored dimension (ui-spec
    /// §2.1: the second click "commits point B and opens the value/group
    /// property bar" — authoring happens only on the explicit Accept, never on
    /// the click itself, fuzzy-never-sneaky). `Some` between the second click
    /// and Accept/Reject; while it is `Some`, further picks are ignored (the
    /// operator is reviewing). The circular/scale tools review live and do not
    /// use this (circular authors [`CircularPick::author`] at Accept, scale
    /// commits its dialog), so it is the linear tool's alone.
    pub pending: Option<DimensionKind>,
    /// The most recent ACCEPT's disclosures, rendered verbatim until the next
    /// Accept or tool exit (ui-spec §6, the standing verbatim-disclosure rule).
    pub last_disclosures: Vec<String>,
    /// The Tool Options pane asked to put the tool away (Pass 34.1 slice 3).
    ///
    /// The Close button draws in the LEFT DOCK, which egui resolves before the
    /// `CentralPanel` the canvas pass runs in — so this is set and consumed in
    /// the same frame. Drained by `run_measure_tool`; never crosses a frame
    /// boundary.
    pub queued_close_tool: bool,
    /// The Tool Options pane asked to open the ce-dimension Group Manager
    /// (Pass 34.1 slice 3). Same one-frame contract as
    /// [`Self::queued_close_tool`].
    pub queued_open_groups: bool,
    /// Commit / discard the current measure gesture, asked for by the Tool
    /// Options pane (Pass 34.1 slice 4). Same one-frame contract as the two
    /// above.
    pub queued_accept: bool,
    pub queued_reject: bool,
    /// The live pointer in PDF page space, cached by the canvas pass for the
    /// Tool Options pane's readout (Pass 34.1 slice 4).
    ///
    /// The pane draws before the pass that computes it, so it shows the
    /// previous frame's pointer. On a value that follows the mouse, one frame
    /// is invisible; deriving it in the pane instead is impossible, since the
    /// canvas transform lives with the canvas.
    pub derived_pointer: Option<Point>,
    /// Whether the snap candidate under the pointer is a DERIVED point (a
    /// midpoint, a centre) rather than one the drawing states — cached by the
    /// canvas pass for the Tool Options pane (Pass 34.1 slice 4).
    ///
    /// The pane uses it for the two-click-confirm disclosure (ui-spec §2.3):
    /// a derived point is pdfce's inference, so rule 4 requires it to be
    /// announced before it is picked, not after.
    pub derived_is_derived: bool,
}

impl MeasureState {
    /// Build fresh tool state for `page_index`, seeding the active group to the
    /// always-present default group (ui-spec §5.3), snapping ON.
    #[must_use]
    pub fn new(page_index: usize) -> Self {
        Self {
            page_index,
            group: pdfce_core::dimension::DEFAULT_GROUP_ID,
            snap_master: true,
            snap_cycle: 0,
            derived_promoted: None,
            linear: LinearPick::new(),
            circular: CircularPick::new(),
            scale: ScalePick::new(),
            linear_pick_mode: LinearPickMode::default(),
            two_lines: TwoLinePick::new(),
            pending: None,
            last_disclosures: Vec::new(),
            queued_close_tool: false,
            queued_open_groups: false,
            queued_accept: false,
            queued_reject: false,
            derived_pointer: None,
            derived_is_derived: false,
        }
    }

    /// Switch which geometry the linear tool picks, discarding any in-progress
    /// gesture if the mode actually changed (`Pass 68.0`).
    ///
    /// # Why this is a method and not two lines at the call site
    ///
    /// Because the discard is the load-bearing half and it is easy to omit. A
    /// half-finished point pick means nothing to the line gesture and vice
    /// versa, so carrying one across the switch would leave the tool holding
    /// state its current mode cannot interpret — and the failure would be
    /// invisible until the operator's next click produced something strange.
    /// Living here, it is unit-testable; living in `main.rs` it would not be
    /// (that file is a compile-and-launch shell by design).
    ///
    /// Discarding is free, because nothing has committed. The same rule
    /// `MarkupKind` follows on a kind change.
    ///
    /// A no-op when `mode` is already current — re-clicking the armed mode
    /// button must not silently throw away a pick in progress.
    pub fn set_linear_pick_mode(&mut self, mode: LinearPickMode) {
        if self.linear_pick_mode == mode {
            return;
        }
        self.linear_pick_mode = mode;
        self.linear.clear();
        self.two_lines.clear();
        self.pending = None;
    }

    /// Discard every in-progress gesture across the three tools (Escape stage 1
    /// / page navigation, ui-spec §1.3) and reset the snap cycle. Keeps the
    /// active group, snap toggle, and last disclosures.
    pub fn clear_gesture(&mut self) {
        self.linear.clear();
        self.circular.clear();
        self.scale.clear();
        self.two_lines.clear();
        self.pending = None;
        self.snap_cycle = 0;
        self.derived_promoted = None;
    }

    /// Resolve a click on the active snap candidate into an outcome (ui-spec
    /// §2.3): a routine candidate (or a raw, unsnapped pick) commits at once;
    /// a **derived-centerline** candidate needs the proportional two-click
    /// confirm — the first click PROMOTES it (returns [`ClickOutcome::Promoted`],
    /// nothing committed), a second click on the same point CONFIRMS it
    /// (returns [`ClickOutcome::Commit`]). `is_derived` is the active
    /// candidate's `SnapKind::is_derived()`; `point` is the (possibly snapped)
    /// pick point. This is the fuzzy-never-sneaky gate (rule 4) for the fuzzy
    /// inference, kept in one testable place.
    pub fn resolve_click(&mut self, point: Point, is_derived: bool) -> ClickOutcome {
        if is_derived && self.derived_promoted != Some(point) {
            self.derived_promoted = Some(point);
            ClickOutcome::Promoted
        } else {
            self.derived_promoted = None;
            ClickOutcome::Commit(point)
        }
    }

    /// Whether ANY tool has a discardable in-progress gesture (drives the
    /// two-stage Escape's stage-1 vs. stage-2 choice — ui-spec §1.3).
    #[must_use]
    pub fn gesture_in_progress(&self) -> bool {
        self.linear.in_progress()
            || self.circular.in_progress()
            || self.scale.in_progress()
            || self.two_lines.in_progress()
            || self.pending.is_some()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    // ---- display format (Pass 25.5) -------------------------------------

    /// A fields set that commits: real-length path, 100 units over a drawn
    /// line of 200 pt.
    fn calibrated(unit: Unit) -> ScaleEntryFields {
        ScaleEntryFields {
            use_real_length: true,
            real_length: 100.0,
            real_length_text: "100".to_owned(),
            unit,
            ..ScaleEntryFields::default()
        }
    }

    #[test]
    fn with_no_explicit_choice_the_units_default_format_is_used() {
        let f = calibrated(Unit::Inch);
        let (_scale, format) = f.commit(Some(200.0)).expect("commits");
        assert_eq!(
            format,
            Unit::Inch.default_format(),
            "an operator who never opens the display controls must get the \
             unchanged behaviour"
        );
    }

    /// **The operator's ask.** An explicit fraction choice reaches the format.
    ///
    /// Without this the display type was pinned to `Unit::default_format()`,
    /// so a drawing dimensioned in inches always read `55.63"` and could never
    /// read `55 5/8"` — the notation the drawing uses, and the notation the
    /// scale field already ACCEPTS as input.
    #[test]
    fn an_explicit_fraction_choice_is_what_commits() {
        let mut f = calibrated(Unit::Inch);
        f.fraction = Some(FractionMode::Fraction {
            denominator: 16,
            reduce: false,
        });
        let (_scale, format) = f.commit(Some(200.0)).expect("commits");
        assert_eq!(
            format.fraction,
            FractionMode::Fraction {
                denominator: 16,
                reduce: false
            }
        );
        assert_eq!(format.unit, Unit::Inch);
    }

    #[test]
    fn an_explicit_choice_survives_a_unit_change() {
        // Choosing eighths and then switching unit must not silently revert to
        // that unit's default notation — the operator asked for a notation,
        // not for a notation-on-this-unit.
        let mut f = calibrated(Unit::Inch);
        f.fraction = Some(FractionMode::Fraction {
            denominator: 8,
            reduce: true,
        });
        f.unit = Unit::FeetInches;
        let (_scale, format) = f.commit(Some(200.0)).expect("commits");
        assert_eq!(format.unit, Unit::FeetInches);
        assert_eq!(
            format.fraction,
            FractionMode::Fraction {
                denominator: 8,
                reduce: true
            }
        );
    }

    // ---- LinearPick A→B state machine (ui-spec §2.1) --------------------

    /// **Three clicks: what, to what, WHERE** (Pass 27.1).
    ///
    /// This test previously asserted that the SECOND click authored and reset.
    /// It changed because the operator asked for SolidWorks behaviour, and
    /// SolidWorks dimensions in three steps — the third is what says how far
    /// off the drawing the dimension sits. Committing on the second click
    /// meant every ce dimension landed on top of the geometry it measured,
    /// with a zero standoff, and had to be dragged clear afterwards.
    #[test]
    fn linear_pick_needs_a_third_placing_click_then_resets() {
        let mut lp = LinearPick::new();
        assert_eq!(lp.commit_point(p(10.0, 20.0)), None, "first: what");
        assert!(lp.in_progress());
        assert_eq!(lp.commit_point(p(50.0, 20.0)), None, "second: to what");
        assert!(lp.is_placing(), "both points known, awaiting placement");

        // Third: where. 15 above the measured line, and 5 right of its middle.
        let kind = lp.commit_point(p(35.0, 35.0)).unwrap();
        let DimensionKind::Linear {
            a,
            b,
            offset,
            text_along,
            ..
        } = kind
        else {
            panic!("expected a linear dimension")
        };
        assert_eq!((a, b), (p(10.0, 20.0), p(50.0, 20.0)), "the picks are kept");
        assert!(
            (offset - 15.0).abs() < 0.001,
            "the placing click's perpendicular component is the standoff, got {offset}"
        );
        assert!(
            (text_along - 5.0).abs() < 0.001,
            "and its parallel component is where the number sits, got {text_along}"
        );
        assert!(!lp.in_progress(), "the pick resets, ready for the next dim");
    }

    /// What is previewed while placing is what commits (R85).
    #[test]
    fn the_placing_preview_is_exactly_what_the_placing_click_authors() {
        let mut lp = LinearPick::new();
        lp.commit_point(p(10.0, 20.0));
        lp.commit_point(p(50.0, 20.0));
        let previewed = lp.placing_preview(p(35.0, 35.0)).expect("previewing");
        let committed = lp.commit_point(p(35.0, 35.0)).expect("commits");
        assert_eq!(
            previewed, committed,
            "the operator must not be shown one dimension and given another"
        );
    }

    /// A scale reference line still commits on the SECOND click.
    ///
    /// `ScalePick` reuses this state machine for a line that is never drawn as
    /// a dimension, so asking where to place it would be ceremony with no
    /// meaning. The opt-out is what keeps one state machine serving both.
    #[test]
    fn a_reference_line_pick_still_commits_on_the_second_click() {
        let mut lp = LinearPick::reference_line();
        assert_eq!(lp.commit_point(p(10.0, 20.0)), None);
        assert!(
            lp.commit_point(p(50.0, 20.0)).is_some(),
            "a reference line must not wait for a placing click"
        );
        assert!(!lp.in_progress());
    }

    #[test]
    fn linear_pick_stores_the_raw_second_point_even_under_hv_constraint() {
        // The byte-equivalence invariant: the STORED b is the raw pick, NOT
        // the constrained projection — the constraint is recorded alongside so
        // the measured length / appearance apply it (matching the CLI, module
        // docs). Only the PREVIEW segment is constrained.
        let mut lp = LinearPick::new();
        lp.constraint = AxisConstraint::Horizontal;
        lp.commit_point(p(10.0, 20.0));
        lp.commit_point(p(50.0, 80.0));
        // Placed on the measured line itself, so the placement is neutral and
        // this test stays about the STORED points.
        let kind = lp.commit_point(p(30.0, 20.0)).unwrap();
        let DimensionKind::Linear {
            a, b, constraint, ..
        } = kind
        else {
            panic!("expected a linear dimension")
        };
        assert_eq!(a, p(10.0, 20.0));
        assert_eq!(
            b,
            p(50.0, 80.0),
            "the stored b is the RAW pick, not (50,20)"
        );
        assert_eq!(constraint, AxisConstraint::Horizontal);
        // The measured value still honours the constraint (|Δx| = 40).
        assert_eq!(kind.measured_points(), 40.0);
    }

    #[test]
    fn linear_preview_segment_is_the_constrained_line_display_only() {
        let mut lp = LinearPick::new();
        lp.constraint = AxisConstraint::Horizontal;
        assert_eq!(lp.preview_segment(p(50.0, 80.0)), None); // awaiting A
        lp.commit_point(p(10.0, 20.0));
        // The preview projects the second point onto the page X axis (shares
        // A.y) — what you see is what's measured (ui-spec §2.5).
        assert_eq!(
            lp.preview_segment(p(50.0, 80.0)),
            Some((p(10.0, 20.0), p(50.0, 20.0)))
        );
        assert_eq!(lp.measured(p(50.0, 80.0)), Some(40.0));
    }

    #[test]
    fn linear_clear_discards_the_first_pick() {
        let mut lp = LinearPick::new();
        lp.commit_point(p(1.0, 1.0));
        assert!(lp.in_progress());
        lp.clear();
        assert!(!lp.in_progress());
    }

    /// **The canvas-authored == CLI-authored equivalence check (linear).** The
    /// GUI's `LinearPick` produces the IDENTICAL `DimensionKind` the CLI's
    /// `dimension-add` builds from the same two raw `--points` + constraint
    /// (`pdfce-cli` `main.rs`: `Linear { a: *a, b: *b, constraint }`). Identical
    /// kind ⇒ identical `EditSession::add_dimension` call ⇒ byte-identical
    /// additive output (same engine path — the acceptance gate).
    #[test]
    fn gui_linear_kind_equals_cli_linear_kind() {
        let a = p(72.0, 144.0);
        let b = p(216.0, 144.0);
        let constraint = AxisConstraint::Horizontal;

        // GUI path: two snapped picks, then the Pass 27.1 placing click.
        // Placed at the midpoint of the measured line, which is the NEUTRAL
        // placement — zero standoff, centred text — so this test still compares
        // the two paths' DEFAULTS rather than accidentally comparing a placed
        // dimension against an unplaced one.
        let mut lp = LinearPick::new();
        lp.constraint = constraint;
        lp.commit_point(a);
        lp.commit_point(b);
        let gui_kind = lp.commit_point(p(144.0, 144.0)).unwrap();

        // CLI path: the exact construction from pdfce-cli/src/main.rs with its
        // default --offset/--text-along.
        let cli_kind = DimensionKind::Linear {
            a,
            b,
            constraint,
            offset: 0.0,
            text_along: 0.0,
        };

        assert_eq!(gui_kind, cli_kind);
    }

    // ---- CircularPick fit-set → fit → author (ui-spec §3) ----------------

    #[test]
    fn circular_toggle_adds_then_removes_objects() {
        let mut cp = CircularPick::new();
        assert!(cp.toggle_object(3, vec![p(0.0, 0.0)]));
        assert!(cp.toggle_object(7, vec![p(1.0, 1.0)]));
        assert_eq!(cp.object_count(), 2);
        // Toggling an already-picked object removes it.
        assert!(!cp.toggle_object(3, vec![p(0.0, 0.0)]));
        assert_eq!(cp.object_count(), 1);
        assert_eq!(cp.object_indices().collect::<Vec<_>>(), vec![7]);
    }

    #[test]
    fn circular_fits_a_circle_from_picked_samples_and_authors_it() {
        // Four points on a unit circle centred at (5,5): a clean fit.
        let mut cp = CircularPick::new();
        cp.toggle_object(0, vec![p(6.0, 5.0), p(5.0, 6.0)]);
        cp.toggle_object(1, vec![p(4.0, 5.0), p(5.0, 4.0)]);
        let fit = cp.fit().expect("a 4-point circle fits");
        assert!((fit.center.x - 5.0).abs() < 1e-9);
        assert!((fit.center.y - 5.0).abs() < 1e-9);
        assert!((fit.radius - 1.0).abs() < 1e-9);
        // Author: radius by default.
        let kind = cp.author().unwrap();
        assert_eq!(
            kind,
            DimensionKind::Circular {
                fit,
                show_diameter: false
            }
        );
        assert_eq!(kind.measured_points(), 1.0);
        // Flip the display toggle → diameter (SAME fit, no re-fit).
        cp.show_diameter = true;
        let dia = cp.author().unwrap();
        assert!(matches!(
            dia,
            DimensionKind::Circular {
                show_diameter: true,
                ..
            }
        ));
        assert_eq!(dia.measured_points(), 2.0);
    }

    #[test]
    fn circular_degenerate_set_authors_nothing() {
        let mut cp = CircularPick::new();
        // Fewer than 3 points → no fit, nothing to accept (fuzzy-never-sneaky).
        cp.toggle_object(0, vec![p(0.0, 0.0), p(1.0, 0.0)]);
        assert!(cp.fit().is_none());
        assert!(cp.author().is_none());
    }

    /// **The canvas-authored == CLI-authored equivalence check (circular).**
    /// The GUI concatenates its picked objects' sample points and fits; the CLI
    /// fits the same `--points` vector. Same points ⇒ same `fit_circle_taubin`
    /// result ⇒ identical `DimensionKind::Circular` ⇒ byte-identical output.
    #[test]
    fn gui_circular_kind_equals_cli_circular_kind() {
        let pts = vec![p(10.0, 0.0), p(0.0, 10.0), p(-10.0, 0.0), p(0.0, -10.0)];

        // GUI path: the points arrive as two toggled objects.
        let mut cp = CircularPick::new();
        cp.toggle_object(0, vec![pts[0], pts[1]]);
        cp.toggle_object(1, vec![pts[2], pts[3]]);
        cp.show_diameter = true;
        let gui_kind = cp.author().unwrap();

        // CLI path: fit the same points, diameter display.
        let cli_kind = DimensionKind::Circular {
            fit: fit_circle_taubin(&pts).unwrap(),
            show_diameter: true,
        };

        assert_eq!(gui_kind, cli_kind);
    }

    // ---- Scale dialog back-calc plumbing (ui-spec §4) -------------------

    #[test]
    fn scale_pick_draws_a_line_then_opens_the_dialog() {
        let mut sp = ScalePick::new();
        assert!(!sp.dialog_open());
        // First reference point: no dialog yet.
        assert!(!sp.commit_point(p(0.0, 0.0)));
        assert!(!sp.dialog_open());
        // Second reference point: the dialog opens, drawn length recorded.
        assert!(sp.commit_point(p(42.3, 0.0)));
        assert!(sp.dialog_open());
        assert!((sp.drawn_pdf_length.unwrap() - 42.3).abs() < 1e-9);
        // Further picks are ignored while the dialog is open.
        assert!(!sp.commit_point(p(99.0, 99.0)));
    }

    #[test]
    fn scale_real_length_path_back_calcs_via_the_engine() {
        let mut sp = ScalePick::new();
        sp.commit_point(p(0.0, 0.0));
        sp.commit_point(p(42.3, 0.0)); // 42.3 pt reference line
        sp.fields.use_real_length = true;
        sp.fields.real_length = 25.0;
        sp.fields.unit = Unit::DecimalFeet;
        let preview = sp.preview().expect("a real-length preview");
        assert!((preview.scale - 25.0 / 42.3).abs() < 1e-12);
        assert_eq!(preview.unit, Unit::DecimalFeet);
        // Commit resolves to a Calibrated tri-state + the unit's default format.
        let (state, format) = sp.commit().unwrap();
        assert!(matches!(state, ScaleState::Calibrated { .. }));
        assert_eq!(format.unit, Unit::DecimalFeet);
    }

    #[test]
    fn scale_ratio_path_needs_no_drawn_line() {
        // The group-panel path: ratio entry with no reference line.
        let fields = ScaleEntryFields {
            use_real_length: false,
            ratio_paper: 1.0,
            ratio_real: 100.0,
            basis: Unit::Inch,
            ..ScaleEntryFields::default()
        };
        let preview = fields.preview(None).expect("a ratio preview with no line");
        assert!((preview.scale - 100.0 / 72.0).abs() < 1e-12);
        assert_eq!(preview.ratio_label, "1:100");
        // for_group_panel() pre-selects the ratio path.
        assert!(!ScaleEntryFields::for_group_panel().use_real_length);
    }

    #[test]
    fn scale_entry_routes_to_ratio_when_no_line_even_if_real_length_selected() {
        // Real-length selected but no drawn length → falls back to the ratio
        // entry (the only computable one), never a degenerate real-length call.
        let fields = ScaleEntryFields {
            use_real_length: true,
            ..ScaleEntryFields::default()
        };
        assert!(matches!(fields.entry(None), ScaleEntry::Ratio { .. }));
        assert!(matches!(
            fields.entry(Some(42.3)),
            ScaleEntry::RealLength { .. }
        ));
    }

    // ---- Group-panel action mapping (ui-spec §5.4) ----------------------

    #[test]
    fn group_actions_construct_the_expected_engine_intents() {
        // Each panel action names exactly one shipped EditSession command.
        let create = GroupAction::Create {
            name: "Floor Plan".to_owned(),
            unit: Unit::Meter,
        };
        assert_eq!(
            create,
            GroupAction::Create {
                name: "Floor Plan".to_owned(),
                unit: Unit::Meter
            }
        );
        let toggle = GroupAction::ToggleLayer {
            group: GroupId(2),
            visible: false,
        };
        assert!(matches!(
            toggle,
            GroupAction::ToggleLayer { visible: false, .. }
        ));
        let set = GroupAction::SetScale {
            group: GroupId(1),
            scale: ScaleState::OneToOne,
            format: Unit::Inch.default_format(),
        };
        assert!(matches!(set, GroupAction::SetScale { .. }));
    }

    // ---- Derived-centerline two-click confirm (ui-spec §2.3.1) ----------

    #[test]
    fn routine_pick_commits_immediately_derived_needs_two_clicks() {
        let mut st = MeasureState::new(0);
        // A routine (non-derived) pick commits on the first click.
        assert_eq!(
            st.resolve_click(p(10.0, 10.0), false),
            ClickOutcome::Commit(p(10.0, 10.0))
        );
        assert!(st.derived_promoted.is_none());
        // A derived candidate: first click only promotes (nothing committed —
        // fuzzy-never-sneaky), second click on the same point confirms.
        assert_eq!(st.resolve_click(p(5.0, 5.0), true), ClickOutcome::Promoted);
        assert_eq!(st.derived_promoted, Some(p(5.0, 5.0)));
        assert_eq!(
            st.resolve_click(p(5.0, 5.0), true),
            ClickOutcome::Commit(p(5.0, 5.0))
        );
        assert!(st.derived_promoted.is_none());
    }

    // ---- Container state (tool entry / teardown) ------------------------

    #[test]
    fn measure_state_starts_clean_and_clears_all_gestures() {
        let mut st = MeasureState::new(4);
        assert_eq!(st.page_index, 4);
        assert_eq!(st.group, pdfce_core::dimension::DEFAULT_GROUP_ID);
        assert!(st.snap_master);
        assert!(!st.gesture_in_progress());
        // A gesture in any tool marks the state in-progress...
        st.linear.commit_point(p(0.0, 0.0));
        st.circular.toggle_object(0, vec![p(1.0, 1.0)]);
        st.scale.commit_point(p(2.0, 2.0));
        assert!(st.gesture_in_progress());
        // ...and clear_gesture discards them all (Escape stage 1).
        st.clear_gesture();
        assert!(!st.gesture_in_progress());
        assert_eq!(st.snap_cycle, 0);
    }

    // ---- Two-line pick (`Pass 68.0`) ------------------------------------

    /// A picked line with its pick point at the midpoint.
    fn picked(sx: f64, sy: f64, ex: f64, ey: f64) -> PickedLine {
        PickedLine {
            target: pdfce_core::vector::HitTarget::Object(0),
            subpath: 0,
            segment: 0,
            start: p(sx, sy),
            end: p(ex, ey),
            pick: p(f64::midpoint(sx, ex), f64::midpoint(sy, ey)),
        }
    }

    const EPS: f64 = 0.5;

    #[test]
    fn two_line_pick_needs_both_lines_before_it_authors_anything() {
        let mut pick = TwoLinePick::new();
        assert!(!pick.in_progress());
        assert!(
            pick.authoring(EPS, TwoLinePlacement::default()).is_none(),
            "an empty pick authors nothing"
        );

        assert!(pick.offer_line(picked(100.0, 100.0, 300.0, 100.0), EPS));
        assert!(pick.in_progress() && !pick.is_complete());
        assert!(
            pick.authoring(EPS, TwoLinePlacement::default()).is_none(),
            "one line is not a pair"
        );

        assert!(pick.offer_line(picked(100.0, 140.0, 300.0, 140.0), EPS));
        assert!(pick.is_complete());
        assert!(pick.authoring(EPS, TwoLinePlacement::default()).is_some());
    }

    /// Two parallel edges author a LINEAR ce dimension of the gap.
    #[test]
    fn two_parallel_picks_author_a_linear_ce_dimension() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(100.0, 100.0, 300.0, 100.0), EPS);
        pick.offer_line(picked(100.0, 140.0, 300.0, 140.0), EPS);

        let authored = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(authored.is_linear());
        assert!(matches!(authored.kind, DimensionKind::Linear { .. }));
    }

    /// Two edges that meet author an ANGULAR ce dimension.
    #[test]
    fn two_angled_picks_author_an_angular_ce_dimension() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(100.0, 100.0, 300.0, 100.0), EPS);
        pick.offer_line(picked(100.0, 100.0, 100.0, 300.0), EPS);

        let authored = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(!authored.is_linear());
        assert!(matches!(authored.kind, DimensionKind::Angular { .. }));
    }

    /// A collinear pair surfaces the refusal rather than authoring a
    /// zero-length ce dimension the operator would go hunting for.
    #[test]
    fn a_collinear_pair_surfaces_the_refusal() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        pick.offer_line(picked(200.0, 0.0, 300.0, 0.0), EPS);

        let result = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("the pair is complete");
        assert_eq!(result, Err(TwoLineRefusal::Collinear));
    }

    /// ★ The whole reason the verdict is derived rather than cached: ticking
    /// the override AFTER both picks must change the answer immediately, with
    /// no re-pick and no cache to invalidate.
    #[test]
    fn ticking_the_override_after_both_picks_changes_the_verdict_immediately() {
        let t = 5f64.to_radians().tan();
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        pick.offer_line(picked(0.0, 40.0, 100.0, 100.0f64.mul_add(t, 40.0)), EPS);

        let before = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(!before.is_linear(), "5 degrees apart reads as angled");

        // The operator ticks the box while looking at that verdict.
        pick.force_parallel = true;

        let after = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(after.is_linear(), "the override must take effect at once");
        assert!(after.forced_parallel);
        // ...and the overridden angle is still available to disclose.
        let measured = after
            .measured_angle_degrees
            .expect("the true angle survives the override");
        assert!((measured - 5.0).abs() < 0.01, "got {measured}");
    }

    /// ★ Changing the epsilon SETTING re-reads the same two lines too — the
    /// second state transition a cached verdict would have had to chase.
    #[test]
    fn changing_the_epsilon_setting_re_reads_the_same_pair() {
        let t = 0.2f64.to_radians().tan();
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        pick.offer_line(picked(0.0, 20.0, 100.0, 100.0f64.mul_add(t, 20.0)), EPS);

        let loose = pick
            .authoring(0.5, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(loose.is_linear(), "0.2 degrees is parallel under 0.5");

        let strict = pick
            .authoring(0.1, TwoLinePlacement::default())
            .expect("complete")
            .expect("dimensionable");
        assert!(!strict.is_linear(), "the same pair is angled under 0.1");
    }

    /// ★ A third pick is IGNORED while a valid verdict is under review — an
    /// inference awaiting Accept must not be swapped out by a stray click.
    /// Mirrors `pending`'s own documented "further picks are ignored" rule.
    #[test]
    fn a_third_pick_is_ignored_while_a_valid_verdict_is_under_review() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        pick.offer_line(picked(0.0, 40.0, 100.0, 40.0), EPS);
        assert!(pick.is_complete());

        assert!(
            !pick.offer_line(picked(0.0, 80.0, 100.0, 80.0), EPS),
            "the pick must be refused while the operator is reviewing"
        );
        assert_eq!(
            pick.second.map(|l| l.start.y),
            Some(40.0),
            "line B must be untouched"
        );
    }

    /// ★ A REFUSED pair does yield: the new line replaces line B, so "try a
    /// different second line" costs one click and keeps line A.
    #[test]
    fn a_new_pick_replaces_line_b_when_the_pair_was_refused() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        // Collinear with A — refused.
        pick.offer_line(picked(200.0, 0.0, 300.0, 0.0), EPS);
        assert_eq!(
            pick.authoring(EPS, TwoLinePlacement::default()),
            Some(Err(TwoLineRefusal::Collinear))
        );

        assert!(
            pick.offer_line(picked(0.0, 40.0, 100.0, 40.0), EPS),
            "a refused pair must accept a replacement line B"
        );
        assert_eq!(
            pick.first.map(|l| l.start.y),
            Some(0.0),
            "line A is kept through the recovery"
        );
        let authored = pick
            .authoring(EPS, TwoLinePlacement::default())
            .expect("complete")
            .expect("the replacement pair is dimensionable");
        assert!(authored.is_linear());
    }

    // ---- Shared ce-dimension preview shape (`Pass 68.0`) ----------------

    /// A linear ce dimension previews as its dimension line plus two extension
    /// lines — three segments, the same three the placement drag draws.
    #[test]
    fn a_linear_ce_dimension_previews_as_line_plus_two_extensions() {
        let segs = dimension_preview_segments(&DimensionKind::Linear {
            a: p(0.0, 0.0),
            b: p(100.0, 0.0),
            constraint: AxisConstraint::Horizontal,
            offset: 20.0,
            text_along: 0.0,
        });
        assert_eq!(segs.len(), 3, "dimension line + two extension lines");
    }

    /// An angular ce dimension previews as an arc plus a leg out along each
    /// arm. The legs are what show where a VIRTUAL apex sits.
    #[test]
    fn an_angular_ce_dimension_previews_as_an_arc_plus_two_arm_legs() {
        let apex = p(50.0, 50.0);
        let radius = 30.0;
        let segs = dimension_preview_segments(&DimensionKind::Angular {
            apex,
            dir_a: p(1.0, 0.0),
            dir_b: p(0.0, 1.0),
            radius,
            text_along: 0.0,
        });
        assert_eq!(segs.len(), ARC_PREVIEW_STEPS + 2, "arc chords + two legs");

        // Every arc point sits on the circle of the stated radius about the
        // apex — the check that catches a centre/radius mix-up.
        for (a, _) in segs.iter().take(ARC_PREVIEW_STEPS) {
            let r = (a.x - apex.x).hypot(a.y - apex.y);
            assert!((r - radius).abs() < 1e-9, "off the arc: {a:?} r={r}");
        }
        // The two legs start at the apex.
        for (a, _) in segs.iter().skip(ARC_PREVIEW_STEPS) {
            assert!(
                (a.x - apex.x).abs() < 1e-9 && (a.y - apex.y).abs() < 1e-9,
                "a leg must start at the apex, got {a:?}"
            );
        }
    }

    /// ★ A wedge whose arms straddle the ±π discontinuity takes the SHORT way
    /// round. Without the fold the preview sweeps the long way and draws a
    /// reflex arc — the correct angle illustrated by the wrong picture, which
    /// no unit on the value side would catch.
    #[test]
    fn an_arc_spanning_the_angle_wraparound_takes_the_short_way() {
        const RADIUS: f64 = 10.0;
        let apex = p(0.0, 0.0);
        let at = |deg: f64| p(deg.to_radians().cos(), deg.to_radians().sin());
        let segs = dimension_preview_segments(&DimensionKind::Angular {
            apex,
            // +150° and -150°. The short way between them is 60°, crossing
            // ±180°; the long way is 300°. A naive subtraction gives -300 and
            // draws the reflex arc.
            dir_a: at(150.0),
            dir_b: at(-150.0),
            radius: RADIUS,
            text_along: 0.0,
        });
        // Summing the chord lengths approximates the arc length. 60° of a
        // radius-10 circle is ~10.47; the reflex 300° would be ~52.4.
        let arc_len: f64 = segs
            .iter()
            .take(ARC_PREVIEW_STEPS)
            .map(|(a, b)| (b.x - a.x).hypot(b.y - a.y))
            .sum();
        let short_way = RADIUS * 60f64.to_radians();
        assert!(
            (arc_len - short_way).abs() < 0.05,
            "expected the short way ({short_way:.2}), got {arc_len:.2} \
             (the reflex arc would be {:.2})",
            RADIUS * 300f64.to_radians()
        );
    }

    /// A circular ce dimension has no such preview — the circular tool
    /// outlines its source objects instead, and inventing an arc here would
    /// draw a shape that tool never commits.
    #[test]
    fn a_circular_ce_dimension_has_no_preview_segments() {
        let fit =
            pdfce_core::dimension::fit_circle_taubin(&[p(10.0, 0.0), p(0.0, 10.0), p(-10.0, 0.0)])
                .expect("fits");
        assert!(
            dimension_preview_segments(&DimensionKind::Circular {
                fit,
                show_diameter: false
            })
            .is_empty()
        );
    }

    /// ★ Switching pick mode discards whatever pick was in progress — the
    /// other mode cannot interpret it, and carrying it over would surface as
    /// a strange result on the operator's NEXT click rather than as an error.
    #[test]
    fn switching_pick_mode_discards_the_in_progress_gesture() {
        let mut st = MeasureState::new(0);
        st.linear.commit_point(p(10.0, 10.0));
        assert!(st.gesture_in_progress());

        st.set_linear_pick_mode(LinearPickMode::TwoLines);
        assert_eq!(st.linear_pick_mode, LinearPickMode::TwoLines);
        assert!(
            !st.gesture_in_progress(),
            "the half-finished point pick must not survive into line mode"
        );

        // ...and the same in the other direction.
        st.two_lines.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        assert!(st.gesture_in_progress());
        st.set_linear_pick_mode(LinearPickMode::Points);
        assert!(!st.gesture_in_progress());
    }

    /// Re-selecting the mode already armed is a no-op, not a silent discard.
    /// An operator clicking the button that is already lit has asked for
    /// nothing, and must not lose a pick for it.
    #[test]
    fn re_selecting_the_current_pick_mode_keeps_the_gesture() {
        let mut st = MeasureState::new(0);
        st.set_linear_pick_mode(LinearPickMode::TwoLines);
        st.two_lines.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);

        st.set_linear_pick_mode(LinearPickMode::TwoLines);
        assert!(
            st.two_lines.in_progress(),
            "re-clicking the armed mode must not discard the pick"
        );
    }

    /// ★ The override SURVIVES a clear, like `snap_master` and the group.
    ///
    /// The instinct is the opposite — it is an assertion about two specific
    /// lines. What settles it is the friction the override exists to remove:
    /// `linepick.rs` documents that without it the remedy would be changing a
    /// global setting per dimension, *"which is how a setting becomes a thing
    /// people fight"*. Resetting per pair recreates that at smaller scale for
    /// anyone dimensioning a whole drawing out of a sloppy exporter, and it is
    /// safe to persist because the verdict says "forced" before any Accept.
    #[test]
    fn the_override_survives_a_clear_like_the_other_tool_preferences() {
        let mut pick = TwoLinePick::new();
        pick.offer_line(picked(0.0, 0.0, 100.0, 0.0), EPS);
        pick.force_parallel = true;

        pick.clear();
        assert!(!pick.in_progress(), "the picks are discarded");
        assert!(
            pick.force_parallel,
            "the operator's override is a standing preference, not part of the gesture"
        );
    }
}
