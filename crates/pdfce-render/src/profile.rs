//! # `profile` — feature-gated render instrumentation
//!
//! Counters the renderer feeds while rasterizing, so that claims about
//! *where the time goes* and *what the content looks like* can be
//! re-measured instead of remembered.
//!
//! **Compiled out entirely unless the `profile` feature is on.** Every
//! public function here is `#[inline]` and has an empty body without the
//! feature, so a shipping build carries no counter, no atomic, and no
//! branch. `tools/render-profile` is the intended consumer.
//!
//! ## Why this exists, which is not "profiling is nice to have"
//!
//! On 2026-08-07 three separate throwaway probes were written into
//! `interpret.rs` and deleted again within hours, and **two of them
//! produced figures that were wrong by two orders of magnitude**:
//!
//! 1. `Mask::new` was reported as 10.1 s of an 18 s render. It is
//!    1.02 s. The 10.1 s came from an ablation that skipped
//!    `intersect_clip` entirely — which also makes every `q` cheap and
//!    lets tiny-skia skip mask sampling. It measured construction plus
//!    use and attributed all of it to construction (**R164**: a phase
//!    verdict derived from an aggregate that moved more than the phase).
//! 2. Mean clip bounding box was reported as **0.663% of the page**. It
//!    is **66.36%** — a fraction printed as a percent. That single
//!    100× error is written into `intersect_clip`'s own doc comment as
//!    "clips in real drawings are SMALL relative to the paper", and it
//!    was the entire premise of a follow-on optimization that was
//!    scoped, dispatched, and only killed once the number was measured
//!    again.
//!
//! Both survived because **nothing standing could contradict them**. A
//! harness that has to be rewritten every session is one nobody runs,
//! and a number nobody can re-run is a number that ages into a fact.
//! That is the failure this module exists to make impossible.
//!
//! ## What it deliberately does NOT do
//!
//! It does not time sub-phases by wrapping them in `Instant::now()`
//! pairs. Timer calls inside a loop that runs 148,517 times perturb the
//! thing being measured, and the resulting per-phase numbers invite
//! exactly the subtract-two-totals reasoning that produced error (1).
//! **Counts and geometry are cheap and honest; timings belong at the
//! whole-render boundary**, where `tools/render-profile` takes them.

/// Everything the renderer reports about one rasterization.
///
/// Counts, not times — see the module docs on why timing lives at the
/// render boundary instead.
#[derive(Debug, Default, Clone, Copy)]
pub struct Counters {
    /// Paint operations issued (path fills, strokes, and glyph paints).
    pub paints: u64,
    /// Paints issued with no clip mask in force.
    pub paints_unclipped: u64,
    /// Clipped paints whose device bounds do not intersect the clip's
    /// bounding box — i.e. paints a bounding-box cull could skip.
    ///
    /// **Measured at 1.34% on the reference CAD sheet**, which is why no
    /// such cull was built. Kept as a counter so the next person to
    /// propose one gets the number instead of the intuition.
    pub paints_cullable: u64,
    /// `W`/`W*` clip operations applied.
    pub clips: u64,
    /// Sum over clips of (this clip path's device bbox area ÷ page area),
    /// in parts per million. Divide by [`Self::clips`] for the mean.
    pub clip_indiv_area_ppm: u64,
    /// Sum over clips of (the *accumulated* clip bbox area ÷ page area),
    /// in parts per million, after intersecting with the clip already in
    /// force.
    ///
    /// Separate from [`Self::clip_indiv_area_ppm`] deliberately: the
    /// accumulated figure is only correct if the bbox is saved and
    /// restored by `q`/`Q` exactly as the mask is. A probe that tracks it
    /// outside the graphics state shrinks monotonically, never widens on
    /// `Q`, and reports a clip far smaller than the real one — which is
    /// how a 1.34% cull rate first measured as 73.71%.
    pub clip_accum_area_ppm: u64,
}

impl Counters {
    /// Mean individual clip-path bbox, as a percentage of page area.
    #[must_use]
    pub fn mean_clip_indiv_pct(&self) -> f64 {
        if self.clips == 0 {
            return 0.0;
        }
        self.clip_indiv_area_ppm as f64 / self.clips as f64 / 10_000.0
    }

    /// Mean accumulated clip bbox, as a percentage of page area.
    #[must_use]
    pub fn mean_clip_accum_pct(&self) -> f64 {
        if self.clips == 0 {
            return 0.0;
        }
        self.clip_accum_area_ppm as f64 / self.clips as f64 / 10_000.0
    }

    /// Share of clipped paints a bounding-box cull could skip, as a
    /// percentage.
    #[must_use]
    pub fn cullable_pct(&self) -> f64 {
        let clipped = self.paints.saturating_sub(self.paints_unclipped);
        if clipped == 0 {
            return 0.0;
        }
        self.paints_cullable as f64 * 100.0 / clipped as f64
    }
}

#[cfg(feature = "profile")]
mod imp {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    pub(super) static PAINTS: AtomicU64 = AtomicU64::new(0);
    pub(super) static PAINTS_UNCLIPPED: AtomicU64 = AtomicU64::new(0);
    pub(super) static PAINTS_CULLABLE: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIPS: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIP_INDIV: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIP_ACCUM: AtomicU64 = AtomicU64::new(0);

    pub(super) fn snapshot() -> super::Counters {
        super::Counters {
            paints: PAINTS.load(Relaxed),
            paints_unclipped: PAINTS_UNCLIPPED.load(Relaxed),
            paints_cullable: PAINTS_CULLABLE.load(Relaxed),
            clips: CLIPS.load(Relaxed),
            clip_indiv_area_ppm: CLIP_INDIV.load(Relaxed),
            clip_accum_area_ppm: CLIP_ACCUM.load(Relaxed),
        }
    }

    pub(super) fn reset() {
        for c in [
            &PAINTS,
            &PAINTS_UNCLIPPED,
            &PAINTS_CULLABLE,
            &CLIPS,
            &CLIP_INDIV,
            &CLIP_ACCUM,
        ] {
            c.store(0, Relaxed);
        }
    }
}

/// Read the counters accumulated since the last [`reset`].
///
/// Returns all-zero without the `profile` feature.
#[must_use]
pub fn snapshot() -> Counters {
    #[cfg(feature = "profile")]
    {
        imp::snapshot()
    }
    #[cfg(not(feature = "profile"))]
    {
        Counters::default()
    }
}

/// Zero the counters. No-op without the `profile` feature.
pub fn reset() {
    #[cfg(feature = "profile")]
    imp::reset();
}

/// Record one paint. `cullable` is true when the paint's device bounds
/// miss the clip bbox entirely.
#[inline]
#[cfg_attr(not(feature = "profile"), allow(unused_variables))]
pub(crate) fn note_paint(clipped: bool, cullable: bool) {
    #[cfg(feature = "profile")]
    {
        use std::sync::atomic::Ordering::Relaxed;
        imp::PAINTS.fetch_add(1, Relaxed);
        if !clipped {
            imp::PAINTS_UNCLIPPED.fetch_add(1, Relaxed);
        } else if cullable {
            imp::PAINTS_CULLABLE.fetch_add(1, Relaxed);
        }
    }
}

/// Record one clip application, with both area fractions in `0.0..=1.0`.
#[inline]
#[cfg_attr(not(feature = "profile"), allow(unused_variables))]
pub(crate) fn note_clip(indiv_area_frac: f32, accum_area_frac: f32) {
    #[cfg(feature = "profile")]
    {
        use std::sync::atomic::Ordering::Relaxed;
        imp::CLIPS.fetch_add(1, Relaxed);
        imp::CLIP_INDIV.fetch_add((f64::from(indiv_area_frac) * 1e6) as u64, Relaxed);
        imp::CLIP_ACCUM.fetch_add((f64::from(accum_area_frac) * 1e6) as u64, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The percentage helpers must divide by 10,000 — ppm to percent —
    /// and not by 1,000,000.
    ///
    /// This test exists because that exact confusion, a fraction printed
    /// as a percent, produced a 100× wrong clip-size figure that was
    /// believed for hours and used to scope a whole optimization. The
    /// arithmetic is trivial; the consequence of getting it wrong was
    /// not.
    #[test]
    fn ppm_to_percent_conversion_is_not_off_by_a_hundred() {
        let c = Counters {
            clips: 2,
            // Two clips each covering exactly half the page: 500,000 ppm.
            clip_indiv_area_ppm: 1_000_000,
            clip_accum_area_ppm: 1_000_000,
            ..Counters::default()
        };
        assert!(
            (c.mean_clip_indiv_pct() - 50.0).abs() < 1e-9,
            "half a page must report as 50%, got {}",
            c.mean_clip_indiv_pct()
        );
        assert!((c.mean_clip_accum_pct() - 50.0).abs() < 1e-9);
    }

    /// `cullable_pct` is a share of CLIPPED paints, not of all paints —
    /// unclipped paints cannot be culled by a clip bbox and must not
    /// dilute the denominator.
    #[test]
    fn cullable_share_excludes_unclipped_paints() {
        let c = Counters {
            paints: 100,
            paints_unclipped: 50,
            paints_cullable: 25,
            ..Counters::default()
        };
        assert!(
            (c.cullable_pct() - 50.0).abs() < 1e-9,
            "25 of 50 clipped paints is 50%, not 25%; got {}",
            c.cullable_pct()
        );
    }

    /// Zero clips must not divide by zero.
    #[test]
    fn empty_counters_report_zero_rather_than_nan() {
        let c = Counters::default();
        assert_eq!(c.mean_clip_indiv_pct(), 0.0);
        assert_eq!(c.mean_clip_accum_pct(), 0.0);
        assert_eq!(c.cullable_pct(), 0.0);
    }
}
