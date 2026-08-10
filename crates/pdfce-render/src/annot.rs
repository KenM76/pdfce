//! # Annotation appearance placement + painting (ISO 32000-1 §12.5.5)
//!
//! The **paint half** of Pass 6.0 (docs/decisions/008). [`pdfce_core::annot`]
//! walks a page's `/Annots`, decodes flags, and *selects* each
//! annotation's normal (`/AP` `/N`) appearance stream; this module
//! computes the §12.5.5 placement and paints the selected stream over the
//! page content through the **existing** §8.10.1 form-execution path
//! ([`crate::interpret::run_form_at`]). Nothing here synthesises an
//! appearance (R43): an annotation with no usable `/AP` is counted, not
//! drawn.
//!
//! ## The §12.5.5 placement algorithm (implemented verbatim, cited)
//!
//! Given the appearance form XObject's `/BBox` (required) and `/Matrix`
//! (default identity) and the annotation's `/Rect`:
//!
//! - **step a** — transform the four corners of `/BBox` by `/Matrix` to a
//!   quadrilateral, and take the **smallest upright rectangle** enclosing
//!   it (the *transformed appearance box*). A rotating `/Matrix` grows
//!   this box to the axis-aligned bounds of the rotated `/BBox`.
//! - **step b** — compute a matrix **A** that maps the transformed box's
//!   lower-left→`/Rect` lower-left and upper-right→`/Rect` upper-right,
//!   **independently in x and y**. This is an **anisotropic** scale:
//!   aspect ratio is *not* preserved — a square stamp in a wide `/Rect`
//!   is stretched wide. That is **normative**, not a bug (§12.5.5 RAG).
//! - **step c** — the effective transform is **AA = Matrix × A**.
//!
//! ## How the placement is applied without re-implementing §8.10.1
//!
//! `AA = Matrix × A`, so painting the raw appearance stream under
//! `AA × base_device_ctm` is identical to painting it under the ordinary
//! §8.10.1 `Do` procedure (which *itself* concatenates `/Matrix`) if the
//! interpreter's incoming CTM is **`A × base_device_ctm`**. So this module
//! computes only **A**, sets the initial CTM to `A × base`, and hands the
//! stream to [`crate::interpret::run_form_at`], which applies `/Matrix`,
//! clips to `/BBox`, and runs the content — inheriting the resource
//! scoping (X8), cycle guard, depth bound, and font cache the page's own
//! forms use. `/Matrix` is therefore applied **exactly once**; folding it
//! into `A` here would double-apply it (the §12.5.5 RAG's named trap).
//!
//! ## Negative results, all named and counted (R20/R27/R43/R50)
//!
//! - **`/Popup`** (§12.5.6.14): a reader UI window, **never** page
//!   content. Skipped before flags or appearance — a structural rule
//!   stronger than R43 (risk X4). Counted in the total, never painted.
//! - **Hidden / NoView** (§12.5.3): not painted on screen; **counted**
//!   (R50). NoView still prints on the future print path if Print is set;
//!   Pass 6.0 is the screen path, so both suppress here.
//! - **Degenerate transformed box** (zero width/height ⇒ step-b matrix
//!   singular): painted as **nothing**, counted, named — never a
//!   divide-by-zero, never a fabricated placement (risk X2). Likewise a
//!   missing `/Rect` or `/BBox`.
//! - **NoZoom / NoRotate** (§12.5.3): the special post-`AA` transform
//!   about the `/Rect` upper-left corner is a **documented Pass-6.0
//!   deferral** — the base `AA` placement is used and the deviation is
//!   counted+named. These flags appear almost exclusively on icon
//!   subtypes that carry no `/AP` (so are named-not-painted anyway), and
//!   no acceptance fixture exercises them; a wrong post-transform would be
//!   worse than a disclosed omission (fuzzy-never-sneaky). See the Pass
//!   6.0 report / ROADMAP residuals.

use pdfce_core::annot::{Annotation, Appearance};
// decision 018: read paths take a `DocumentView` (graph + byte source), so
// the same code renders a loaded file or an editing session's unsaved state.
use pdfce_core::graph::ObjectGraph;
use pdfce_core::object::{Dict, ObjId, Object};
use pdfce_core::page_tree::{Page, Rect};
use pdfce_core::view::DocumentView;
use tiny_skia::{Pixmap, Point, Transform};

use crate::font::{FontEnvironment, RenderPolicy};
use crate::gstate::GraphicsState;
use crate::interpret::{self, Diagnostics};

/// A transformed appearance box that is thinner than this on either axis
/// is treated as degenerate: the §12.5.5 step-b fit matrix would divide
/// by (near) zero, so `A` is singular and there is no honest placement.
///
/// A small positive epsilon rather than exact zero, because a `/Matrix`
/// that collapses `/BBox` to a sliver is degenerate for placement purposes
/// well before the extent is bit-exactly `0.0`, and dividing by `1e-9`
/// produces a placement no one can see anyway.
const MIN_BOX_EXTENT: f32 = 1e-6;

/// Survey every annotation on `page`, updating `diag`'s annotation
/// counters, and — when `paint` is true — paint each visible annotation's
/// appearance over the already-rendered page content (ISO 32000-1 §12.5).
///
/// Called by [`crate::render_page_with`] **after** the page content is
/// interpreted, so appearances composite on top (their natural z-order).
/// `base_ctm` is the page's device CTM (CropBox → origin, y-flip, scale,
/// `/Rotate`) — the same transform the page content was drawn under, so an
/// annotation rotates with the page by default (unless NoRotate, deferred).
///
/// The **counting is unconditional**; only the *painting* is gated by
/// `paint`. So a `render-page --no-annotations` (or the GUI toggle off)
/// still discloses how many annotations the page carries, how many are
/// hidden, and how many have no appearance — a suppressed render is
/// honest about what it is *not* showing (R50/R27), and the pre-6.0
/// content-only raster is reproduced exactly because no appearance pixels
/// are laid down.
///
/// Over clippy's argument bound by one, since 2026-08-07's cancellation
/// parameter. Same `#[allow]` and same reasoning as
/// [`crate::interpret::run_nested`]: this is one link in the renderer's
/// argument-threading chain, and a params struct here would only move
/// the list somewhere less visible.
#[allow(clippy::too_many_arguments)]
pub(crate) fn survey_page_annotations(
    doc: &DocumentView<'_>,
    page: &Page,
    base_ctm: Transform,
    fonts: &FontEnvironment,
    paint: bool,
    diag: &mut Diagnostics,
    pixmap: &mut Pixmap,
    cancel: Option<&crate::cancel::RenderCancel>,
    policy: RenderPolicy,
) {
    // Pass 12.M2 (§8.11.3.3): the set of optional-content groups the catalog
    // /OCProperties /D config leaves OFF by default. An annotation whose /OC
    // resolves to an OFF group is not painted (authored-layer /OC honouring;
    // full content-stream BDC/EMC /OC stays deferred — decision 011 §2.4).
    // Computed once; empty (⇒ nothing hidden) when the file has no optional
    // content, so this is a no-op on every pre-12.M2 file.
    // Annotation `/OC` (§8.11.3.3) answers to the same layer state as
    // content-stream `/OC`, and must read it from the same place: an
    // operator who hides a layer expects the dimension annotations ON
    // that layer to go with it. Splitting the two sources is how a
    // toggle ends up half-working.
    let oc_off = match policy.layers {
        Some(v) => v.hidden_set().clone(),
        None => pdfce_core::annot::optional_content_default_off(doc),
    };

    // `AS-A1` (R169): what to show for a multi-entry /AP /N subdictionary
    // that carries no /AS. §12.5.5 makes /AS Required there and states no
    // recovery, so the direction is the operator's — and it is decided
    // HERE, at appearance SELECTION, not at paint time, which is why the
    // policy goes into `page_annotations_with` rather than being consulted
    // below. The default paints nothing and the annotation is counted as
    // state-unresolved either way.
    for annot in &pdfce_core::annot::page_annotations_with(doc, page.id, policy.missing_as) {
        diag.annotations_total += 1;
        if annot.is_widget() {
            diag.annotations_widget += 1;
        }

        // §12.5.6.14 (risk X4): a /Popup is a reader window, never page
        // content — checked before flags/appearance. Counted in the
        // total, provably never painted.
        if annot.is_popup {
            continue;
        }
        // §12.5.3 Table 165 (R50): Hidden (screen+print) and NoView
        // (screen) suppress on-screen painting — honoured AND counted.
        if annot.flags.suppressed_on_screen() {
            diag.annotations_hidden += 1;
            continue;
        }
        // §8.11.3.3: annotation visibility = (flags permit) AND (OC state
        // visible). An /OC pointing at an OFF group hides the annotation,
        // counted alongside the flag-hidden ones (Pass 12.M2).
        if let Some(oc) = annot.oc
            && pdfce_core::annot::oc_is_hidden(doc, oc, &oc_off)
        {
            diag.annotations_hidden += 1;
            continue;
        }

        match &annot.appearance {
            Appearance::Normal { stream_id } => {
                // `annotations_painted` and the placement counters only
                // mean something when painting is enabled; when suppressed
                // the annotation is disclosed by `annotations_total` alone.
                if paint {
                    paint_appearance(
                        doc, page, base_ctm, fonts, annot, *stream_id, diag, pixmap, cancel, policy,
                    );
                }
            }
            // R43 named-not-painted, counted by subtype — the measured
            // demand signal for the later generation Passes.
            Appearance::None => {
                *diag
                    .annotations_without_ap
                    .entry(annot.subtype_label())
                    .or_insert(0) += 1;
            }
            // §12.5.5 NOTE 3: an /AS that could not be resolved — display
            // nothing, counted separately (the annotation HAS appearances;
            // only selection failed).
            Appearance::StateUnresolved => {
                diag.annotations_appearance_state_missing += 1;
            }
        }
    }
}

/// Place and paint one annotation's selected normal appearance
/// (§12.5.5), or refuse it by a named, counted diagnostic.
#[allow(clippy::too_many_arguments)] // every argument is placement input.
fn paint_appearance(
    doc: &DocumentView<'_>,
    page: &Page,
    base_ctm: Transform,
    fonts: &FontEnvironment,
    annot: &Annotation,
    stream_id: Option<ObjId>,
    diag: &mut Diagnostics,
    pixmap: &mut Pixmap,
    cancel: Option<&crate::cancel::RenderCancel>,
    policy: RenderPolicy,
) {
    // /Rect is Required (Table 164) and is the §12.5.5 placement target.
    let Some(rect) = annot.rect else {
        diag.annotations_placement_degenerate += 1;
        diag.note_annotation("annotation /AP present but /Rect is missing - not placed");
        return;
    };
    // Streams are indirect (§7.3.8.1), so a well-formed /N carries an id.
    let Some(id) = stream_id else {
        diag.annotations_placement_degenerate += 1;
        diag.note_annotation("annotation /AP /N stream not reachable by reference - not placed");
        return;
    };
    let Object::Stream(stream) = doc.resolved(id) else {
        // Selection said this resolved to a stream; a disagreement here is
        // a race no read-only path can produce, but it is refused not
        // panicked.
        diag.annotations_placement_degenerate += 1;
        diag.note_annotation("annotation /AP /N did not resolve to a stream - not placed");
        return;
    };

    // §12.5.5 step a needs /BBox (Table 95, Required for a form XObject).
    let Some(bbox) = read_rect_numbers(doc, &stream.dict, b"BBox") else {
        diag.annotations_placement_degenerate += 1;
        diag.note_annotation("annotation appearance has no /BBox - cannot place");
        return;
    };
    let matrix = read_matrix(doc, &stream.dict);

    // step a: transform /BBox by /Matrix, take the upright bounding box.
    let Some(tbox) = transformed_appearance_box(bbox, matrix) else {
        // Degenerate transformed box ⇒ step-b matrix singular (risk X2).
        diag.annotations_placement_degenerate += 1;
        diag.note_annotation(
            "annotation appearance box is degenerate (zero width or height) - not placed",
        );
        return;
    };

    // step b: A maps the transformed box to /Rect (anisotropic).
    let a = fit_matrix(tbox, rect);
    // AA = Matrix × A applied to the page CTM: initial = A × base, and
    // `run_form_at`'s `do_form` concatenates /Matrix on top (module docs).
    let placement = a.post_concat(base_ctm);
    let initial = GraphicsState::default_with_ctm(placement);

    let sub = interpret::run_form_at(
        doc,
        stream,
        Some(id),
        &page.resources,
        fonts,
        initial,
        pixmap,
        cancel,
        policy,
    );
    diag.merge(sub);
    diag.annotations_painted += 1;

    // NoZoom/NoRotate special placement is a documented Pass-6.0 deferral
    // (module docs): the base AA placement is used and the deviation is
    // disclosed rather than approximated wrongly.
    if annot.flags.no_zoom() || annot.flags.no_rotate() {
        diag.note_annotation(
            "annotation NoZoom/NoRotate placement adjustment deferred (base AA placement used)",
        );
    }
}

/// §12.5.5 step a: transform the corners of `bbox` (normalised
/// `[minx, miny, maxx, maxy]`) by `matrix` and return the smallest upright
/// rectangle enclosing the resulting quadrilateral as
/// `[minx, miny, maxx, maxy]`.
///
/// Returns `None` when that box is degenerate (either extent
/// ≤ [`MIN_BOX_EXTENT`]) — the step-b fit matrix is then singular
/// (division by zero on the collapsed axis), and §12.5.5 specifies no
/// handling, so the caller paints nothing and names it rather than
/// fabricating a placement (risk X2 / §12.5.5 RAG negative result).
fn transformed_appearance_box(bbox: [f64; 4], matrix: Transform) -> Option<[f32; 4]> {
    let [minx, miny, maxx, maxy] = bbox;
    let mut corners = [
        Point::from_xy(minx as f32, miny as f32),
        Point::from_xy(maxx as f32, miny as f32),
        Point::from_xy(maxx as f32, maxy as f32),
        Point::from_xy(minx as f32, maxy as f32),
    ];
    matrix.map_points(&mut corners);

    let (mut tminx, mut tminy) = (f32::INFINITY, f32::INFINITY);
    let (mut tmaxx, mut tmaxy) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in corners {
        tminx = tminx.min(p.x);
        tminy = tminy.min(p.y);
        tmaxx = tmaxx.max(p.x);
        tmaxy = tmaxy.max(p.y);
    }
    // NaN/inf guard (a hostile /Matrix could produce them) and degeneracy.
    if !(tminx.is_finite() && tminy.is_finite() && tmaxx.is_finite() && tmaxy.is_finite()) {
        return None;
    }
    if (tmaxx - tminx) <= MIN_BOX_EXTENT || (tmaxy - tminy) <= MIN_BOX_EXTENT {
        return None;
    }
    Some([tminx, tminy, tmaxx, tmaxy])
}

/// §12.5.5 step b: the scale-and-translate matrix **A** mapping the
/// transformed appearance box `tbox` (`[minx, miny, maxx, maxy]`) onto the
/// annotation `/Rect`, **independently in x and y** (anisotropic — aspect
/// ratio is not preserved; normative).
///
/// A maps `tbox` lower-left → `/Rect` lower-left and `tbox` upper-right →
/// `/Rect` upper-right, so
/// `sx = Rect.width / tbox.width`, `sy = Rect.height / tbox.height`,
/// `tx = Rect.llx − sx·tbox.minx`, `ty = Rect.lly − sy·tbox.miny`.
/// `tbox`'s extents are guaranteed positive by
/// [`transformed_appearance_box`], so the divisions are safe here.
fn fit_matrix(tbox: [f32; 4], rect: Rect) -> Transform {
    let [tminx, tminy, tmaxx, tmaxy] = tbox;
    let sx = (rect.width() as f32) / (tmaxx - tminx);
    let sy = (rect.height() as f32) / (tmaxy - tminy);
    let tx = rect.llx as f32 - sx * tminx;
    let ty = rect.lly as f32 - sy * tminy;
    Transform::from_row(sx, 0.0, 0.0, sy, tx, ty)
}

/// Read a `/Matrix` array (Table 95) as a [`Transform`], defaulting to the
/// identity when absent or malformed (Table 95's documented default).
fn read_matrix(doc: &DocumentView<'_>, dict: &Dict) -> Transform {
    let Some(items) = dict
        .get(b"Matrix")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)
    else {
        return Transform::identity();
    };
    let n: Vec<f32> = items
        .iter()
        .filter_map(|o| doc.resolve(o).as_number().map(|v| v as f32))
        .collect();
    match n.as_slice() {
        &[a, b, c, d, e, f] => Transform::from_row(a, b, c, d, e, f),
        _ => Transform::identity(),
    }
}

/// Read a four-number rectangle entry (each element possibly indirect,
/// §7.3.10) as `[minx, miny, maxx, maxy]`, normalising corners per §7.9.5.
///
/// Returns `None` when the value is not an array of four resolvable
/// numbers — a malformed `/BBox`, which the caller reports as a placement
/// refusal rather than repairs.
fn read_rect_numbers(doc: &DocumentView<'_>, dict: &Dict, key: &[u8]) -> Option<[f64; 4]> {
    let items = doc.resolve(dict.get(key)?).as_array()?;
    let n: Vec<f64> = items
        .iter()
        .filter_map(|o| doc.resolve(o).as_number())
        .collect();
    match n.as_slice() {
        &[x0, y0, x1, y1] => Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    // `Document` is a test-only name here since decision 018 moved the
    // module's own parameter type to `DocumentView`: the fixtures build a
    // real parsed file and then render it through the `&Document`
    // back-compat wrappers, which is exactly the coverage those wrappers
    // need.
    use crate::{RenderOptions, render_page, render_page_with};
    use pdfce_core::document::Document;

    /// Assemble a classic-xref PDF from numbered object bodies (raw bytes,
    /// for stream objects). Non-contiguous numbering is tolerated (gaps
    /// become free entries), so annotation fixtures can skip ids.
    fn build_pdf(objects: &[(u32, Vec<u8>)]) -> (Document, Page) {
        let mut buf = b"%PDF-1.7\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        for (num, body) in objects {
            offsets.push((*num, buf.len()));
            buf.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            buf.extend_from_slice(body);
            buf.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = buf.len();
        let max_num = objects.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let size = max_num + 1;
        buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f\r\n").as_bytes());
        for num in 1..=max_num {
            match offsets.iter().find(|(n, _)| *n == num) {
                Some((_, off)) => {
                    buf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
                }
                None => buf.extend_from_slice(b"0000000000 65535 f\r\n"),
            }
        }
        buf.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
                .as_bytes(),
        );
        let doc = Document::from_bytes(buf).unwrap();
        let page = pdfce_core::page_tree::pages(&doc).unwrap().remove(0);
        (doc, page)
    }

    fn stream_object(dict_extra: &str, data: &[u8]) -> Vec<u8> {
        let mut out = format!("<< {dict_extra} /Length {} >>\nstream\n", data.len()).into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\nendstream");
        out
    }

    /// A 100×100-MediaBox one-page document carrying the given `/Annots`
    /// array text plus the given extra objects (numbered from 5). The page
    /// is object 3.
    fn doc_with_annots(annots: &str, extra: &[(u32, Vec<u8>)]) -> (Document, Page) {
        let mut objects: Vec<(u32, Vec<u8>)> = vec![
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 100] \
                  /Resources << >> >>"
                    .to_vec(),
            ),
            (
                3,
                format!("<< /Type /Page /Parent 2 0 R /Annots {annots} >>").into_bytes(),
            ),
        ];
        objects.extend_from_slice(extra);
        build_pdf(&objects)
    }

    /// An appearance form XObject that fills its whole `/BBox` black, with
    /// the given extra dict entries (a `/BBox`, optionally a `/Matrix`).
    fn black_fill_ap(dict_extra: &str, bbox: &str) -> Vec<u8> {
        // Fill a rectangle exactly covering the declared BBox so placement
        // is visible across the whole /Rect.
        let (x0, y0, x1, y1) = parse_bbox(bbox);
        let body = format!("0 0 0 rg {} {} {} {} re f", x0, y0, x1 - x0, y1 - y0);
        stream_object(
            &format!("/Type /XObject /Subtype /Form /BBox {bbox} {dict_extra}"),
            body.as_bytes(),
        )
    }

    fn parse_bbox(bbox: &str) -> (f32, f32, f32, f32) {
        let n: Vec<f32> = bbox
            .trim_matches(|c| c == '[' || c == ']')
            .split_whitespace()
            .map(|t| t.parse().unwrap())
            .collect();
        (n[0], n[1], n[2], n[3])
    }

    fn pixel(pm: &Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
        let p = pm.pixel(x, y).unwrap();
        (p.red(), p.green(), p.blue())
    }

    fn ink_bbox(pm: &Pixmap) -> Option<(u32, u32, u32, u32)> {
        let mut bbox: Option<(u32, u32, u32, u32)> = None;
        for y in 0..pm.height() {
            for x in 0..pm.width() {
                if pixel(pm, x, y) != (255, 255, 255) {
                    bbox = Some(match bbox {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        bbox
    }

    // -----------------------------------------------------------------
    // §12.5.5 placement — pinned from both directions (acceptance crit 4)
    // -----------------------------------------------------------------

    #[test]
    fn identity_bbox_maps_one_to_one_into_rect() {
        // /BBox [0 0 20 20], identity /Matrix, /Rect [40 30 60 50]: the
        // black fill lands exactly in that 20×20 rect. Device y-down: user
        // y 30..50 → device y 50..70.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [40 30 60 50] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 20 20]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        // Centre of the rect (user 50,40 → device 50,60): black.
        assert_eq!(pixel(&out.pixmap, 50, 60), (0, 0, 0));
        // Outside the rect: paper white.
        assert_eq!(pixel(&out.pixmap, 10, 10), (255, 255, 255));
        // Ink is confined to the rect: device x 40..60, y 50..70.
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        assert!(x0 >= 39 && x1 <= 61, "x extent {x0}..{x1}");
        assert!(y0 >= 49 && y1 <= 71, "y extent {y0}..{y1}");
    }

    // -----------------------------------------------------------------
    // §8.11.3.3 authored-layer /OC visibility (Pass 12.M2)
    // -----------------------------------------------------------------

    /// A one-page doc whose catalog carries `/OCProperties` and whose only
    /// annotation sits on OCG object 10, with the given `/D` config body.
    fn doc_with_oc_annot(d_config: &str) -> (Document, Page) {
        let objects: Vec<(u32, Vec<u8>)> = vec![
            (
                1,
                format!(
                    "<< /Type /Catalog /Pages 2 0 R /OCProperties \
                     << /OCGs [10 0 R] /D << {d_config} >> >> >>"
                )
                .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 100] \
                  /Resources << >> >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Annots [5 0 R] >>".to_vec(),
            ),
            (
                5,
                b"<< /Subtype /Stamp /Rect [40 30 60 50] /OC 10 0 R /AP << /N 6 0 R >> >>".to_vec(),
            ),
            (6, black_fill_ap("/Resources << >>", "[0 0 20 20]")),
            (10, b"<< /Type /OCG /Name (Dimensions) >>".to_vec()),
        ];
        build_pdf(&objects)
    }

    #[test]
    fn an_annotation_on_an_off_layer_is_not_painted() {
        // The OCG is registered and placed in /D /OFF ⇒ hidden by default.
        let (doc, page) = doc_with_oc_annot("/Order [10 0 R] /OFF [10 0 R]");
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(
            out.diagnostics.annotations_painted, 0,
            "an /OC annotation on an OFF layer must not paint"
        );
        assert_eq!(out.diagnostics.annotations_hidden, 1);
        // No ink at all: the layer is hidden.
        assert!(ink_bbox(&out.pixmap).is_none(), "the page must be blank");
    }

    #[test]
    fn an_annotation_on_an_on_layer_is_painted() {
        // Same OCG, but NOT in /OFF ⇒ ON by default (BaseState default ON).
        let (doc, page) = doc_with_oc_annot("/Order [10 0 R]");
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(
            out.diagnostics.annotations_painted, 1,
            "an /OC annotation on an ON layer paints normally"
        );
        assert_eq!(pixel(&out.pixmap, 50, 60), (0, 0, 0));
    }

    #[test]
    fn non_origin_bbox_is_translated_to_rect() {
        // /BBox [100 100 120 120] (far from origin) must still fill the
        // /Rect exactly — step b translates the transformed box onto Rect.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 40 40] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[100 100 120 120]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        // Rect [0 0 40 40] → device y 60..100. Centre user (20,20) →
        // device (20,80): black.
        assert_eq!(pixel(&out.pixmap, 20, 80), (0, 0, 0));
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        assert!(x0 <= 1 && (39..=41).contains(&x1), "x extent {x0}..{x1}");
        assert!(y0 >= 59 && y1 >= 99, "y extent {y0}..{y1}");
    }

    #[test]
    fn bbox_larger_than_rect_scales_down() {
        // /BBox [0 0 80 80] into /Rect [10 10 30 30] (20×20): scaled DOWN
        // to fit. Ink confined to the small rect.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [10 10 30 30] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 80 80]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        // Rect x 10..30, device y 70..90.
        assert!(x0 >= 9 && x1 <= 31, "x {x0}..{x1} not confined to Rect");
        assert!(y0 >= 69 && y1 <= 91, "y {y0}..{y1} not confined to Rect");
    }

    #[test]
    fn bbox_smaller_than_rect_scales_up() {
        // /BBox [0 0 5 5] into /Rect [0 0 100 100]: scaled UP to fill the
        // whole page.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 100 100] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 5 5]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        assert!(
            x0 <= 1 && y0 <= 1 && x1 >= 98 && y1 >= 98,
            "should fill page: {x0},{y0}..{x1},{y1}"
        );
    }

    #[test]
    fn scaling_matrix_grows_the_transformed_box() {
        // /BBox [0 0 10 10] with /Matrix [2 0 0 2 0 0] → transformed box
        // 20×20; then fit to /Rect. The whole thing still fills /Rect (the
        // fit absorbs the Matrix scale), which proves Matrix is applied
        // once (not twice) — a double-apply would misplace/clip the fill.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 40 40] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (
                    6,
                    black_fill_ap("/Matrix [2 0 0 2 0 0] /Resources << >>", "[0 0 10 10]"),
                ),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        // Centre of Rect user (20,20) → device (20,80): black.
        assert_eq!(pixel(&out.pixmap, 20, 80), (0, 0, 0));
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        assert!(
            x0 <= 1 && x1 >= 39 && y0 >= 59,
            "Matrix double-applied? {x0},{y0}..{x1},{y1}"
        );
    }

    #[test]
    fn rotating_matrix_places_within_rect() {
        // A 90° /Matrix rotates /BBox; step a takes the axis-aligned
        // bounds, step b fits them to /Rect. The fill must stay inside
        // /Rect (no spill), which is the placement property under rotation.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [20 20 60 60] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (
                    6,
                    // Matrix [0 1 -1 0 20 0] rotates 90° and translates so the
                    // box stays in positive space; fill covers the BBox.
                    black_fill_ap("/Matrix [0 1 -1 0 20 0] /Resources << >>", "[0 0 20 20]"),
                ),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        let (x0, y0, x1, y1) = ink_bbox(&out.pixmap).unwrap();
        // Rect [20 20 60 60] → device x 20..60, y 40..80. Ink stays inside.
        assert!(
            x0 >= 19 && x1 <= 61 && y0 >= 39 && y1 <= 81,
            "spilled: {x0},{y0}..{x1},{y1}"
        );
    }

    #[test]
    fn inverted_rect_corners_are_normalized() {
        // /Rect [60 50 40 30] (corners reversed, §7.9.5) is the same target
        // box as [40 30 60 50]: identical placement, no divide-by-negative.
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [60 50 40 30] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 20 20]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        assert_eq!(pixel(&out.pixmap, 50, 60), (0, 0, 0), "normalized Rect");
    }

    #[test]
    fn degenerate_bbox_is_named_not_placed() {
        // /BBox [10 10 10 90] has zero width ⇒ transformed box degenerate ⇒
        // step-b matrix singular. Paint NOTHING, count + name — never a
        // divide-by-zero (risk X2).
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 40 40] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (
                    6,
                    stream_object(
                        "/Type /XObject /Subtype /Form /BBox [10 10 10 90] /Resources << >>",
                        b"0 0 0 rg 0 0 100 100 re f",
                    ),
                ),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(ink_bbox(&out.pixmap), None, "degenerate box painted");
        assert_eq!(out.diagnostics.annotations_painted, 0);
        assert_eq!(out.diagnostics.annotations_placement_degenerate, 1);
        assert!(
            out.diagnostics
                .annotation_notes
                .iter()
                .any(|s| s.contains("degenerate")),
            "must name the degenerate refusal: {:?}",
            out.diagnostics.annotation_notes
        );
    }

    // -----------------------------------------------------------------
    // Suppression + non-goals (acceptance criteria 5, 6)
    // -----------------------------------------------------------------

    #[test]
    fn hidden_annotation_is_not_painted_but_counted() {
        // /F 2 = Hidden. A fill that would cover the whole page must NOT
        // appear, and the suppression is counted (R50).
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 100 100] /F 2 /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 100 100]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(ink_bbox(&out.pixmap), None, "Hidden annotation painted");
        assert_eq!(out.diagnostics.annotations_hidden, 1);
        assert_eq!(out.diagnostics.annotations_painted, 0);
        assert_eq!(out.diagnostics.annotations_total, 1);
    }

    #[test]
    fn noview_annotation_is_not_painted_on_screen_but_counted() {
        // /F 32 = NoView: screen-suppressed (this is the screen path).
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 100 100] /F 32 /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 100 100]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(
            ink_bbox(&out.pixmap),
            None,
            "NoView annotation painted on screen"
        );
        assert_eq!(out.diagnostics.annotations_hidden, 1);
    }

    #[test]
    fn popup_is_never_painted_as_page_content() {
        // Even with a (malformed) /AP, a /Popup must never paint (X4).
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Popup /Rect [0 0 100 100] /Open true /AP << /N 6 0 R >> >>"
                        .to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 100 100]")),
            ],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(
            ink_bbox(&out.pixmap),
            None,
            "/Popup painted as page content"
        );
        assert_eq!(out.diagnostics.annotations_painted, 0);
        assert_eq!(out.diagnostics.annotations_total, 1);
    }

    #[test]
    fn no_ap_annotation_is_counted_by_subtype() {
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[(
                5,
                b"<< /Subtype /Circle /Rect [0 0 40 40] /IC [1 0 0] >>".to_vec(),
            )],
        );
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(
            ink_bbox(&out.pixmap),
            None,
            "R43: nothing synthesised from /IC"
        );
        assert_eq!(
            out.diagnostics.annotations_without_ap.get("Circle"),
            Some(&1)
        );
    }

    // -----------------------------------------------------------------
    // X8 — appearance resource scoping (the named-once correctness bug)
    // -----------------------------------------------------------------

    #[test]
    fn appearance_uses_its_own_resources_not_the_page_font() {
        // Page /Resources and the appearance both define /F1, but as
        // DIFFERENT fonts. The appearance text must resolve /F1 against the
        // APPEARANCE's own /Resources (X8), which run_form_at inherits from
        // do_form. We prove it via the substitution diagnostic: the
        // appearance names a font the page does not, so the substituted
        // set must include the appearance's font, not (only) the page's.
        //
        // Page /F1 = Helvetica; appearance /F1 = Times-Roman. If the wrong
        // resources were used, the appearance's text would resolve to
        // Helvetica.
        let objects: Vec<(u32, Vec<u8>)> = vec![
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 100] \
                  /Resources << /Font << /F1 8 0 R >> >> >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R \
                  /Resources << /Font << /F1 8 0 R >> >> /Annots [5 0 R] >>"
                    .to_vec(),
            ),
            // Page content draws nothing text-wise (keep the page's own
            // render clean so the appearance's font is what we measure).
            (4, stream_object("", b"")),
            (
                5,
                b"<< /Subtype /Stamp /Rect [0 0 100 100] /AP << /N 6 0 R >> >>".to_vec(),
            ),
            (
                6,
                stream_object(
                    "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
                     /Resources << /Font << /F1 7 0 R >> >>",
                    b"BT /F1 20 Tf 5 40 Td (T) Tj ET",
                ),
            ),
            // Appearance /F1 = Times-Roman.
            (
                7,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>".to_vec(),
            ),
            // Page /F1 = Helvetica.
            (
                8,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            ),
        ];
        let (doc, page) = build_pdf(&objects);
        let out = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(out.diagnostics.annotations_painted, 1);
        // The appearance's glyph painted from ITS /F1 (Times-Roman), which
        // is the X8 correctness proof: the page's /F1 is Helvetica.
        assert!(
            out.diagnostics
                .substituted_fonts
                .iter()
                .any(|f| f == "Times-Roman"),
            "appearance resolved /F1 against the wrong resources: {:?}",
            out.diagnostics.substituted_fonts
        );
    }

    // -----------------------------------------------------------------
    // The suppression flag (acceptance: pre-6.0 raster reproducible)
    // -----------------------------------------------------------------

    #[test]
    fn no_annotations_option_reproduces_content_only_raster() {
        let (doc, page) = doc_with_annots(
            "[5 0 R]",
            &[
                (
                    5,
                    b"<< /Subtype /Stamp /Rect [0 0 100 100] /AP << /N 6 0 R >> >>".to_vec(),
                ),
                (6, black_fill_ap("/Resources << >>", "[0 0 100 100]")),
            ],
        );
        let opts = RenderOptions::default().with_annotations(false);
        let out = render_page_with(&doc, &page, 1.0, &opts).unwrap();
        // Nothing painted (the appearance is suppressed), and — crucially —
        // the annotation counters are STILL recorded so a suppressed
        // render discloses how many annotations exist (R50/R27).
        assert_eq!(ink_bbox(&out.pixmap), None);
        assert_eq!(out.diagnostics.annotations_painted, 0);
        assert_eq!(
            out.diagnostics.annotations_total, 1,
            "suppressed but disclosed"
        );
    }
    /// **The override reaches ANNOTATIONS too, not only page content.**
    ///
    /// pdfce's own authored dimensions live on annotation `/OC`
    /// (§8.11.3.3), and an operator who hides that layer means the
    /// dimensions. Two code paths read layer state — the interpreter and
    /// the annotation walk — and a toggle that reached only one of them
    /// would look like it half-worked.
    #[test]
    fn an_override_reaches_annotation_oc_as_well_as_page_content() {
        let (doc, page) = doc_with_oc_annot("/Order [10 0 R]");
        let shown = render_page(&doc, &page, 1.0).unwrap();
        assert_eq!(shown.diagnostics.annotations_painted, 1);

        let options = RenderOptions::default()
            .with_layers(crate::LayerVisibility::hiding([ObjId::new(10, 0)]));
        let hidden = render_page_with(&doc, &page, 1.0, &options).unwrap();
        assert_eq!(
            hidden.diagnostics.annotations_painted, 0,
            "hiding a layer must hide the annotations on it"
        );
        assert_eq!(hidden.diagnostics.annotations_hidden, 1);
    }
}
