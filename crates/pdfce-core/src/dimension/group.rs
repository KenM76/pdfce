//! # Dimension groups + the authoritative model (decision 011 §2.4)
//!
//! The named-group model that carries per-group **scale + units + number
//! format + OCG layer + membership** — pdfce's *authoritative* dimensioning
//! state (the `/PieceInfo` sidecar; the portable `/Measure` dict is a
//! projection of it, decision 011 §2.4). It is pure data over the value model
//! ([`super::units`]) and the fit ([`super::fit`]); the storage layer
//! ([`super::sidecar`], [`super::measure_dict`]) serialises it and
//! [`crate::edit`] wires the resulting objects into a document.
//!
//! ## Non-geometric grouping — a genuine exceed-Acrobat (ui-spec §0.2)
//!
//! A [`Group`] is a *named* entity any dimension can join **regardless of its
//! page position**. Acrobat's own `/Viewport` scoping is a hard geometric
//! partition of the page; pdfce's authoritative group model is not, and this
//! is why the portable `/Measure` mirror can only ever be a best-effort
//! projection for overlapping different-scale groups on one page (decision
//! 011 §2.4 "native_scale") — the sidecar is the authority.
//!
//! ## A default group always exists (ui-spec §5.3)
//!
//! [`DimensionModel::new`] seeds exactly one group, [`DEFAULT_GROUP_ID`],
//! named "Default", scale never-set — so a dimension always has a home and the
//! group panel is never empty. The default group is un-hideable (hiding the
//! only fallback group is a foot-gun with no benefit — ui-spec §5.3).

use crate::object::ObjId;
use crate::vector::{AxisConstraint, Point, measured_length};

use super::fit::FitCircle;
use super::units::{MeasurementDisplay, NumberFormat, ScaleState, Unit, format_measurement};

/// A stable dimension-group identifier (index-independent, so the sidecar can
/// reference a group across a save/reload without depending on Vec order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u32);

/// A stable per-dimension identifier within the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionId(pub u32);

/// The always-present default/active group (ui-spec §5.3).
pub const DEFAULT_GROUP_ID: GroupId = GroupId(0);

/// A named dimension group: scale + units + number format + its optional-
/// content layer + default visibility (decision 011 §2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Stable identifier.
    pub id: GroupId,
    /// Operator-facing name (e.g. "Floor Plan").
    pub name: String,
    /// The tri-state scale (never-set / explicit-1:1 / calibrated) — ui-spec
    /// §4.3, so a legit 1:1 is distinct from uncalibrated.
    pub scale: ScaleState,
    /// The number format (carries the group's display [`Unit`]).
    pub format: NumberFormat,
    /// The group's Optional Content Group object, once authored into a
    /// document (`None` before the first dimension is saved).
    pub ocg: Option<ObjId>,
    /// The layer's **default visibility** (the `/D` config state): `true` ⇒
    /// the OCG is ON by default; `false` ⇒ registered in `/D /OFF` (hidden).
    pub visible: bool,
    /// The drafting standard this group's ce dimensions are DRAWN to
    /// (Pass 27.2): terminator form, whether the dimension line is broken for
    /// the text, text orientation, and whether the extension-line gap and
    /// overshoot are absolute or line-width-relative.
    ///
    /// # Why per group
    ///
    /// The group already owns every other display-governing property — scale,
    /// unit, number format, layer — and the standard is the same class of
    /// thing. Per ce dimension would be a foot-gun with no use case (nobody
    /// wants dimension #3 ISO and #4 ANSI); per document would mean inventing
    /// a document tier, and merge semantics for it, for one field. The unit
    /// interaction settles it: the standards' decimal conventions are
    /// unit-dependent, and the unit is per group.
    ///
    /// Changing it regenerates every member, exactly like a scale change — a
    /// group exists so its members agree.
    pub standard: DimStandard,
}

/// The drafting standard governing how ce dimensions are DRAWN (Pass 27.2).
///
/// Does not govern the numeric string, with one researched exception: ISO
/// 129-1:2018 cl. 4.1.1 mandates a comma decimal marker, which lives on
/// [`NumberFormat`] and is SET from this as a disclosed side effect rather
/// than implied by it.
///
/// **pdfce draws "ISO-style", never "ISO 129-1 conformant".** ISO 129-1's
/// normative Annex A (symbol proportions against lettering height, ISO 3098
/// class B) is paywalled and was not obtained, so conformance cannot honestly
/// be claimed — only the practice that was verifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DimStandard {
    /// ANSI/ASME practice — the factory default (operator, 2026-08-04: "My
    /// default is ANSI, but ISO should be an option too").
    ///
    /// Line/arrowhead and lettering conventions are **ASME Y14.2**, not
    /// Y14.5: Y14.5 is the GD&T/tolerancing standard and is routinely
    /// miscited for this. No clause number is given here because the document
    /// is paywalled and was not obtained.
    #[default]
    Ansi,
    /// ISO 129-1 practice.
    Iso,
}

impl Group {
    /// A fresh group with `unit`'s default number format, never-set scale, and
    /// default (visible) layer state.
    #[must_use]
    pub fn new(id: GroupId, name: impl Into<String>, unit: Unit) -> Self {
        Self {
            id,
            name: name.into(),
            scale: ScaleState::NeverSet,
            format: unit.default_format(),
            ocg: None,
            visible: true,
            standard: DimStandard::default(),
        }
    }

    /// The group's display unit (from its [`NumberFormat`]).
    #[must_use]
    pub const fn unit(&self) -> Unit {
        self.format.unit
    }
}

/// The immutable geometry a dimension stores (decision 011 §2.3 value model:
/// geometry is stored, the displayed value is derived). Radius vs diameter is
/// a *display-only* toggle on the same [`Self::Circular`] geometry (ui-spec
/// §1.1) — never a separate fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimensionKind {
    /// A linear dimension between two page-space points under an axis
    /// constraint. The stored measured length is
    /// `measured_length(a, b, constraint)` (points).
    Linear {
        /// First picked point (page space, points).
        a: Point,
        /// Second picked point (page space, points).
        b: Point,
        /// The alignment constraint (Aligned / Horizontal / Vertical).
        constraint: AxisConstraint,
        /// Signed standoff of the dimension line from point `a`, along the
        /// constraint's canonical normal, in points (Pass 27.0).
        ///
        /// # Why a scalar based at `a`, and why the default is exactly zero
        ///
        /// The dimension line does not run between the picked points — it runs
        /// PARALLEL to the constraint axis, offset away from the drawing, with
        /// extension (witness) lines reaching back to the points. This is the
        /// distance it stands off by. Positive is up (horizontal dimensions)
        /// or right (vertical ones); see [`DimensionKind::axis_frame`], whose
        /// normal is canonicalised so the sign does not depend on which point
        /// the operator clicked first.
        ///
        /// The default is **0.0**, which places the dimension line through `a`
        /// — reproducing exactly what the tool's own preview already draws. So
        /// an existing ce dimension deserialised without this key looks
        /// identical to how it looked before the field existed, and the
        /// sidecar migration costs nothing.
        offset: f64,
        /// Signed position of the value TEXT along the dimension line,
        /// measured from its midpoint, in points (Pass 27.1).
        ///
        /// # Why this is a second field and not folded into `offset`
        ///
        /// SolidWorks stores a dimension's placement as a POINT — its API
        /// takes one (`AddDimension2(x, y, z)`, "the text-placement point").
        /// In the dimension's own frame that point has two components: how far
        /// the line stands off the drawing (perpendicular, [`Self::Linear`]'s
        /// `offset`) and where the number sits along it (parallel, this). One
        /// drag sets both, which is what makes a SolidWorks dimension drag feel
        /// like moving an object rather than operating two controls.
        ///
        /// Keeping them as separate scalars rather than storing the raw point
        /// means neither can drift out of the frame when the picks or the
        /// constraint change — the point would have to be re-projected, and a
        /// re-projection that ever disagreed with the geometry would put the
        /// text somewhere the operator never dropped it.
        ///
        /// 0.0 is centred, which is where every dimension authored before this
        /// field existed puts its label.
        text_along: f64,
    },
    /// A radius/diameter dimension over a best-fit circle. Stores the fit
    /// (centre + radius + residual — the residual surfaced per decision 011
    /// §2.3) and whether the DISPLAY is diameter (`2×radius`) or radius.
    Circular {
        /// The fitted circle (page space, points).
        fit: FitCircle,
        /// `true` ⇒ display the diameter; `false` ⇒ the radius.
        show_diameter: bool,
    },
}

impl DimensionKind {
    /// The dimension line's own frame: the unit vector along the axis being
    /// measured, and the canonical normal the standoff is measured along
    /// (Pass 27.0).
    ///
    /// # Why this exists as its own function
    ///
    /// The constraint decides the direction the dimension line RUNS, and until
    /// Pass 27.0 nothing consumed that fact: `leader_endpoints` returned the
    /// two picked points verbatim, so a dimension constrained to Horizontal
    /// was drawn at whatever angle the operator's two clicks happened to make.
    /// The value was right and the line was wrong — the operator's report,
    /// 2026-08-04: *"It looks like it give me the correct horizontal or
    /// vertical dimension but it shows at an angle."*
    ///
    /// # The canonical normal, and why the sign must not depend on click order
    ///
    /// The normal is the axis rotated a quarter turn counter-clockwise, then
    /// flipped if necessary so it points up (or right, for a vertical
    /// dimension). Without that flip, clicking right-to-left instead of
    /// left-to-right would negate the normal, and the same positive standoff
    /// would put the dimension line on the opposite side of the drawing. An
    /// operator would experience that as the offset control working backwards
    /// half the time, for no reason they could see.
    ///
    /// Returns `None` for a degenerate `Aligned` dimension (two coincident
    /// picks), which has no axis to speak of — refused rather than fabricated.
    #[must_use]
    pub fn axis_frame(&self) -> Option<(Point, Point)> {
        let Self::Linear {
            a, b, constraint, ..
        } = *self
        else {
            return None;
        };
        let u = match constraint {
            AxisConstraint::Horizontal => Point::new(1.0, 0.0),
            AxisConstraint::Vertical => Point::new(0.0, 1.0),
            AxisConstraint::Aligned => {
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len = dx.hypot(dy);
                if !len.is_finite() || len <= f64::EPSILON {
                    return None;
                }
                Point::new(dx / len, dy / len)
            }
        };
        // Perpendicular, counter-clockwise, then canonicalised into the upper
        // half-plane (ties broken toward +x) so the sign of `offset` means the
        // same thing regardless of pick order.
        let mut n = Point::new(-u.y, u.x);
        if n.y < 0.0 || (n.y == 0.0 && n.x < 0.0) {
            n = Point::new(-n.x, -n.y);
        }
        Some((u, n))
    }

    /// Where the dimension LINE runs, and where each extension line reaches
    /// from — everything the appearance needs (Pass 27.0).
    ///
    /// Returns `(dim_a, dim_b, ext_from_a, ext_from_b)`: the two ends of the
    /// dimension line, and the two measured points the extension lines run to.
    ///
    /// # The invariant this exists to hold
    ///
    /// `|dim_b - dim_a|` equals [`Self::measured_points`], always. The drawn
    /// line is exactly as long as the number printed on it. Before Pass 27.0 a
    /// Horizontal dimension whose picks differed in y drew `hypot(dx, dy)`
    /// while its label said `dx` — a line that disagreed with its own caption,
    /// which is the specific thing an operator cannot be expected to catch.
    ///
    /// Where the value TEXT is anchored: the midpoint of the dimension line,
    /// slid along it by `text_along` (Pass 27.1).
    ///
    /// `None` for a non-linear or degenerate dimension.
    #[must_use]
    pub fn label_anchor(&self) -> Option<Point> {
        let Self::Linear { text_along, .. } = *self else {
            return None;
        };
        let (u, _) = self.axis_frame()?;
        let (dim_a, dim_b, _, _) = self.linear_geometry()?;
        let mid = dim_a.midpoint(dim_b);
        Some(Point::new(
            mid.x + text_along * u.x,
            mid.y + text_along * u.y,
        ))
    }

    /// Set the placement from a pointer position in page space — the
    /// SolidWorks drag, resolved into this dimension's own frame (Pass 27.1).
    ///
    /// Returns the `(offset, text_along)` pair the point implies. The picked
    /// points are NOT touched: in SolidWorks, dragging a dimension never
    /// re-measures it, it only decides where the dimension is drawn. That
    /// separation is what makes the gesture safe enough to be the default —
    /// it writes fields the value function does not read, so the number cannot
    /// change no matter where the operator drops it.
    ///
    /// `None` for a non-linear or degenerate dimension.
    #[must_use]
    pub fn placement_from_point(&self, p: Point) -> Option<(f64, f64)> {
        let Self::Linear { a, b, .. } = *self else {
            return None;
        };
        let (u, n) = self.axis_frame()?;
        // Perpendicular component, measured from `a` — the standoff.
        let offset = (p.x - a.x) * n.x + (p.y - a.y) * n.y;
        // Parallel component, measured from the MIDPOINT of the measured
        // extent, because that is where a centred label sits.
        let t = (b.x - a.x) * u.x + (b.y - a.y) * u.y;
        let along = ((p.x - a.x) * u.x + (p.y - a.y) * u.y) - t / 2.0;
        Some((offset, along))
    }

    /// `None` for a non-linear or degenerate dimension.
    #[must_use]
    pub fn linear_geometry(&self) -> Option<(Point, Point, Point, Point)> {
        let Self::Linear { a, b, offset, .. } = *self else {
            return None;
        };
        let (u, n) = self.axis_frame()?;
        // Projection of the pick-to-pick vector onto the axis: the measured
        // extent. Its SIGN matters — it decides which way the terminators
        // point — so it is not taken as an absolute here.
        let t = (b.x - a.x) * u.x + (b.y - a.y) * u.y;
        let o = Point::new(offset * n.x, offset * n.y);
        let dim_a = Point::new(a.x + o.x, a.y + o.y);
        let dim_b = Point::new(dim_a.x + t * u.x, dim_a.y + t * u.y);
        Some((dim_a, dim_b, a, b))
    }

    /// This geometry translated by a page-space `(dx, dy)`.
    ///
    /// # Why the measured value is untouched, on purpose
    ///
    /// A translation is a rigid motion: every distance it preserves. So a
    /// moved ce dimension reads exactly the same as before, and that is
    /// correct rather than a shortcut — moving a dimension repositions the
    /// annotation on the page, it does not re-measure anything. An operator
    /// who wants a different value is picking different points, which is a
    /// different operation.
    ///
    /// A `Circular` dimension moves its fitted CENTRE and keeps its radius,
    /// for the same reason; the residual is a property of the fit that
    /// produced it and is carried unchanged.
    #[must_use]
    pub fn translated(self, dx: f64, dy: f64) -> Self {
        match self {
            Self::Linear {
                a,
                b,
                constraint,
                offset,
                text_along,
            } => Self::Linear {
                a: Point::new(a.x + dx, a.y + dy),
                b: Point::new(b.x + dx, b.y + dy),
                constraint,
                // Both placement components are relative to the picked points,
                // which moved with the dimension, so a translation leaves them
                // untouched.
                offset,
                text_along,
            },
            Self::Circular { fit, show_diameter } => Self::Circular {
                fit: FitCircle {
                    center: Point::new(fit.center.x + dx, fit.center.y + dy),
                    ..fit
                },
                show_diameter,
            },
        }
    }

    /// The stored geometry length in PDF points that the display value scales
    /// (`measured_points × scale`). Linear = the constrained length; Circular
    /// = radius, or `2×radius` when displaying diameter.
    #[must_use]
    pub fn measured_points(&self) -> f64 {
        match *self {
            DimensionKind::Linear {
                a, b, constraint, ..
            } => measured_length(a, b, constraint),
            DimensionKind::Circular { fit, show_diameter } => {
                if show_diameter {
                    2.0 * fit.radius
                } else {
                    fit.radius
                }
            }
        }
    }

    /// A short label prefix a circular dimension's caption carries (`R`/`⌀`);
    /// empty for a linear dimension. ASCII-safe (`DIA` where a symbol font is
    /// not guaranteed) — the caption generator prepends it.
    #[must_use]
    pub const fn caption_prefix(&self) -> &'static str {
        match self {
            DimensionKind::Linear { .. } => "",
            DimensionKind::Circular { show_diameter, .. } => {
                if *show_diameter {
                    "DIA "
                } else {
                    "R "
                }
            }
        }
    }
}

/// One dimension in the model: its immutable geometry, its group, and its
/// in-document wiring handles (the annotation + appearance object numbers,
/// filled in once saved — so a scale change can find and regenerate each
/// member's baked `/AP`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionRecord {
    /// Stable identifier.
    pub id: DimensionId,
    /// The group this dimension belongs to.
    pub group: GroupId,
    /// The immutable geometry.
    pub kind: DimensionKind,
    /// The authored annotation object (`None` until wired into a document).
    pub annot: Option<ObjId>,
    /// The authored `/AP` `/N` appearance-stream object (`None` until wired).
    pub ap: Option<ObjId>,
}

/// The authoritative dimensioning model — the whole `/PieceInfo` sidecar's
/// logical content (decision 011 §2.4). Groups + dimensions + the id
/// counters. Serialised by [`super::sidecar`]; never depends on Vec order for
/// identity (uses [`GroupId`]/[`DimensionId`]).
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionModel {
    groups: Vec<Group>,
    dimensions: Vec<DimensionRecord>,
    next_group: u32,
    next_dim: u32,
}

impl Default for DimensionModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DimensionModel {
    /// A fresh model with the single always-present default group (ui-spec
    /// §5.3): "Default", millimetre unit, scale never-set, layer visible.
    #[must_use]
    pub fn new() -> Self {
        let default = Group::new(DEFAULT_GROUP_ID, "Default", Unit::Millimeter);
        Self {
            groups: vec![default],
            dimensions: Vec::new(),
            next_group: 1,
            next_dim: 0,
        }
    }

    /// An empty model with **no** groups — the reconstruction seed for the
    /// sidecar deserializer, which then re-inserts every stored group
    /// (including the default) with its exact id and fields. Not a normal
    /// entry point (a live model always has the default group — use
    /// [`Self::new`]).
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            groups: Vec::new(),
            dimensions: Vec::new(),
            next_group: 0,
            next_dim: 0,
        }
    }

    /// Every group, in insertion order (the default group first).
    #[must_use]
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// Every dimension, in insertion order.
    #[must_use]
    pub fn dimensions(&self) -> &[DimensionRecord] {
        &self.dimensions
    }

    /// The group with `id`, or `None`.
    #[must_use]
    pub fn group(&self, id: GroupId) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Mutable access to the group with `id`, or `None`.
    pub fn group_mut(&mut self, id: GroupId) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// The dimension with `id`, or `None`.
    #[must_use]
    pub fn dimension(&self, id: DimensionId) -> Option<&DimensionRecord> {
        self.dimensions.iter().find(|d| d.id == id)
    }

    /// Mutable access to the dimension with `id`.
    pub fn dimension_mut(&mut self, id: DimensionId) -> Option<&mut DimensionRecord> {
        self.dimensions.iter_mut().find(|d| d.id == id)
    }

    /// Create a new named group with `unit`'s default format, returning its
    /// fresh id. The group starts scale-never-set (ui-spec §4.3) and visible.
    pub fn add_group(&mut self, name: impl Into<String>, unit: Unit) -> GroupId {
        let id = GroupId(self.next_group);
        self.next_group += 1;
        self.groups.push(Group::new(id, name, unit));
        id
    }

    /// Insert a group with a caller-supplied id (used by the sidecar
    /// deserializer to reconstruct exact ids). Keeps `next_group` ahead of any
    /// inserted id. Does nothing if the id already exists.
    pub(crate) fn insert_group(&mut self, group: Group) {
        if self.groups.iter().any(|g| g.id == group.id) {
            return;
        }
        self.next_group = self.next_group.max(group.id.0 + 1);
        self.groups.push(group);
    }

    /// Set a group's scale state and number format in one step (the
    /// scale-entry / group-panel commit). No-op for an unknown group.
    pub fn set_group_scale(&mut self, id: GroupId, scale: ScaleState, format: NumberFormat) {
        if let Some(g) = self.group_mut(id) {
            g.scale = scale;
            g.format = format;
        }
    }

    /// Toggle (or set) a group's default layer visibility. The default group
    /// is un-hideable (ui-spec §5.3): a request to hide it is ignored.
    /// Returns the resulting visibility.
    pub fn set_group_visible(&mut self, id: GroupId, visible: bool) -> bool {
        if id == DEFAULT_GROUP_ID && !visible {
            return true; // un-hideable
        }
        if let Some(g) = self.group_mut(id) {
            g.visible = visible;
            g.visible
        } else {
            true
        }
    }

    /// Add a dimension to `group` (falling back to the default group if the id
    /// is unknown), returning its fresh id. Wiring handles start `None`.
    pub fn add_dimension(&mut self, group: GroupId, kind: DimensionKind) -> DimensionId {
        let group = if self.group(group).is_some() {
            group
        } else {
            DEFAULT_GROUP_ID
        };
        let id = DimensionId(self.next_dim);
        self.next_dim += 1;
        self.dimensions.push(DimensionRecord {
            id,
            group,
            kind,
            annot: None,
            ap: None,
        });
        id
    }

    /// Insert a dimension with a caller-supplied record (sidecar
    /// deserializer). Keeps `next_dim` ahead of the inserted id.
    pub(crate) fn insert_dimension(&mut self, record: DimensionRecord) {
        if self.dimensions.iter().any(|d| d.id == record.id) {
            return;
        }
        self.next_dim = self.next_dim.max(record.id.0 + 1);
        self.dimensions.push(record);
    }

    /// The dimensions belonging to `group`, in insertion order.
    pub fn members(&self, group: GroupId) -> impl Iterator<Item = &DimensionRecord> + '_ {
        self.dimensions.iter().filter(move |d| d.group == group)
    }

    /// How many dimensions belong to `group` (the count the group panel shows).
    #[must_use]
    pub fn member_count(&self, group: GroupId) -> usize {
        self.members(group).count()
    }

    /// The displayed value of a dimension under its group's scale + format —
    /// the derived value (decision 011 §2.3). `None` for an unknown dimension.
    #[must_use]
    pub fn display(&self, id: DimensionId) -> Option<MeasurementDisplay> {
        let d = self.dimension(id)?;
        let g = self.group(d.group)?;
        Some(format_measurement(
            d.kind.measured_points(),
            g.scale,
            g.format,
        ))
    }

    /// Delete `group`, reassigning its members to the default group (ui-spec
    /// §5.2: never a silent orphan). The default group cannot be deleted.
    /// Returns how many members were reassigned.
    /// Remove one dimension from the model, returning whether it was there.
    ///
    /// The record only — the caller owns removing the annotation objects it
    /// names. Keeping those two halves in one method would put document
    /// mutation inside the pure model, which is what keeps this type testable
    /// without a `Document` at all.
    ///
    /// The dimension's GROUP is deliberately left, even when this was its last
    /// member: a group is a named container carrying a scale the operator
    /// calibrated, and discarding that as a side effect of removing the last
    /// dimension would silently throw away work that is not cheap to redo.
    pub fn remove_dimension(&mut self, id: DimensionId) -> bool {
        let before = self.dimensions.len();
        self.dimensions.retain(|d| d.id != id);
        self.dimensions.len() != before
    }

    pub fn delete_group(&mut self, id: GroupId) -> usize {
        if id == DEFAULT_GROUP_ID {
            return 0;
        }
        let mut moved = 0;
        for d in &mut self.dimensions {
            if d.group == id {
                d.group = DEFAULT_GROUP_ID;
                moved += 1;
            }
        }
        self.groups.retain(|g| g.id != id);
        moved
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
    use crate::dimension::units::ScaleState;

    fn linear(a: (f64, f64), b: (f64, f64), c: AxisConstraint) -> DimensionKind {
        DimensionKind::Linear {
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            constraint: c,
            offset: 0.0,
            text_along: 0.0,
        }
    }

    #[test]
    fn a_fresh_model_has_exactly_the_default_group() {
        let m = DimensionModel::new();
        assert_eq!(m.groups().len(), 1);
        assert_eq!(m.groups()[0].id, DEFAULT_GROUP_ID);
        assert_eq!(m.groups()[0].name, "Default");
        assert!(m.groups()[0].scale.is_never_set());
    }

    #[test]
    fn measured_points_linear_and_circular() {
        let horiz = linear((10.0, 20.0), (50.0, 80.0), AxisConstraint::Horizontal);
        assert_eq!(horiz.measured_points(), 40.0);
        let aligned = linear((0.0, 0.0), (3.0, 4.0), AxisConstraint::Aligned);
        assert_eq!(aligned.measured_points(), 5.0);
        let circ = DimensionKind::Circular {
            fit: FitCircle {
                center: Point::new(0.0, 0.0),
                radius: 10.0,
                residual: 0.0,
            },
            show_diameter: false,
        };
        assert_eq!(circ.measured_points(), 10.0);
        let dia = DimensionKind::Circular {
            fit: FitCircle {
                center: Point::new(0.0, 0.0),
                radius: 10.0,
                residual: 0.0,
            },
            show_diameter: true,
        };
        assert_eq!(dia.measured_points(), 20.0);
        assert_eq!(dia.caption_prefix(), "DIA ");
    }

    #[test]
    fn scale_change_re_propagates_to_every_member() {
        // The decision-011 headline: change the group scale → all members
        // update. Two dimensions in one group; change the scale once; both
        // displayed values change.
        let mut m = DimensionModel::new();
        let g = m.add_group("Floor Plan", Unit::Meter);
        let d1 = m.add_dimension(
            g,
            linear((0.0, 0.0), (100.0, 0.0), AxisConstraint::Horizontal),
        );
        let d2 = m.add_dimension(
            g,
            linear((0.0, 0.0), (200.0, 0.0), AxisConstraint::Horizontal),
        );

        // Never-set → raw page units.
        assert!(m.display(d1).unwrap().raw_page_units);

        // Calibrate: 1 pt = 0.01 m.
        m.set_group_scale(
            g,
            ScaleState::Calibrated { scale: 0.01 },
            NumberFormat::decimal(Unit::Meter, 2),
        );
        assert_eq!(m.display(d1).unwrap().text, "1.00 m");
        assert_eq!(m.display(d2).unwrap().text, "2.00 m");

        // Re-scale once → BOTH members update.
        m.set_group_scale(
            g,
            ScaleState::Calibrated { scale: 0.05 },
            NumberFormat::decimal(Unit::Meter, 2),
        );
        assert_eq!(m.display(d1).unwrap().text, "5.00 m");
        assert_eq!(m.display(d2).unwrap().text, "10.00 m");
    }

    #[test]
    fn deleting_a_group_reassigns_members_never_orphans() {
        let mut m = DimensionModel::new();
        let g = m.add_group("Temp", Unit::Millimeter);
        let d = m.add_dimension(g, linear((0.0, 0.0), (10.0, 0.0), AxisConstraint::Aligned));
        assert_eq!(m.member_count(g), 1);
        let moved = m.delete_group(g);
        assert_eq!(moved, 1);
        assert!(m.group(g).is_none());
        // The dimension survived, now in the default group.
        assert_eq!(m.dimension(d).unwrap().group, DEFAULT_GROUP_ID);
        assert_eq!(m.member_count(DEFAULT_GROUP_ID), 1);
    }

    #[test]
    fn default_group_is_un_hideable_and_undeletable() {
        let mut m = DimensionModel::new();
        assert!(m.set_group_visible(DEFAULT_GROUP_ID, false));
        assert!(m.group(DEFAULT_GROUP_ID).unwrap().visible);
        assert_eq!(m.delete_group(DEFAULT_GROUP_ID), 0);
        assert!(m.group(DEFAULT_GROUP_ID).is_some());
    }

    #[test]
    fn adding_to_an_unknown_group_falls_back_to_default() {
        let mut m = DimensionModel::new();
        let d = m.add_dimension(
            GroupId(999),
            linear((0.0, 0.0), (1.0, 0.0), AxisConstraint::Aligned),
        );
        assert_eq!(m.dimension(d).unwrap().group, DEFAULT_GROUP_ID);
    }
}
