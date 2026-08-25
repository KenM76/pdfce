//! A **subtractive (CMYK) compositing buffer** — ISO 32000-1 §11.7.2,
//! §11.6.6, §11.4.7, §11.3.4.
//!
//! # Why this module exists
//!
//! Until `Pass 97.1e` every buffer in this crate was a
//! `tiny_skia::Pixmap`: 8-bit **premultiplied sRGB**, one alpha channel,
//! three additive components. That is a correct raster target and it is
//! the wrong *model* for a document whose blending colour space is
//! subtractive, and the standard says so three times, in `shall` strength:
//!
//! 1. **§11.7.2** — a graphics object whose colour space is not equivalent
//!    to the group's **shall** be converted to the group's space, and all
//!    blending and compositing computations **shall** be done in that
//!    space.
//! 2. **§11.6.6** — painting operators **shall** convert source colours to
//!    the group colour space **before compositing objects into the group**.
//! 3. **§11.4.7** — all page-level compositing **shall** be done in the
//!    page's default blending colour space, and the entire result **shall**
//!    then be converted to the device's native space.
//!
//! ISO 32000-1:2008 §11.7.2 NOTE 1 states the rationale in the free
//! edition, and it is worth quoting because it is this module's entire
//! justification:
//!
//! > "After all the artwork has been placed on the page, the conversion
//! > from the group's colour space to the page's device colour space will
//! > be done as the last step, without any further transparency
//! > compositing. … the reason for adopting it is to avoid the loss of
//! > colour information and the introduction of errors resulting from
//! > unnecessary colour space conversions."
//!
//! (That NOTE was **deleted** in ISO 32000-2:2020 — the requirement
//! survives, the explanation does not. Cite the 2008 text.)
//!
//! # The measurement that made this a Pass rather than a nicety
//!
//! On the print-conformance suite, **13 of 51 files declare a subtractive
//! blending space and 107 of 107 blend-mode applications ran in the wrong
//! one** — 100 %. Every transparency patch in that suite declares
//! `/Group /CS /DeviceCMYK` on the **page**, so no amount of per-object
//! correctness reaches them. On `fixtures/external` (4,012 files) the same
//! census finds 15 files and 2 wrong blends — and **both** of those are
//! veraPDF transparency *conformance* fixtures, so **zero organic
//! documents in a 4,012-file corpus are affected**. This buffer buys
//! prepress conformance, not a better-looking corpus, and the render
//! parity buckets are expected **not** to move.
//!
//! # ★ The round trip this exists to delete, measured
//!
//! `DeviceCMYK 0 1 0 0` painted into the sRGB buffer and recovered from it
//! comes back as `(0, 0.995, 0.409, 0.071)`. The `Y = 0.41` is not a
//! rounding error; ISO 32000-1 §8.6.5.7 NOTE 2 names the 4→3→4 trip by
//! hand as "unnecessary and results in a loss of fidelity in the black
//! component". [`crate::overprint::composite`] performs exactly that trip
//! on every overprinted pixel today, and its own doc comment concedes it:
//! "a real n-channel buffer remains the eventual fix". This is that
//! buffer, for `N = 4`.
//!
//! # What is here, and what deliberately is not
//!
//! Here: the buffer type, the per-pixel accessors, the two compositing
//! entry points (a coverage mask with one solid colorant, and an sRGB
//! pixmap bridged in), and the §11.4.7 collapse.
//!
//! **Not** here: the compositing arithmetic itself — that is
//! [`crate::compositor::composite_element_cmyk`] and
//! [`crate::compositor::Blend::apply_subtractive`], written and tested one
//! Pass earlier and deliberately left as pure per-pixel functions that
//! know nothing about buffers. **Not** here: spot colorants. Four
//! components rather than a runtime `N` is a decision recorded on
//! [`crate::compositor::PixelCmyk`] — the leading deliverable is the
//! *blending space*, which is `DeviceCMYK` for every file that matters
//! here, and a runtime-`N` buffer wants a different storage layout again.
//! Building `N` now would fuse two questions that fail independently.
//!
//! # ★★ Three traps this module is shaped around
//!
//! ## 1. The collapse order is convert-then-flatten, not flatten-then-convert
//!
//! §11.4.7 requires that the page group's result be converted to the
//! device's native colour space **before being composited with the
//! context-dependent backdrop**. The media-white composite
//! `C = (1 − α_g)·W + α_g·C_g` therefore sits on the **far side** of the
//! CMYK→sRGB conversion, in the destination space.
//!
//! This is not academic. The conversion is non-affine (it clamps, and it
//! is a fitted lattice), so the two orders give different pixels and
//! **both look like a page**. [`CmykBuffer::to_srgb_over_white`] does them
//! in the required order and its test pins the distinction.
//!
//! *(Sourced from `iccce`, 2026-08-21, checked clause-by-clause against
//! both ISO editions with two independent extraction engines. The ISO
//! 32000-2 errata are unapplied PDF annotations in the sponsored copy, so
//! a naive text extraction returns the uncorrected standard — none of
//! §11.3.4, §11.4.7, §11.7.2, §11.7.4.2 or §11.7.4.3 carries an erratum.)*
//!
//! ## 2. A zeroed subtractive buffer is WHITE, and that is a real trap
//!
//! §8.6.4.4 gives `DeviceCMYK` an **initial colour of `[0 0 0 1]`** — pure
//! black. So `memset(0)` over the colorant planes yields **no ink**, which
//! is *white paper*, and a luminosity soft mask built over such a buffer
//! comes out **inverted**. The same zero fill is correct in sRGB and wrong
//! in CMYK, which is exactly why the trap appears at this boundary and
//! nowhere earlier.
//!
//! [`CmykBuffer::new`] zero-fills anyway, and that is **safe for a reason
//! that must not be generalised**: it also zeroes `alpha`, and a pixel at
//! `α = 0` has, per §11.3.2, an *undefined* colour that every formula in
//! [`crate::compositor`] multiplies by its own zero alpha. The zero fill
//! is an initialiser for a **transparent** buffer, not for an opaque one.
//! Any future code that wants an *opaque* subtractive backdrop —
//! a soft-mask group's `/BC`, a non-isolated group's initial backdrop —
//! must set `[0, 0, 0, 1]` explicitly and must not reach for
//! [`CmykBuffer::new`].
//!
//! ## 3. The element type is `f32`, and the reason is a division
//!
//! §11.4.4's backdrop removal contains a single `1/α_gn`. At
//! `α_gn = 0.02` a half-level 8-bit error becomes **25 levels** — which is
//! why every production engine's 8-bit buffer either flattens
//! non-isolated groups or accepts the artefact, and pdfce is fixing
//! precisely the non-isolated case. `f32`'s equivalent amplified error at
//! the same point is about `1.5e-6`, roughly **1/2600th of a single 8-bit
//! level**, and the final quantisation to 8 bits dominates the error
//! budget by three orders of magnitude.
//!
//! `f64` was considered and declined on 2026-08-21: it doubles memory and
//! halves SIMD lane count to shrink an error that is already invisible.
//! The one argument for it — `iccce`'s evaluation surface is `f64`-only —
//! does not survive, because widening `f32`→`f64` is **exact** and happens
//! once per pixel at the collapse, not inside the blend loop. [`Chan`] is
//! the single place that decision lives, so revisiting it is a one-line
//! change rather than a sweep.
//!
//! # Storage layout: plane-major
//!
//! The four colorants and the alpha are **five contiguous planes**, not an
//! interleaved array of structs. Measured on this machine (i9-10900KF,
//! archived under `D:\Dev\Rag-Specialized\Compositor\bench\`), plane-major
//! beats pixel-major on **every** kernel at **every** channel count:
//! fill 3.0–5.4×, group composite 2.6–3.7×, whole-plane op 3.8–10.3×. The
//! folk rule — "per-pixel operations want interleaved" — does not survive
//! a runtime channel count, because the compiler cannot vectorise across a
//! stride it does not know. `N = 4` here is a compile-time constant and
//! would not suffer that, but the *next* buffer (spot planes) is runtime-N
//! and this layout is what it needs, so adopting it now costs nothing and
//! avoids a rewrite.
//!
//! [`crate::compositor::PixelCmyk`] stays the **accessor view**: the
//! arithmetic is written against one pixel, the storage is written against
//! one plane, and [`CmykBuffer::pixel`] / [`CmykBuffer::set_pixel`] are
//! the only two places that know both.

use tiny_skia::{Mask, Pixmap};

use crate::compositor::{
    Blend, PixelCmyk, composite_element_cmyk, composite_element_knockout_cmyk, remove_backdrop_cmyk,
};

/// The extra planes a **knockout** group needs — ISO 32000-1 §11.4.6,
/// §11.4.8.
///
/// # Why a knockout group needs state an ordinary one does not
///
/// In a knockout group each element composites against the group's
/// **initial** backdrop rather than against the elements beneath it, so
/// that backdrop has to survive the whole group rather than being consumed
/// by the first paint. And §11.4.8's recurrence carries two quantities
/// that cannot be recovered from the accumulated pixel afterwards:
///
/// - **`α_g`**, the group's own alpha *excluding* the backdrop, which
///   §11.4.4's backdrop removal divides by on the way out;
/// - **`f_g`**, the group's **shape**, which §11.4.6 requires *"shall be
///   computed in any group that is subsequently used as an element of a
///   knockout group"*.
///
/// # ★ Shape and alpha are not the same number, and an opaque fixture
/// cannot tell
///
/// `α = f × q`. They coincide exactly when opacity is 1 — which is most
/// artwork — so a test built from opaque fills passes under both the
/// correct model and the collapsed one. §11.4.8 reads `(1 − f_si)` where
/// §11.4.4 reads `(1 − α_si)`: a knockout element **erases more** of what
/// is under it than an ordinary element does, and only a fixture with
/// `/ca < 1` can see the difference.
///
/// # What this costs, and why it is cheap here
///
/// Four planes: the initial backdrop's colorants and alpha (shared with
/// the buffer's own layout), plus `α_g` and `f_g`. Notably it needs **no
/// scratch buffer**, unlike [`crate::canvas::KnockoutTarget`] — that one
/// must rasterise each element into a spare pixmap first, because
/// `tiny_skia` rasterises and composites in the same call and there is no
/// other way to recover an element's shape in isolation. A colorant paint
/// already arrives as a separate coverage mask, so `f_s` is simply the
/// coverage byte and `α_s` is that times the constant alpha. The
/// subtractive implementation is the simpler of the two, which is not the
/// direction one expects.
#[derive(Debug, Clone)]
struct KnockoutPlanes {
    /// The group's initial backdrop colorants, `[C, M, Y, K]`.
    initial: [Vec<Chan>; 4],
    /// The group's initial backdrop alpha, `α_0`.
    initial_alpha: Vec<Chan>,
    /// `α_gi` — the group's own accumulated alpha, excluding the backdrop.
    group_alpha: Vec<Chan>,
    /// `f_gi` — the group's own accumulated shape. Tracked because
    /// §11.4.6 makes it a `shall` for any group that may itself become an
    /// element of a knockout group, and because adding a plane later means
    /// revisiting every write site.
    group_shape: Vec<Chan>,
}

/// The buffer's element type.
///
/// **This alias is the whole `f32`-vs-`f64` decision**, deliberately in
/// one place so that revisiting it is a single edit and not a sweep. See
/// the module documentation's trap 3 for the numbers behind the choice.
///
/// Consequences of changing it, so they are not rediscovered:
///
/// - Memory scales linearly. At `N = 4` plus alpha the buffer costs
///   `5 × size_of::<Chan>()` bytes per pixel — **20 B/px** at `f32`,
///   **40 B/px** at `f64`. A US-Letter page at 300 DPI is 8.4 M pixels,
///   so 161 MiB against 321 MiB. [`MAX_CMYK_BUFFER_BYTES`] is expressed in
///   bytes, not pixels, so the ceiling adjusts itself.
/// - [`CmykBuffer::to_srgb_over_white`] converts through
///   [`pdfce_core::color::cmyk_to_srgb_with`], whose surface is `f32`; a
///   `f64` buffer would narrow there. Narrowing is lossy, widening is not.
pub(crate) type Chan = f32;

/// The largest buffer this module will allocate, in bytes.
///
/// Matched deliberately to [`crate::display_list::MAX_DISPLAY_LIST_BYTES`]
/// so the two ceilings in this crate that bound a page-sized allocation
/// agree, and a future reader does not have to work out why they differ.
///
/// # Why a ceiling at all
///
/// `docs/ARCHITECTURE.md` §10: no untrusted-input-sized allocation without
/// a ceiling. Page dimensions come from the file. At 20 B/px this permits
/// 13.4 M pixels — a US-Letter page up to roughly 375 DPI, or A0 at 96 DPI
/// — and refuses beyond that.
///
/// # What happens at the ceiling, and why it is not an error
///
/// The caller falls back to the ordinary sRGB path and **discloses that it
/// did** (`cmyk_buffer_refused`). That is the honest outcome: a page
/// rendered in the wrong blending space is a known, counted approximation
/// that pdfce has shipped for its entire life, whereas a failed render is
/// a regression. Project rule 4 — the fallback prints what it did.
pub(crate) const MAX_CMYK_BUFFER_BYTES: usize = 256 * 1024 * 1024;

/// Bytes of storage per pixel: four colorant planes plus alpha.
const BYTES_PER_PIXEL: usize = 5 * core::mem::size_of::<Chan>();

/// A page- or group-sized **subtractive** compositing buffer.
///
/// # Model
///
/// Five plane-major buffers of [`Chan`], each `width × height` long and
/// indexed by `y * width + x` — the **same indexing `tiny_skia` uses for
/// its pixels and for a [`Mask`]'s coverage bytes**, which is what lets a
/// coverage mask rasterised by `tiny_skia` gate a composite computed here
/// without any coordinate translation. That correspondence is load-bearing
/// and is asserted by [`CmykBuffer::composite_mask`]'s debug assertion.
///
/// Colour is **un-premultiplied**, because `B(C_b, C_s)` is defined on
/// un-premultiplied values and premultiplying-then-blending is a different
/// function for every non-linear blend mode. `tiny_skia` stores
/// premultiplied and pays a divide on every read; this buffer pays the
/// memory instead.
///
/// # What this type deliberately does NOT carry
///
/// **Shape (`f`) as a plane separate from alpha (`α`).** §11.4.6 requires
/// it — "the separate shape value shall be computed in any group that is
/// subsequently used as an element of a knockout group" — and the
/// requirement is real, because §11.4.8's knockout formula reads `(1 − f_s)`
/// where the ordinary formula reads `(1 − α_s)`, and `α = f × q` makes
/// those differ wherever opacity is below 1.
///
/// It is omitted **here, at page scope, on a specific argument**: the page
/// group is never an *element* of anything (§11.4.7 makes it the outermost
/// group and composites it directly onto the medium), so no knockout
/// formula ever reads its shape. A **group** buffer is a different case
/// and the plane must be added when CMYK groups land — which is `Pass
/// 97.1f`, named here so the omission cannot be mistaken for an oversight.
/// [`crate::canvas::KnockoutTarget`] already carries `group_shape` as a
/// separate plane for exactly this reason and is the model to copy.
#[derive(Debug, Clone)]
pub(crate) struct CmykBuffer {
    /// Device width in pixels.
    width: u32,
    /// Device height in pixels.
    height: u32,
    /// The four colorant planes, `[C, M, Y, K]`, each `width × height`.
    ///
    /// Subtractive **tints** in `0.0..=1.0`: `0.0` is no ink, `1.0` is
    /// full ink. Note this is the opposite polarity from every additive
    /// plane in this crate, and it is why [`crate::compositor::BlendSpace`]
    /// is a type rather than a flag.
    planes: [Vec<Chan>; 4],
    /// The alpha plane, `0.0..=1.0`, `width × height`.
    alpha: Vec<Chan>,
    /// Pixels whose colour reached this buffer through the sRGB bridge
    /// rather than as authored colorants.
    ///
    /// A **disclosure** counter, not a shortfall: an image is decoded to
    /// sRGB texels long before it reaches a canvas, so bridging it is the
    /// only thing that can be done at this Pass and the count is how the
    /// operator learns the page was not composited entirely from authored
    /// ink. Read out by [`CmykBuffer::bridged_pixels`].
    bridged: u64,
    /// Transparency groups on this page that could **not** be composited
    /// natively in ink, for either of two reasons.
    ///
    /// | case | what is lost |
    /// |---|---|
    /// | a **knockout** group (§11.4.6) | its interior runs in sRGB and its result is converted back; §11.4.6's own semantics are preserved, the blending space inside it is not |
    /// | a **non-isolated** group (§11.4.4) | it is composited as if isolated: its backdrop is dropped and §11.4.4's backdrop removal is skipped |
    ///
    /// An ordinary isolated group is **not** counted here, because since
    /// `Pass 97.1e` it gets a child [`CmykBuffer`] and no conversion
    /// happens at its boundary at all.
    ///
    /// This is a shortfall, not a cost. Both cases are `Pass 97.1f`'s work
    /// and both are measurable: routing the suite knockout patch
    /// `PCS1_161` through the sRGB path costs it two traps against its
    /// pre-Pass baseline, and that number is the one to watch when the
    /// native knockout target lands.
    groups_approximated: u64,
    /// Image brushes that reached a subtractive paint through a path that
    /// cannot bridge them.
    ///
    /// Reachable only from a replayed display list, which is refused on a
    /// subtractive page — so this should always be zero, and it is counted
    /// rather than asserted because "unreachable" claims decay and a
    /// counter that stays zero costs nothing.
    unbridged_images: u64,
    /// The `DeviceCMYK` → sRGB rendering intent this buffer converts with.
    ///
    /// # Why the buffer owns it rather than taking it per call
    ///
    /// Because it is consulted from **two** places that are far apart —
    /// the §11.4.7 collapse at the end of the page, and the backdrop
    /// hand-off to every nested group ([`CmykBuffer::snapshot_srgb_backdrop`]) —
    /// and those two must not be able to disagree. A group composited over
    /// a backdrop converted one way, then collapsed another way, produces a
    /// seam at the group's own edge: correct inside, correct outside,
    /// wrong along the boundary. Threading the intent through
    /// `Canvas::group` and `Canvas::knockout_group` as a parameter would
    /// make that possible; storing it here does not.
    ///
    /// ISO 32000-2 §11.4.7 asks for `RelativeColorimetric` on the final
    /// conversion "unless the processor has an implementation-dependent way
    /// of specifying otherwise". This setting is that way.
    intent: pdfce_core::settings::CmykIntent,
    /// The §11.4.6 knockout state, when this buffer **is** a knockout
    /// group's accumulator.
    ///
    /// `None` is the ordinary case and costs one `Option` discriminant.
    /// `Some` changes what every composite on this buffer means — §11.4.8
    /// replaces §11.4.4 — which is why it lives here rather than as a flag
    /// a call site could forget to consult.
    knockout: Option<Box<KnockoutPlanes>>,
    /// A page-sized coverage mask, **reused across every paint**.
    ///
    /// ★ THIS FIELD IS A PERFORMANCE FIX FOR A REGRESSION THIS MODULE
    /// SHIPPED, and the number is worth carrying because the mistake was
    /// documented-and-deferred rather than overlooked.
    ///
    /// `Pass 97.1e` allocated a fresh `Mask::new(page)` per paint. Its own
    /// module documentation said so, cited pdfce's measurement of that
    /// allocation at **259 µs**, and deferred the fix on the grounds that
    /// bundling a layout optimisation into a correctness Pass makes two
    /// kinds of regression indistinguishable. That reasoning was sound and
    /// the arithmetic behind it was never done: **every glyph is a fill**,
    /// a text-heavy page has thousands, and the per-paint cost was not one
    /// page-sized pass but roughly four — allocate, zero, copy the clip
    /// (`to_vec`), and multiply, each over the whole page regardless of
    /// how small the mark was.
    ///
    /// Measured on the suite combined document at scale 2 (1224×1584):
    /// page 1 went from **632 ms to 3,713 ms**, a 5.9× slowdown on a page
    /// carrying a *single* transparency group — so the cost tracked the
    /// paint count, not the group count.
    ///
    /// ⇒ The deferral was right about the risk and wrong about the size.
    /// "Defer the optimisation" is only safe when somebody has multiplied
    /// the per-unit cost by the unit count.
    coverage: Option<Mask>,
    /// The rectangle this buffer has actually been written in, as
    /// `(x0, y0, x1, y1)` with the upper bounds exclusive — or `None` when
    /// nothing has been written at all.
    ///
    /// # ★ Why a group buffer needs this, with the number
    ///
    /// A transparency group gets a **page-sized** child buffer, because its
    /// contents are drawn under the same CTM as its parent and a smaller
    /// buffer would need a translation threaded through every paint site
    /// and every clip mask. That is the right call and it has a
    /// consequence: compositing the child back walked the **whole page**,
    /// per group.
    ///
    /// Measured on the suite combined document at scale 2 (1224×1584,
    /// 1.94 M pixels): page 1 carries **1** group and renders in 330 ms;
    /// page 2 carries **142** and rendered in 3,445 ms. That is ≈ 22 ms
    /// per group, for groups whose artwork is a swatch a few hundred
    /// pixels across.
    ///
    /// A group's result can only be non-transparent where something was
    /// painted, so tracking that rectangle turns an O(page) merge into an
    /// O(mark) one — and `None` means the group painted nothing, which is
    /// then free rather than a full-page scan of zeroes.
    ///
    /// ⇒ Same shape as the per-paint coverage mask this buffer already
    /// learned once today: **page-sized work for mark-sized content**. It
    /// is worth asking of every remaining loop in this module.
    dirty: Option<(u32, u32, u32, u32)>,
    /// One spare child buffer, kept for the next transparency group.
    ///
    /// # ★ Why, with the measurement that forced it
    ///
    /// Every transparency group gets a page-sized child buffer, and at
    /// 1224×1584 that is five `f32` planes — **38.8 MB** — allocated,
    /// zeroed and dropped **per group**. suite page 2 carries 142 groups.
    ///
    /// Measured per-group cost against page area, which is what identified
    /// it (the first hypothesis — that the full-page *merge* was the cost —
    /// was tested by bounding the merge to a dirty rectangle and bought
    /// only 9 %, so it was wrong):
    ///
    /// | scale | page area | ms per group |
    /// |---|---|---:|
    /// | 0.5 | 1× | 2.76 |
    /// | 1.0 | 4× | 4.49 |
    /// | 2.0 | 16× | **16.68** |
    ///
    /// Cost tracks area, sub-linearly — the signature of allocation and
    /// page-faulting rather than of a per-pixel loop. At scale 2 that is
    /// 2.4 s of page 2's 2.66 s.
    ///
    /// # Why ONE spare and not a pool
    ///
    /// Groups on a page are overwhelmingly **siblings** — 142 of them in
    /// sequence, not 142 deep — so a single spare handed back and forth
    /// covers the common case exactly. Nesting still allocates, once per
    /// level, which is `O(depth)` and is what §11.4's own memory
    /// discussion says to expect.
    ///
    /// # What makes reuse cheap, and it is the previous fix
    ///
    /// Handing a buffer back means clearing it, and clearing a page-sized
    /// buffer is the cost being avoided. It is affordable only because
    /// [`CmykBuffer::dirty`] says which rectangle was actually written —
    /// so the clear is `O(mark)`, not `O(page)`. The dirty rectangle
    /// bought little on its own and is what makes this possible.
    spare: Option<Box<Self>>,
}

impl CmykBuffer {
    /// Allocate a transparent buffer of `width × height`.
    ///
    /// # Returns
    ///
    /// `None` if the dimensions are zero, if `width × height` overflows
    /// `usize`, or if the buffer would exceed [`MAX_CMYK_BUFFER_BYTES`].
    /// All three are **refusals, not errors** — see that constant's
    /// documentation for why the caller falls back and discloses rather
    /// than failing the render.
    ///
    /// # ★ The zero fill, and the one thing it must not be read as
    ///
    /// Every plane starts at `0.0`, including alpha. That makes every
    /// pixel **transparent**, whose colour §11.3.2 declares undefined and
    /// which every formula in [`crate::compositor`] multiplies by its own
    /// zero alpha before reading.
    ///
    /// It does **not** make the buffer white, and it must never be reused
    /// as an initialiser for an *opaque* subtractive backdrop: §8.6.4.4
    /// gives `DeviceCMYK` an initial colour of `[0 0 0 1]`, so a zeroed
    /// colorant plane with a **non**-zero alpha would be white paper, and
    /// a luminosity soft mask built over it would be inverted. See the
    /// module documentation's trap 2.
    pub(crate) fn new(
        width: u32,
        height: u32,
        intent: pdfce_core::settings::CmykIntent,
    ) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let n = (width as usize).checked_mul(height as usize)?;
        if n.checked_mul(BYTES_PER_PIXEL)? > MAX_CMYK_BUFFER_BYTES {
            return None;
        }
        Some(Self {
            width,
            height,
            planes: [vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]],
            alpha: vec![0.0; n],
            bridged: 0,
            groups_approximated: 0,
            unbridged_images: 0,
            intent,
            knockout: None,
            // Allocated once, here, and reused by every paint for the life
            // of the buffer.
            coverage: Mask::new(width, height),
            dirty: None,
            spare: None,
        })
    }

    /// Device width in pixels.
    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    /// Device height in pixels.
    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    /// How many pixels reached this buffer through the sRGB bridge.
    ///
    /// See [`CmykBuffer::bridged`] for why this is a disclosure rather
    /// than a shortfall.
    pub(crate) const fn bridged_pixels(&self) -> u64 {
        self.bridged
    }

    /// How many transparency groups could not be composited natively. See
    /// the field's documentation for the two cases it covers.
    pub(crate) const fn groups_approximated(&self) -> u64 {
        self.groups_approximated
    }

    /// Fold a child buffer's disclosure counters into this one.
    ///
    /// A group's child buffer is part of the same page, so its bridged
    /// pixels and its own approximated sub-groups are the page's. Without
    /// this, a page whose every image sits inside a transparency group
    /// would report **zero** bridging — a disclosure that is not merely
    /// incomplete but exactly backwards, since that page is the one most
    /// affected.
    pub(crate) const fn absorb_counters(&mut self, child: &Self) {
        self.bridged += child.bridged;
        self.groups_approximated += child.groups_approximated;
        self.unbridged_images += child.unbridged_images;
    }

    /// How many image brushes could not be bridged at all.
    pub(crate) const fn unbridged_images(&self) -> u64 {
        self.unbridged_images
    }

    /// Record one transparency group that could not be composited
    /// natively in ink.
    pub(crate) const fn note_group_approximated(&mut self) {
        self.groups_approximated += 1;
    }

    /// Record one image brush that reached a paint path with no bridge.
    pub(crate) const fn note_unbridged_image(&mut self) {
        self.unbridged_images += 1;
    }

    /// Read one pixel into the standard's model.
    ///
    /// # Panics
    ///
    /// Never for an `idx` produced from this buffer's own dimensions; the
    /// slice index is bounds-checked by Rust and a caller that violates it
    /// has a bug this function should not paper over.
    #[inline]
    pub(crate) fn pixel(&self, idx: usize) -> PixelCmyk {
        PixelCmyk {
            c: [
                self.planes[0][idx],
                self.planes[1][idx],
                self.planes[2][idx],
                self.planes[3][idx],
            ],
            a: self.alpha[idx],
        }
    }

    /// Write one pixel, clamping into range.
    ///
    /// The clamp is not defensive tidiness: §11.3.6's weighted average is
    /// exact in theory and `f32` in practice, and a blend function such as
    /// `Difference` on values a hair outside `[0, 1]` compounds rather
    /// than settles. Clamping on write means every value this buffer hands
    /// back to a blend function is a legal colorant tint.
    /// Widen the dirty rectangle to include `region`.
    ///
    /// Called by every composite entry point rather than by `set_pixel`,
    /// deliberately: a per-pixel widen would cost a branch and two
    /// comparisons in the innermost loop of the renderer to compute
    /// something the caller already knows as a rectangle.
    fn mark_dirty(&mut self, region: (u32, u32, u32, u32)) {
        let (x0, y0, x1, y1) = region;
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        self.dirty = Some(match self.dirty {
            None => (x0, y0, x1, y1),
            Some((ax0, ay0, ax1, ay1)) => (ax0.min(x0), ay0.min(y0), ax1.max(x1), ay1.max(y1)),
        });
    }

    /// The rectangle worth scanning: what has been written, clamped to the
    /// buffer. `None` means nothing has been written.
    fn dirty_region(&self) -> Option<(u32, u32, u32, u32)> {
        let (x0, y0, x1, y1) = self.dirty?;
        let r = (x0, y0, x1.min(self.width), y1.min(self.height));
        (r.0 < r.2 && r.1 < r.3).then_some(r)
    }

    #[inline]
    pub(crate) fn set_pixel(&mut self, idx: usize, px: PixelCmyk) {
        for i in 0..4 {
            self.planes[i][idx] = px.c[i].clamp(0.0, 1.0);
        }
        self.alpha[idx] = px.a.clamp(0.0, 1.0);
    }

    /// Composite a **solid colorant** through a coverage mask — the
    /// workhorse, and the operation every native CMYK paint is made of.
    ///
    /// `coverage` is a page-sized [`Mask`] rasterised by the *same*
    /// `tiny_skia` call a normal paint would have used, so an edge painted
    /// through this path has identical geometry to one painted through the
    /// sRGB path. `region` is `(x0, y0, x1, y1)` with the upper bounds
    /// exclusive, in device pixels, and exists so a small fill does not
    /// walk the page — the same convention
    /// [`crate::overprint::composite`] uses.
    ///
    /// # The arithmetic, and where it is not
    ///
    /// Per pixel: `α_s = alpha × coverage/255`, then §11.4.4's element
    /// formula via [`composite_element_cmyk`]. **Coverage multiplies alpha
    /// and never the colorant value** — that is what makes anti-aliasing
    /// compose correctly with a subtractive blend, and getting it backwards
    /// produces edges that are the right shape and the wrong colour.
    ///
    /// # Returns
    ///
    /// The number of pixels whose stored alpha or colorants changed, for
    /// the caller's own disclosure counters. Zero is a legitimate answer
    /// (a fully clipped paint) and is not an error.
    pub(crate) fn composite_mask(
        &mut self,
        coverage: &Mask,
        region: (u32, u32, u32, u32),
        colour: [Chan; 4],
        alpha: Chan,
        blend: Blend,
    ) -> u32 {
        debug_assert_eq!(
            coverage.width(),
            self.width,
            "coverage mask and colorant buffer must share a device grid"
        );
        let cov = coverage.data();
        let (x0, y0, x1, y1) = region;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        let alpha = alpha.clamp(0.0, 1.0);
        self.mark_dirty((x0, y0, x1, y1));
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                let c = Chan::from(cov[idx]) / 255.0;
                if c <= 0.0 {
                    continue;
                }
                // ★ COVERAGE IS SHAPE; COVERAGE TIMES OPACITY IS ALPHA.
                // §11.4's `α = f × q`, and the two are handed on
                // separately because §11.4.8 reads `f_s` alone. Collapsing
                // them here would make every knockout group behave as if
                // its elements were opaque — correct on most artwork and
                // wrong exactly where the clause exists.
                let source = PixelCmyk {
                    c: colour,
                    a: alpha * c,
                };
                if self.composite_at(idx, source, c, blend) {
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Composite an **sRGB pixmap** into this buffer — the bridge.
    ///
    /// # Why a bridge exists at all, stated plainly
    ///
    /// Images arrive at a canvas as decoded sRGB texels (`DecodedImage`
    /// holds a `Pixmap`), and shadings evaluate their colour ramp to sRGB
    /// before the pixel loop. Neither can hand this buffer authored
    /// colorants at `Pass 97.1e`, so their source colour is converted with
    /// [`crate::overprint::rgb_to_cmyk`] on the way in.
    ///
    /// That conversion is a **max-GCR** transform chosen for exact
    /// round-tripping rather than for colorimetric accuracy — see its own
    /// documentation. Using it here is §11.6.6's required "convert the
    /// source to the group's space", performed with the only transform
    /// this crate has; it is not a claim that the result is what a press
    /// would print. Every pixel that takes this path is counted
    /// ([`CmykBuffer::bridged_pixels`]) precisely so the approximation is
    /// disclosed rather than assumed away.
    ///
    /// # Parameters
    ///
    /// `src` must share this buffer's device grid. `region` is
    /// `(x0, y0, x1, y1)`, upper bounds exclusive. `alpha` scales the
    /// source's own alpha, exactly as a constant `/ca` would.
    ///
    /// # Returns
    ///
    /// The number of pixels changed.
    pub(crate) fn composite_srgb(
        &mut self,
        src: &Pixmap,
        region: (u32, u32, u32, u32),
        alpha: Chan,
        blend: Blend,
    ) -> u32 {
        debug_assert_eq!(
            src.width(),
            self.width,
            "bridged pixmap and colorant buffer must share a device grid"
        );
        let (x0, y0, x1, y1) = region;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        let alpha = alpha.clamp(0.0, 1.0);
        self.mark_dirty((x0, y0, x1, y1));
        let pixels = src.pixels();
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                let px = pixels[idx];
                let a = Chan::from(px.alpha()) / 255.0;
                if a <= 0.0 {
                    continue;
                }
                // Un-premultiply before converting: `rgb_to_cmyk` is
                // defined on colour, and a premultiplied triple is colour
                // scaled by an alpha that the compositing formula is about
                // to apply again.
                let r = Chan::from(px.red()) / 255.0 / a;
                let g = Chan::from(px.green()) / 255.0 / a;
                let b = Chan::from(px.blue()) / 255.0 / a;
                let source = PixelCmyk {
                    c: crate::overprint::rgb_to_cmyk(r, g, b),
                    a: a * alpha,
                };
                // An image's own alpha is SHAPE, not opacity: §11.6.4.2
                // makes an image's `/SMask` an object-shape input unless
                // `/AIS` says otherwise. So `f_s = a` and `q_s` is the
                // constant alpha, which is the same split
                // `Canvas::fill_image`'s knockout arm already makes.
                if self.composite_at(idx, source, a, blend) {
                    changed += 1;
                }
                self.bridged += 1;
            }
        }
        changed
    }

    /// **Table 149's `CompatibleOverprint`** — §11.7.4.3 — composited
    /// natively, with no colour-space round trip.
    ///
    /// # ★ What this deletes, and it is the reason overprint was listed as
    /// approximate for pdfce's entire life
    ///
    /// [`crate::overprint::composite`] does the same job against an sRGB
    /// pixmap, and to do it at all it must, **per pixel**: un-premultiply,
    /// `rgb_to_cmyk` the backdrop, apply the four rules, `cmyk_to_rgb` the
    /// result, re-premultiply. Its own documentation concedes the problem —
    /// *"the backdrop's component split is reconstructed from the composite
    /// rather than remembered"*. Here the backdrop's component split **is**
    /// remembered, because the planes are the backdrop. The four
    /// [`ComponentRule`](crate::overprint::ComponentRule)s are the same
    /// code, transcribed from Table 149 cell by cell and tested; only their
    /// input improves.
    ///
    /// # ★★ And a convention that becomes correct rather than merely tolerable
    ///
    /// `overprint::composite` treats a fully transparent backdrop pixel as
    /// **white paper**, `(1, 1, 1)` — a deliberate deviation from the rest
    /// of this crate, which since `Pass 97.0a` refuses that convention
    /// because §11.4.7 composites the medium in once at the end.
    ///
    /// In a subtractive buffer the tension disappears. A transparent pixel
    /// holds `[0, 0, 0, 0]` — **no ink** — and no ink *is* white paper.
    /// The two readings coincide, so this function needs no special case
    /// and no deviation: it reads the planes and Table 149 gets the answer
    /// it was written for. That is a small thing arithmetically and a large
    /// one to be able to stop explaining.
    ///
    /// # Alpha, and why overprint raises it
    ///
    /// Overprint **adds ink**; it does not make the sheet more
    /// transparent. So alpha rises toward full by the same `t = coverage ×
    /// alpha` that mixes the colorants, exactly as the sRGB implementation
    /// does — the two must agree, because a document can contain both an
    /// overprinted and a non-overprinted copy of the same mark.
    ///
    /// # Returns
    ///
    /// The number of pixels changed.
    /// §11.7.4.3's composite where the source colour **differs per pixel** —
    /// the shading twin of [`CmykBuffer::composite_overprint`].
    ///
    /// # Why this exists beside its solid-colour sibling
    ///
    /// [`CmykBuffer::composite_overprint`] takes one `source: [Chan; 4]`,
    /// because a filled path has one colour. The entire point of a shading is
    /// that every pixel has a different one, so the source arrives as a
    /// callback returning the pixel's authored colorants and its coverage.
    ///
    /// **`rules` are still computed once by the caller, not per pixel**, and
    /// that is correct rather than an optimisation: for a
    /// `SourceKind::SeparationOrDeviceN` — the case a shading reaches — Table
    /// 149's selection depends only on **which colorants the space names**,
    /// never on their tints. (It *is* tint-dependent for
    /// `DeviceCmykDirect` under `/OPM 1`, which is why that source kind must
    /// not be routed here without revisiting this.)
    ///
    /// # What overprint does here, in one sentence
    ///
    /// A `/DeviceN [/Cyan /Magenta]` shading names two of the four process
    /// colorants, so under overprint `C` and `M` take the source while `Y`
    /// and `K` keep the backdrop — which is how a cyan-to-magenta gradient
    /// over an orange ground comes out green rather than blue.
    pub(crate) fn composite_overprint_varying(
        &mut self,
        region: (u32, u32, u32, u32),
        rules: [crate::overprint::ComponentRule; 4],
        alpha: Chan,
        mut source_at: impl FnMut(u32, u32) -> Option<([Chan; 4], Chan)>,
    ) -> u32 {
        let (x0, y0, x1, y1) = region;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return 0;
        }
        let alpha = alpha.clamp(0.0, 1.0);
        self.mark_dirty((x0, y0, x1, y1));
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let Some((source, coverage)) = source_at(x, y) else {
                    continue;
                };
                let a = alpha * coverage.clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let idx = (y * self.width + x) as usize;
                let before = self.pixel(idx);
                let mut out = before;
                for ch in 0..4 {
                    // Table 149 per colorant, then §11.4.4's ordinary
                    // source-over weighting on the value the rule selected.
                    // The rule decides WHICH tint competes; alpha decides how
                    // much of it lands. Collapsing the two would make a
                    // `Backdrop` component fade toward zero as alpha fell,
                    // which is the opposite of preserving it.
                    let target = rules[ch].apply(before.c[ch], source[ch]);
                    out.c[ch] = target.mul_add(a, before.c[ch] * (1.0 - a));
                }
                out.a = a.mul_add(1.0 - before.a, before.a);
                self.set_pixel(idx, out);
                if out != before {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub(crate) fn composite_overprint(
        &mut self,
        coverage: &Mask,
        region: (u32, u32, u32, u32),
        rules: [crate::overprint::ComponentRule; 4],
        source: [Chan; 4],
        alpha: Chan,
    ) -> u32 {
        debug_assert_eq!(coverage.width(), self.width);
        let cov = coverage.data();
        let (x0, y0, x1, y1) = region;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        let alpha = alpha.clamp(0.0, 1.0);
        self.mark_dirty((x0, y0, x1, y1));
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                let c = Chan::from(cov[idx]) / 255.0;
                if c <= 0.0 {
                    continue;
                }
                let t = c * alpha;
                let before = self.pixel(idx);
                let mut out = [0.0_f32; 4];
                for i in 0..4 {
                    out[i] = rules[i].apply(before.c[i], source[i]).clamp(0.0, 1.0);
                }
                // Interpolate between the backdrop and the overprint result
                // by `t`, so partial coverage and partial alpha behave the
                // way every other paint in the renderer does.
                let mut mixed = [0.0_f32; 4];
                for i in 0..4 {
                    mixed[i] = t.mul_add(out[i] - before.c[i], before.c[i]);
                }
                let after = PixelCmyk {
                    c: mixed,
                    a: t.mul_add(1.0 - before.a, before.a),
                };
                if after != before {
                    changed += 1;
                }
                self.set_pixel(idx, after);
            }
        }
        changed
    }

    /// This buffer's conversion intent, so a child buffer can be built to
    /// match its parent.
    ///
    /// Two buffers in one page that converted differently would produce a
    /// seam at every group boundary — see [`CmykBuffer::intent`]'s field
    /// documentation for why that is the failure mode worth designing
    /// against.
    pub(crate) const fn intent(&self) -> pdfce_core::settings::CmykIntent {
        self.intent
    }

    /// Multiply this buffer's alpha by a soft mask — §11.4.5.
    ///
    /// The subtractive twin of `canvas::apply_mask`, and it is **simpler**
    /// rather than merely different: `tiny_skia`'s storage is
    /// premultiplied, so the additive version has to scale the colour and
    /// the alpha together to leave the un-premultiplied colour unchanged.
    /// This buffer stores un-premultiplied colour, so scaling the alpha
    /// *is* the whole operation and the colorants are untouched by
    /// construction — which is what "the mask changes how much of the
    /// group you see, not what colour it is" means arithmetically.
    ///
    /// §11.6.4.1's `/AIS` split applies here exactly as it does there: the
    /// mask value is the *opacity* under the default `/AIS false`, so it
    /// scales `α_s` and leaves shape alone. `/AIS true` is not yet
    /// distinguished, in either implementation.
    pub(crate) fn apply_mask(&mut self, mask: &Mask) {
        // Only where something was painted: masking transparency leaves it
        // transparent, so the rest of the page is a scan of zeroes.
        let Some((x0, y0, x1, y1)) = self.dirty_region() else {
            return;
        };
        let data = mask.data();
        for y in y0..y1 {
            let row = (y * self.width) as usize;
            for x in x0 as usize..x1 as usize {
                let i = row + x;
                let m8 = data[i];
                if m8 == u8::MAX || self.alpha[i] <= 0.0 {
                    continue;
                }
                self.alpha[i] *= Chan::from(m8) / 255.0;
            }
        }
    }

    /// Composite a child buffer's **result** into this one as a single
    /// object — §11.4.5.
    ///
    /// # ★ Why this exists rather than reusing the sRGB bridge
    ///
    /// Because the bridge is a **round trip**, and a round trip through a
    /// group is what makes a group's contents a different colour from
    /// identical contents painted outside it. That is not a subtle
    /// artefact on the suite transparency patches — it is precisely the
    /// mechanism the trap X detects, since the X is drawn inside the group
    /// and its surround outside it, authored to match only if both survive
    /// to the same value.
    ///
    /// With a child buffer of the same type there is no conversion at all:
    /// the group's colorants are the group's colorants, `alpha` scales its
    /// result per §11.4.5, and [`composite_element_cmyk`] applies §11.4.4's
    /// formula in the space both buffers already share.
    ///
    /// # What this does NOT do
    ///
    /// It does not perform §11.4.4's **backdrop removal**, because a child
    /// built by [`CmykBuffer::new`] starts transparent — `α_0 = 0` — which
    /// is §11.4.5's isolated case, where the correction is identically
    /// zero. A non-isolated CMYK group would need the removal and is not
    /// yet implemented; see `Canvas::group`'s `Cmyk` arm, where the
    /// approximation is named and counted.
    ///
    /// # Returns
    ///
    /// The number of pixels changed.
    pub(crate) fn composite_buffer(&mut self, child: &Self, alpha: Chan, blend: Blend) -> u32 {
        debug_assert_eq!(child.width, self.width);
        debug_assert_eq!(child.height, self.height);
        let alpha = alpha.clamp(0.0, 1.0);
        // ★ ONLY WHERE THE CHILD WAS ACTUALLY PAINTED. A group's result is
        // transparent everywhere else by construction, and
        // `composite_element_cmyk` of a transparent source is the identity
        // -- so the rest of the page was being read, tested and skipped,
        // once per group. See the `dirty` field for the measurement.
        let Some((x0, y0, x1, y1)) = child.dirty_region() else {
            return 0;
        };
        self.mark_dirty((x0, y0, x1, y1));
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                let mut source = child.pixel(idx);
                if source.a <= 0.0 {
                    continue;
                }
                // A group's result has shape too, and it is the group's own
                // `f_g` rather than its alpha. `alpha` here is §11.4.5's outer
                // constant opacity, which scales `α` and leaves shape alone —
                // so the shape handed on is the child's UNSCALED alpha, which
                // for a child built by `CmykBuffer::new` is `f_g` exactly
                // (nothing has scaled it yet).
                let shape = source.a;
                source.a *= alpha;
                if self.composite_at(idx, source, shape, blend) {
                    changed += 1;
                }
            }
        }
        changed
    }

    /// The buffer's current content as a **premultiplied sRGB pixmap with
    /// its alpha intact** — a backdrop, not a finished page.
    ///
    /// # Why this exists at all, given that the round trip is what the
    /// whole module is here to delete
    ///
    /// Because two nested-drawing constructs read their backdrop rather
    /// than merely painting over it, and both would otherwise be handed
    /// **nothing**:
    ///
    /// - a **knockout** group (§11.4.6) composites every element against
    ///   the group's *initial* backdrop;
    /// - a **non-isolated** group (§11.4.4) composites its contents over a
    ///   copy of the backdrop and removes it again afterwards.
    ///
    /// Their interiors run in sRGB on a subtractive page (see
    /// `Canvas::group`), so the backdrop they read has to be sRGB too.
    /// Handing them a transparent buffer instead is not a smaller error —
    /// it is a **larger** one, and it is measured: routing a subtractive
    /// page's knockout groups over a transparent initial backdrop took
    /// suite `PCS1_161` from **2 traps to 15**, undoing `Pass 97.0c`'s
    /// knockout implementation on exactly the pages that test it.
    ///
    /// So the rule this function encodes is: *a round trip is worse than
    /// no round trip and better than no backdrop.* Every pixel it converts
    /// is counted as bridged, because that is what it is.
    ///
    /// # ★★ WHICH CONVERSION, AND WHY IT IS NOT THE CALIBRATED ONE
    ///
    /// pdfce has **two** `DeviceCMYK` → sRGB transforms and they are for
    /// different jobs:
    ///
    /// | | transform | property |
    /// |---|---|---|
    /// | [`CmykBuffer::to_srgb_over_white`] | `pdfce_core::color::cmyk_to_srgb_with` | **accurate** — a lattice fitted against a reference renderer |
    /// | **here** | [`crate::overprint::cmyk_to_rgb`] | **exactly invertible** — max-GCR, the precise inverse of `rgb_to_cmyk` |
    ///
    /// The collapse is a **terminal** conversion: nothing comes back, so
    /// accuracy is the only criterion. This one is one leg of a **round
    /// trip** — out to the group's sRGB interior and back through
    /// [`CmykBuffer::composite_srgb`] — so *invertibility* is the only
    /// criterion, and accuracy is irrelevant because the value never
    /// reaches a screen in this form.
    ///
    /// ★ Mixing them is not a small error, and it was measured: converting
    /// the backdrop with the calibrated lattice and converting the result
    /// back with max-GCR left suite `PCS1_161` at **10 traps** against a
    /// pre-Pass baseline of **2**, because the two transforms are not
    /// inverses and every knockout element accumulated the difference.
    /// Using the invertible pair on both legs is what makes an untouched
    /// backdrop pixel survive the trip unchanged.
    ///
    /// # Not [`CmykBuffer::to_srgb_over_white`]
    ///
    /// That one is §11.4.7's **final** composite and returns an opaque
    /// page. A backdrop that arrived opaque would make every group think
    /// it was painting over a full sheet of white, which is the
    /// `α_b = 1.0` mistake `Pass 97.0a` removed from three separate
    /// functions in this crate. Alpha is preserved here precisely so that
    /// §11.4.4's formulas see the transparency they are written against.
    ///
    /// # Returns
    ///
    /// `None` if a `Pixmap` of this buffer's dimensions cannot be
    /// allocated.
    pub(crate) fn snapshot_srgb_backdrop(&mut self) -> Option<Pixmap> {
        let mut out = Pixmap::new(self.width, self.height)?;
        let dst = out.pixels_mut();
        for (idx, slot) in dst.iter_mut().enumerate() {
            let a = self.alpha[idx].clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            self.bridged += 1;
            // ★★ THE NAIVE TRANSFORM, DELIBERATELY, AND NOT THE CALIBRATED
            // ONE. See this function's "Which conversion" section: this is
            // one leg of a ROUND TRIP and the return leg is
            // `overprint::rgb_to_cmyk`, of which this is the exact inverse.
            let (r, g, bl) = crate::overprint::cmyk_to_rgb([
                self.planes[0][idx],
                self.planes[1][idx],
                self.planes[2][idx],
                self.planes[3][idx],
            ]);
            let rgb = [r, g, bl];
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let q = |v: f32| (v.clamp(0.0, 1.0) * a * 255.0).round() as u8;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let a8 = (a * 255.0).round() as u8;
            if let Some(px) =
                tiny_skia::PremultipliedColorU8::from_rgba(q(rgb[0]), q(rgb[1]), q(rgb[2]), a8)
            {
                *slot = px;
            }
        }
        Some(out)
    }

    /// A cleared, page-sized child buffer for a transparency group —
    /// reused from the spare slot when one is there.
    ///
    /// Returns `None` only if no spare exists and a fresh allocation
    /// fails, which the caller treats exactly as it treats any other
    /// failed buffer allocation: the group does not run.
    pub(crate) fn take_child(&mut self) -> Option<Self> {
        match self.spare.take() {
            Some(b) => Some(*b),
            None => Self::new(self.width, self.height, self.intent),
        }
    }

    /// Does this buffer hold **any** marked pixel — i.e. is there a
    /// backdrop for a non-isolated group to see?
    ///
    /// # Why this is not just `dirty_region().is_some()`
    ///
    /// The dirty rectangle records where something was *written*, and a
    /// write of a fully transparent pixel counts. §11.4.5's substitution is
    /// about the backdrop being transparent, not about it being untouched:
    /// *"an isolated group's initial backdrop is transparent"*, so a
    /// backdrop that was written and is transparent anyway IS the isolated
    /// case and must take the one-walk path. Testing the rectangle alone
    /// would send it down the two-walk path, where the removal then divides
    /// by an `α_gn` it did not need to and returns the same answer more
    /// slowly — correct, but for no reason, on every page whose group sits
    /// over blank paper.
    ///
    /// The additive twin is `p.pixels().iter().any(|px| px.alpha() > 0)`,
    /// and this is the same test over the alpha plane, restricted to the
    /// only region that can be non-zero.
    ///
    /// # Cost
    ///
    /// One pass over the dirty rectangle's alpha, short-circuiting on the
    /// first marked pixel — so it is `O(1)` on the common case of a group
    /// over existing content and `O(area)` only when the answer is `false`.
    pub(crate) fn backdrop_present(&self) -> bool {
        let Some((x0, y0, x1, y1)) = self.dirty_region() else {
            return false;
        };
        (y0..y1).any(|y| {
            let row = (y * self.width) as usize;
            self.alpha[row + x0 as usize..row + x1 as usize]
                .iter()
                .any(|a| *a > 0.0)
        })
    }

    /// A child buffer **pre-loaded with this buffer's own content** — the
    /// initial backdrop a non-isolated group (§11.4.4) is entitled to see.
    ///
    /// # Why this exists next to [`CmykBuffer::take_child`] rather than
    /// instead of it
    ///
    /// A non-isolated group is rendered **twice**, and the two runs need
    /// different starting states. Run 1 starts transparent, so its alpha is
    /// `α_gn` — the group's *own* accumulated alpha, with no backdrop in
    /// it — which is precisely the divisor §11.4.4's removal needs. Run 2
    /// starts from the backdrop, so its colour is the group composited
    /// *over* that backdrop, which is what the group's elements actually
    /// saw. Neither run alone is the answer; the answer is
    /// [`CmykBuffer::composite_non_isolated`] of the two.
    ///
    /// `take_child` gives run 1's buffer. This gives run 2's.
    ///
    /// # Why only the dirty rectangle is copied
    ///
    /// Outside it the parent is untouched, and an untouched buffer is all
    /// zeros — `α = 0`, every colorant `0` — which is byte-for-byte what a
    /// child returned by [`CmykBuffer::give_back_child`] already holds. So
    /// copying the rest would write zeros over zeros. This is not only an
    /// optimisation: it keeps the child's dirty rectangle equal to the
    /// parent's rather than the whole page, so the merge below and the
    /// eventual clear both stay proportional to what was actually painted.
    ///
    /// The additive twin (`Canvas::group`'s `Paint` arm) clones the whole
    /// `Pixmap` instead. That is not a disagreement — a `Pixmap` carries no
    /// dirty rectangle to preserve, so there is nothing there to be careful
    /// about.
    ///
    /// # Returns
    ///
    /// `None` on the same condition as [`CmykBuffer::take_child`]: no spare
    /// and no allocation. The caller falls back to the isolated
    /// approximation and counts it, rather than dropping the group.
    pub(crate) fn child_from_backdrop(&mut self) -> Option<Self> {
        let mut child = self.take_child()?;
        let Some((x0, y0, x1, y1)) = self.dirty_region() else {
            // Nothing painted, so the backdrop IS transparent, and a
            // transparent initial backdrop is §11.4.5's isolated case. The
            // cleared child is already correct; the caller's own
            // `backdrop_present` test normally means we never get here.
            return Some(child);
        };
        for y in y0..y1 {
            let row = (y * self.width) as usize;
            let (a, b) = (row + x0 as usize, row + x1 as usize);
            for plane in 0..4 {
                child.planes[plane][a..b].copy_from_slice(&self.planes[plane][a..b]);
            }
            child.alpha[a..b].copy_from_slice(&self.alpha[a..b]);
        }
        // The copied span counts as written: `give_back_child` clears only
        // the dirty rectangle, and a child handed back with backdrop still
        // in it would hand that backdrop to the NEXT group as if it were
        // its own content. Silent, and wrong in a way that grows with how
        // many groups a page has.
        child.mark_dirty((x0, y0, x1, y1));
        Some(child)
    }

    /// **§11.4.4's backdrop removal**, then §11.4.4's element formula —
    /// the subtractive twin of `canvas::composite_non_isolated_group`.
    ///
    /// `self` is the parent buffer, and on entry it still holds the frozen
    /// initial backdrop: nothing has been composited into it since the
    /// group began. That is what makes it readable as `C_0` here.
    ///
    /// * `iso` — run 1's buffer (started transparent). **Only its alpha is
    ///   read**, and that alpha is `α_gn`.
    /// * `nis` — run 2's buffer (started from the backdrop): the group's
    ///   colour accumulated *over* that backdrop.
    /// * `alpha` — §11.4.5's constant alpha at the `Do`.
    /// * `mask` — the soft mask's data, or `None`.
    ///
    /// # ★ The removal divides by the UNMASKED `α_gn`
    ///
    /// The soft mask is not part of the group's own accumulation — §11.4.5
    /// applies it to the *finished* result — so masking before the removal
    /// would divide by the wrong number and shift the group's **colour**,
    /// not merely its opacity. The mask is therefore applied to `source.a`
    /// after [`remove_backdrop_cmyk`] has run, never before. This is the
    /// same ordering the additive path documents, and it is the one an
    /// implementation gets wrong by calling an existing `apply_mask` on the
    /// child buffer first because that is the shorter line of code.
    ///
    /// # Why the walk is over `iso`'s dirty rectangle
    ///
    /// Where the group marked nothing, `α_gn` is zero and §11.4.4's result
    /// is unreachable whatever colour `nis` holds there — `nis` holds the
    /// backdrop unchanged, and compositing the backdrop onto itself is the
    /// one operation §11.4.3 forbids (*"the backdrop's contribution … shall
    /// be applied only once"*). Walking `nis`'s rectangle instead would do
    /// exactly that over the whole backdrop, which is why the choice of
    /// rectangle here is a correctness question and not a performance one.
    ///
    /// # Returns
    ///
    /// The number of pixels changed.
    pub(crate) fn composite_non_isolated(
        &mut self,
        iso: &Self,
        nis: &Self,
        alpha: Chan,
        blend: Blend,
        mask: Option<&[u8]>,
    ) -> u32 {
        debug_assert_eq!(iso.width, self.width);
        debug_assert_eq!(nis.width, self.width);
        let alpha = alpha.clamp(0.0, 1.0);
        let Some((x0, y0, x1, y1)) = iso.dirty_region() else {
            return 0;
        };
        self.mark_dirty((x0, y0, x1, y1));
        let mut changed = 0_u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * self.width + x) as usize;
                let agn = iso.alpha[idx];
                if agn <= 0.0 {
                    continue;
                }
                let backdrop = self.pixel(idx);
                let over = nis.pixel(idx);
                let c = remove_backdrop_cmyk(over, backdrop, agn);
                let m = mask.map_or(1.0, |d| d.get(idx).map_or(1.0, |v| Chan::from(*v) / 255.0));
                // Shape is the group's own `f_g`, unscaled by the outer
                // constant alpha -- §11.4.5 scales alpha and leaves shape
                // alone. `composite_at` ignores shape outside a knockout
                // group, but passing the scaled value would be a latent
                // bug the moment this buffer ever is one.
                let source = PixelCmyk {
                    c,
                    a: agn * alpha * m,
                };
                if self.composite_at(idx, source, agn, blend) {
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Hand a finished child buffer back for the next group to use.
    ///
    /// Clears **only the rectangle the child actually wrote**, which is
    /// what makes reuse cheaper than reallocation, and resets every piece
    /// of per-group state so the next group cannot inherit any of it:
    ///
    /// - the **dirty rectangle**, or the next group would clear more than
    ///   it wrote and report a merge region larger than its own marks;
    /// - the **knockout planes**, which are `Option` and whose presence
    ///   changes which clause every composite obeys (§11.4.8 instead of
    ///   §11.4.4);
    /// - the **disclosure counters**, which the caller has already folded
    ///   into the parent with `absorb_counters` and which would otherwise
    ///   be counted once per subsequent sibling.
    ///
    /// The child's own spare is dropped rather than chained, so the memory
    /// held between groups stays `O(depth)`.
    pub(crate) fn give_back_child(&mut self, mut child: Self) {
        if let Some((x0, y0, x1, y1)) = child.dirty_region() {
            for y in y0..y1 {
                let row = (y * child.width) as usize;
                let (a, b) = (row + x0 as usize, row + x1 as usize);
                for plane in &mut child.planes {
                    plane[a..b].fill(0.0);
                }
                child.alpha[a..b].fill(0.0);
            }
        }
        child.dirty = None;
        child.knockout = None;
        child.bridged = 0;
        child.groups_approximated = 0;
        child.unbridged_images = 0;
        child.spare = None;
        self.spare = Some(Box::new(child));
    }

    /// Borrow the reusable coverage mask, leaving `None` behind.
    ///
    /// Take/put rather than a `&mut` accessor because the caller needs the
    /// mask **and** the buffer mutably at the same time — it rasterises
    /// into the first and composites into the second — and those are two
    /// mutable borrows of one struct. Moving the mask out for the duration
    /// is the cheap, obvious way to say that, and it costs an `Option`
    /// discriminant.
    ///
    /// A caller that takes must put back; one that does not merely makes
    /// the next paint allocate its own, which is the pre-fix behaviour and
    /// is slow rather than wrong.
    pub(crate) fn take_coverage(&mut self) -> Option<Mask> {
        self.coverage.take()
    }

    /// Return the coverage mask borrowed by [`CmykBuffer::take_coverage`].
    pub(crate) fn put_coverage(&mut self, mask: Mask) {
        self.coverage = Some(mask);
    }

    /// Turn a fresh buffer into a **knockout group accumulator** over
    /// `initial` — ISO 32000-1 §11.4.6, §11.4.8.
    ///
    /// # The initialisation, which is where the clause is easy to misread
    ///
    /// §11.4.8 initialises `C_0 = C_b`, `α_0 = α_b`, and — crucially —
    /// `f_g0 = α_g0 = 0` **unconditionally**, isolated or not. So the
    /// accumulator *starts as the backdrop* while the group's own alpha
    /// and shape start at zero, and the whole of the isolation difference
    /// lives in the value of `C_b`/`α_b` rather than in a second branch.
    /// That is the single most useful structural fact in the clause and it
    /// is stated nowhere in it.
    ///
    /// Pass a transparent `initial` for an isolated knockout group; pass a
    /// copy of the parent's current content for a non-isolated one.
    ///
    /// # Returns
    ///
    /// `None` if the extra planes cannot be allocated.
    pub(crate) fn into_knockout(mut self, initial: &Self) -> Option<Self> {
        debug_assert_eq!(initial.width, self.width);
        debug_assert_eq!(initial.height, self.height);
        let n = (self.width as usize).checked_mul(self.height as usize)?;
        // C_0 = C_b, α_0 = α_b: the accumulator IS the backdrop at element
        // zero. Copying rather than referencing because the backdrop must
        // survive every element while the accumulator is overwritten by
        // each one.
        self.planes = initial.planes.clone();
        self.alpha = initial.alpha.clone();
        // The accumulator STARTS as the backdrop, so everything the
        // backdrop touched is already written here.
        self.dirty = initial.dirty;
        self.knockout = Some(Box::new(KnockoutPlanes {
            initial: initial.planes.clone(),
            initial_alpha: initial.alpha.clone(),
            group_alpha: vec![0.0; n],
            group_shape: vec![0.0; n],
        }));
        Some(self)
    }

    /// The knockout group's **result**: §11.4.4's backdrop removal applied,
    /// alpha replaced by the group's own `α_g`.
    ///
    /// # Why the alpha is replaced rather than kept
    ///
    /// Because the accumulator's `α_i` includes the backdrop that was
    /// composited into it at element zero, and the parent is about to
    /// composite this result **onto that same backdrop again**. §11.4.3
    /// states the requirement — *"the backdrop's contribution … shall be
    /// applied only once"* — and §11.4.4's `C_n + (C_n − C_0)·(α_0/α_gn −
    /// α_0)` together with `α = α_gn` is how it is met. Returning `α_i`
    /// here instead is the double-count, and it darkens every non-isolated
    /// knockout group by exactly its own backdrop.
    ///
    /// # Returns
    ///
    /// An ordinary (non-knockout) buffer the caller can composite with
    /// [`CmykBuffer::composite_buffer`]. `self` is consumed because the
    /// accumulator is meaningless afterwards.
    #[must_use]
    pub(crate) fn finish_knockout(mut self) -> Self {
        let Some(ko) = self.knockout.take() else {
            return self;
        };
        // Backdrop removal only where the group was written; elsewhere
        // `α_g` is zero and the correction is the identity.
        let Some((x0, y0, x1, y1)) = self.dirty_region() else {
            return self;
        };
        let width = self.width;
        for idx in (y0..y1).flat_map(move |y| {
            let row = y * width;
            (x0..x1).map(move |x| (row + x) as usize)
        }) {
            let ag = ko.group_alpha[idx];
            let accum = self.pixel(idx);
            let initial = PixelCmyk {
                c: [
                    ko.initial[0][idx],
                    ko.initial[1][idx],
                    ko.initial[2][idx],
                    ko.initial[3][idx],
                ],
                a: ko.initial_alpha[idx],
            };
            let c = remove_backdrop_cmyk(accum, initial, ag);
            self.set_pixel(idx, PixelCmyk { c, a: ag });
        }
        self
    }

    /// Composite one element at `idx`, dispatching between §11.4.4 and
    /// §11.4.8.
    ///
    /// `shape` is the element's coverage; `source.a` is that coverage
    /// times its constant alpha. In the ordinary case the shape is unused —
    /// §11.4.4 never reads it — and in the knockout case both are read,
    /// separately, which is the entire reason they are two parameters.
    ///
    /// Returning `bool` rather than writing unconditionally lets the
    /// callers keep their changed-pixel tallies without each one repeating
    /// the dispatch.
    fn composite_at(&mut self, idx: usize, source: PixelCmyk, shape: Chan, blend: Blend) -> bool {
        let before = self.pixel(idx);
        let after = if let Some(ko) = self.knockout.as_deref_mut() {
            let initial = PixelCmyk {
                c: [
                    ko.initial[0][idx],
                    ko.initial[1][idx],
                    ko.initial[2][idx],
                    ko.initial[3][idx],
                ],
                a: ko.initial_alpha[idx],
            };
            let (px, ag) = composite_element_knockout_cmyk(
                initial,
                before,
                source,
                shape,
                ko.group_alpha[idx],
                blend,
            );
            ko.group_alpha[idx] = ag;
            // f_gi = Union(f_g(i−1), f_si) — the shape recurrence, which
            // §11.4.8 gives the same form as the alpha one with `f` in
            // place of `α`.
            let f_prev = ko.group_shape[idx];
            ko.group_shape[idx] = crate::compositor::union_(f_prev, shape.clamp(0.0, 1.0));
            px
        } else {
            composite_element_cmyk(before, source, blend)
        };
        self.set_pixel(idx, after);
        after != before
    }

    /// **§11.4.7's collapse**: convert to sRGB, then composite over the
    /// white medium — in that order.
    ///
    /// # ★★ The order is the whole point of this function
    ///
    /// §11.4.7 requires the page group's result be converted to the
    /// device's native colour space **before being composited with the
    /// context-dependent backdrop**. So:
    ///
    /// ```text
    /// C_srgb = cmyk_to_srgb(C_g)                 // first
    /// C_out  = (1 − α_g) × White + α_g × C_srgb  // second
    /// ```
    ///
    /// and **not** the reverse. The reverse — flatten onto CMYK white
    /// (`[0,0,0,0]`, no ink) and then convert — is the intuitive order and
    /// is wrong, because the conversion is non-affine: it is a fitted
    /// lattice with clamping, so it does not commute with a linear
    /// interpolation. Both orders produce something that looks like a
    /// page; only one is conformant.
    ///
    /// A worked number using the standard's *own* crude §10.4.2.5
    /// conversion, whose `min()` supplies the non-affinity: a 50 % Normal
    /// composite of `C = M = Y = K = 0.9` over paper white gives
    /// `0.100` per channel composited-then-converted (8-bit `25`) against
    /// `0.500` converted-then-composited (8-bit `128`). The worst
    /// single-channel divergence over 4 × 10⁵ random CMYK pairs is
    /// `0.459` — 8-bit `0` against `117`.
    ///
    /// # The intent the standard asks for, and the knob pdfce actually has
    ///
    /// ISO 32000-2 §11.4.7 (absent from 2008) requires this conversion use
    /// **`RelativeColorimetric`** unless the processor has an
    /// implementation-dependent way of specifying otherwise, and leaves
    /// black point compensation implementation-dependent. `intent` is
    /// pdfce's own [`pdfce_core::settings::CmykIntent`] — the setting the
    /// operator can already change — which *is* that
    /// implementation-dependent way, and its default is the lattice fitted
    /// against a reference renderer rather than a naive formula.
    ///
    /// ★ **Do not read `CmykIntent` as an ICC rendering intent.** It names a
    /// fitted lookup table (`calibrated` / `neutral_black` / `naive`), not a
    /// colorimetric mapping, and pdfce carries **no** PDF rendering intent at
    /// all — `/RI` in an `/ExtGState` is never read and the `ri` operator is
    /// an explicit no-op. That gap is `docs/ROADMAP.md`'s own backlog item,
    /// and it is the reason no table here can reach a gamut clamp: a clamp
    /// is a discontinuity at the gamut boundary and an interpolated lattice
    /// smooths across it, whichever way it is fitted.
    ///
    /// # Returns
    ///
    /// `None` only if a `Pixmap` of this buffer's dimensions cannot be
    /// allocated, which cannot happen for a buffer that already exists at
    /// those dimensions but is propagated rather than unwrapped because
    /// `Pixmap::new` is the authority on its own invariants.
    pub(crate) fn to_srgb_over_white(&self) -> Option<Pixmap> {
        let mut out = Pixmap::new(self.width, self.height)?;
        let dst = out.pixels_mut();
        for (idx, slot) in dst.iter_mut().enumerate() {
            let a = self.alpha[idx].clamp(0.0, 1.0);
            // ★ Step one: convert the group's colour, in the group's
            // space, to the device's. This happens for EVERY pixel,
            // including fully transparent ones, because the media
            // composite below needs a device-space colour to interpolate
            // toward -- and for a transparent pixel that colour is
            // multiplied by zero anyway, so the value is free to be
            // whatever the undefined colour converts to.
            let rgb = pdfce_core::color::cmyk_to_srgb_with(
                self.intent,
                self.planes[0][idx],
                self.planes[1][idx],
                self.planes[2][idx],
                self.planes[3][idx],
            );
            // ★ Step two, and ONLY now: §11.4.7's media composite, in the
            // DESTINATION space. White is 1.0 per channel here because
            // this is sRGB; in CMYK it would have been zero ink, and
            // performing this step there is the defect this ordering
            // exists to prevent.
            let over_white = |c: f32| a.mul_add(c, 1.0 - a);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            // The page is opaque once composited onto the medium: §11.4.7
            // composites the page group onto an opaque white backdrop, and
            // an opaque backdrop yields an opaque result. Emitting the
            // group's own alpha here instead would hand a downstream
            // consumer a page that is transparent where the artwork is
            // thin, which is not what a sheet of paper does.
            if let Some(px) = tiny_skia::PremultipliedColorU8::from_rgba(
                q(over_white(rgb[0])),
                q(over_white(rgb[1])),
                q(over_white(rgb[2])),
                255,
            ) {
                *slot = px;
            }
        }
        Some(out)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use pdfce_core::settings::CmykIntent;

    /// A page-sized coverage mask that is fully covered inside `region`.
    fn full_mask(w: u32, h: u32) -> Mask {
        let mut m = Mask::new(w, h).unwrap();
        for b in m.data_mut() {
            *b = 255;
        }
        m
    }

    /// Paint one solid colorant over the whole of `b`, the way a content
    /// stream element would, so a test can build a backdrop or a group
    /// interior without going through the interpreter.
    fn paint_all(b: &mut CmykBuffer, c: [Chan; 4], alpha: Chan, blend: Blend) {
        let (w, h) = (b.width, b.height);
        let m = full_mask(w, h);
        b.composite_mask(&m, (0, 0, w, h), c, alpha, blend);
    }

    /// ★★ THE IDENTITY THE THREE-WAY TEST IN `Canvas::group` RESTS ON.
    ///
    /// `Pass 97.1g` skips the second content walk whenever the group's
    /// interior is backdrop-INdependent (§11.4.4 NOTE 2), on the claim that
    /// isolated and non-isolated composition then agree **exactly**. That
    /// claim is load-bearing and cheap to get wrong, so it is asserted here
    /// rather than trusted: an interior painted entirely with `Normal` must
    /// give the same page whether it was composed in one walk or two.
    ///
    /// If this ever fails, the shortcut in `Canvas::group` is unsound and
    /// the renderer has been silently substituting isolated semantics on
    /// every ordinary group over a non-empty backdrop -- which is exactly
    /// the defect this Pass exists to end, reintroduced through its own
    /// optimisation.
    #[test]
    fn a_normal_only_interior_makes_one_walk_and_two_walks_agree() {
        let intent = CmykIntent::default();
        let backdrop = [0.6, 0.1, 0.0, 0.05];
        let inner = [0.0, 0.7, 0.3, 0.0];

        // --- one walk: the isolated route -------------------------------
        let mut one = CmykBuffer::new(4, 4, intent).unwrap();
        paint_all(&mut one, backdrop, 1.0, Blend::Normal);
        let mut iso = one.take_child().unwrap();
        paint_all(&mut iso, inner, 0.5, Blend::Normal);
        one.composite_buffer(&iso, 0.8, Blend::Normal);

        // --- two walks: the non-isolated route ---------------------------
        let mut two = CmykBuffer::new(4, 4, intent).unwrap();
        paint_all(&mut two, backdrop, 1.0, Blend::Normal);
        let mut iso2 = two.take_child().unwrap();
        paint_all(&mut iso2, inner, 0.5, Blend::Normal);
        let mut nis = two.child_from_backdrop().unwrap();
        paint_all(&mut nis, inner, 0.5, Blend::Normal);
        two.composite_non_isolated(&iso2, &nis, 0.8, Blend::Normal, None);

        for idx in 0..16 {
            let a = one.pixel(idx);
            let b = two.pixel(idx);
            for ch in 0..4 {
                assert!(
                    (a.c[ch] - b.c[ch]).abs() < 1e-4,
                    "colorant {ch} at {idx}: one-walk {} vs two-walk {}",
                    a.c[ch],
                    b.c[ch]
                );
            }
            assert!(
                (a.a - b.a).abs() < 1e-4,
                "alpha at {idx}: one-walk {} vs two-walk {}",
                a.a,
                b.a
            );
        }
    }

    /// The complementary half, and the reason this pair is two tests
    /// rather than one: the identity above would also hold if
    /// `composite_non_isolated` were quietly doing nothing at all.
    ///
    /// With a backdrop-READING interior -- here `Multiply` inside the
    /// group -- the two routes MUST differ, because that is the entire
    /// content of §11.4.4 NOTE 2. A test that only asserts agreement
    /// cannot tell a correct removal from an absent one (`R162`: could
    /// this ever have come out false?).
    #[test]
    fn a_blending_interior_makes_the_two_routes_differ() {
        let intent = CmykIntent::default();
        let backdrop = [0.6, 0.1, 0.0, 0.05];
        let inner = [0.0, 0.7, 0.3, 0.0];

        let mut one = CmykBuffer::new(2, 2, intent).unwrap();
        paint_all(&mut one, backdrop, 1.0, Blend::Normal);
        let mut iso = one.take_child().unwrap();
        paint_all(&mut iso, inner, 1.0, Blend::Multiply);
        one.composite_buffer(&iso, 1.0, Blend::Normal);

        let mut two = CmykBuffer::new(2, 2, intent).unwrap();
        paint_all(&mut two, backdrop, 1.0, Blend::Normal);
        let mut iso2 = two.take_child().unwrap();
        paint_all(&mut iso2, inner, 1.0, Blend::Multiply);
        let mut nis = two.child_from_backdrop().unwrap();
        paint_all(&mut nis, inner, 1.0, Blend::Multiply);
        two.composite_non_isolated(&iso2, &nis, 1.0, Blend::Normal, None);

        let a = one.pixel(0);
        let b = two.pixel(0);
        let delta: Chan = (0..4).map(|i| (a.c[i] - b.c[i]).abs()).sum();
        assert!(
            delta > 1e-3,
            "isolated and non-isolated must differ when the interior blends; \
             one-walk {:?} two-walk {:?}",
            a.c,
            b.c
        );
    }

    /// A child seeded from the backdrop carries the backdrop's marks, and
    /// `give_back_child` must clear ALL of them.
    ///
    /// Not a hypothetical: `give_back_child` clears only the dirty
    /// rectangle, so `child_from_backdrop` marking that rectangle is the
    /// only thing standing between the next group and a buffer that starts
    /// with someone else's page in it. A leak here is invisible on a
    /// one-group page and grows with group count, which is the worst shape
    /// a bug can have -- absent from every small reproduction.
    #[test]
    fn a_backdrop_seeded_child_comes_back_clean() {
        let intent = CmykIntent::default();
        let mut b = CmykBuffer::new(4, 4, intent).unwrap();
        paint_all(&mut b, [0.9, 0.9, 0.9, 0.9], 1.0, Blend::Normal);
        let seeded = b.child_from_backdrop().unwrap();
        assert!(seeded.pixel(0).a > 0.0, "the seed must actually carry ink");
        b.give_back_child(seeded);
        let next = b.take_child().unwrap();
        for idx in 0..16 {
            let px = next.pixel(idx);
            assert_eq!(px.a, 0.0, "reused child still marked at {idx}");
            assert_eq!(px.c, [0.0; 4], "reused child still inked at {idx}");
        }
    }

    /// `backdrop_present` answers §11.4.5's question -- *is the initial
    /// backdrop transparent?* -- and not the easier one the dirty rectangle
    /// answers, *was anything written?*
    ///
    /// The distinction is the whole reason the helper exists: a buffer
    /// written with fully transparent paint has a dirty rectangle and no
    /// backdrop, and sending it down the two-walk path would be wasted work
    /// on every page whose group sits over blank paper.
    #[test]
    fn a_written_but_transparent_backdrop_is_still_absent() {
        let intent = CmykIntent::default();
        let mut b = CmykBuffer::new(4, 4, intent).unwrap();
        assert!(!b.backdrop_present(), "an untouched buffer has no backdrop");
        paint_all(&mut b, [0.5; 4], 0.0, Blend::Normal);
        assert!(
            !b.backdrop_present(),
            "alpha-zero paint leaves the backdrop transparent, whatever the \
             dirty rectangle says"
        );
        paint_all(&mut b, [0.5; 4], 1.0, Blend::Normal);
        assert!(b.backdrop_present(), "opaque paint IS a backdrop");
    }

    #[test]
    fn a_fresh_buffer_is_transparent_and_not_white() {
        let b = CmykBuffer::new(4, 4, CmykIntent::default()).unwrap();
        let px = b.pixel(0);
        assert_eq!(px.a, 0.0, "a new buffer must be transparent");
        // The colorants are zero -- which is NO INK -- and that is only
        // safe because alpha is zero too. The assertion is here so that a
        // future change making the buffer opaque-by-default fails loudly
        // rather than silently inverting every luminosity mask.
        assert_eq!(px.c, [0.0; 4]);
    }

    #[test]
    fn the_ceiling_refuses_rather_than_allocating() {
        // One pixel past the ceiling, computed from the constant so the
        // test cannot drift away from the value it is checking.
        let px = MAX_CMYK_BUFFER_BYTES / BYTES_PER_PIXEL + 1;
        #[allow(clippy::cast_possible_truncation)]
        let w = px as u32;
        assert!(
            CmykBuffer::new(w, 1, CmykIntent::default()).is_none(),
            "a buffer past the byte ceiling must be refused, not allocated"
        );
        assert!(CmykBuffer::new(0, 10, CmykIntent::default()).is_none());
        assert!(CmykBuffer::new(10, 0, CmykIntent::default()).is_none());
    }

    #[test]
    fn a_solid_cmyk_paint_survives_with_its_components_intact() {
        // The measurement in the module docs, run forwards: `0 1 0 0`
        // painted into this buffer comes back as `0 1 0 0`, where the sRGB
        // round trip returned `(0, 0.995, 0.409, 0.071)`.
        let mut b = CmykBuffer::new(2, 2, CmykIntent::default()).unwrap();
        let m = full_mask(2, 2);
        b.composite_mask(&m, (0, 0, 2, 2), [0.0, 1.0, 0.0, 0.0], 1.0, Blend::Normal);
        assert_eq!(b.pixel(0).c, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(b.pixel(0).a, 1.0);
    }

    #[test]
    fn coverage_scales_alpha_and_never_the_colorant() {
        // Half coverage of a full-magenta paint must be half-alpha
        // full-magenta, NOT full-alpha half-magenta. Getting this
        // backwards yields edges of the right shape and the wrong colour,
        // and both look plausible.
        let mut b = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        let mut m = Mask::new(1, 1).unwrap();
        m.data_mut()[0] = 128;
        b.composite_mask(&m, (0, 0, 1, 1), [0.0, 1.0, 0.0, 0.0], 1.0, Blend::Normal);
        let px = b.pixel(0);
        assert!(
            (px.a - 128.0 / 255.0).abs() < 1e-6,
            "alpha carries coverage"
        );
        assert!(
            (px.c[1] - 1.0).abs() < 1e-6,
            "the colorant is untouched by coverage"
        );
    }

    #[test]
    fn suite_16_2_difference_cell_lands_on_its_surround_through_the_buffer() {
        // The cell this whole Pass was derived from: magenta `0 1 0 0 k`
        // under black `0 0 0 1 k` with `/BM /Difference`. §11.3.4's
        // complement gives `1 - |cb' - cs'|` = `DeviceCMYK 1 0 1 0`, which
        // is the patch's surround colour exactly. Rendered through the
        // sRGB path pdfce produced `(237, 1, 140)` and pdfium `(202, 29,
        // 108)` -- both blending in RGB, both wrong, differently.
        //
        // The arithmetic itself is pinned by `compositor.rs`'s own test;
        // this one pins that the BUFFER delivers the same answer, which is
        // the claim `Pass 97.1e` actually makes.
        let mut b = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        let m = full_mask(1, 1);
        b.composite_mask(&m, (0, 0, 1, 1), [0.0, 0.0, 0.0, 1.0], 1.0, Blend::Normal);
        b.composite_mask(
            &m,
            (0, 0, 1, 1),
            [0.0, 1.0, 0.0, 0.0],
            1.0,
            Blend::Difference,
        );
        let px = b.pixel(0);
        for (got, want) in px.c.iter().zip([1.0, 0.0, 1.0, 0.0].iter()) {
            assert!(
                (got - want).abs() < 1e-5,
                "expected DeviceCMYK 1 0 1 0, got {:?}",
                px.c
            );
        }
    }

    #[test]
    fn the_collapse_converts_before_it_flattens() {
        // The order §11.4.7 requires, checked against the order it is easy
        // to write by accident.
        //
        // Half-alpha rich black, `C = M = Y = K = 0.9` -- the worked
        // example iccce derived the divergence from. Converting first
        // gives `over_white(srgb(0.9,0.9,0.9,0.9))`; flattening first
        // would give `srgb(0.45,0.45,0.45,0.45)`. The two differ because
        // the conversion is a fitted lattice with clamping and does not
        // commute with a linear interpolation.
        //
        // ★ The fixture is deliberately NOT pure K. A first draft used
        // `0 0 0 1` at half alpha and the two orders AGREED to the byte,
        // because the fitted lattice happens to be near-linear along the
        // K axis for the red channel. A test whose two branches coincide
        // proves nothing while looking like it proves everything, which
        // is why the inequality below is asserted rather than assumed.
        let ink = [0.9_f32, 0.9, 0.9, 0.9];
        let mut b = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        b.set_pixel(0, PixelCmyk { c: ink, a: 0.5 });
        let out = b.to_srgb_over_white().unwrap();
        let got = out.pixels()[0];

        let ink_srgb = pdfce_core::color::cmyk_to_srgb_with(
            CmykIntent::default(),
            ink[0],
            ink[1],
            ink[2],
            ink[3],
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let want_r = ((0.5_f32.mul_add(ink_srgb[0], 0.5)).clamp(0.0, 1.0) * 255.0).round() as u8;

        let wrong_order = pdfce_core::color::cmyk_to_srgb_with(
            CmykIntent::default(),
            ink[0] * 0.5,
            ink[1] * 0.5,
            ink[2] * 0.5,
            ink[3] * 0.5,
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let wrong_r = (wrong_order[0].clamp(0.0, 1.0) * 255.0).round() as u8;

        assert_eq!(
            got.red(),
            want_r,
            "convert-then-flatten is the order 11.4.7 requires"
        );
        assert_ne!(
            want_r, wrong_r,
            "if these ever coincide this test proves nothing and the fixture must change"
        );
        assert_eq!(got.alpha(), 255, "the page is opaque once on the medium");
    }

    #[test]
    fn the_bridge_counts_every_pixel_it_converts() {
        let mut b = CmykBuffer::new(2, 1, CmykIntent::default()).unwrap();
        let mut src = Pixmap::new(2, 1).unwrap();
        src.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let changed = b.composite_srgb(&src, (0, 0, 2, 1), 1.0, Blend::Normal);
        assert_eq!(changed, 2);
        assert_eq!(
            b.bridged_pixels(),
            2,
            "every bridged pixel is disclosed, not just the ones that changed"
        );
        // Pure red through max-GCR is `0 1 1 0`.
        let px = b.pixel(0);
        assert!((px.c[0] - 0.0).abs() < 1e-6);
        assert!((px.c[1] - 1.0).abs() < 1e-6);
        assert!((px.c[2] - 1.0).abs() < 1e-6);
    }

    /// ★ THE FIXTURE THE COMPOSITOR RAG WARNS IS THE ONLY KIND THAT CAN
    /// SEE THIS BUG — built with `/ca < 1` on purpose.
    ///
    /// Shape and alpha are equal when opacity is 1, so an all-opaque
    /// knockout test passes under both the correct model and a collapsed
    /// one that uses `α_s` where §11.4.8 says `f_s`. At half opacity they
    /// separate, and a knockout element erases more of what is under it
    /// than an ordinary element does.
    ///
    /// Two half-opacity elements painted over each other: in an ORDINARY
    /// group the second composites over the first and the two accumulate;
    /// in a KNOCKOUT group the second composites against the group's
    /// initial backdrop and the first is knocked out. So the knockout
    /// group's result is the second element alone.
    #[test]
    fn a_knockout_group_knocks_out_by_shape_not_by_alpha() {
        let full = full_mask(1, 1);
        let cyan = [1.0, 0.0, 0.0, 0.0];
        let magenta = [0.0, 1.0, 0.0, 0.0];

        // The ordinary group, for contrast.
        let mut plain = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        plain.composite_mask(&full, (0, 0, 1, 1), cyan, 0.5, Blend::Normal);
        plain.composite_mask(&full, (0, 0, 1, 1), magenta, 0.5, Blend::Normal);

        // The knockout group over the same (transparent) backdrop.
        let backdrop = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        let mut ko = CmykBuffer::new(1, 1, CmykIntent::default())
            .unwrap()
            .into_knockout(&backdrop)
            .unwrap();
        ko.composite_mask(&full, (0, 0, 1, 1), cyan, 0.5, Blend::Normal);
        ko.composite_mask(&full, (0, 0, 1, 1), magenta, 0.5, Blend::Normal);
        let ko = ko.finish_knockout();

        assert!(
            plain.pixel(0).c[0] > 0.2,
            "the ordinary group keeps the cyan underneath, got {:?}",
            plain.pixel(0).c
        );
        assert!(
            ko.pixel(0).c[0] < 1e-5,
            "the knockout group must have knocked the cyan out entirely, got {:?}",
            ko.pixel(0).c
        );
        assert!(
            (ko.pixel(0).c[1] - 1.0).abs() < 1e-5,
            "…leaving the magenta"
        );
        assert!(
            (ko.pixel(0).a - 0.5).abs() < 1e-5,
            "and the group's own alpha is the last element's, not the union"
        );
    }

    /// A knockout element blends against the group's **initial** backdrop,
    /// never against the elements beneath it.
    ///
    /// Using the accumulator here is the mistake that turns a knockout
    /// group back into a normal one while still looking entirely plausible
    /// on opaque artwork, so it is asserted with a blend mode whose answer
    /// differs between the two backdrops.
    #[test]
    fn a_knockout_element_blends_against_the_initial_backdrop() {
        let full = full_mask(1, 1);
        let mut backdrop = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        backdrop.set_pixel(
            0,
            PixelCmyk {
                c: [0.0, 0.0, 0.0, 1.0],
                a: 1.0,
            },
        );
        let mut ko = CmykBuffer::new(1, 1, CmykIntent::default())
            .unwrap()
            .into_knockout(&backdrop)
            .unwrap();
        // First element: yellow, which a wrong implementation would then
        // blend the second element against.
        ko.composite_mask(
            &full,
            (0, 0, 1, 1),
            [0.0, 0.0, 1.0, 0.0],
            1.0,
            Blend::Normal,
        );
        // Second: magenta with Difference. Against the INITIAL backdrop
        // (K = 1) §11.3.4 gives `1 − |c_b′ − c_s′|` = `1 0 1 0`, the same
        // answer `compositor.rs`'s own suite test pins.
        ko.composite_mask(
            &full,
            (0, 0, 1, 1),
            [0.0, 1.0, 0.0, 0.0],
            1.0,
            Blend::Difference,
        );
        let out = ko.finish_knockout().pixel(0).c;
        for (got, want) in out.iter().zip([1.0, 0.0, 1.0, 0.0].iter()) {
            assert!(
                (got - want).abs() < 1e-4,
                "blended against the accumulator instead of the initial backdrop: {out:?}"
            );
        }
    }

    /// A non-isolated knockout group's backdrop must be counted **once**.
    ///
    /// The accumulator starts as the backdrop (§11.4.8's `C_0 = C_b`) and
    /// the parent is about to composite the result onto that same backdrop
    /// again, so the result's alpha has to be the group's own `α_g` with
    /// §11.4.4's removal applied. Skipping that is a double-count, and on a
    /// fully covered opaque backdrop it is invisible — hence the partial
    /// alpha here.
    #[test]
    fn a_non_isolated_knockout_group_does_not_count_its_backdrop_twice() {
        let full = full_mask(1, 1);
        let mut backdrop = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        backdrop.set_pixel(
            0,
            PixelCmyk {
                c: [0.0, 0.0, 0.0, 1.0],
                a: 0.5,
            },
        );
        let mut ko = CmykBuffer::new(1, 1, CmykIntent::default())
            .unwrap()
            .into_knockout(&backdrop)
            .unwrap();
        ko.composite_mask(
            &full,
            (0, 0, 1, 1),
            [1.0, 0.0, 0.0, 0.0],
            0.5,
            Blend::Normal,
        );
        let done = ko.finish_knockout();
        assert!(
            (done.pixel(0).a - 0.5).abs() < 1e-5,
            "the result's alpha is the GROUP's alpha, not the union with its backdrop: {}",
            done.pixel(0).a
        );
    }

    #[test]
    fn a_transparent_source_leaves_the_buffer_alone() {
        let mut b = CmykBuffer::new(1, 1, CmykIntent::default()).unwrap();
        b.set_pixel(
            0,
            PixelCmyk {
                c: [0.1, 0.2, 0.3, 0.4],
                a: 1.0,
            },
        );
        let before = b.pixel(0);
        let src = Pixmap::new(1, 1).unwrap();
        assert_eq!(b.composite_srgb(&src, (0, 0, 1, 1), 1.0, Blend::Normal), 0);
        assert_eq!(b.pixel(0), before);
    }
}
