//! Rasterise-then-composite for a **subtractive canvas** — the two helper
//! functions [`crate::canvas::Canvas`]'s `Cmyk` arms are built from.
//!
//! # Why this is a module and not four inline blocks
//!
//! `tiny_skia` rasterises **and** composites in one call: `fill_path`
//! turns a path into coverage and blends that coverage into an RGBA pixmap
//! in the same pass. A colorant buffer can use the first half and not the
//! second, so every native subtractive paint is the same two-step move:
//!
//! 1. rasterise the path into a page-sized [`Mask`] with the **same**
//!    `tiny_skia` call an sRGB paint would have used, then multiply the
//!    clip into it;
//! 2. composite that coverage per pixel through
//!    [`crate::cmyk_buffer::CmykBuffer::composite_mask`].
//!
//! Step 1 is what keeps an edge painted on a subtractive page geometrically
//! identical to the same edge on an additive one — same rasteriser, same
//! anti-aliasing, same winding rule. Only the composite differs, which is
//! exactly the difference §11.7.2 requires and no more.
//!
//! # The precedent this copies
//!
//! This is not a new technique in this crate; it is an existing one given a
//! name. `Interpreter::paint_overprint`, `Interpreter::paint_nonseparable`,
//! `Interpreter::paint_with_pattern` and `Interpreter::shading_operator`
//! all already do exactly this, each with its own copy of the mask-build
//! and the clip multiply. Four copies is why this Pass factored it out
//! rather than writing a fifth.
//!
//! # The cost that is deliberately not optimised here
//!
//! The coverage mask is **page-sized**, because a [`Mask`] shares the
//! buffer's device grid and the composite indexes both with the same
//! `y * width + x`. pdfce has measured a page-sized `Mask::new` plus
//! `fill_path` at **259 µs** (`clip_cache.rs`), and the compositor RAG's
//! own recommendation is a bbox-sized mask plus a reused scratch. That
//! change is **not** made in this Pass on purpose: it alters the indexing
//! relationship the composite depends on, and bundling a layout
//! optimisation into the Pass that first makes the arithmetic correct is
//! how a performance regression and a correctness regression become
//! indistinguishable. The scan itself is already bbox-limited by
//! [`device_region`], so the waste is the allocation and the clear, not the
//! per-pixel work.

use tiny_skia::{FillRule, Mask, Path, Stroke, Transform};

use crate::canvas::{Brush, BrushSpec, ClipRef};
use crate::cmyk_buffer::CmykBuffer;
use crate::compositor::Blend;
use crate::display_list::DeviceBounds;

/// Clamp device-space bounds to a pixel region the compositor can scan.
///
/// `pad` is added on every side before flooring/ceiling: a stroke marks
/// outside its path's own bounds by half the line width and more at a
/// miter, and an anti-aliased fill touches the pixel just outside its
/// mathematical edge. Under-covering here would clip a paint's outermost
/// row of anti-aliased pixels, which reads as a hairline gap rather than as
/// a bug.
///
/// # Returns
///
/// `None` when the bounds are absent (a non-finite CTM) or the clamped
/// region is empty — entirely off-page. An empty region is a **success**
/// that touched nothing, not a failure, and callers treat it as such.
#[must_use]
pub(crate) fn device_region(
    bounds: Option<DeviceBounds>,
    pad: f32,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let b = bounds?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let region = (
        (b.left - pad).floor().max(0.0) as u32,
        (b.top - pad).floor().max(0.0) as u32,
        (((b.right + pad).ceil().max(0.0)) as u32).min(width),
        (((b.bottom + pad).ceil().max(0.0)) as u32).min(height),
    );
    if region.0 >= region.2 || region.1 >= region.3 {
        return None;
    }
    Some(region)
}

/// Build a page-sized coverage mask for a fill or a stroke, with the clip
/// already multiplied in.
///
/// Exactly one of `rule` / `stroke` is meaningful: `Some(rule)` fills,
/// `None` strokes with `stroke`. That shape is copied from
/// `Interpreter::paint_overprint`, whose `rule: Option<FillRule>` carries
/// the same either/or, so the two cannot drift apart in how they rasterise
/// the same path.
///
/// # Why the clip is multiplied rather than passed to `fill_path`
///
/// `Mask::fill_path` takes no clip. The multiply is `(new × old) / 255`
/// per byte — the same arithmetic `tiny_skia` performs internally when it
/// intersects a clip mask with coverage — so a clipped subtractive edge and
/// a clipped additive edge agree to the byte.
/// # Why `#[allow(clippy::too_many_arguments)]`
///
/// Eight, and every one is an independent input to a rasterisation: two
/// for the device grid, three for the geometry, one for the transform, one
/// for the clip, one for the anti-alias switch. Bundling them into a
/// struct would move the same eight values one line up and add a type
/// nobody else names — the lint is aimed at functions whose arguments
/// travel together, and these do not.
#[allow(clippy::too_many_arguments)]
fn coverage(
    width: u32,
    height: u32,
    path: &Path,
    rule: Option<FillRule>,
    stroke: Option<&Stroke>,
    ctm: Transform,
    clip: ClipRef<'_>,
    anti_alias: bool,
) -> Option<Mask> {
    let mut cov = Mask::new(width, height)?;
    if let Some(r) = rule {
        cov.fill_path(path, r, anti_alias, ctm);
    } else {
        let stroked = path.clone().stroke(stroke?, 1.0)?;
        cov.fill_path(&stroked, FillRule::Winding, anti_alias, ctm);
    }
    if let Some(old) = clip.mask {
        let old_data = old.data().to_vec();
        for (n, o) in cov.data_mut().iter_mut().zip(old_data.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *n = ((u16::from(*n) * u16::from(*o)) / 255) as u8;
            }
        }
    }
    Some(cov)
}

/// Paint a solid [`BrushSpec`] into a colorant buffer.
///
/// # Where the colour comes from, and why the fallback is not silent
///
/// [`BrushSpec::cmyk`] carries the tints the *file* stated, when the
/// interpreter could resolve them. When it is `None` the colour is
/// reconstructed from the paint's quantised sRGB with
/// [`crate::overprint::rgb_to_cmyk`] — a max-GCR transform chosen for
/// exact round-tripping, not for accuracy. That reconstruction is §11.6.6's
/// required "convert the source to the group's colour space" performed with
/// the only transform this crate has; it is **not** equivalent to the
/// authored value, and the measured gap is why the field exists at all.
/// Every such paint is counted by the buffer's own bridge counter, so a
/// page composited largely from reconstructions says so.
///
/// # Alpha
///
/// Taken from the quantised RGBA's alpha byte, which is where
/// [`BrushSpec::solid`] put the graphics state's `/ca` or `/CA`. Reading it
/// back rather than threading a second parameter keeps the two paths
/// reading the *same* number: if a future change alters how constant alpha
/// is quantised, both the additive and the subtractive paint move together.
///
/// # An image brush reaching here
///
/// Only a replayed display list can produce one, and a display list is
/// refused outright on a subtractive page. The arm is written anyway — as a
/// bridge through a scratch pixmap rather than as a `todo!()` — because a
/// panic in a renderer is never the right answer to an unexpected input,
/// and because "unreachable" claims decay.
pub(crate) fn paint_solid_into_cmyk(
    buf: &mut CmykBuffer,
    path: &Path,
    brush: &BrushSpec,
    rule: Option<FillRule>,
    stroke: Option<&Stroke>,
    ctm: Transform,
    clip: ClipRef<'_>,
) {
    let (w, h) = (buf.width(), buf.height());
    let bounds = match (rule, stroke) {
        (Some(_), _) => crate::display_list::fill_bounds(path, ctm),
        (None, Some(s)) => crate::display_list::stroke_bounds(path, s, ctm),
        (None, None) => return,
    };
    // Anti-aliased coverage reaches one pixel beyond the geometric edge;
    // a stroke reaches further still and `stroke_bounds` has already
    // accounted for the line width, so one pixel of slack is enough here.
    let Some(region) = device_region(bounds, 1.0, w, h) else {
        return;
    };
    let Some(cov) = coverage(w, h, path, rule, stroke, ctm, clip, brush.anti_alias) else {
        return;
    };
    let blend = Blend::from_tiny_skia(brush.blend).unwrap_or(Blend::Normal);
    match &brush.brush {
        Brush::Solid { rgba } => {
            let alpha = f32::from(rgba[3]) / 255.0;
            let colour = brush.cmyk.unwrap_or_else(|| {
                crate::overprint::rgb_to_cmyk(
                    f32::from(rgba[0]) / 255.0,
                    f32::from(rgba[1]) / 255.0,
                    f32::from(rgba[2]) / 255.0,
                )
            });
            buf.composite_mask(&cov, region, colour, alpha, blend);
        }
        Brush::Image { .. } => {
            // See the doc comment: unreachable through the interpreter,
            // handled rather than asserted.
            buf.note_unbridged_image();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use pdfce_core::settings::CmykIntent;
    use tiny_skia::PathBuilder;

    fn unit_square(w: f32, h: f32) -> Path {
        let mut pb = PathBuilder::new();
        pb.push_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, w, h).unwrap());
        pb.finish().unwrap()
    }

    #[test]
    fn an_authored_colorant_reaches_the_buffer_unreconstructed() {
        // The whole point of `BrushSpec::cmyk`, pinned: `0 1 0 0` arrives
        // as `0 1 0 0`, where the sRGB reconstruction of the same paint
        // would land at roughly `0, 0.995, 0.409, 0.071`.
        let mut buf = CmykBuffer::new(8, 8, CmykIntent::default()).unwrap();
        let brush = BrushSpec::solid(
            crate::gstate::Rgb {
                r: 0.9,
                g: 0.0,
                b: 0.5,
            },
            1.0,
            tiny_skia::BlendMode::SourceOver,
        )
        .with_cmyk([0.0, 1.0, 0.0, 0.0]);
        paint_solid_into_cmyk(
            &mut buf,
            &unit_square(8.0, 8.0),
            &brush,
            Some(FillRule::Winding),
            None,
            Transform::identity(),
            ClipRef {
                mask: None,
                id: None,
            },
        );
        assert_eq!(buf.pixel(0).c, [0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn without_authored_colorants_the_paint_is_reconstructed_from_srgb() {
        // The fallback path, asserted so that its existence is visible in
        // the suite rather than only in a doc comment. Pure red through
        // max-GCR is `0 1 1 0`.
        let mut buf = CmykBuffer::new(8, 8, CmykIntent::default()).unwrap();
        let brush = BrushSpec::solid(
            crate::gstate::Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0,
            },
            1.0,
            tiny_skia::BlendMode::SourceOver,
        );
        paint_solid_into_cmyk(
            &mut buf,
            &unit_square(8.0, 8.0),
            &brush,
            Some(FillRule::Winding),
            None,
            Transform::identity(),
            ClipRef {
                mask: None,
                id: None,
            },
        );
        let c = buf.pixel(0).c;
        assert!((c[0] - 0.0).abs() < 1e-6);
        assert!((c[1] - 1.0).abs() < 1e-6);
        assert!((c[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn an_off_page_paint_touches_nothing_and_is_not_an_error() {
        let mut buf = CmykBuffer::new(4, 4, CmykIntent::default()).unwrap();
        let brush = BrushSpec::solid(
            crate::gstate::Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            1.0,
            tiny_skia::BlendMode::SourceOver,
        )
        .with_cmyk([1.0, 1.0, 1.0, 1.0]);
        paint_solid_into_cmyk(
            &mut buf,
            &unit_square(4.0, 4.0),
            &brush,
            Some(FillRule::Winding),
            None,
            Transform::from_translate(1000.0, 1000.0),
            ClipRef {
                mask: None,
                id: None,
            },
        );
        assert_eq!(buf.pixel(0).a, 0.0);
    }

    #[test]
    fn the_clip_is_multiplied_into_the_coverage() {
        // Half the page clipped away: the clipped half must stay
        // untouched, which is what proves the clip reached the composite
        // rather than being dropped on the way in.
        let mut buf = CmykBuffer::new(4, 1, CmykIntent::default()).unwrap();
        let mut clip = Mask::new(4, 1).unwrap();
        clip.data_mut()[0] = 255;
        clip.data_mut()[1] = 255;
        let brush = BrushSpec::solid(
            crate::gstate::Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            1.0,
            tiny_skia::BlendMode::SourceOver,
        )
        .with_cmyk([0.0, 0.0, 0.0, 1.0]);
        paint_solid_into_cmyk(
            &mut buf,
            &unit_square(4.0, 1.0),
            &brush,
            Some(FillRule::Winding),
            None,
            Transform::identity(),
            ClipRef {
                mask: Some(&clip),
                id: None,
            },
        );
        assert!(buf.pixel(0).a > 0.9, "inside the clip");
        assert_eq!(buf.pixel(3).a, 0.0, "outside the clip");
    }
}
