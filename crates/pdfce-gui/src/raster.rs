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

use pdfce_core::settings::CmykIntent;
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
    /// The layer-override generation these pixels were drawn with — a
    /// FIFTH staleness key, for the same reason the fourth exists.
    pub layers_generation: u64,
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
// One over clippy's bound, for the same reason `interpret::run` is: these
// are the render's inputs already decomposed, not a value that wants a
// name. See that function's note.
#[allow(clippy::too_many_arguments)]
fn rasterize(
    ctx: &egui::Context,
    id: &str,
    doc: &DocumentView<'_>,
    page: &Page,
    scale: f32,
    annotations: bool,
    fonts: &FontEnvironment,
    cmyk_intent: CmykIntent,
) -> Result<(egui::TextureHandle, Diagnostics), String> {
    // Pass 6.0: paint annotation appearances (§12.5) unless the operator
    // toggled them off. `render_page` (annotations on) is the reader
    // default; the toggle threads through `render_page_with`. decision
    // 012: `fonts` carries any operator-supplied faces (bundled default
    // when the operator has configured no font folders).
    // §8.6.4.4 has no mandated CMYK conversion, so the operator's choice
    // travels with every render rather than being decided here (R169).
    let mut options = pdfce_render::RenderOptions::default()
        .with_annotations(annotations)
        .with_cmyk_intent(cmyk_intent);
    options.fonts = fonts.clone();
    let rendered = pdfce_render::render_page_with_view(doc, page, scale, &options)
        .map_err(|e| e.to_string())?;
    let image = pixmap_to_color_image(&rendered.pixmap);
    let texture = ctx.load_texture(id, image, egui::TextureOptions::LINEAR);
    Ok((texture, rendered.diagnostics))
}

/// Upload a pixmap as a texture, under a caller-chosen name.
///
/// # Why this is here rather than at its one call site
///
/// The print preview needs a page bitmap on screen, and the tempting
/// version is four lines of `ColorImage::from_…` inline in
/// `print_flow.rs`. This module's header explains what those four lines
/// get wrong: `tiny-skia` stores pixels PREMULTIPLIED, both egui
/// constructors accept the bytes without complaint, and the wrong one
/// silently darkens every antialiased glyph edge. That is not a mistake a
/// second call site should be given the opportunity to make — the whole
/// reason [`pixmap_to_color_image`] exists is that the convention is
/// enforced by there being one function, not by review.
///
/// `id` must be unique per LIVE texture (egui reuses the allocation when
/// the same name is loaded again), which is exactly the property that
/// makes it right for a single cached preview: re-rendering the preview
/// replaces the previous upload instead of leaking a new one per page
/// step.
pub fn texture_from_pixmap(
    ctx: &egui::Context,
    id: &str,
    pixmap: &tiny_skia::Pixmap,
) -> egui::TextureHandle {
    // LINEAR, matching both page paths above: a bitmap drawn at a size
    // other than its native one should read as soft rather than blocky,
    // and the preview draws at whatever the operator's zoom implies.
    ctx.load_texture(
        id,
        pixmap_to_color_image(pixmap),
        egui::TextureOptions::LINEAR,
    )
}

/// Upload pixels a background worker produced, as a [`PageTexture`].
///
/// # Why this exists separately from the synchronous `rasterize`
///
/// Rasterization can happen on any thread; **texture upload cannot** —
/// it needs an `egui::Context`, which belongs to the UI thread. That
/// split is the whole reason `render_worker` returns a `Pixmap` rather
/// than a `TextureHandle`, and this is the other half of it.
///
/// The premultiplied-alpha contract in this module's header applies
/// here exactly as it does to the synchronous path: the same
/// [`pixmap_to_color_image`] is used, so an off-thread render cannot
/// acquire a different colour convention from an in-thread one. That is
/// not a coincidence to preserve by review — it is one function.
#[must_use]
pub fn texture_from_pixels(
    ctx: &egui::Context,
    pixels: &crate::render_worker::RenderedPixels,
) -> PageTexture {
    let image = pixmap_to_color_image(&pixels.pixmap);
    // Same texture name and filtering as the synchronous path: LINEAR is
    // what makes a stale texture drawn at a new zoom read as *soft*
    // rather than blocky, which is the free staleness signal the canvas
    // relies on while a background render is in flight.
    let texture = ctx.load_texture("pdfce-page", image, egui::TextureOptions::LINEAR);
    PageTexture {
        texture,
        page_index: pixels.page_index,
        raster_scale: pixels.raster_scale,
        annotations: pixels.annotations,
        font_env_generation: pixels.font_env_generation,
        layers_generation: pixels.layers_generation,
        diagnostics: pixels.diagnostics.clone(),
    }
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
        cmyk_intent: CmykIntent,
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
            // Thumbnails use the operator's colour choice like everything
            // else: a rail whose blacks differed from the canvas's would
            // read as a rendering bug, not as a deliberate default.
            cmyk_intent,
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
