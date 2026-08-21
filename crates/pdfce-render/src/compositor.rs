//! The transparency **compositor** — ISO 32000-1 §11.3.6, §11.4.4, §11.4.8.
//!
//! # Why this module exists
//!
//! Until `Pass 97.0` pdfce had no compositing arithmetic of its own. Every
//! per-pixel blend was delegated to `tiny_skia`, which composites **8-bit
//! premultiplied sRGB** with Porter-Duff over a **transparent-initialised**
//! buffer. That is a correct *rasteriser*; it is not the model clause 11
//! specifies, and three of its properties are load-bearing differences:
//!
//! 1. Clause 11 composites in the **group's** colour space, over a
//!    **non-transparent initial backdrop**, with a **backdrop-removal**
//!    correction on the way out (§11.4.4). A `Pixmap` carries one alpha
//!    channel and cannot represent `α` and `α_g` at once (§11.4 corpus §8).
//! 2. A **knockout** group composites each element against the group's
//!    *initial* backdrop, scaling the destination by `1 − f_s` rather than
//!    `1 − α_s` (§11.4.8). Shape and alpha must therefore stay separate.
//! 3. Where pdfce **already** computed a blend itself — the four
//!    non-separable modes, `Pass 85.4b` — it did so with a hand-written
//!    formula that assumed a **fully opaque backdrop**. See
//!    [`composite_element`]'s own notes: that assumption is what made
//!    Ghent's `1_GWG162` paint four cells solid white.
//!
//! This module is the single place the standard's formulas live, so the
//! three call sites that need them (the non-separable per-paint composite,
//! the non-separable group composite, and the group-result composite)
//! cannot drift apart.
//!
//! # What is here, and what deliberately is not
//!
//! Here: [`Union`](union_), the thirteen Table 136 separable blend
//! functions ([`blend_separable`]), the dispatcher over both tables
//! ([`Blend`]), the §11.4.4 element-composite formula
//! ([`composite_element`]), the §11.4.8 knockout variant
//! ([`composite_element_knockout`]), §11.4.4's backdrop removal
//! ([`remove_backdrop`]), and the two conversions between the standard's
//! un-premultiplied model and `tiny_skia`'s premultiplied storage.
//!
//! Not here: colour-space conversion (that is `crate::color`, and
//! §11.3.4's *subtractive complement* is `Pass 97.1`'s, because it needs a
//! colorant buffer to be meaningful); the four non-separable blend
//! functions themselves (they are `crate::blend_nonsep`, transcribed from
//! Table 137 with their own nine documented traps); and any buffer
//! management — this module is pure arithmetic over single pixels and
//! knows nothing about pixmaps beyond the two conversion helpers.
//!
//! # The one convention this module deliberately does NOT adopt
//!
//! **A fully transparent backdrop pixel is NOT white paper.** Three
//! functions in this crate used to say it was — `blend_nonsep::composite`,
//! `blend_nonsep::composite_layer` and `overprint::composite` — and
//! `crate::render_page`'s own page-group comment already explains at length
//! why it is wrong: §11.4.7 makes the page an **isolated** group whose
//! buffer starts transparent, with the white medium composited in **once,
//! at the end**. Handing a blend function a backdrop of `1.0` is only
//! harmless for the four modes satisfying `B(1.0, c_s) = c_s`
//! (`Normal`, `Compatible`, `Multiply`, `Darken`). For the other eleven it
//! is visibly wrong, and for `Hue`/`Saturation`/`Color` it is catastrophic:
//! `Sat(white) = 0` and `Lum(white) = 1`, so the blend of *anything* over
//! white is white.
//!
//! §11.4.4's formula needs no such convention, because `α_b` appears
//! explicitly: at `α_b = 0` the blend term is multiplied by zero and the
//! result is the source colour, which is the correct answer and is what
//! [`composite_element`] computes without a special case.

use tiny_skia::PremultipliedColorU8;

use crate::blend_nonsep::NonSeparableBlend;

/// §11.3.7.3's `Union` — the "either" of two shape or alpha values.
///
/// ```text
/// Union(b, s) = 1 − [(1 − b) × (1 − s)] = b + s − (b × s)
/// ```
///
/// Named with a trailing underscore because `union` is a reserved word in
/// Rust 2024 only as a contextual keyword for `union` types; the underscore
/// removes any question at the call site.
///
/// # Why `max` is not a substitute
///
/// `max(0.5, 0.5) = 0.5`; `Union(0.5, 0.5) = 0.75`. Two half-covering
/// elements leave a quarter of the pixel uncovered, not a half. Using
/// `max` under-reports accumulated alpha everywhere two partially
/// transparent things overlap, which is every anti-aliased edge in a
/// transparency group.
#[must_use]
#[inline]
pub fn union_(b: f32, s: f32) -> f32 {
    b.mul_add(-s, b + s)
}

/// A blend mode, over **both** of clause 11.3.5's tables.
///
/// # Why one enum rather than `tiny_skia::BlendMode`
///
/// Because this module must be able to name a blend that `tiny_skia`
/// cannot compute correctly. `crate::gstate::blend_mode_from_name` returns
/// a `tiny_skia::BlendMode` and deliberately refuses the four
/// non-separable modes (decision 066: the dependency's `clip_color` gates
/// its low-gamut rescale on the wrong comparison and is measurably wrong by
/// up to 107/255). A compositor that owns the arithmetic needs a type that
/// can carry all seventeen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Table 136's `Normal` — and its PDF 1.3 alias `Compatible`, which
    /// §11.3.5 defines as exactly source-over.
    Normal,
    /// `B(cb, cs) = cb × cs`.
    Multiply,
    /// `B(cb, cs) = cb + cs − (cb × cs)`.
    Screen,
    /// `HardLight(cs, cb)` — the arguments are **swapped**, §11.3.5 §3.1.
    Overlay,
    /// `min(cb, cs)`.
    Darken,
    /// `max(cb, cs)`.
    Lighten,
    /// `min(1, cb / (1 − cs))` if `cs < 1`, else `1`.
    ColorDodge,
    /// `1 − min(1, (1 − cb) / cs)` if `cs > 0`, else `0`.
    ColorBurn,
    /// `Multiply(cb, 2cs)` if `cs ≤ 0.5`, else `Screen(cb, 2cs − 1)`.
    HardLight,
    /// The two-branch soft light with `D(x)`.
    SoftLight,
    /// `|cb − cs|` — note the **absolute value**; ISO 32000-1's printed
    /// table drops the bars (corpus erratum `GD-4`).
    Difference,
    /// `cb + cs − 2 × cb × cs`.
    Exclusion,
    /// One of Table 137's four, computed by [`crate::blend_nonsep`].
    NonSeparable(NonSeparableBlend),
}

impl Blend {
    /// Map a `tiny_skia::BlendMode` that `blend_mode_from_name` produced
    /// back onto this enum.
    ///
    /// # Why this direction exists
    ///
    /// The graphics state stores the rasteriser's type (it has to — that is
    /// what the ordinary paint path hands to `tiny_skia`). The compositor
    /// needs the standard's type. Rather than storing both and letting them
    /// disagree, the conversion happens at the one boundary that needs it.
    ///
    /// Returns `None` for a mode `blend_mode_from_name` never produces —
    /// `tiny_skia::BlendMode` has thirty-odd Porter-Duff operators and only
    /// the twelve below are reachable from a PDF `/BM` name.
    #[must_use]
    pub fn from_tiny_skia(mode: tiny_skia::BlendMode) -> Option<Self> {
        use tiny_skia::BlendMode as B;
        Some(match mode {
            B::SourceOver => Self::Normal,
            B::Multiply => Self::Multiply,
            B::Screen => Self::Screen,
            B::Overlay => Self::Overlay,
            B::Darken => Self::Darken,
            B::Lighten => Self::Lighten,
            B::ColorDodge => Self::ColorDodge,
            B::ColorBurn => Self::ColorBurn,
            B::HardLight => Self::HardLight,
            B::SoftLight => Self::SoftLight,
            B::Difference => Self::Difference,
            B::Exclusion => Self::Exclusion,
            _ => return None,
        })
    }

    /// `true` when `B(cb, cs) = cs` for every `cb` — i.e. the mode is
    /// indistinguishable from `Normal` and the whole blend can be skipped.
    #[must_use]
    pub fn is_normal(self) -> bool {
        self == Self::Normal
    }

    /// Evaluate `B(C_b, C_s)` for this mode.
    ///
    /// Both arguments are **un-premultiplied, additive** components in
    /// `[0, 1]` — §11.3.4's requirement. A subtractive (CMYK/Separation/
    /// DeviceN) space must complement its components before calling this
    /// and complement the result back; that is `Pass 97.1`'s work and is
    /// deliberately **not** done here, because doing it in an sRGB buffer
    /// would be a second wrong answer rather than a first right one.
    #[must_use]
    pub fn apply(self, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Normal => cs,
            Self::NonSeparable(m) => crate::blend_nonsep::blend(m, cb, cs),
            _ => [
                blend_separable(self, cb[0], cs[0]),
                blend_separable(self, cb[1], cs[1]),
                blend_separable(self, cb[2], cs[2]),
            ],
        }
    }
}

/// §11.3.5.2 Table 136 — one component of one separable blend mode.
///
/// Transcribed from `iso32000__s__11.3.5.md` §3, **including its four
/// recorded printing errata** (`GD-1`…`GD-4`), each of which is
/// independently corroborated there by ISO 32000-2:2020 and by W3C
/// Compositing Level 1:
///
/// * `GD-1` — `ColorDodge`'s second branch is `1` **if `cs = 1`**.
/// * `GD-2` — `ColorBurn`'s second branch is `0` **if `cs = 0`**.
/// * `GD-3` — `SoftLight`'s `D(x)` is `((16x − 12)x + 4)x` for `x ≤ 0.25`.
/// * `GD-4` — `Difference` is `|cb − cs|`. ISO 32000-1's printed table
///   shows `cb – cs`; the bars are path-drawn and lost by every text
///   extractor. NOTE 14's own words ("subtracts the darker … from the
///   lighter") require the absolute value.
///
/// # Panics
///
/// Never. [`Blend::Normal`] and [`Blend::NonSeparable`] are handled by
/// [`Blend::apply`] before this is reached; they fall through to `cs`
/// here rather than panicking, because a panic in a per-pixel loop is a
/// worse failure than a `Normal` blend.
#[must_use]
fn blend_separable(mode: Blend, cb: f32, cs: f32) -> f32 {
    match mode {
        Blend::Multiply => cb * cs,
        Blend::Screen => cb.mul_add(-cs, cb + cs),
        // §11.3.5 §3.1: Overlay(cb, cs) = HardLight(cs, cb). The swap is
        // the whole definition — writing the HardLight branches out again
        // with the arguments in place is how the swap gets lost.
        Blend::Overlay => blend_separable(Blend::HardLight, cs, cb),
        Blend::Darken => cb.min(cs),
        Blend::Lighten => cb.max(cs),
        Blend::ColorDodge => {
            if cs < 1.0 {
                (cb / (1.0 - cs)).min(1.0)
            } else {
                1.0
            }
        }
        Blend::ColorBurn => {
            if cs > 0.0 {
                1.0 - ((1.0 - cb) / cs).min(1.0)
            } else {
                0.0
            }
        }
        Blend::HardLight => {
            if cs <= 0.5 {
                cb * (2.0 * cs)
            } else {
                let s = cs.mul_add(2.0, -1.0);
                cb.mul_add(-s, cb + s)
            }
        }
        Blend::SoftLight => {
            if cs <= 0.5 {
                // cb − (1 − 2cs) × cb × (1 − cb)
                cs.mul_add(2.0, -1.0).mul_add(cb * (1.0 - cb), cb)
            } else {
                let d = if cb <= 0.25 {
                    // ((16x − 12)x + 4)x
                    (cb.mul_add(16.0, -12.0).mul_add(cb, 4.0)) * cb
                } else {
                    cb.sqrt()
                };
                cs.mul_add(2.0, -1.0).mul_add(d - cb, cb)
            }
        }
        Blend::Difference => (cb - cs).abs(),
        Blend::Exclusion => (cb * cs).mul_add(-2.0, cb + cs),
        // Unreachable through `Blend::apply`; see the Panics note.
        Blend::Normal | Blend::NonSeparable(_) => cs,
    }
}

/// A colour and an alpha in the standard's own model: **un-premultiplied**
/// components in `[0, 1]`, and an alpha in `[0, 1]`.
///
/// # Why un-premultiplied
///
/// Because `B(C_b, C_s)` is defined on un-premultiplied values, and
/// premultiplying-then-blending is a *different function* for every
/// non-linear mode. Storage may well be premultiplied (`tiny_skia`'s is,
/// and §7.1 of the §11.4 corpus derives the premultiplied recurrence); the
/// **arithmetic** is not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pixel {
    /// Un-premultiplied colour components, `[0, 1]`.
    pub c: [f32; 3],
    /// Alpha, `[0, 1]`.
    pub a: f32,
}

impl Pixel {
    /// A fully transparent pixel. Its colour is **undefined** per §11.3.2
    /// ("when opacity is equal to zero, the corresponding colour is
    /// undefined"), so any value is conformant; zero is chosen because it
    /// is what a zeroed buffer already holds.
    pub const TRANSPARENT: Self = Self {
        c: [0.0, 0.0, 0.0],
        a: 0.0,
    };

    /// Read one `tiny_skia` premultiplied pixel into the standard's model.
    ///
    /// # The division, and why it is guarded rather than avoided
    ///
    /// `tiny_skia` stores `P = α × C`. Recovering `C` needs `P / α`, which
    /// is undefined at `α = 0` — and §11.3.2 says exactly that: the colour
    /// of a zero-opacity pixel *is* undefined, and the only obligation is
    /// that the computation "shall not malfunction because of exceptions
    /// caused by overflow or division by zero". So the guard returns
    /// [`Self::TRANSPARENT`], whose colour is subsequently multiplied by
    /// `α_b = 0` in every formula that reads it.
    #[must_use]
    pub fn from_premultiplied(px: PremultipliedColorU8) -> Self {
        let a = f32::from(px.alpha()) / 255.0;
        if a <= 0.0 {
            return Self::TRANSPARENT;
        }
        Self {
            c: [
                f32::from(px.red()) / 255.0 / a,
                f32::from(px.green()) / 255.0 / a,
                f32::from(px.blue()) / 255.0 / a,
            ],
            a,
        }
    }

    /// Quantise back to `tiny_skia`'s premultiplied 8-bit storage.
    ///
    /// # Returns
    ///
    /// `None` only if the quantised quadruple is not a valid premultiplied
    /// colour (`component > alpha`), which this function's own clamping
    /// makes unreachable — but `PremultipliedColorU8::from_rgba` is the
    /// authority on its own invariant and its answer is propagated rather
    /// than unwrapped. A caller that gets `None` must leave the destination
    /// pixel alone; writing a fabricated value would be worse than not
    /// writing.
    #[must_use]
    pub fn to_premultiplied(self) -> Option<PremultipliedColorU8> {
        let a = self.a.clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let q = |v: f32| (v.clamp(0.0, 1.0) * a * 255.0).round() as u8;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let a8 = (a * 255.0).round() as u8;
        PremultipliedColorU8::from_rgba(q(self.c[0]), q(self.c[1]), q(self.c[2]), a8)
    }
}

/// **§11.4.4's element-compositing formula** — the one operation every
/// transparency computation in this crate is made of.
///
/// ```text
/// α_i = Union(α_(i−1), α_si)
/// C_i = (1 − α_si/α_i) × C_(i−1)
///     + (α_si/α_i) × [ (1 − α_(i−1)) × C_si + α_(i−1) × B_i(C_(i−1), C_si) ]
/// ```
///
/// `backdrop` is `⟨C_(i−1), α_(i−1)⟩`, `source` is `⟨C_si, α_si⟩`.
///
/// # ★ The bug this function exists to make unrepresentable
///
/// Both hand-written composites `Pass 85.4b` shipped computed
///
/// ```text
/// out = C_b + α_s × (B(C_b, C_s) − C_b)
/// ```
///
/// which is the formula above **specialised to `α_b = 1` and `α_i = 1`**,
/// and both papered over the `α_b = 0` case by substituting a **white**
/// backdrop. Measured consequence, Ghent `1_GWG162` at scale 2.0: the
/// `Hue`, `Saturation` and `Color` cells rendered `(255, 255, 255)` where
/// pdfium renders `(184, 184, 184)`, and `Luminosity` rendered a flat grey
/// `(106, 106, 106)` where pdfium renders `(255, 22, 158)`. The mechanism
/// is exact rather than approximate: `Sat(white) = 0` and `Lum(white) = 1`,
/// so `Hue`/`Saturation`/`Color` of *any* source over white is white, and
/// `Luminosity` of any source over white is a neutral grey at the source's
/// luminosity.
///
/// With `α_b` carried explicitly there is no case to special-case:
/// at `α_b = 0` the `α_(i−1) × B(...)` term vanishes, `α_si/α_i = 1`, and
/// the result is `C_si` — the correct answer, reached by arithmetic rather
/// than by a convention.
///
/// # Degenerate inputs
///
/// `α_i = 0` implies `α_si = α_(i−1) = 0`; the result is
/// [`Pixel::TRANSPARENT`], whose colour §11.3.2 declares undefined and
/// which every downstream consumer multiplies by that zero. `0 ÷ 0 = 0` is
/// adopted unconditionally: a `should` in ISO 32000-1 §11.3.2 and a
/// **`shall`** in ISO 32000-2 §11.3.2.
#[must_use]
pub fn composite_element(backdrop: Pixel, source: Pixel, blend: Blend) -> Pixel {
    let ab = backdrop.a.clamp(0.0, 1.0);
    let a_s = source.a.clamp(0.0, 1.0);
    let ai = union_(ab, a_s);
    if ai <= 0.0 {
        return Pixel::TRANSPARENT;
    }
    // The one division, guarded above. `w` is the source's share of the
    // result: 1 when the backdrop contributes nothing, 0 when the source
    // does.
    let w = a_s / ai;
    // §11.4.4's inner bracket: the source, mixed with its own blend against
    // the backdrop in proportion to how opaque that backdrop is. At α_b = 0
    // this is C_s untouched — which is why a blend against "nothing" is the
    // source colour and not a blend against white.
    let blended = if blend.is_normal() || ab <= 0.0 {
        source.c
    } else {
        let b = blend.apply(backdrop.c, source.c);
        [
            ab.mul_add(b[0] - source.c[0], source.c[0]),
            ab.mul_add(b[1] - source.c[1], source.c[1]),
            ab.mul_add(b[2] - source.c[2], source.c[2]),
        ]
    };
    Pixel {
        c: [
            w.mul_add(blended[0] - backdrop.c[0], backdrop.c[0]),
            w.mul_add(blended[1] - backdrop.c[1], backdrop.c[1]),
            w.mul_add(blended[2] - backdrop.c[2], backdrop.c[2]),
        ],
        a: ai,
    }
}

/// **§11.4.8's element-compositing formula for a KNOCKOUT group.**
///
/// In a knockout group each element composites against the group's
/// **initial** backdrop rather than against the elements beneath it, and
/// the accumulated result is a weighted average taken with the element's
/// **shape** — not its alpha — as the weight:
///
/// ```text
/// α_gi = (1 − f_si) × α_g(i−1) + (f_si − α_si) × α_gb + α_si     (α_gb = 0, always)
/// C_t  = (f_si − α_si) × α_b × C_b + α_si × [ (1 − α_b) × C_si + α_b × B(C_b, C_si) ]
/// C_i  = [ (1 − f_si) × α_(i−1) × C_(i−1) + C_t ] / α_i
/// ```
///
/// # Why `α_gb` is dropped
///
/// `b = 0` in a knockout group, and §11.4.8's initialisation sets
/// `f_g0 = α_g0 = 0` unconditionally — isolated or not. So `α_gb` is
/// **always** zero and the middle term of the `α_gi` line vanishes. That is
/// the single most useful structural fact in the clause and it is stated
/// nowhere in it (`iso32000__s__11.4.md` §6.4): **the alpha recurrence of a
/// knockout group is the same for isolated and non-isolated groups.** All
/// of the isolation difference lives in `C_b` and `α_b`.
///
/// # Why shape and alpha cannot be collapsed here
///
/// `f_si` appears **alone**, in `(1 − f_si)`, where the non-knockout
/// formula has `(1 − α_si)`. Since `α_s = f_s × q_s ≤ f_s`, a knockout
/// element **erases more** of what is under it than a normal element does,
/// and the two coincide exactly when `q_s = 1`. §11.4.6 states the
/// consequence as a `shall`: *"The separate shape value shall be computed
/// in any group that is subsequently used as an element of a knockout
/// group."*
///
/// ★ **And it is why a fixture of opaque fills proves nothing.** At
/// `q_s = 1` this function and [`composite_element`] agree exactly, so an
/// all-opaque test passes under both the correct and the collapsed model.
/// Any knockout test must set `/ca < 1`.
///
/// # Arguments
///
/// * `initial` — `⟨C_0, α_0⟩`, the group's **initial** backdrop, frozen at
///   group entry. For an isolated knockout group this is
///   [`Pixel::TRANSPARENT`].
/// * `accum` — `⟨C_(i−1), α_(i−1)⟩`, the running result. Note `α_(i−1)` is
///   the *complete* alpha, `Union(α_0, α_g(i−1))`.
/// * `source` — `⟨C_si, α_si⟩`.
/// * `shape` — `f_si`, the element's shape. Must satisfy
///   `α_si ≤ f_si ≤ 1`; values outside that are clamped rather than
///   rejected, because a per-pixel loop is the wrong place to fail.
/// * `accum_group_alpha` — `α_g(i−1)`, the group's own accumulated alpha,
///   excluding the initial backdrop. Returned updated as the second tuple
///   element.
/// * `blend` — `B_i`, applied against the **initial** backdrop.
///
/// # Returns
///
/// `(⟨C_i, α_i⟩, α_gi)`.
#[must_use]
pub fn composite_element_knockout(
    initial: Pixel,
    accum: Pixel,
    source: Pixel,
    shape: f32,
    accum_group_alpha: f32,
    blend: Blend,
) -> (Pixel, f32) {
    let a0 = initial.a.clamp(0.0, 1.0);
    let a_s = source.a.clamp(0.0, 1.0);
    let f_s = shape.clamp(a_s, 1.0);
    let ag_prev = accum_group_alpha.clamp(0.0, 1.0);

    // α_gi = (1 − f_si)·α_g(i−1) + α_si   (the α_gb term is identically 0)
    let ag = (1.0 - f_s).mul_add(ag_prev, a_s).clamp(0.0, 1.0);
    let ai = union_(a0, ag);
    if ai <= 0.0 {
        return (Pixel::TRANSPARENT, 0.0);
    }

    // C_t, premultiplied by f_si per §11.4.8 NOTE 4.
    let blended = if a0 <= 0.0 || blend.is_normal() {
        source.c
    } else {
        // The blend function for a knockout element reads the INITIAL
        // backdrop, never the accumulated one. Using `accum` here is the
        // mistake that turns a knockout group back into a normal one while
        // still looking plausible on opaque artwork.
        let b = blend.apply(initial.c, source.c);
        [
            a0.mul_add(b[0] - source.c[0], source.c[0]),
            a0.mul_add(b[1] - source.c[1], source.c[1]),
            a0.mul_add(b[2] - source.c[2], source.c[2]),
        ]
    };
    let k = (f_s - a_s) * a0;
    let mut c = [0.0_f32; 3];
    for i in 0..3 {
        let ct = a_s.mul_add(blended[i], k * initial.c[i]);
        c[i] = (1.0 - f_s).mul_add(accum.a * accum.c[i], ct) / ai;
    }
    (Pixel { c, a: ai }, ag)
}

/// **§11.4.4's backdrop removal** — the correction that makes §11.4.3's
/// *"the backdrop's contribution … shall be applied only once"* true.
///
/// ```text
/// C = C_n + (C_n − C_0) × ( α_0 / α_gn − α_0 )
/// ```
///
/// # Why this is not optional
///
/// A **non-isolated** group's elements are composited *onto* the group's
/// backdrop — §11.4.4 NOTE 2 says why: *"This is done to achieve the
/// correct effects of the blend modes, most of which are dependent on both
/// the backdrop and source colours being blended."* The group's result is
/// then composited onto that same backdrop a second time. Without this
/// correction the backdrop is counted twice and every non-isolated group
/// darkens its own backdrop.
///
/// # The division, and what happens as `α_gn → 0`
///
/// `α_gn = 0` means no element of the group contributed any alpha — an
/// empty group, an all-`/ca 0` group, a content stream that painted
/// nothing. It is not exotic. §11.3.2 governs it: *"In any formula that
/// uses such an undefined quantity, the quantity has no effect on the
/// ultimate result… the computation shall not malfunction because of
/// exceptions caused by overflow or division by zero."* So the guard
/// returns `C_n` — any finite colour is conformant, `C_n` costs nothing,
/// and the returned `α = α_gn = 0` makes it unreachable anyway.
///
/// # Isolated groups
///
/// Pass `α_0 = 0` (which is what an isolated group's initial backdrop
/// *is*) and this reduces to `C = C_n` exactly, as §11.4.5 NOTE 2 states.
/// There is therefore **no branch on isolation** in this function or in its
/// caller — the isolation flag is expressed entirely by the value of
/// `α_0`, which is the whole of the normative change §11.4.5 makes.
#[must_use]
pub fn remove_backdrop(group: Pixel, initial: Pixel, group_alpha: f32) -> [f32; 3] {
    let a0 = initial.a.clamp(0.0, 1.0);
    let agn = group_alpha.clamp(0.0, 1.0);
    if a0 <= 0.0 || agn <= 0.0 {
        return group.c;
    }
    let k = a0.mul_add(-1.0, a0 / agn);
    [
        k.mul_add(group.c[0] - initial.c[0], group.c[0]),
        k.mul_add(group.c[1] - initial.c[1], group.c[1]),
        k.mul_add(group.c[2] - initial.c[2], group.c[2]),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < EPS)
    }

    #[test]
    fn union_is_not_max() {
        assert!((union_(0.5, 0.5) - 0.75).abs() < EPS);
        assert!((union_(0.0, 0.3) - 0.3).abs() < EPS);
        assert!((union_(1.0, 0.3) - 1.0).abs() < EPS);
    }

    /// ★ THE REGRESSION THIS MODULE WAS WRITTEN FOR.
    ///
    /// Compositing **anything** onto a fully transparent backdrop must give
    /// back the source colour unchanged, for **every** blend mode. The
    /// previous hand-written composites substituted a white backdrop here,
    /// which made `Hue`/`Saturation`/`Color` return white and `Luminosity`
    /// return a neutral grey — measured on Ghent `1_GWG162`.
    #[test]
    fn blend_over_a_transparent_backdrop_is_the_source_for_every_mode() {
        let src = Pixel {
            c: [0.2, 0.7, 0.4],
            a: 1.0,
        };
        for mode in [
            Blend::Normal,
            Blend::Multiply,
            Blend::Screen,
            Blend::Overlay,
            Blend::Darken,
            Blend::Lighten,
            Blend::ColorDodge,
            Blend::ColorBurn,
            Blend::HardLight,
            Blend::SoftLight,
            Blend::Difference,
            Blend::Exclusion,
            Blend::NonSeparable(NonSeparableBlend::Hue),
            Blend::NonSeparable(NonSeparableBlend::Saturation),
            Blend::NonSeparable(NonSeparableBlend::Color),
            Blend::NonSeparable(NonSeparableBlend::Luminosity),
        ] {
            let out = composite_element(Pixel::TRANSPARENT, src, mode);
            assert!(
                close(out.c, src.c) && (out.a - 1.0).abs() < EPS,
                "{mode:?} over a transparent backdrop must be the source, got {out:?}"
            );
        }
    }

    /// The white-backdrop substitution, reproduced so the fix is
    /// attributable. `Hue` of any colour over WHITE is white, because
    /// `Sat(white) = 0` and `Lum(white) = 1`. That is what the old code
    /// computed and it is why the cells came out `(255, 255, 255)`.
    #[test]
    fn hue_over_white_is_white_which_is_why_the_old_convention_failed() {
        let got =
            crate::blend_nonsep::blend(NonSeparableBlend::Hue, [1.0, 1.0, 1.0], [0.2, 0.7, 0.4]);
        assert!(
            close(got, [1.0, 1.0, 1.0]),
            "Hue over white must be white — this is the mechanism, got {got:?}"
        );
    }

    /// With an opaque backdrop the formula must reduce to the familiar
    /// `lerp(C_b, B(C_b, C_s), α_s)`, which is what the previous code
    /// computed and what every other renderer's simple path computes.
    #[test]
    fn opaque_backdrop_reduces_to_the_simple_lerp() {
        let bd = Pixel {
            c: [0.8, 0.2, 0.5],
            a: 1.0,
        };
        let src = Pixel {
            c: [0.3, 0.9, 0.1],
            a: 0.4,
        };
        let out = composite_element(bd, src, Blend::Multiply);
        let b = Blend::Multiply.apply(bd.c, src.c);
        let want = [
            0.4_f32.mul_add(b[0] - bd.c[0], bd.c[0]),
            0.4_f32.mul_add(b[1] - bd.c[1], bd.c[1]),
            0.4_f32.mul_add(b[2] - bd.c[2], bd.c[2]),
        ];
        assert!(close(out.c, want), "got {:?} want {want:?}", out.c);
        assert!((out.a - 1.0).abs() < EPS);
    }

    /// A half-covered backdrop under a half-covering source must reach
    /// `Union(0.5, 0.5) = 0.75`, not `max(0.5, 0.5) = 0.5`. The old code
    /// used `max`, which under-reports alpha on every anti-aliased edge
    /// inside a group.
    #[test]
    fn accumulated_alpha_is_union_not_max() {
        let out = composite_element(
            Pixel {
                c: [1.0, 0.0, 0.0],
                a: 0.5,
            },
            Pixel {
                c: [0.0, 0.0, 1.0],
                a: 0.5,
            },
            Blend::Normal,
        );
        assert!(
            (out.a - 0.75).abs() < EPS,
            "alpha must be Union = 0.75, got {}",
            out.a
        );
    }

    /// §11.4.5 NOTE 2, verified arithmetically: with `α_0 = 0` backdrop
    /// removal is the identity.
    #[test]
    fn backdrop_removal_is_the_identity_for_an_isolated_group() {
        let g = Pixel {
            c: [0.3, 0.6, 0.9],
            a: 0.5,
        };
        let out = remove_backdrop(g, Pixel::TRANSPARENT, 0.5);
        assert!(close(out, g.c));
    }

    /// The round trip the correction exists for: composite a source onto a
    /// backdrop (the non-isolated group's internal step), then remove the
    /// backdrop, and the recovered colour must be the source — because a
    /// `Normal` blend against a backdrop contributes nothing the removal
    /// cannot undo. §11.4.4 NOTE 3 calls this "essentially the reverse of
    /// compositing with the Normal blend mode".
    #[test]
    fn backdrop_removal_inverts_a_normal_composite() {
        for a0 in [0.25_f32, 0.5, 0.75, 1.0] {
            for a_s in [0.2_f32, 0.5, 0.8, 1.0] {
                let bd = Pixel {
                    c: [0.9, 0.1, 0.4],
                    a: a0,
                };
                let src = Pixel {
                    c: [0.2, 0.8, 0.6],
                    a: a_s,
                };
                let n = composite_element(bd, src, Blend::Normal);
                let recovered = remove_backdrop(n, bd, a_s);
                assert!(
                    close(recovered, src.c),
                    "α_0={a0} α_s={a_s}: recovered {recovered:?}, want {:?}",
                    src.c
                );
            }
        }
    }

    /// §11.4.8 collapses to §11.4.4 when every element is opaque
    /// (`q_s = 1` ⇒ `α_s = f_s`). This is §6.5's corollary, and it is the
    /// reason an all-opaque fixture cannot test knockout.
    #[test]
    fn knockout_and_normal_agree_when_the_element_is_opaque() {
        let initial = Pixel {
            c: [0.7, 0.3, 0.1],
            a: 1.0,
        };
        let src = Pixel {
            c: [0.1, 0.5, 0.9],
            a: 1.0,
        };
        let normal = composite_element(initial, src, Blend::Normal);
        let (ko, _) = composite_element_knockout(initial, initial, src, 1.0, 0.0, Blend::Normal);
        assert!(
            close(ko.c, normal.c),
            "knockout {:?} vs normal {:?}",
            ko.c,
            normal.c
        );
    }

    /// …and they must DISAGREE when the element is translucent, which is
    /// the whole visible content of the knockout feature. A second element
    /// at `q_s = 0.5` erases half of the first rather than layering over
    /// it.
    #[test]
    fn knockout_erases_where_normal_layers() {
        let initial = Pixel {
            c: [1.0, 1.0, 1.0],
            a: 1.0,
        };
        let first = Pixel {
            c: [0.0, 0.0, 0.0],
            a: 0.5,
        };
        let second = Pixel {
            c: [0.0, 0.0, 0.0],
            a: 0.5,
        };
        // Non-knockout: two half-black layers ⇒ 75 % black.
        let n1 = composite_element(initial, first, Blend::Normal);
        let n2 = composite_element(n1, second, Blend::Normal);
        // Knockout: the second element knocks the first out entirely
        // (shape 1.0), leaving 50 % black.
        let (k1, ag1) =
            composite_element_knockout(initial, initial, first, 1.0, 0.0, Blend::Normal);
        let (k2, _) = composite_element_knockout(initial, k1, second, 1.0, ag1, Blend::Normal);
        assert!(
            (n2.c[0] - 0.25).abs() < EPS,
            "non-knockout should reach 0.25, got {}",
            n2.c[0]
        );
        assert!(
            (k2.c[0] - 0.5).abs() < EPS,
            "knockout should stay at 0.50, got {}",
            k2.c[0]
        );
    }

    /// Table 136 spot checks, including the two branches every transcription
    /// gets wrong (`GD-1`, `GD-2`) and the swap (`Overlay`).
    #[test]
    fn table_136_spot_checks() {
        assert!((blend_separable(Blend::Multiply, 0.5, 0.4) - 0.2).abs() < EPS);
        assert!((blend_separable(Blend::Screen, 0.5, 0.4) - 0.7).abs() < EPS);
        assert!((blend_separable(Blend::Darken, 0.5, 0.4) - 0.4).abs() < EPS);
        assert!((blend_separable(Blend::Lighten, 0.5, 0.4) - 0.5).abs() < EPS);
        assert!((blend_separable(Blend::Difference, 0.4, 0.9) - 0.5).abs() < EPS);
        assert!((blend_separable(Blend::Exclusion, 0.5, 0.5) - 0.5).abs() < EPS);
        // GD-1: cs = 1 ⇒ 1, not a division by zero.
        assert!((blend_separable(Blend::ColorDodge, 0.3, 1.0) - 1.0).abs() < EPS);
        assert!(blend_separable(Blend::ColorDodge, 0.3, 1.0).is_finite());
        // GD-2: cs = 0 ⇒ 0.
        assert!((blend_separable(Blend::ColorBurn, 0.3, 0.0) - 0.0).abs() < EPS);
        assert!(blend_separable(Blend::ColorBurn, 0.3, 0.0).is_finite());
        // Overlay(cb, cs) = HardLight(cs, cb): the arguments swap.
        for (cb, cs) in [(0.2_f32, 0.8_f32), (0.7, 0.3), (0.5, 0.5)] {
            assert!(
                (blend_separable(Blend::Overlay, cb, cs)
                    - blend_separable(Blend::HardLight, cs, cb))
                .abs()
                    < EPS
            );
        }
    }

    /// `Difference` is `|cb − cs|`, not `cb − cs` (corpus erratum `GD-4`).
    /// The un-absolute form goes negative, which then clamps to zero and
    /// makes the mode asymmetric — visibly wrong, and the sort of thing a
    /// test on `cb > cs` alone never catches.
    #[test]
    fn difference_is_absolute() {
        assert!((blend_separable(Blend::Difference, 0.2, 0.9) - 0.7).abs() < EPS);
        assert!((blend_separable(Blend::Difference, 0.9, 0.2) - 0.7).abs() < EPS);
    }

    /// The premultiplied round trip must be stable to within one 8-bit
    /// level; this is the quantisation the f32 arithmetic sits on top of.
    #[test]
    fn premultiplied_round_trip_is_within_one_level() {
        for a in [0.25_f32, 0.5, 0.75, 1.0] {
            let p = Pixel {
                c: [0.2, 0.6, 0.9],
                a,
            };
            let back = Pixel::from_premultiplied(p.to_premultiplied().unwrap());
            assert!((back.a - a).abs() < 1.0 / 255.0 + EPS);
            for i in 0..3 {
                assert!(
                    (back.c[i] - p.c[i]).abs() < 2.0 / (255.0 * a),
                    "a={a} component {i}: {} vs {}",
                    back.c[i],
                    p.c[i]
                );
            }
        }
    }

    /// §11.3.2's robustness `shall`: no formula here may emit NaN or Inf,
    /// whatever it is handed. Swept over the degenerate corners rather than
    /// argued.
    #[test]
    fn no_formula_emits_nan_or_inf() {
        let corners = [0.0_f32, 1.0];
        for &ab in &corners {
            for &a_s in &corners {
                for mode in [Blend::ColorDodge, Blend::ColorBurn, Blend::SoftLight] {
                    for &cb in &corners {
                        for &cs in &corners {
                            let out = composite_element(
                                Pixel { c: [cb; 3], a: ab },
                                Pixel { c: [cs; 3], a: a_s },
                                mode,
                            );
                            assert!(
                                out.a.is_finite() && out.c.iter().all(|v| v.is_finite()),
                                "{mode:?} ab={ab} as={a_s} cb={cb} cs={cs} -> {out:?}"
                            );
                        }
                    }
                }
                let g = Pixel {
                    c: [0.5; 3],
                    a: a_s,
                };
                let r = remove_backdrop(g, Pixel { c: [0.5; 3], a: ab }, 0.0);
                assert!(r.iter().all(|v| v.is_finite()));
            }
        }
    }
}
