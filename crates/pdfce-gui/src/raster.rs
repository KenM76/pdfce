//! # raster — the bridge from `pdfce-render`'s pixmaps to egui textures
//!
//! One job, kept in one place: take a [`tiny_skia::Pixmap`] out of
//! [`pdfce_render::render_page`] and hand egui a
//! [`egui::TextureHandle`] it can draw, plus the [`Diagnostics`] that
//! came with it. Everything about GPU-texture lifetimes and pixel
//! formats is confined here so `main.rs` deals only in "do I have a
//! current texture for this page at this zoom."
//!
//! ## Premultiplied alpha — the one detail that silently corrupts output
//!
//! `tiny-skia` stores pixels **premultiplied** (`Pixmap::data()` is
//! `[R·A, G·B… , B·A, A]`), and `epaint::ColorImage` offers constructors
//! for both conventions. Passing premultiplied bytes to
//! `from_rgba_unmultiplied` does not fail, error, or look obviously
//! wrong — it silently darkens every partially transparent pixel, which
//! on a page means every antialiased glyph edge. Text would render
//! slightly heavier than it should and nothing would ever say so. Hence
//! [`pixmap_to_color_image`] uses `from_rgba_premultiplied` and this
//! paragraph exists so nobody "cleans up" the choice later.
//!
//! Page rasters are opaque anyway (`pdfce-render` fills the pixmap white
//! before interpreting — PDF has no page background, paper is white), so
//! the practical blast radius is limited to antialiased edges. That is
//! precisely the kind of bug that survives review and shows up as "the
//! text looks a bit off compared to Acrobat."
//!
//! ## Texture filtering
//!
//! Textures are uploaded with [`egui::TextureOptions::LINEAR`], and the
//! canvas draws them at whatever size the *current* zoom implies rather
//! than at their native pixel size. That combination is what makes the
//! debounced zoom in `main.rs` work: between the operator spinning the
//! wheel and the re-render committing, the stale texture is smoothly
//! scaled instead of blocky or absent. Nearest-neighbour filtering would
//! make the interim state look broken rather than merely soft.
//!
//! ## Why rendering is synchronous
//!
//! Rasterization happens inline on the UI thread. For Pass 1 that is the
//! right trade: a typical page renders in single-digit milliseconds, a
//! background worker needs a channel, a cancellation protocol and a
//! "which request was this a reply to" generation counter, and building
//! all of that before there is a measured stall would be speculative
//! complexity. The debounce in `main.rs` already removes the dominant
//! source of wasted renders. When a real corpus produces pages slow
//! enough to drop frames, *that* is the evidence that justifies moving
//! this off-thread — and this module is the seam where it would happen,
//! since nothing outside it knows how a texture gets made.

use std::collections::HashMap;

// `egui` is reached through `eframe` rather than as a direct dependency:
// eframe re-exports the exact egui it was built against, so there is no
// way for the two to drift to incompatible versions in `Cargo.lock`.
// `tiny_skia` likewise comes through `pdfce_render`, for the same reason.
use eframe::egui;
use pdfce_core::page_tree::Page;
// decision 018: the GUI rasterizes a `DocumentView`, not a `&Document`, so
// the canvas can be handed the EDITED state (`session.view()`). See
// `PdfceApp`/`OpenDoc` in `main.rs` for which view each call site passes.
use pdfce_core::view::DocumentView;
use pdfce_render::{Diagnostics, FontEnvironment, tiny_skia};

/// Nominal width, in egui points, of a thumbnail in the page rail.
/// Thumbnails are rasterized to fit this width; the height follows the
/// page's aspect ratio.
pub const THUMBNAIL_WIDTH_PTS: f32 = 140.0;

/// A rasterized page, uploaded and ready to draw.
///
/// `page_index` and `raster_scale` are carried alongside the texture so
/// the caller can answer "is this still the right picture?" without a
/// parallel bookkeeping struct that could disagree with it.
pub struct PageTexture {
    /// The uploaded raster. Freed when this struct drops.
    pub texture: egui::TextureHandle,
    /// Which page (0-based) this is a picture of.
    pub page_index: usize,
    /// The scale it was rasterized at, in **device pixels** per PDF
    /// user-space unit — i.e. the operator-visible zoom already
    /// multiplied by the display's `pixels_per_point`
    /// ([`crate::viewer::raster_scale`]). Staleness is compared against
    /// this, not against the logical zoom, so that dragging the window
    /// to a monitor with a different density re-rasterizes rather than
    /// leaving a soft picture behind.
    pub raster_scale: f32,
    /// Whether annotation appearances (§12.5) were painted into this
    /// raster. Carried alongside `page_index`/`raster_scale` as a third
    /// staleness key: flipping the annotation-visibility toggle changes
    /// neither the page nor the scale, so without this the cached texture
    /// would not invalidate and the toggle would silently do nothing
    /// (Pass 6.0; the ui-specialist's named correctness gap).
    pub annotations: bool,
    /// The [`crate::PdfceApp`] font-environment generation this raster was
    /// drawn with (decision 012). A FOURTH staleness key: adding or
    /// removing a font folder changes neither the page, the scale, nor the
    /// annotation flag, so without this the cached texture would not
    /// invalidate and supplying a font would silently do nothing until an
    /// unrelated repaint (the ui-specialist's named correctness gap #3).
    pub font_env_generation: u64,
    /// The honesty report that came with these pixels (decision 004
    /// §6.4, rule R20). Displayed in the status bar; never discarded.
    pub diagnostics: Diagnostics,
}

impl std::fmt::Debug for PageTexture {
    /// Hand-written because `egui::TextureHandle`'s own `Debug` prints
    /// the whole texture-manager state, which is noise in a panic
    /// message. The three fields below are what identifies a cached
    /// raster.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageTexture")
            .field("page_index", &self.page_index)
            .field("raster_scale", &self.raster_scale)
            .field("diagnostics", &self.diagnostics)
            .finish_non_exhaustive()
    }
}

/// Convert a `tiny-skia` pixmap into an egui image.
///
/// See the module docs on premultiplied alpha — this function is where
/// that convention is honoured, and it is the only place in the crate
/// that touches raw pixel bytes.
fn pixmap_to_color_image(pixmap: &tiny_skia::Pixmap) -> egui::ColorImage {
    egui::ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    )
}

/// Rasterize `page` at `zoom` and upload it as a texture.
///
/// `id` must be unique per live texture; egui uses it as the texture's
/// name and reuses the allocation when the same name is loaded again,
/// so page and thumbnail textures deliberately use different prefixes.
///
/// # Errors
///
/// Returns `pdfce-render`'s error `Display` string on a content-decode
/// failure or a raster-size-guard trip. The caller turns that into the
/// `ui_text::canvas_render_failed` presentation; this function does not
/// know about presentation and does not build user-facing prose.
fn rasterize(
    ctx: &egui::Context,
    id: &str,
    doc: &DocumentView<'_>,
    page: &Page,
    scale: f32,
    annotations: bool,
    fonts: &FontEnvironment,
) -> Result<(egui::TextureHandle, Diagnostics), String> {
    // Pass 6.0: paint annotation appearances (§12.5) unless the operator
    // toggled them off. `render_page` (annotations on) is the reader
    // default; the toggle threads through `render_page_with`. decision
    // 012: `fonts` carries any operator-supplied faces (bundled default
    // when the operator has configured no font folders).
    let mut options = pdfce_render::RenderOptions::default().with_annotations(annotations);
    options.fonts = fonts.clone();
    let rendered = pdfce_render::render_page_with_view(doc, page, scale, &options)
        .map_err(|e| e.to_string())?;
    let image = pixmap_to_color_image(&rendered.pixmap);
    let texture = ctx.load_texture(id, image, egui::TextureOptions::LINEAR);
    Ok((texture, rendered.diagnostics))
}

/// Rasterize one page for the main canvas at `raster_scale` device
/// pixels per PDF user-space unit (see
/// [`crate::viewer::raster_scale`] — this is *not* the logical zoom).
///
/// # Errors
///
/// As [`rasterize`].
// Eight parameters: each is a distinct, independent render input (the
// context, the document, the page + its index, the scale, the annotation
// toggle, the font environment, and that environment's generation as the
// texture's staleness key). Grouping them into a struct would add
// indirection without collapsing any genuine coupling — the generation is
// carried alongside `fonts` only to become a `PageTexture` field, not
// because they are one value — so the documented allow is the honest call.
#[allow(clippy::too_many_arguments)]
pub fn render_page_texture(
    ctx: &egui::Context,
    doc: &DocumentView<'_>,
    page: &Page,
    page_index: usize,
    raster_scale: f32,
    annotations: bool,
    fonts: &FontEnvironment,
    font_env_generation: u64,
) -> Result<PageTexture, String> {
    // Texture names are single tokens on purpose: the `ui-strings` CI
    // job (decision 002 R1) flags whitespace-bearing literals anywhere
    // outside ui_text.rs, and an egui texture name is machine-facing,
    // not user-facing — it should never be a candidate for the catalog.
    let (texture, diagnostics) = rasterize(
        ctx,
        "pdfce-page",
        doc,
        page,
        raster_scale,
        annotations,
        fonts,
    )?;
    Ok(PageTexture {
        texture,
        page_index,
        raster_scale,
        annotations,
        font_env_generation,
        diagnostics,
    })
}

/// Lazily built, page-indexed cache of thumbnail textures for the rail.
///
/// The cache is keyed by page index alone, not by scale: thumbnails are
/// always rasterized at the one scale that makes them
/// [`THUMBNAIL_WIDTH_PTS`] wide, and the rail draws them at whatever
/// size it has (the rail is resizable, so a slightly scaled thumbnail is
/// normal and perfectly legible). Re-rasterizing every thumbnail on
/// every rail resize would be a lot of work for a picture the size of a
/// postage stamp.
#[derive(Default)]
pub struct ThumbnailCache {
    ready: HashMap<usize, egui::TextureHandle>,
    /// Pages whose thumbnail failed to rasterize. Recorded so the rail
    /// does not retry a doomed render on every single frame — a page
    /// that fails once fails deterministically (same bytes, same code),
    /// and hammering it would peg a core.
    failed: std::collections::HashSet<usize>,
}

impl std::fmt::Debug for ThumbnailCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThumbnailCache")
            .field("ready", &self.ready.len())
            .field("failed", &self.failed.len())
            .finish()
    }
}

impl ThumbnailCache {
    /// The cached texture for `page_index`, if one has been built.
    #[must_use]
    pub fn get(&self, page_index: usize) -> Option<&egui::TextureHandle> {
        self.ready.get(&page_index)
    }

    /// Whether this page still needs work — neither built nor known bad.
    #[must_use]
    pub fn is_pending(&self, page_index: usize) -> bool {
        !self.ready.contains_key(&page_index) && !self.failed.contains(&page_index)
    }

    /// Build and cache the thumbnail for `page_index`.
    ///
    /// Called only for pages the rail has determined are actually
    /// visible (see `main.rs`), and only for a bounded number of pages
    /// per frame — rasterizing all of a 900-page document at open time
    /// would stall the Open action for seconds to produce pictures
    /// nobody has scrolled to yet.
    ///
    /// A failure is recorded, not propagated: one unrenderable page
    /// should cost that page's thumbnail, not the whole rail.
    pub fn build(
        &mut self,
        ctx: &egui::Context,
        doc: &DocumentView<'_>,
        page: &Page,
        page_index: usize,
        pixels_per_point: f32,
    ) {
        let (w, _) = crate::viewer::page_extent_pts(page);
        // Guard against a degenerate CropBox before dividing. A zero
        // width would give an infinite scale and then a refused raster.
        let scale = if w > 0.0 {
            crate::viewer::raster_scale(THUMBNAIL_WIDTH_PTS / w, pixels_per_point)
        } else {
            1.0
        };
        let id = format!("pdfce-thumb-{page_index}");
        // Thumbnails always paint annotations (the reader default) —
        // they are a fixed overview, not subject to the canvas toggle,
        // so they show the document as a reader would. They also always
        // use the BUNDLED faces (decision 012): a thumbnail is a rough
        // overview and supplied-font renders are machine-dependent (R63),
        // so the fixed rail deliberately shows the deterministic default
        // even when the canvas is using supplied faces.
        match rasterize(
            ctx,
            &id,
            doc,
            page,
            scale,
            true,
            &FontEnvironment::bundled(),
        ) {
            Ok((texture, _diagnostics)) => {
                // Thumbnail diagnostics are deliberately dropped. R20's
                // disclosure obligation is discharged by the status bar
                // against the page the operator is actually looking at;
                // a badge on every thumbnail was reviewed and deferred
                // (no evidence yet that operators need per-page warnings
                // for pages they have not opened), and keeping the data
                // "just in case" would imply a UI that does not exist.
                self.ready.insert(page_index, texture);
            }
            Err(_) => {
                self.failed.insert(page_index);
            }
        }
    }

    // NOTE: there is deliberately no `clear()`. Opening a document
    // constructs a whole new `OpenDoc`, and with it a whole new cache,
    // so a page index can never refer to a page from a previous file.
    // A `clear()` would be a second, weaker way to achieve the same
    // thing and an invitation to reuse an `OpenDoc` across documents —
    // which is exactly the bug (stale thumbnails from the previous
    // file) that constructing fresh state prevents by design.
}
