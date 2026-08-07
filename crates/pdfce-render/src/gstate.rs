//! # Graphics state (ISO 32000-1 §8.4)
//!
//! The device-independent graphics-state parameters Pass 1's
//! interpreter consults, plus the `q`/`Q` stack. Spec sources:
//! `iso32000__s__8.4.md` (Table 52 initial values, Table 57 operators),
//! `iso32000__s__8.4.3.md` (caps/joins/miter/dash),
//! `iso32000__s__8.6.md` (device colour spaces + per-space initial
//! colours), `iso32000__s__8.3.md` (CTM) in the PDF-spec RAG.
//!
//! ## What is and isn't in the state
//!
//! Per §8.5.2.1 (a deliberate PDF-vs-PostScript divergence): **the
//! current path is NOT part of the graphics state** — `q`/`Q` never
//! save or restore it. The **clipping path IS** (Table 52). The Pass 1
//! subset here: CTM, stroke/fill colour (device spaces only), line
//! width/cap/join/miter/dash, the clip, and the **text state**.
//! ExtGState (`gs`) keys are honored for the subset that maps to these
//! fields (Table 58 triage per the RAG: LW/LC/LJ/ML/D honored; the
//! rest recognize-and-defer).
//!
//! ## Why the text state is in here
//!
//! §9.3's first sentence: "the text state comprises those **graphics
//! state** parameters that only affect text." All nine of Table 104's
//! parameters — including the selected font and size — are therefore
//! saved by `q` and restored by `Q`, exactly like the line width, and
//! §9.3's scope rule adds that they "may appear outside text objects",
//! persist across text objects in a content stream, and are reset only
//! at the start of each page.
//!
//! The text **matrices** are the opposite case and are deliberately NOT
//! here: §9.4.1 confines `Tm`/`Tlm`/`Trm` to a single `BT`…`ET` block,
//! so they live in [`crate::text::TextObject`], owned by the
//! interpreter. A `q`/`Q` pair inside a text object must not move the
//! pen.
//!
//! ## Colour model (Pass 1)
//!
//! Device colour spaces only (§8.6.4): DeviceGray, DeviceRGB,
//! DeviceCMYK, set by `g`/`G`, `rg`/`RG`, `k`/`K` (Table 74). Initial
//! colour is black in every device space (§8.6.4: gray 0 / RGB 0,0,0 /
//! CMYK 0,0,0,1). Colours are stored converted to RGB at set time —
//! the naive un-colour-managed CMYK→RGB conversion
//! (`1 − min(1, x + k)`) is a documented Pass 1 simplification; real
//! colour management is a later Pass.

use tiny_skia::Transform;

/// Line cap style (Table 54: 0 butt, 1 round, 2 projecting square).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    /// 0 — butt: squared off at the endpoint.
    Butt,
    /// 1 — round: semicircle over the endpoint.
    Round,
    /// 2 — projecting square: extends half a line width beyond.
    Square,
}

/// Line join style (Table 55: 0 miter, 1 round, 2 bevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    /// 0 — mitered corner (subject to the miter limit).
    Miter,
    /// 1 — rounded corner.
    Round,
    /// 2 — beveled (truncated) corner.
    Bevel,
}

/// An RGB colour in [0, 1] components — the Pass 1 working colour
/// (module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    /// Red component, 0.0–1.0.
    pub r: f32,
    /// Green component, 0.0–1.0.
    pub g: f32,
    /// Blue component, 0.0–1.0.
    pub b: f32,
}

impl Rgb {
    /// Black — the initial colour in every device space (§8.6.4).
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    /// From a DeviceGray value (`g`/`G`): gray 0 = black, 1 = white.
    #[must_use]
    pub fn from_gray(v: f32) -> Self {
        let v = v.clamp(0.0, 1.0);
        Self { r: v, g: v, b: v }
    }

    /// From DeviceRGB components (`rg`/`RG`).
    #[must_use]
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
        }
    }

    /// From DeviceCMYK components (`k`/`K`) via the naive additive
    /// conversion `component = 1 − min(1, x + k)` — the documented
    /// un-colour-managed Pass 1 simplification (module docs).
    #[must_use]
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self {
            r: 1.0 - (c + k).min(1.0),
            g: 1.0 - (m + k).min(1.0),
            b: 1.0 - (y + k).min(1.0),
        }
    }
}

/// The Pass 1 graphics-state subset (module docs), with Table 52 /
/// §8.4.3.6 / §8.6.4 initial values in `default_with_ctm`.
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix: user space → device space
    /// (§8.3.4; initial value is device-dependent, supplied by the
    /// caller from page geometry + zoom).
    pub ctm: Transform,
    /// Stroking colour (used by `S`/`s` and the stroke half of `B`…).
    pub stroke_color: Rgb,
    /// Non-stroking colour (fills, and the fill half of `B`…).
    pub fill_color: Rgb,
    /// Line width in user-space units (Table 52 initial: 1.0).
    pub line_width: f32,
    /// Line cap (Table 52 initial: 0 = butt).
    pub line_cap: LineCap,
    /// Line join (Table 52 initial: 0 = miter).
    pub line_join: LineJoin,
    /// Miter limit (Table 52 initial: 10.0).
    pub miter_limit: f32,
    /// Dash pattern `(array, phase)` in user-space units (§8.4.3.6
    /// initial: solid — empty array, phase 0).
    pub dash: (Vec<f32>, f32),
    /// Current clipping path as a device-space mask, `None` = the
    /// initial clip = the entire page (§8.5.4). Stored rasterized
    /// (tiny-skia `Mask`) because PDF only ever intersects clips —
    /// never enlarges them (§8.5.4 NOTE 2) — so a mask composes by
    /// per-pixel multiplication without needing path booleans.
    ///
    /// # Why `Arc`, and why sharing is sound
    ///
    /// A `Mask` is page-sized (one byte per pixel: ~1 MB at 1191×842),
    /// and `q` pushes a **clone** of the whole graphics state. On a CAD
    /// sheet measured 2026-08-07 that is 129,951 `q` operations against
    /// a live clip — **6.8 seconds of pure memcpy**, the single largest
    /// cost in a 17.5 s render, larger than rasterizing every clip path.
    ///
    /// Sharing is sound because **a clip is never mutated in place**.
    /// `intersect_clip` builds a *fresh* mask and assigns it; the old
    /// one is only ever read. So `q` needs a new *reference*, not a new
    /// buffer, and `Q` drops one. No copy-on-write is required — there
    /// is no write.
    ///
    /// This is why the type is `Arc<Mask>` and not `Rc<Mask>`: nothing
    /// here is threaded today, but `pdfce-render` is a library whose
    /// callers may render pages in parallel, and `Rc` would make
    /// `GraphicsState` non-`Send` for a saving of one non-atomic
    /// increment per `q`.
    pub clip: Option<std::sync::Arc<tiny_skia::Mask>>,
    /// The nine §9.3 text-state parameters (module docs: they ARE
    /// graphics-state parameters, so `q`/`Q` save and restore them).
    pub text: crate::text::TextState,
}

impl GraphicsState {
    /// The §8.4/§8.6 initial state over a caller-supplied device CTM.
    #[must_use]
    pub fn default_with_ctm(ctm: Transform) -> Self {
        Self {
            ctm,
            stroke_color: Rgb::BLACK,
            fill_color: Rgb::BLACK,
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            dash: (Vec::new(), 0.0),
            clip: None,
            text: crate::text::TextState::default(),
        }
    }
}

/// The `q`/`Q` stack (Table 57). Depth-guarded: Annex C gives 28 as
/// the architectural q/Q nesting limit; pdfce accepts more on read
/// (readers should exceed writer guidance) but bounds it as an
/// ARCHITECTURE.md §10 guard.
#[derive(Debug)]
pub struct GStateStack {
    stack: Vec<GraphicsState>,
    /// The live state.
    pub current: GraphicsState,
}

/// Maximum `q` nesting accepted (pdfce policy; Annex C's writer
/// guidance is 28 — this is ~9× headroom before a hostile stream is
/// refused further nesting).
pub const MAX_Q_DEPTH: usize = 256;

impl GStateStack {
    /// Fresh stack over the initial state.
    #[must_use]
    pub fn new(initial: GraphicsState) -> Self {
        Self {
            stack: Vec::new(),
            current: initial,
        }
    }

    /// `q` — push a copy. Returns false (and does nothing) past
    /// [`MAX_Q_DEPTH`]; the interpreter surfaces that as a diagnostic.
    pub fn push(&mut self) -> bool {
        if self.stack.len() >= MAX_Q_DEPTH {
            return false;
        }
        self.stack.push(self.current.clone());
        true
    }

    /// `Q` — restore. An unbalanced `Q` (empty stack) is a no-op
    /// returning false — the RAG's real-world-tolerance note (spec
    /// says balanced; producers disagree); surfaced as a diagnostic.
    pub fn pop(&mut self) -> bool {
        match self.stack.pop() {
            Some(prev) => {
                self.current = prev;
                true
            }
            None => false,
        }
    }
}
