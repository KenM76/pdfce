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
//! ## What it deliberately does NOT do, and the one exception
//!
//! It does not time the **per-paint** path by wrapping it in
//! `Instant::now()` pairs. That loop runs 148,517 times over work of
//! well under a microsecond, so timer calls would be a large fraction of
//! the thing being measured.
//!
//! **Clip construction is the deliberate exception** (see
//! [`note_clip_phases`]). It runs 24,128 times over work averaging
//! ~350 µs, so a ~25 ns timer is ~1e-4 of the quantity — and unlike an
//! ablation, a direct timing has **no confound at all**: nothing is
//! removed, so nothing else changes. Ablation was the only honest tool
//! while the phases could not be timed; where they can be, it is the
//! weaker instrument, not the stronger one.
//!
//! **The perturbation was measured, not assumed, and the honest answer
//! is that it is below this machine's noise.** Three invocations of the
//! instrumented harness at 1× gave 9.49 / 9.52 / 10.04 s — a 5.8%
//! spread. The un-instrumented figure was 9.28 s, which is 2.2% from the
//! instrumented best and therefore *inside* that spread.
//!
//! So the claim is "not distinguishable from variance", **not** the
//! ~1e-4 the arithmetic above predicts. The prediction and the
//! measurement agree only in direction; the measurement is what stands.
//! Anyone re-checking should run the harness several times before
//! reading a single pair as overhead — one before/after pair here would
//! have shown "6% overhead" and been wrong.

/// One switchable cost centre in the rasterizer.
///
/// # What an ablation is FOR, and the trap it exists to close
///
/// Turning a cost centre off and re-rendering gives you a difference.
/// **That difference is an upper bound on what the centre costs, never
/// its value** — because removing one thing can remove others with it.
///
/// This is not a theoretical caveat. It is the single worst measurement
/// error of 2026-08-07: `Mask::new` was reported at **10.1 s of an 18 s
/// render** and it is **1.02 s**. The probe skipped [`Ablation::CLIP_BUILD`],
/// which does not only stop the mask being built — it leaves
/// `state.clip` at `None`, so every subsequent paint also skips mask
/// *sampling*, and every `q` skips the `Arc` clone. Three effects, one
/// number, all of it attributed to construction (**R164**).
///
/// So every variant here carries [`Ablation::confounds`], and the
/// consumer is expected to print it beside the number. A delta without
/// its confound is not a measurement.
///
/// # The FLOOR
///
/// With every centre off, what remains is content-stream interpretation
/// and path construction: the cost of *walking* the page. **No
/// rasterization change can go below that** without changing the
/// interpreter, which makes it the first number worth knowing before
/// scoping any render optimization — and the reason a standing-rule
/// candidate ("establish the floor by ablation before optimising") was
/// refused in favour of this artifact carrying it mechanically (R163).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ablation {
    /// Skip [`intersect_clip`](crate) entirely — no `Mask::new`, no
    /// `Mask::fill_path`, no multiply.
    pub clip_build: bool,
    /// Build the clip as normal but paint with `None` — isolates
    /// tiny-skia's per-pixel mask sampling from the cost of *making*
    /// the mask.
    pub clip_sample: bool,
    /// Skip `fill_path`/`stroke_path` on the page pixmap. Clip
    /// construction is unaffected (it fills into its own mask).
    pub paint: bool,
}

impl Ablation {
    /// Nothing suppressed — the ordinary render.
    pub const NONE: Self = Self {
        clip_build: false,
        clip_sample: false,
        paint: false,
    };
    /// Every centre off. What remains is the floor.
    pub const ALL: Self = Self {
        clip_build: true,
        clip_sample: true,
        paint: true,
    };

    /// True when nothing is suppressed.
    #[must_use]
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }

    /// Parse a comma-separated set: `clip-build`, `clip-sample`,
    /// `paint`, `all`, `none`. Returns `Err` with the offending token.
    ///
    /// # Errors
    ///
    /// Returns the unrecognised token, so a caller can reject a typo
    /// rather than silently measuring an un-ablated render and
    /// reporting it as ablated — which would produce a delta of zero
    /// and read as "this centre is free".
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut a = Self::NONE;
        for tok in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match tok {
                "clip-build" => a.clip_build = true,
                "clip-sample" => a.clip_sample = true,
                "paint" => a.paint = true,
                "all" => a = Self::ALL,
                "none" => a = Self::NONE,
                other => return Err(other.to_owned()),
            }
        }
        Ok(a)
    }

    /// Short label for a results table.
    #[must_use]
    pub fn label(&self) -> String {
        if self.is_none() {
            return "none".to_owned();
        }
        if *self == Self::ALL {
            return "ALL (floor)".to_owned();
        }
        let mut parts = Vec::new();
        if self.clip_build {
            parts.push("clip-build");
        }
        if self.clip_sample {
            parts.push("clip-sample");
        }
        if self.paint {
            parts.push("paint");
        }
        parts.join("+")
    }

    /// What this ablation suppresses **in addition to** its headline —
    /// the confounds that make its delta an upper bound rather than a
    /// value.
    ///
    /// Empty means the delta is attributable to the named centre alone.
    #[must_use]
    pub fn confounds(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.clip_build && !self.clip_sample {
            // The one that produced the 10.1 s error.
            v.push("clip sampling in every later paint (state.clip stays None)");
            v.push("the Arc clone in every q/Q");
        }
        if self.paint && !self.clip_sample {
            v.push("mask sampling for the paints that no longer happen");
        }
        v
    }

    /// True when the rendered output is no longer the correct picture.
    ///
    /// Every ablation makes it wrong; this exists so a consumer has to
    /// say so rather than let a screenshot escape.
    #[must_use]
    pub fn output_is_wrong(&self) -> bool {
        !self.is_none()
    }
}

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

    /// Nanoseconds in `Mask::new` — allocating and zeroing a page-sized
    /// byte-per-pixel buffer, once per clip.
    pub clip_new_ns: u64,
    /// Nanoseconds in `Mask::fill_path` — rasterizing the clip path into
    /// that buffer.
    pub clip_fill_ns: u64,
    /// Nanoseconds in the bounded multiply that intersects the new mask
    /// with the one already in force.
    pub clip_mul_ns: u64,

    /// Per-clip total nanoseconds, bucketed by magnitude.
    ///
    /// # Why a histogram and not just a mean
    ///
    /// 24,128 clips at a 350 µs *mean* could be 24,000 cheap clips plus
    /// 128 catastrophic ones, or a uniform population. **Those are
    /// different defects with different fixes**, and a mean cannot tell
    /// them apart — a tail is attacked by finding what makes those
    /// clips special, a uniform cost by changing the representation for
    /// all of them.
    ///
    /// Buckets are [`CLIP_BUCKET_EDGES_US`], upper-exclusive, last one
    /// unbounded.
    pub clip_hist: [u64; CLIP_BUCKETS],
}

/// Number of per-clip timing buckets.
pub const CLIP_BUCKETS: usize = 9;

/// Upper edges of [`Counters::clip_hist`] in microseconds; the final
/// bucket is everything above the last edge.
pub const CLIP_BUCKET_EDGES_US: [u64; CLIP_BUCKETS - 1] = [32, 64, 128, 256, 512, 1024, 2048, 4096];

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

    /// Total nanoseconds attributed to the three timed clip phases.
    #[must_use]
    pub fn clip_phase_ns(&self) -> u64 {
        self.clip_new_ns + self.clip_fill_ns + self.clip_mul_ns
    }

    /// The per-clip cost at the given percentile, in microseconds,
    /// resolved to the containing bucket's **upper edge**.
    ///
    /// Deliberately coarse and deliberately an upper bound: a histogram
    /// cannot give an exact percentile, and interpolating inside a
    /// bucket would invent precision the data does not carry. Returns
    /// `None` when no clips were recorded.
    #[must_use]
    pub fn clip_percentile_us(&self, pct: f64) -> Option<u64> {
        let total: u64 = self.clip_hist.iter().sum();
        if total == 0 {
            return None;
        }
        let target = (total as f64 * pct / 100.0).ceil() as u64;
        let mut seen = 0;
        for (i, n) in self.clip_hist.iter().enumerate() {
            seen += n;
            if seen >= target {
                return Some(CLIP_BUCKET_EDGES_US.get(i).copied().unwrap_or(u64::MAX));
            }
        }
        None
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
    pub(super) static CLIP_NEW_NS: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIP_FILL_NS: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIP_MUL_NS: AtomicU64 = AtomicU64::new(0);
    #[allow(
        clippy::declare_interior_mutable_const,
        reason = "array initialiser for statics"
    )]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    pub(super) static CLIP_HIST: [AtomicU64; super::CLIP_BUCKETS] = [ZERO; super::CLIP_BUCKETS];

    pub(super) fn snapshot() -> super::Counters {
        let mut hist = [0u64; super::CLIP_BUCKETS];
        for (dst, src) in hist.iter_mut().zip(CLIP_HIST.iter()) {
            *dst = src.load(Relaxed);
        }
        super::Counters {
            paints: PAINTS.load(Relaxed),
            paints_unclipped: PAINTS_UNCLIPPED.load(Relaxed),
            paints_cullable: PAINTS_CULLABLE.load(Relaxed),
            clips: CLIPS.load(Relaxed),
            clip_indiv_area_ppm: CLIP_INDIV.load(Relaxed),
            clip_accum_area_ppm: CLIP_ACCUM.load(Relaxed),
            clip_new_ns: CLIP_NEW_NS.load(Relaxed),
            clip_fill_ns: CLIP_FILL_NS.load(Relaxed),
            clip_mul_ns: CLIP_MUL_NS.load(Relaxed),
            clip_hist: hist,
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
            &CLIP_NEW_NS,
            &CLIP_FILL_NS,
            &CLIP_MUL_NS,
        ] {
            c.store(0, Relaxed);
        }
        for b in CLIP_HIST.iter() {
            b.store(0, Relaxed);
        }
    }

    /// Bit 0 `clip_build`, bit 1 `clip_sample`, bit 2 `paint`.
    ///
    /// A single atomic rather than three: the predicates are read once
    /// per paint in a 148,517-iteration loop, and one relaxed load that
    /// stays in a register beats three that do not.
    pub(super) static ABLATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

    pub(super) fn set_ablation(a: super::Ablation) {
        let bits =
            u8::from(a.clip_build) | (u8::from(a.clip_sample) << 1) | (u8::from(a.paint) << 2);
        ABLATE.store(bits, Relaxed);
    }

    pub(super) fn ablation() -> super::Ablation {
        let b = ABLATE.load(Relaxed);
        super::Ablation {
            clip_build: b & 1 != 0,
            clip_sample: b & 2 != 0,
            paint: b & 4 != 0,
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

/// Install an ablation set for subsequent renders.
///
/// **No-op without the `profile` feature**, so a shipping build cannot
/// be talked into rendering a wrong picture: the predicates below are
/// `const false` there and every guarded branch folds away.
#[cfg_attr(not(feature = "profile"), allow(unused_variables))]
pub fn set_ablation(a: Ablation) {
    #[cfg(feature = "profile")]
    imp::set_ablation(a);
}

/// The ablation set currently installed. Always [`Ablation::NONE`]
/// without the `profile` feature.
#[must_use]
pub fn ablation() -> Ablation {
    #[cfg(feature = "profile")]
    {
        imp::ablation()
    }
    #[cfg(not(feature = "profile"))]
    {
        Ablation::NONE
    }
}

/// Skip clip construction entirely.
///
/// **Reads `false` as a compile-time constant without the feature**, so
/// `if skip_clip_build() { return; }` leaves no branch in a shipping
/// build.
#[inline(always)]
pub(crate) fn skip_clip_build() -> bool {
    #[cfg(feature = "profile")]
    {
        imp::ABLATE.load(std::sync::atomic::Ordering::Relaxed) & 1 != 0
    }
    #[cfg(not(feature = "profile"))]
    {
        false
    }
}

/// Paint with no clip mask even though one was built.
#[inline(always)]
pub(crate) fn skip_clip_sample() -> bool {
    #[cfg(feature = "profile")]
    {
        imp::ABLATE.load(std::sync::atomic::Ordering::Relaxed) & 2 != 0
    }
    #[cfg(not(feature = "profile"))]
    {
        false
    }
}

/// Skip painting to the page pixmap.
#[inline(always)]
pub(crate) fn skip_paint() -> bool {
    #[cfg(feature = "profile")]
    {
        imp::ABLATE.load(std::sync::atomic::Ordering::Relaxed) & 4 != 0
    }
    #[cfg(not(feature = "profile"))]
    {
        false
    }
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

/// Record one clip's three timed phases, in nanoseconds.
///
/// # Why this times where [`note_paint`] does not
///
/// The module docs' objection to sub-phase timers is about the paint
/// loop: 148,517 iterations of sub-microsecond work, where a ~25 ns
/// timer is a large fraction of the quantity. Clip construction is the
/// opposite regime — 24,128 iterations averaging **~350 µs** — so the
/// same timer is ~1e-4 of what it measures.
///
/// And a direct timing is the **stronger** instrument here, not a
/// compromise: an ablation answers "what does the render cost without
/// this?", which removes other things with it and yields an upper bound
/// (**R164**). A timer answers "how long did this take?" with nothing
/// removed and therefore nothing confounded. Ablation was the only
/// honest tool while phases could not be timed individually.
///
/// No-op without the `profile` feature.
#[inline]
#[cfg_attr(not(feature = "profile"), allow(unused_variables))]
pub(crate) fn note_clip_phases(new_ns: u64, fill_ns: u64, mul_ns: u64) {
    #[cfg(feature = "profile")]
    {
        use std::sync::atomic::Ordering::Relaxed;
        imp::CLIP_NEW_NS.fetch_add(new_ns, Relaxed);
        imp::CLIP_FILL_NS.fetch_add(fill_ns, Relaxed);
        imp::CLIP_MUL_NS.fetch_add(mul_ns, Relaxed);

        let total_us = (new_ns + fill_ns + mul_ns) / 1_000;
        let bucket = CLIP_BUCKET_EDGES_US
            .iter()
            .position(|&e| total_us < e)
            .unwrap_or(CLIP_BUCKETS - 1);
        imp::CLIP_HIST[bucket].fetch_add(1, Relaxed);
    }
}

/// True when clip phases should be timed at all.
///
/// Reads as a compile-time `false` without the feature, so the
/// `Instant::now()` calls fold away entirely in a shipping build —
/// timing instrumentation must cost a shipping render nothing.
#[inline(always)]
pub(crate) fn timing_enabled() -> bool {
    cfg!(feature = "profile")
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

    /// `clip-build` must declare that it also kills clip SAMPLING.
    ///
    /// This is the day's worst measurement error encoded as a test.
    /// `Mask::new` was reported at 10.1 s of an 18 s render — it is
    /// 1.02 s — because the probe skipped clip construction and read the
    /// whole difference as construction cost, when it had also removed
    /// per-pixel mask sampling from every later paint and the `Arc`
    /// clone from every `q`.
    ///
    /// If this list is ever emptied, a consumer printing
    /// `confounds()` beside the delta shows nothing, and the number
    /// reads as attributable. That is precisely how it read the first
    /// time.
    #[test]
    fn clip_build_ablation_declares_the_sampling_confound() {
        let c = Ablation {
            clip_build: true,
            ..Ablation::NONE
        }
        .confounds();
        assert!(
            c.iter().any(|s| s.contains("sampling")),
            "clip-build suppresses mask sampling too and must say so; got {c:?}"
        );
        assert!(
            c.iter().any(|s| s.contains("q/Q")),
            "clip-build also skips the Arc clone in q/Q; got {c:?}"
        );
    }

    /// `clip-sample` alone has NO confound — that is the entire reason
    /// it exists as a separate switch.
    ///
    /// Construction still happens, so its delta is attributable to
    /// sampling. An empty confound list here is the tool's only honest
    /// route to a per-centre cost, and if this ever grows an entry the
    /// separation has been broken.
    #[test]
    fn clip_sample_ablation_is_attributable() {
        let c = Ablation {
            clip_sample: true,
            ..Ablation::NONE
        }
        .confounds();
        assert!(
            c.is_empty(),
            "clip-sample must isolate sampling with no side effects; got {c:?}"
        );
    }

    /// A typo must be REJECTED, not silently ignored.
    ///
    /// Ignoring it would run an un-ablated render, report a delta of
    /// zero, and read as "this cost centre is free" — a wrong answer
    /// that looks like a finding.
    #[test]
    fn an_unknown_ablation_token_is_an_error_not_a_no_op() {
        assert!(Ablation::parse("clip-buidl").is_err());
        assert_eq!(Ablation::parse("clip-buidl").unwrap_err(), "clip-buidl");
        assert_eq!(
            Ablation::parse("clip-build,paint").unwrap(),
            Ablation {
                clip_build: true,
                paint: true,
                clip_sample: false
            }
        );
        assert_eq!(Ablation::parse("all").unwrap(), Ablation::ALL);
    }

    /// **Without the `profile` feature, ablation cannot be turned on.**
    ///
    /// A shipping build must be unable to render a deliberately wrong
    /// picture, whatever it is asked to do. This test runs in BOTH
    /// configurations and asserts the appropriate one, so the guarantee
    /// is checked rather than assumed from the `cfg` blocks.
    #[test]
    fn a_shipping_build_cannot_be_ablated() {
        set_ablation(Ablation::ALL);
        let got = ablation();
        #[cfg(not(feature = "profile"))]
        assert!(
            got.is_none(),
            "without the profile feature, ablation must be inert; got {got:?}"
        );
        #[cfg(feature = "profile")]
        assert_eq!(got, Ablation::ALL);
        set_ablation(Ablation::NONE);
    }
}
