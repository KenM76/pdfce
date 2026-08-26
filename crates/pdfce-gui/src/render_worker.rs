//! # `render_worker` — rasterization on a background thread
//!
//! One job: keep a slow page from freezing the application. This module
//! owns the worker thread, the channel, the cancellation token and the
//! generation counter that `raster.rs` named when it documented itself
//! as the seam where off-thread rendering would happen.
//!
//! ## Why this exists, and what it is NOT
//!
//! **It does not make anything faster.** A page that took 10 s still
//! takes 10 s. What changes is that the 10 s is spent on a thread the
//! operator is not waiting on, so the window keeps repainting, the
//! zoom keeps responding, and the render can be abandoned.
//!
//! The evidence that justified building it: a real CAD sheet measured
//! **~10 s at 1× and ~58 s at 2×**, rasterized inline on the UI thread.
//! At those numbers the application does not render slowly — it stops
//! answering. `raster.rs` predicted exactly this and deferred the work
//! until "a real corpus produces pages slow enough to drop frames".
//!
//! ## The three things that make it correct
//!
//! **A generation counter.** A worker that finishes after its request
//! was superseded must have its result *discarded*, not painted. Every
//! spawn takes the next generation; a reply whose generation is not the
//! current one is dropped. Without this, releasing a zoom gesture would
//! paint whichever render happened to finish last rather than the one
//! that matches the screen.
//!
//! **Cancellation that stops work.** [`RenderCancel`] is polled between
//! content-stream operators, so a superseded render abandons the page
//! rather than running to completion and having its output thrown away.
//! At 58 s a discarded result still occupies a core and still delays
//! whatever the operator asked for next. Measured: **28.9 ms** from
//! `cancel()` to thread exit mid-render, against **10,367 ms** to let
//! one finish.
//!
//! **A bounded in-frame wait.** See [`RenderWorker::spawn`] — this is
//! what keeps a fast page indistinguishable from the synchronous
//! behaviour it replaces.
//!
//! ## What this module does not decide
//!
//! Whether, and how, the canvas discloses that it is showing a stale
//! picture. That is a rule-4 question (decision 024 §4.4) and it lives
//! in the shell, not here. This module only reports, via
//! [`RenderWorker::in_flight_since`], how long the current render has
//! been outstanding, so the shell can decide.

use pdfce_core::settings::CmykIntent;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pdfce_core::edit::EditSession;
use pdfce_core::page_tree::Page;
use pdfce_render::cancel::RenderCancel;
use pdfce_render::{Diagnostics, FontEnvironment, tiny_skia::Pixmap};

/// How long [`RenderWorker::spawn`] will wait, on the UI thread, for a
/// render it just started.
///
/// # Why a blocking wait is the right answer here
///
/// The requirement is that a page rasterizing in milliseconds behaves
/// exactly as it did when rendering was synchronous — no flash, no
/// spinner, no frame of stale content. Handing every render to a worker
/// and collecting it next frame would cost such a page one frame of
/// staleness for no benefit.
///
/// So the spawn waits briefly and collects the result inline when it
/// arrives. One frame at 60 Hz is ~16.7 ms; this is deliberately under
/// that, so even in the worst case the wait cannot itself drop a frame.
/// A page that beats the deadline never touches the asynchronous path
/// at all, and a page that misses it hands control back to the event
/// loop after a delay the operator cannot perceive.
///
/// This is the one place the UI thread blocks on rendering, it is
/// bounded by a constant, and the bound is the whole point.
const IN_FRAME_BUDGET: Duration = Duration::from_millis(12);

/// A finished rasterization, ready for the shell to upload as a texture.
///
/// The worker produces pixels; it does not touch egui. Texture upload
/// needs an `egui::Context` and belongs on the UI thread, which is also
/// what keeps this module free of any GUI type beyond the ones the
/// shell hands back.
pub struct RenderedPixels {
    /// The rasterized page.
    pub pixmap: Pixmap,
    /// Render-time findings for the diagnostics surface.
    pub diagnostics: Diagnostics,
    /// The page this render was for, so the shell can key its texture.
    pub page_index: usize,
    /// The scale it was rendered at, likewise.
    pub raster_scale: f32,
    /// The annotation-visibility flag it was rendered with.
    pub annotations: bool,
    /// The font-environment generation it was rendered against.
    pub font_env_generation: u64,
    /// The layer-override generation it was rendered against.
    pub layers_generation: u64,
}

/// What a worker sends back: pixels, a failure, or nothing at all.
enum Outcome {
    Done(Box<RenderedPixels>),
    Failed(String),
    /// The render observed its cancellation token and stopped early.
    /// Distinguished from a failure so the shell does not report a
    /// deliberate abandonment as a render error.
    Cancelled,
}

/// What a render is *of* — the staleness keys, as one comparable value.
///
/// # Why this is load-bearing rather than bookkeeping
///
/// The shell decides "the texture is stale" by comparing these keys
/// against the cached texture, and re-runs that decision every frame.
/// While a background render is in flight the texture has NOT been
/// replaced yet, so the decision keeps coming out the same way. Without
/// a way to recognise that the render already running is *for the very
/// request being asked for again*, each frame would cancel the previous
/// render and start an identical one — and a page slower than one frame
/// would never finish. Not a slow render: a render that can never
/// complete, on a page that used to merely be slow.
///
/// `raster_scale` is compared by bit pattern rather than by `==`
/// because it comes from the same arithmetic each frame; an exact float
/// comparison is right here and a tolerance would be wrong, since any
/// difference at all means the shell wants a different picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct RenderKey {
    page_index: usize,
    raster_scale_bits: u32,
    annotations: bool,
    font_env_generation: u64,
    layers_generation: u64,
}

impl RenderKey {
    fn of(request: &RenderRequest) -> Self {
        Self {
            page_index: request.page_index,
            raster_scale_bits: request.raster_scale.to_bits(),
            annotations: request.annotations,
            font_env_generation: request.font_env_generation,
            layers_generation: request.layers_generation,
        }
    }
}

/// A render currently running on a worker thread.
struct InFlight {
    rx: Receiver<Outcome>,
    cancel: RenderCancel,
    handle: Option<JoinHandle<()>>,
    key: RenderKey,
    generation: u64,
    started: Instant,
}

/// Owns at most one in-flight rasterization.
///
/// Deliberately single-slot: the canvas shows one page at one scale, so
/// a second concurrent render is always a superseded first one. Keeping
/// a queue would mean deciding which of several stale results to paint,
/// which is a question with no good answer.
#[derive(Default)]
pub struct RenderWorker {
    in_flight: Option<InFlight>,
    next_generation: u64,
}

impl std::fmt::Debug for RenderWorker {
    // Hand-written: `Receiver` and `JoinHandle` are not `Debug`, and the
    // useful state is whether something is running and which request it
    // belongs to — not the channel internals.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderWorker")
            .field(
                "in_flight_generation",
                &self.in_flight.as_ref().map(|f| f.generation),
            )
            .field("next_generation", &self.next_generation)
            .finish()
    }
}

/// Everything a render needs, owned, so it can cross a thread boundary.
///
/// `DocumentView<'a>` borrows its graph, so the worker cannot be handed
/// one — it is handed the `Arc<EditSession>` and calls `view()` on the
/// far side, where the borrow stays local to the closure. That is the
/// whole reason `OpenDoc::session` is an `Arc`, and the reason
/// `ObjectGraph` had to gain `Send + Sync`.
pub struct RenderRequest {
    pub session: Arc<EditSession>,
    pub page: Page,
    pub page_index: usize,
    pub raster_scale: f32,
    pub annotations: bool,
    pub fonts: FontEnvironment,
    pub font_env_generation: u64,
    /// The operator's layer-visibility override, or `None` to render the
    /// document as its own default configuration asks (§8.11.4.3).
    pub layers: Option<pdfce_render::LayerVisibility>,
    /// Bumped on every layer toggle. A FIFTH staleness key, for the same
    /// reason `font_env_generation` is a fourth: hiding a layer changes
    /// neither the page, the scale, the annotation flag nor the fonts, so
    /// without it the cached texture would not invalidate and the toggle
    /// would appear to do nothing.
    pub layers_generation: u64,
    /// The document magnification for §8.11.4.4 `View`-event `/AS`
    /// usage application — the operator's ZOOM, never `raster_scale`.
    ///
    /// Those differ by `pixels_per_point`, and using the raster scale
    /// would make a layer's visibility depend on the MONITOR: a document
    /// banding a layer to `[1.0, 2.0)` would show it on a 1× display and
    /// hide it on a 2× one at the same nominal zoom. The magnification
    /// the standard means is the one the operator sees.
    pub view_magnification: f32,
    /// The operator's DeviceCMYK conversion choice (§8.6.4.4), carried on
    /// the request so a render always says which conversion produced it.
    pub cmyk_intent: CmykIntent,
    /// The operator's ceiling on the subtractive compositing buffer, or
    /// `None` for the renderer's default. Carried on the request for the
    /// same reason the intent is: it changes the pixels, so a render must
    /// say which budget produced it.
    pub max_cmyk_buffer_bytes: Option<usize>,
}

impl RenderWorker {
    /// Start rendering `request`, abandoning whatever was running.
    ///
    /// Returns `Some` when the render finished inside
    /// [`IN_FRAME_BUDGET`] — the fast path, which behaves exactly as
    /// the previous synchronous code did. Returns `None` when it is
    /// still running, in which case the shell should keep drawing the
    /// previous texture and call [`Self::poll`] on later frames.
    ///
    /// Cancels the previous render *before* spawning rather than after:
    /// two rasterizations of a CAD page competing for cores make both
    /// slower, and the old one's output is already known to be unwanted.
    pub fn spawn(&mut self, request: RenderRequest) -> Option<Result<RenderedPixels, String>> {
        let key = RenderKey::of(&request);

        // Already rendering exactly this? Leave it alone. See `RenderKey`
        // — without this the per-frame staleness check would cancel and
        // restart the same render forever, and any page slower than one
        // frame would never appear at all.
        if self.in_flight.as_ref().is_some_and(|f| f.key == key) {
            return None;
        }

        self.cancel_in_flight();

        self.next_generation = self.next_generation.wrapping_add(1);
        let generation = self.next_generation;
        let cancel = RenderCancel::new();

        // Capacity 1: the worker sends exactly one message and exits.
        // A bounded channel makes that a compile-time-ish guarantee
        // rather than an unbounded buffer nobody drains.
        let (tx, rx): (SyncSender<Outcome>, Receiver<Outcome>) = sync_channel(1);
        let worker_cancel = cancel.clone();

        let handle = std::thread::spawn(move || {
            let outcome = render_on_worker(&request, &worker_cancel);
            // A send failure means the shell dropped the receiver — the
            // document was closed, or a later render superseded this
            // one and the slot was replaced. Both are ordinary; there is
            // nobody left to tell.
            let _ = tx.send(outcome);
        });

        let started = Instant::now();

        // The bounded in-frame wait. See IN_FRAME_BUDGET.
        match rx.recv_timeout(IN_FRAME_BUDGET) {
            Ok(outcome) => {
                // Finished inside the budget: join immediately so no
                // thread outlives the call, and return inline.
                let _ = handle.join();
                let elapsed_ms = started.elapsed().as_millis();
                // ui-text-exempt: diagnostic trace, never displayed in the UI
                crate::diag::trace(|| {
                    format!("render-inline gen={generation} ms={elapsed_ms} async=0")
                });
                Self::outcome_to_result(outcome)
            }
            Err(RecvTimeoutError::Timeout) => {
                // ui-text-exempt: diagnostic trace, never displayed in the UI
                crate::diag::trace(|| {
                    format!(
                        "render-async-started gen={generation} budget_ms={}",
                        IN_FRAME_BUDGET.as_millis()
                    )
                });
                self.in_flight = Some(InFlight {
                    rx,
                    cancel,
                    handle: Some(handle),
                    key,
                    generation,
                    started,
                });
                None
            }
            Err(RecvTimeoutError::Disconnected) => {
                // The worker panicked without sending. Surface it as a
                // render failure rather than hanging forever waiting for
                // a message that will never arrive.
                let _ = handle.join();
                Some(Err(
                    crate::ui_text::canvas_render_worker_stopped().to_owned()
                ))
            }
        }
    }

    /// Collect a finished render, if one is ready. Never blocks.
    ///
    /// Returns `None` both when nothing is running and when the render
    /// is still going — the shell's action is the same either way.
    pub fn poll(&mut self) -> Option<Result<RenderedPixels, String>> {
        let flight = self.in_flight.as_mut()?;
        match flight.rx.try_recv() {
            Ok(outcome) => {
                let mut flight = self.in_flight.take()?;
                if let Some(handle) = flight.handle.take() {
                    let _ = handle.join();
                }
                let elapsed_ms = flight.started.elapsed().as_millis();
                let generation = flight.generation;
                let kind = match &outcome {
                    Outcome::Done(_) => "done",
                    Outcome::Cancelled => "cancelled",
                    Outcome::Failed(_) => "failed",
                };
                // ui-text-exempt: diagnostic trace, never displayed in the UI
                crate::diag::trace(|| {
                    format!("render-async-done gen={generation} ms={elapsed_ms} outcome={kind}")
                });
                Self::outcome_to_result(outcome)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                let mut flight = self.in_flight.take()?;
                if let Some(handle) = flight.handle.take() {
                    let _ = handle.join();
                }
                Some(Err(
                    crate::ui_text::canvas_render_worker_stopped().to_owned()
                ))
            }
        }
    }

    /// How long the current render has been outstanding, if any.
    ///
    /// The shell uses this to decide whether the canvas has been stale
    /// long enough to say so. Returning the duration rather than a
    /// boolean keeps the threshold — a presentation decision — out of
    /// this module.
    pub fn in_flight_since(&self) -> Option<Duration> {
        self.in_flight.as_ref().map(|f| f.started.elapsed())
    }

    /// Whether a render is currently running.
    pub fn is_rendering(&self) -> bool {
        self.in_flight.is_some()
    }

    /// Stop any in-flight render and wait for the thread to exit.
    ///
    /// **This is the choke point that makes `Arc<EditSession>`
    /// workable.** A worker holds a clone of the session for as long as
    /// it renders, so `Arc::get_mut` fails while one is running. Every
    /// mutation goes through `OpenDoc::session_mut`, which calls this
    /// first — so by the time any edit touches the session, the render
    /// holding the other reference has exited.
    ///
    /// The alternative rulings were rejected with numbers: blocking the
    /// edit until the render finishes costs up to 58 s, which is the
    /// freeze this whole module exists to remove; snapshotting the
    /// session would need a public deep-copy impl on `EditSession`
    /// (which is not `Clone`) and would copy the document per edit.
    /// Cancel-then-mutate costs the measured **28.9 ms** of teardown.
    pub fn cancel_and_wait(&mut self) {
        self.cancel_in_flight();
    }

    /// Cancel, drain and join. Idempotent.
    fn cancel_in_flight(&mut self) {
        let Some(mut flight) = self.in_flight.take() else {
            return;
        };
        flight.cancel.cancel();
        if let Some(handle) = flight.handle.take() {
            // Join rather than detach: the whole point is that the
            // session's other reference is gone when this returns. A
            // detached thread might still be holding it.
            let _ = handle.join();
        }
    }

    fn outcome_to_result(outcome: Outcome) -> Option<Result<RenderedPixels, String>> {
        match outcome {
            Outcome::Done(pixels) => Some(Ok(*pixels)),
            Outcome::Failed(message) => Some(Err(message)),
            // A cancelled render has no result and is not a failure.
            // The shell keeps whatever it was already showing.
            Outcome::Cancelled => None,
        }
    }
}

impl Drop for RenderWorker {
    /// Closing a document must not leave a 58-second render running
    /// against a session nobody can see.
    fn drop(&mut self) {
        self.cancel_in_flight();
    }
}

/// The worker body. Runs on the spawned thread; touches no GUI type.
fn render_on_worker(request: &RenderRequest, cancel: &RenderCancel) -> Outcome {
    let mut options = pdfce_render::RenderOptions::default()
        .with_annotations(request.annotations)
        .with_cmyk_intent(request.cmyk_intent)
        .with_max_cmyk_buffer_bytes(request.max_cmyk_buffer_bytes);
    options.fonts = request.fonts.clone();
    options.cancel = Some(cancel.clone());
    // `None` stays `None`: a document nobody has toggled renders as the
    // document asks. Only an operator who touched a layer produces a set.
    options.layers = request.layers.clone();
    // A viewer, so `/AS` View-event usage applies (§8.11.4.5). The
    // texture cache already keys on scale, which is what satisfies the
    // clause's "shall be reapplied whenever [zoom] changes".
    options.view_magnification = Some(request.view_magnification);

    // `session.view()`, NOT `session.document()` (decision 018 §1) — the
    // view composes the overlay and the R45 staging buffer, so unsaved
    // edits are what gets drawn. The borrow lives and dies inside this
    // function, which is why the request can own the `Arc` and still
    // hand `render_page_with_view` a reference.
    let view = request.session.view();
    match pdfce_render::render_page_with_view(&view, &request.page, request.raster_scale, &options)
    {
        Ok(rendered) => Outcome::Done(Box::new(RenderedPixels {
            pixmap: rendered.pixmap,
            diagnostics: rendered.diagnostics,
            page_index: request.page_index,
            raster_scale: request.raster_scale,
            annotations: request.annotations,
            font_env_generation: request.font_env_generation,
            layers_generation: request.layers_generation,
        })),
        Err(e) if cancel.is_cancelled() => {
            // Deliberate abandonment, not a defect. Checking the token
            // rather than matching the error variant keeps this correct
            // if the render gains other early-exit paths.
            let _ = e;
            Outcome::Cancelled
        }
        Err(e) => Outcome::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `RenderKey` with every field distinct from the defaults, so a
    /// field that is silently dropped from the comparison shows up.
    fn key(page_index: usize, scale: f32, annotations: bool, generation: u64) -> RenderKey {
        RenderKey {
            page_index,
            raster_scale_bits: scale.to_bits(),
            annotations,
            font_env_generation: generation,
            // Held at zero by the shared helper; the layer key gets its
            // own test below rather than riding on `generation`, so a
            // regression names which key was dropped.
            layers_generation: 0,
        }
    }

    /// **A layer toggle makes the key differ.**
    ///
    /// The fifth staleness key, tested separately from the font one so a
    /// failure says WHICH key stopped counting. Without it a toggle
    /// would be recognised as "the render already running", the cached
    /// texture would stand, and hiding a layer would appear to do
    /// nothing at all — the same silent-no-op the font generation was
    /// added to prevent.
    #[test]
    fn a_layer_toggle_is_a_different_render() {
        let a = key(0, 1.0, true, 0);
        let mut b = a;
        b.layers_generation = 1;
        assert_ne!(a, b, "a layer override must not reuse another's raster");
    }

    /// Two renders of the same thing must compare EQUAL.
    ///
    /// # Why this is the load-bearing test and not bookkeeping
    ///
    /// The shell re-runs its staleness check every frame, and while a
    /// background render is in flight the cached texture has not been
    /// replaced — so the check keeps saying "stale" and keeps asking
    /// for the same render. `spawn` recognises that request as the one
    /// already running *only* through this equality.
    ///
    /// If it fails, every frame cancels the render the previous frame
    /// started and begins an identical one. A page slower than a single
    /// frame then **never finishes at all** — which is strictly worse
    /// than the freeze this module was written to remove, and it would
    /// look like a hang rather than a bug.
    ///
    /// This was a real defect in the first draft: the guard did not
    /// exist, and the livelock was reasoned out before it could be
    /// observed.
    #[test]
    fn the_same_request_twice_is_recognised_as_the_same_render() {
        assert_eq!(key(3, 2.0, true, 7), key(3, 2.0, true, 7));
    }

    /// Every staleness key must be part of the comparison.
    ///
    /// R162: the test above cannot distinguish a correct `RenderKey`
    /// from one that compares nothing at all and reports every pair as
    /// equal — and that failure is not hypothetical. A key that ignored
    /// a field would make the guard swallow a *genuine* new request:
    /// change the zoom, and the shell would decline to re-render
    /// because it believes the in-flight job already covers it. The
    /// page would stop responding to zoom entirely.
    ///
    /// So each field is varied one at a time. Dropping any single field
    /// from `RenderKey`'s `PartialEq` fails exactly one of these.
    #[test]
    fn changing_any_single_render_input_makes_a_different_key() {
        let base = key(3, 2.0, true, 7);
        assert_ne!(base, key(4, 2.0, true, 7), "page index must be compared");
        assert_ne!(base, key(3, 2.5, true, 7), "raster scale must be compared");
        assert_ne!(
            base,
            key(3, 2.0, false, 7),
            "annotation flag must be compared"
        );
        assert_ne!(
            base,
            key(3, 2.0, true, 8),
            "font generation must be compared"
        );
    }

    /// A scale difference far below any perceptible threshold is still a
    /// different render.
    ///
    /// Comparing `f32` by bit pattern rather than by a tolerance is
    /// deliberate. The shell derives `raster_scale` from the same
    /// arithmetic each frame, so an unchanged zoom yields bit-identical
    /// values and the guard holds; but any difference at all means the
    /// shell has asked for a different picture, and a tolerance would
    /// silently serve it the wrong one.
    #[test]
    fn a_one_bit_scale_difference_is_a_different_render() {
        let a = key(0, 1.0, true, 0);
        let b = key(0, f32::from_bits(1.0f32.to_bits() + 1), true, 0);
        assert_ne!(a, b);
    }

    /// A fresh worker is idle, and reports no in-flight age.
    ///
    /// Guards the status-bar disclosure against the most embarrassing
    /// failure mode: announcing that the canvas is behind when nothing
    /// is rendering.
    #[test]
    fn an_idle_worker_reports_nothing_in_flight() {
        let worker = RenderWorker::default();
        assert!(!worker.is_rendering());
        assert!(worker.in_flight_since().is_none());
    }
}
