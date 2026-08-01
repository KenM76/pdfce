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
    /// The stored geometry length in PDF points that the display value scales
    /// (`measured_points × scale`). Linear = the constrained length; Circular
    /// = radius, or `2×radius` when displaying diameter.
    #[must_use]
    pub fn measured_points(&self) -> f64 {
        match *self {
            DimensionKind::Linear { a, b, constraint } => measured_length(a, b, constraint),
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
