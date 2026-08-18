//! `Canvas` — the one thing the content-stream interpreter draws onto.
//!
//! # Why this module exists
//!
//! Until `Pass 75.0` the interpreter threaded `&mut tiny_skia::Pixmap`
//! through sixteen signatures and painted straight into it. That is
//! perfectly correct and perfectly un-reusable: **every render re-walks the
//! whole content stream**, because the only artefact a walk produces is
//! pixels, and pixels cannot be replayed at a different viewport.
//!
//! Measured on the reference A3 CAD sheet
//! (`ncored-benchmark-cad-drawing.pdf`, 148,517 paints · 24,128 clip ops —
//! `docs/render-region-measurements.md` §4a):
//!
//! | fact | number |
//! |---|---:|
//! | a **1 × 1 point** region — 2 pixels | ~667 ms |
//! | the whole page, scale 1 — 1,002,822 pixels | ~941 ms |
//! | the same run with every `fill_path`/`stroke_path` ablated away | ~591 ms |
//! | ⇒ time actually spent **painting** | **~11 %** |
//! | ⇒ time spent **outside** `Interpreter::paint` | **~83 %** |
//!
//! A two-pixel render costing 667 ms is not a rasteriser problem. It is
//! the cost of *interpretation* — tokenising, operator dispatch, graphics
//! state, and `PathBuilder` pushes — paid in full for a viewport the size
//! of a full stop. A shell that pans by re-rendering a region therefore
//! pays ~0.7 s per frame on this document, which is the regression
//! `Pass 75.0` exists to prevent.
//!
//! `Canvas` is the seam that makes the walk's output *substitutable*. The
//! interpreter no longer knows whether the thing it draws onto is a
//! pixmap or a tape recorder; it hands over finished paths, decomposed
//! brushes and clip references, and something downstream decides whether
//! those become pixels now or a [`crate::display_list::DisplayList`] to
//! replay later.
//!
//! # The contract this type must honour, and why it is stated so bluntly
//!
//! **In paint mode, `Canvas` must be byte-for-byte indistinguishable from
//! painting into the `Pixmap` directly.** Not "visually identical", not
//! "within a rounding step" — identical, because this crate's whole test
//! suite asserts exact pixels and the pdfium parity harness compares
//! whole-page rasters. Every method here is therefore a *forward*, and the
//! brush decomposition in [`BrushSpec::to_paint`] rebuilds exactly the
//! `tiny_skia::Paint` the call site used to build inline.
//!
//! That is also why the indirection landed as its own commit, with the
//! full suite and the parity harness green, **before** any recording code
//! existed: a green run at that point proves the plumbing is transparent,
//! so every later failure is in the recorder rather than in the seam.
//!
//! # What lives here and what deliberately does not
//!
//! Here: the target abstraction ([`Canvas`]), the owned description of a
//! paint ([`Brush`], [`BrushSpec`]), and the layer primitive
//! ([`Canvas::layer`]) that models "composite this sub-drawing as one
//! object" — which is both §11.4.5's transparency-group composite and an
//! annotation's `/CA` constant-alpha composite, because those two are the
//! same operation and always were.
//!
//! Not here: anything that knows what a PDF is. `Canvas` has no opinion
//! about operators, resources or the standard; it is a drawing target.

use std::sync::Arc;

use tiny_skia::{
    BlendMode, FillRule, FilterQuality, Mask, Paint, Path, Pattern, Pixmap, PixmapPaint,
    SpreadMode, Stroke, Transform,
};

use crate::display_list::{
    ClipDef, ClipId, Op, PoisonReason, Recorder, fill_bounds, stroke_bounds,
};

/// What a paint is made of, in **owned** terms.
///
/// # Why this exists at all — `tiny_skia::Paint` cannot be stored
///
/// `Paint<'a>` holds `Shader<'a>`, and the image shader
/// ([`Pattern`]) **borrows the texel pixmap it samples**. A recorder that
/// tried to keep a `Paint` around would be keeping a borrow of a buffer
/// the interpreter is about to drop. So a paint is decomposed into owned
/// parts on the way in and rebuilt on the way out — which is also why
/// [`BrushSpec::to_paint`] is the single place the rebuild happens, and
/// why it must reproduce the old inline construction exactly.
#[derive(Debug, Clone)]
pub(crate) enum Brush {
    /// A solid colour, stored as the **8-bit RGBA quadruple** rather than
    /// a `tiny_skia::Color`.
    ///
    /// Deliberate: the interpreter's `solid()` has always built its paint
    /// with `Paint::set_color_rgba8`, so storing floats and converting
    /// back would re-run a lossy quantisation and could land a step away
    /// from the byte the old code produced. Storing what was actually
    /// handed to `tiny_skia` removes the question.
    Solid {
        /// `[r, g, b, a]`, already quantised exactly as the call site did.
        rgba: [u8; 4],
    },
    /// An image, sampled through [`Pattern`] over §8.9.4's unit square.
    ///
    /// Constructed **only** by the recording branch of
    /// [`Canvas::fill_image`]: paint mode builds its shader straight off
    /// the interpreter's borrow, so the texel copy this variant implies is
    /// paid exactly where something will read it back.
    Image {
        /// The decoded texels, owned because the interpreter's own copy
        /// goes out of scope while a display list outlives the walk.
        texels: Arc<Pixmap>,
        /// Nearest or bilinear, as chosen by `/Interpolate` and the
        /// operator's minification setting (`IM-A1`).
        quality: FilterQuality,
        /// Image space to user space (the unit-square flip).
        transform: Transform,
    },
}

/// A [`Brush`] plus the two paint-level flags that are not part of the
/// brush itself: §11.3.5's blend mode and the anti-alias switch.
///
/// Kept separate from [`Brush`] because both flags are properties of *this
/// paint*, not of the colour or image being painted with — an image's
/// anti-alias flag, in particular, is a function of the CTM
/// (`image_edge_needs_antialiasing`), not of the image.
#[derive(Debug, Clone)]
pub(crate) struct BrushSpec {
    /// What to paint with.
    pub brush: Brush,
    /// §11.3.5 `/BM`, carried on the paint so path fill, path stroke,
    /// glyph fill and glyph stroke cannot come to disagree about it.
    pub blend: BlendMode,
    /// Whether tiny_skia anti-aliases this paint's edges.
    pub anti_alias: bool,
}

impl BrushSpec {
    /// A solid colour at `alpha`, quantised exactly as the interpreter has
    /// always quantised it.
    ///
    /// The `as u8` truncations and the `round()` on alpha are **copied
    /// deliberately** from the previous inline `solid()` — changing either
    /// would shift colours by a level on some documents, and a
    /// "correctness improvement" smuggled in under a refactor is exactly
    /// the change nobody would think to look for when the parity harness
    /// moved.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn solid(c: crate::gstate::Rgb, alpha: f32, blend: BlendMode) -> Self {
        Self {
            brush: Brush::Solid {
                rgba: [
                    (c.r * 255.0) as u8,
                    (c.g * 255.0) as u8,
                    (c.b * 255.0) as u8,
                    (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
                ],
            },
            blend,
            // Every solid paint in this renderer has always been
            // anti-aliased; the flag is spelled out rather than defaulted
            // so the image case below reads as the exception it is.
            anti_alias: true,
        }
    }

    /// Rebuild the `tiny_skia::Paint` this spec describes.
    ///
    /// The returned paint borrows `self` (for the image case's texels),
    /// which is why this returns a value with a lifetime rather than a
    /// `'static` paint.
    pub(crate) fn to_paint(&self) -> Paint<'_> {
        match &self.brush {
            Brush::Solid { rgba } => {
                let mut paint = Paint::default();
                paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
                paint.anti_alias = self.anti_alias;
                paint.blend_mode = self.blend;
                paint
            }
            Brush::Image {
                texels,
                quality,
                transform,
            } => Paint {
                shader: Pattern::new(
                    texels.as_ref().as_ref(),
                    SpreadMode::Pad,
                    *quality,
                    1.0,
                    *transform,
                ),
                blend_mode: self.blend,
                anti_alias: self.anti_alias,
                force_hq_pipeline: false,
            },
        }
    }

    /// The 8-bit quadruple this spec paints with, when it is a solid.
    ///
    /// Exists for the round-trip assertions below and for nothing else — a
    /// paint's colour never affects *where* it lands, so this is
    /// diagnostic rather than geometric.
    #[cfg(test)]
    pub(crate) const fn solid_rgba(&self) -> Option<[u8; 4]> {
        match &self.brush {
            Brush::Solid { rgba } => Some(*rgba),
            Brush::Image { .. } => None,
        }
    }
}

/// How a sub-drawing is composited back into its parent — the
/// `draw_pixmap` half of a transparency group or a `/CA` annotation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerPaint {
    /// Constant alpha applied to the layer **as a whole**, which is the
    /// entire reason a layer exists: applying it per-operator instead
    /// darkens every place the drawing overlaps itself.
    pub opacity: f32,
    /// The blend mode in force at the composite (§11.4.5: the outer
    /// state applies to the group's *result*, not to its contents).
    pub blend: BlendMode,
}

/// The clip in force at a paint, in **both** the representations the two
/// canvas modes need.
///
/// # Why one type rather than two arguments
///
/// Because the two are the same fact and must not be able to disagree.
/// Painting needs a device-sized coverage `Mask`; recording needs an index
/// into a clip table, because a mask is valid only for the pixmap geometry
/// that built it (`crate::display_list` module docs §2.2). Threading them
/// separately would let a call site pass one and forget the other, and the
/// failure mode of forgetting the id is a recorded paint that **ignores its
/// clip** — content spilling outside a clipped region, on replay only, on
/// documents nobody thought to check.
///
/// Both are read out of the graphics state together
/// ([`crate::gstate::GraphicsState::clip_ref`]), so `q`/`Q` carry them as
/// the pair they are.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ClipRef<'a> {
    /// The built coverage mask — always `None` while recording, because a
    /// recording canvas never builds one.
    pub mask: Option<&'a Mask>,
    /// The recorded clip definition — always `None` while painting.
    pub id: Option<ClipId>,
}

/// The interpreter's drawing target.
///
/// The interpreter cannot tell these apart, and that is the whole design:
/// one content-stream walk serves both rasterising now and recording for
/// later, so there is no second interpreter to drift.
pub(crate) enum Canvas<'a> {
    /// Draw straight into a pixmap — the original behaviour, byte for byte.
    Paint(&'a mut Pixmap),
    /// Draw nowhere; record what *would* have been drawn, for replay
    /// against a viewport chosen later (`crate::display_list`).
    Record(&'a mut Recorder),
}

impl<'a> Canvas<'a> {
    /// Wrap a pixmap as a paint-mode canvas.
    pub(crate) fn paint(pixmap: &'a mut Pixmap) -> Self {
        Self::Paint(pixmap)
    }

    /// Wrap a recorder as a recording canvas.
    pub(crate) fn record(recorder: &'a mut Recorder) -> Self {
        Self::Record(recorder)
    }

    /// Refuse the recording, by name, keeping the first reason.
    ///
    /// A no-op in paint mode: a painter has nothing to refuse. Callers can
    /// therefore call it **unconditionally** at a site that cannot be
    /// recorded, and that is what keeps the refusal impossible to forget —
    /// the alternative shape, `if recording { poison }`, is a branch
    /// somebody eventually writes without the poison in it.
    pub(crate) fn refuse(&mut self, reason: PoisonReason) {
        if let Self::Record(r) = self {
            r.poison(reason);
        }
    }

    /// Record a clipping path, when recording.
    ///
    /// Returns the new clip id, or `None` in paint mode — where the caller
    /// builds a real mask instead.
    pub(crate) fn record_clip(&mut self, def: ClipDef) -> Option<ClipId> {
        match self {
            Self::Paint(_) => None,
            Self::Record(r) => Some(r.push_clip(def)),
        }
    }

    /// Device width in pixels.
    ///
    /// Load-bearing beyond the obvious: clip masks, soft masks and
    /// overprint coverage buffers are all allocated at exactly this size,
    /// so a canvas that lied about it would produce masks that do not
    /// align with the paints they gate.
    pub(crate) fn width(&self) -> u32 {
        match self {
            Self::Paint(p) => p.width(),
            Self::Record(r) => r.width,
        }
    }

    /// Device height in pixels. See [`Canvas::width`].
    pub(crate) fn height(&self) -> u32 {
        match self {
            Self::Paint(p) => p.height(),
            Self::Record(r) => r.height,
        }
    }

    /// Fill `path` (given in the space `ctm` maps to device space).
    pub(crate) fn fill(
        &mut self,
        path: &Path,
        brush: &BrushSpec,
        rule: FillRule,
        ctm: Transform,
        clip: ClipRef<'_>,
    ) {
        match self {
            Self::Paint(p) => p.fill_path(path, &brush.to_paint(), rule, ctm, clip.mask),
            Self::Record(r) => r.push(Op::Fill {
                bounds: fill_bounds(path, ctm),
                path: Arc::new(path.clone()),
                brush: brush.clone(),
                rule,
                ctm,
                clip: clip.id,
            }),
        }
    }

    /// Stroke `path` with `stroke`, in the space `ctm` maps to device
    /// space.
    pub(crate) fn stroke(
        &mut self,
        path: &Path,
        brush: &BrushSpec,
        stroke: &Stroke,
        ctm: Transform,
        clip: ClipRef<'_>,
    ) {
        match self {
            Self::Paint(p) => p.stroke_path(path, &brush.to_paint(), stroke, ctm, clip.mask),
            Self::Record(r) => r.push(Op::Stroke {
                bounds: stroke_bounds(path, stroke, ctm),
                path: Arc::new(path.clone()),
                brush: brush.clone(),
                // One `Arc<Stroke>` per op rather than an interned table.
                // A CAD sheet sets a line width once and strokes ten
                // thousand segments with it, so interning is the obvious
                // win — and is deliberately NOT taken here, because a key
                // over a float-bearing struct with a dash `Vec` is a
                // correctness question, and this Pass's budget is spent on
                // byte-identity. Named as a follow-on rather than left as a
                // silent inefficiency.
                stroke: Arc::new(stroke.clone()),
                ctm,
                clip: clip.id,
            }),
        }
    }

    /// Fill `path` with an **image**, sampled through `tiny_skia`'s
    /// pattern shader over §8.9.4's unit square.
    ///
    /// # Why images do not go through [`Canvas::fill`]
    ///
    /// Because [`Brush::Image`] owns its texels (`Arc<Pixmap>`) and the
    /// interpreter does not — it holds a freshly decoded `Pixmap` by
    /// reference. Building a `BrushSpec` before the call would therefore
    /// copy the whole decoded raster **on every image paint, in paint
    /// mode, where nothing ever reads the copy**.
    ///
    /// Taking the borrow here moves that copy inside the recording branch,
    /// which is the only branch that needs an owned image. Paint mode
    /// builds the shader straight off the borrow, exactly as the inline
    /// code it replaced did.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_image(
        &mut self,
        path: &Path,
        texels: &Pixmap,
        quality: FilterQuality,
        image_to_user: Transform,
        blend: BlendMode,
        anti_alias: bool,
        ctm: Transform,
        clip: ClipRef<'_>,
    ) {
        match self {
            Self::Record(r) => {
                r.push(Op::Fill {
                    bounds: fill_bounds(path, ctm),
                    path: Arc::new(path.clone()),
                    brush: BrushSpec {
                        // HERE is the copy this method's borrow exists to
                        // defer — paid once, in the only branch that will
                        // ever read it.
                        brush: Brush::Image {
                            texels: Arc::new(texels.clone()),
                            quality,
                            transform: image_to_user,
                        },
                        blend,
                        anti_alias,
                    },
                    rule: FillRule::Winding,
                    ctm,
                    clip: clip.id,
                });
            }
            Self::Paint(p) => {
                let paint = Paint {
                    shader: Pattern::new(
                        texels.as_ref(),
                        SpreadMode::Pad,
                        quality,
                        1.0,
                        image_to_user,
                    ),
                    blend_mode: blend,
                    anti_alias,
                    force_hq_pipeline: false,
                };
                p.fill_path(path, &paint, FillRule::Winding, ctm, clip.mask);
            }
        }
    }

    /// The raw destination buffer, for the operations that **read pixels
    /// back** and therefore cannot be expressed as a recorded draw.
    ///
    /// Exactly two callers: `paint_overprint` (§11.7.4.3 composites
    /// against the destination's own colorants) and the soft-mask path.
    /// Both are destination-dependent by definition — there is no "record
    /// this and replay it later" formulation of *"read what is already
    /// there"*, because what is already there depends on the viewport.
    ///
    /// A recording canvas returns `None` here, and the caller's documented
    /// job is then to **poison the recording by name** rather than to
    /// quietly skip the effect. Returning `None` and letting a caller
    /// treat it as "nothing to do" would be precisely the silent
    /// wrongness rule 4 forbids.
    pub(crate) fn pixmap_mut(&mut self) -> Option<&mut Pixmap> {
        match self {
            Self::Paint(p) => Some(p),
            Self::Record(_) => None,
        }
    }

    /// Draw a sub-drawing into its own buffer and composite the result as
    /// **one object**.
    ///
    /// `f` is handed a nested canvas of the same device size. The nested
    /// drawing starts fully **transparent** — which is §11.4.7's isolated
    /// backdrop, and is also what an annotation's `/CA` composite needs
    /// (a white scratch would composite an opaque rectangle over the
    /// page).
    ///
    /// # Returns
    ///
    /// `Some(f(..))` when the layer ran. `None` when the layer **could not
    /// be started at all**, in which case `f` was *never called* and the
    /// caller must decide what to do instead — `do_form` falls back to
    /// painting inline and counts the group as flattened; an annotation's
    /// `/CA` path reports a degenerate placement and paints nothing. Those
    /// two fallbacks differ, which is exactly why this returns "did not
    /// start" rather than performing a fallback of its own choosing.
    pub(crate) fn layer<R>(
        &mut self,
        paint: LayerPaint,
        f: impl FnOnce(&mut Canvas<'_>) -> R,
    ) -> Option<R> {
        match self {
            Self::Paint(p) => {
                // Same size as the parent, deliberately, and not the
                // sub-drawing's bounding box: the contents are drawn under
                // the SAME CTM as the parent, so a smaller buffer would
                // need a translation threaded through every paint site and
                // every clip mask. Page-sized costs ~4 bytes per pixel per
                // nesting level and needs no coordinate change at all.
                let mut buf = Pixmap::new(p.width(), p.height())?;
                let result = {
                    let mut sub = Canvas::Paint(&mut buf);
                    f(&mut sub)
                };
                p.draw_pixmap(
                    0,
                    0,
                    buf.as_ref(),
                    &PixmapPaint {
                        opacity: paint.opacity.clamp(0.0, 1.0),
                        blend_mode: paint.blend,
                        quality: FilterQuality::Nearest,
                    },
                    Transform::identity(),
                    // No mask: the contents were already clipped while
                    // being drawn, so re-applying the clip here would
                    // double-multiply its anti-aliased edge and darken
                    // every clipped boundary by one pass.
                    None,
                );
                Some(result)
            }
            Self::Record(r) => {
                // A recorded layer is a frame on the op stack: everything
                // `f` draws lands in it, and popping turns it into one
                // `Op::Layer` in the parent. No buffer is allocated,
                // because no pixels exist yet — which is also why this
                // branch cannot fail the way the paint branch can.
                r.frames.push(Vec::new());
                let result = {
                    let mut sub = Canvas::Record(r);
                    f(&mut sub)
                };
                let ops = r.frames.pop().unwrap_or_default();
                r.push(Op::Layer { paint, ops });
                Some(result)
            }
        }
    }
}

/// Shader-shaped assertions that the decomposition round-trips.
///
/// These exist because the whole safety argument for `Pass 75.0`'s
/// plumbing commit is *"`to_paint` rebuilds what the call site used to
/// build inline"*, and an argument that is only made in a comment is an
/// argument nobody can re-run.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_brush_quantises_exactly_as_the_old_inline_paint_did() {
        let c = crate::gstate::Rgb {
            r: 0.5,
            g: 0.25,
            b: 1.0,
        };
        let spec = BrushSpec::solid(c, 0.5, BlendMode::Multiply);

        // The old inline construction, reproduced here verbatim.
        let mut expected = Paint::default();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        expected.set_color_rgba8(
            (c.r * 255.0) as u8,
            (c.g * 255.0) as u8,
            (c.b * 255.0) as u8,
            (0.5_f32.clamp(0.0, 1.0) * 255.0).round() as u8,
        );
        expected.anti_alias = true;
        expected.blend_mode = BlendMode::Multiply;

        let got = spec.to_paint();
        assert_eq!(got.blend_mode, expected.blend_mode);
        assert_eq!(got.anti_alias, expected.anti_alias);
        match (got.shader, expected.shader) {
            (tiny_skia::Shader::SolidColor(a), tiny_skia::Shader::SolidColor(b)) => {
                assert_eq!(a, b)
            }
            _ => panic!("a solid brush must produce a SolidColor shader"),
        }
    }

    #[test]
    fn alpha_rounds_rather_than_truncates() {
        // 0.5 × 255 = 127.5. Truncation gives 127, rounding gives 128, and
        // the interpreter has always rounded. A regression here would be
        // one level of alpha on every semi-transparent object in every
        // document — visible in aggregate, invisible in review.
        let c = crate::gstate::Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let spec = BrushSpec::solid(c, 0.5, BlendMode::SourceOver);
        assert_eq!(spec.solid_rgba().map(|q| q[3]), Some(128));
    }
}
