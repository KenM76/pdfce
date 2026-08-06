//! # diag — an opt-in trace of what the canvas actually received
//!
//! ## Why this exists
//!
//! A GUI defect in this project has exactly one honest oracle: the running
//! application (standing rule R86). Everything else — reading the dispatch
//! chain, unit-testing the pure decision functions, checking the CLI's answer
//! to the same query — can be entirely green while the operator still cannot
//! select an object, because the thing that failed sits between the window
//! manager and our first line of code.
//!
//! That happened. On 2026-08-04 the operator reported that clicking a drawing
//! object selected nothing. The hit-test was verified correct through
//! `pdfce-cli` (the same `pdfce-core` query, same fixture, right answer), every
//! selection decision function passed headless, and the dispatch from toolbar
//! toggle to `run_vector_edit_tool` read correctly line by line. Reading harder
//! was not going to close the gap: the remaining candidates were all of the
//! form "does `Response::clicked()` fire at all", which is unobservable from
//! the source.
//!
//! ## Why it does not just take a screenshot
//!
//! The operator was using the machine for real work and explicitly asked that
//! the screen not be commandeered. So the diagnostic has to come out of the
//! process as *text*, from a window that need never be looked at — which also
//! makes it usable from a script, a CI run, or a machine with no display at
//! all.
//!
//! ## Contract
//!
//! - **Off unless asked.** Enabled only when the `PDFCE_DIAG` environment
//!   variable is set to a non-empty value, read once per process. With it
//!   unset, [`enabled`] is a relaxed atomic load and [`trace`]'s argument
//!   closure is never called — so a call site costs nothing and may be left in
//!   place permanently rather than added and deleted around each investigation
//!   (which is how the *next* defect ends up needing this file written again).
//! - **Writes to stderr, one line per event, `key=value` fields.** stderr
//!   because it needs no path, no handle to keep open, no failure mode of its
//!   own, and redirects with `2>`. `key=value` because the consumer is a grep
//!   or an LLM, not a person reading a log.
//! - **Never a user-facing string.** Nothing here is shown in the interface, so
//!   none of it belongs in `ui_text` (rule R1 governs operator-visible copy).
//! - **Never load-bearing.** No behaviour may depend on the trace. If deleting
//!   this module changed what the application does, the trace would have become
//!   a feature with no tests.
//!
//! ## Usage
//!
//! ```text
//! PDFCE_DIAG=1 pdfce-gui file.pdf 2> trace.txt
//! ```

use std::sync::OnceLock;

/// The tools a script can arm.
///
/// A closed set rather than a free-text name: a typo in an environment
/// variable would otherwise arm nothing and look exactly like a tool whose
/// dispatch is broken, which is the confusion this whole module exists to
/// prevent. Deliberately not `CanvasTool` itself — that type lives in
/// `canvas` and carries variants a script has no way to set up state for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTool {
    /// Leave every tool.
    None,
    /// The object-edit ("Obj") tool.
    Obj,
    /// The linear measure tool — the ce-dimension authoring surface.
    Measure,
    /// The Edit-Text tool (Pass 34.0) — in-place editing of existing page text.
    Text,
    /// The Add-Text tool (Pass 34.0) — authoring brand-new page content.
    AddText,
}

/// One step of a scripted input run — see [`Script`].
// `Copy` was dropped in Pass 34.0 when `Text(String)` landed. Nothing needed
// it: `Script::advance` clones one step per frame, and that path is off unless
// `PDFCE_DIAG_SCRIPT` is set.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// Move the pointer to a screen position, in egui points.
    Move(f32, f32),
    /// Press the primary button at a position.
    Down(f32, f32),
    /// Release the primary button at a position.
    Up(f32, f32),
    /// Press (`true`) or release (`false`) the primary button with **Alt**
    /// held — the click-through modifier. Its own step because the modifier
    /// has to be set on the event itself; egui reads modifiers from the event,
    /// not from a separate key-state channel.
    AltClick(bool, f32, f32),
    /// Press (`true`) or release (`false`) the MIDDLE button at a position —
    /// the pan gesture, which the primary-button steps cannot express.
    Middle(bool, f32, f32),
    /// A Ctrl+wheel zoom step by this factor, as `egui::Event::Zoom`.
    ///
    /// Exists so zoom-to-cursor can be checked in the LIVE application and not
    /// only in `canvas::zoom_anchor_offset`'s unit tests — the unit tests prove
    /// the solve, this proves it is wired to the wheel and to the scroll area.
    Zoom(f32),
    /// Press and release the Escape key.
    Escape,
    /// Press and release the Delete key.
    Delete,
    /// Arm a tool: `tool:none`, `tool:obj`, `tool:measure`.
    ///
    /// Set directly rather than by clicking the toolbar, so a script isolates
    /// the question "does this tool's canvas dispatch work" from the separate
    /// question "is its toolbar button wired up". A harness that could only
    /// reach a tool through its button would confuse the two.
    Tool(ScriptTool),
    /// Flip the "show points" view option (`view:points`), Pass 36.3.
    ///
    /// Scripted directly rather than by clicking the toolbar toggle, for the
    /// same reason `tool:` and `panel:` are: it isolates "does this view
    /// option change what is drawn" from "is the button that flips it wired
    /// up". Two questions, two failures, and a harness that could only reach
    /// the option through its button would confuse them.
    ShowPoints,
    /// Open the Dimension Groups panel (`panel:groups`).
    ///
    /// Scripted directly rather than by clicking through the Measure menu, for
    /// the same reason `tool:` is: it isolates "does this panel work" from
    /// "is the button that opens it wired up".
    Groups,
    /// Open the redaction review panel (`panel:redact`).
    ///
    /// Added by the Pass 37 GUI-gap sweep for the same reason [`Self::Groups`]
    /// exists: the whole Redact surface — mark-whole-page, search-and-mark,
    /// the literal/pattern switch, the mark list, apply — was UNREACHABLE from
    /// the harness, so every question about it had to be answered by reading
    /// the code. A panel the observation harness cannot open is a panel whose
    /// defects can only be found by the operator.
    Redact,
    /// Open the interactive-form field list (`panel:forms`).
    ///
    /// Same argument as [`Self::Redact`] one variant up: a panel the
    /// observation harness cannot open is a panel whose defects only the
    /// operator finds.
    Forms,
    /// Press and release a named navigation key (`key:left`, `key:right`,
    /// `key:up`, `key:down`, `key:home`, `key:end`).
    ///
    /// Added chasing the caret-versus-focus defect: the operator reported that
    /// ArrowLeft while editing text moved focus to the side panel instead of
    /// moving the caret, and the harness could not press an arrow key at all,
    /// so the defect could neither be reproduced nor its fix proven. `type:`
    /// sends text, `delete`/`escape` send those two keys, and every other key
    /// was unreachable.
    NavKey(&'static str),

    /// Burn a frame. Used to let a texture, a provider rebuild, or egui's own
    /// click detection settle between steps.
    Wait,
    /// Type literal text into whatever has keyboard focus (`type:hello`).
    ///
    /// Added by Pass 34.0, because without it the harness could not reach the
    /// question that Pass exists to answer. "Does clicking away commit what I
    /// typed" needs three things — arm the text tool, type, click elsewhere —
    /// and the script could express only the third. A defect about typed text
    /// that can only be checked by typing on the operator's own keyboard is a
    /// defect that gets checked by asking him, which is exactly what the
    /// harness exists to avoid.
    ///
    /// Injected as one `egui::Event::Text`, which is what a real keystroke
    /// produces after IME/layout translation — so this exercises the same
    /// branch of the tool's composer that a human does. Semicolons cannot
    /// appear in the payload (they are the step separator); nothing this is
    /// used for needs one.
    Text(String),
}

/// A scripted sequence of input events, one step per frame, injected into
/// egui's `RawInput` before the frame is built.
///
/// # Why inject at egui's seam rather than at the window's
///
/// The obvious harness posts `WM_MOUSEMOVE`/`WM_LBUTTONDOWN` to the window and
/// lets the real stack carry them. That was tried first on 2026-08-04 and does
/// not work for an off-screen window: winit calls `TrackMouseEvent` on the
/// move, Windows answers `WM_MOUSELEAVE` because the physical cursor is
/// elsewhere, and the button message is then dropped before it becomes an egui
/// event. The observed event list was `[PointerMoved, PointerGone]` — forever,
/// no matter how the messages were ordered.
///
/// Injecting `egui::Event`s directly sidesteps a plumbing layer that is not
/// the one under suspicion. Every reported selection defect has been at or
/// below `Response::clicked()`; the operator's own clicks demonstrably reach
/// egui (they produce hover feedback). So the layer this skips is the layer
/// already known to work, and the layers it exercises — hit test, selection
/// resolution, tool dispatch, outline drawing — are exactly the ones in doubt.
///
/// **This is a diagnostic, not a substitute for a unit test.** It proves what
/// the *live application* does with an input; a passing script is evidence, not
/// a regression guard. Anything it discovers should end up pinned by a headless
/// test as well.
///
/// # Format
///
/// `PDFCE_DIAG_SCRIPT` is a semicolon-separated step list:
///
/// ```text
/// PDFCE_DIAG_SCRIPT="wait;wait;wait;move:800,550;down:800,550;up:800,550;wait;wait"
/// ```
#[derive(Debug, Default)]
pub struct Script {
    steps: Vec<Step>,
    next: usize,
}

impl Script {
    /// Parse the environment's script, or `None` if none was requested.
    ///
    /// Unparseable steps are skipped rather than fatal: a harness that dies on
    /// a typo mid-investigation wastes a whole run, and the trace shows which
    /// steps actually ran.
    pub fn from_env() -> Option<Self> {
        // ui-text-exempt: environment variable name, never displayed
        let raw = std::env::var("PDFCE_DIAG_SCRIPT").ok()?;
        let steps: Vec<Step> = raw.split(';').filter_map(parse_step).collect();
        if steps.is_empty() {
            return None;
        }
        Some(Self { steps, next: 0 })
    }

    /// The step for this frame, consuming it. `None` once the script is done.
    ///
    /// Clones rather than copies: [`Step::Text`] carries a `String` (Pass
    /// 34.0), so `Step` is no longer `Copy`. The clone is one short string per
    /// frame in a diagnostic build path that is off by default.
    pub fn advance(&mut self) -> Option<Step> {
        let step = self.steps.get(self.next).cloned();
        if step.is_some() {
            self.next += 1;
        }
        step
    }
}

/// Parse one `verb:args` step. Kept a free function so it is unit-testable
/// without an egui context or an environment variable.
fn parse_step(s: &str) -> Option<Step> {
    let s = s.trim();
    let (verb, rest) = s.split_once(':').unwrap_or((s, ""));
    let mut nums = rest.split(',').filter_map(|n| n.trim().parse::<f32>().ok());
    let mut xy = || Some((nums.next()?, nums.next()?));
    match verb {
        "move" => xy().map(|(x, y)| Step::Move(x, y)),
        "down" => xy().map(|(x, y)| Step::Down(x, y)),
        "up" => xy().map(|(x, y)| Step::Up(x, y)),
        "zoom" => rest.trim().parse().ok().map(Step::Zoom),
        "altdown" => xy().map(|(x, y)| Step::AltClick(true, x, y)),
        "altup" => xy().map(|(x, y)| Step::AltClick(false, x, y)),
        "mdown" => xy().map(|(x, y)| Step::Middle(true, x, y)),
        "mup" => xy().map(|(x, y)| Step::Middle(false, x, y)),
        "delete" => Some(Step::Delete),
        "escape" => Some(Step::Escape),
        "wait" => Some(Step::Wait),
        "panel" if rest.trim() == "groups" => Some(Step::Groups),
        "panel" if rest.trim() == "redact" => Some(Step::Redact),
        "panel" if rest.trim() == "forms" => Some(Step::Forms),
        "key" => match rest.trim() {
            "left" => Some(Step::NavKey("left")),
            "right" => Some(Step::NavKey("right")),
            "up" => Some(Step::NavKey("up")),
            "down" => Some(Step::NavKey("down")),
            "home" => Some(Step::NavKey("home")),
            "end" => Some(Step::NavKey("end")),
            _ => None,
        },
        "view" if rest.trim() == "points" => Some(Step::ShowPoints),
        // NOT `rest.trim()`: leading and trailing spaces are legitimate text.
        "type" if !rest.is_empty() => Some(Step::Text(rest.to_owned())),
        "tool" => match rest.trim() {
            "none" => Some(Step::Tool(ScriptTool::None)),
            "obj" => Some(Step::Tool(ScriptTool::Obj)),
            "measure" => Some(Step::Tool(ScriptTool::Measure)),
            "text" => Some(Step::Tool(ScriptTool::Text)),
            "addtext" => Some(Step::Tool(ScriptTool::AddText)),
            _ => None,
        },
        _ => None,
    }
}

/// Whether tracing was requested for this process.
///
/// Resolved once and cached: the check sits in a per-frame path, and re-reading
/// the environment there would put a lock and an allocation in the frame loop
/// to answer a question that cannot change after start-up.
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var_os("PDFCE_DIAG").is_some_and(|v| !v.is_empty()) // ui-text-exempt: environment variable name, never displayed
    })
}

/// Emit one trace line, building the message only if tracing is on.
///
/// Takes a closure rather than a `String` so a disabled build path performs no
/// formatting — the call sites interpolate rects, pointer positions and hit
/// counts, and doing that work every frame to throw it away would be a real
/// cost in the one loop that must not get slower.
pub fn trace(f: impl FnOnce() -> String) {
    if enabled() {
        eprintln!("pdfce-diag {}", f()); // ui-text-exempt: diagnostic trace, never displayed in the UI
    }
}

/// Font folders to pre-register at launch, from `PDFCE_DIAG_FONT_DIR`
/// (`;`-separated on Windows, one path per entry).
///
/// # Why this exists
///
/// Operator-supplied font folders (decision 012) are added through a NATIVE
/// FOLDER PICKER, and a native modal dialog is exactly what the scripted-input
/// harness cannot drive. So the entire supplied-font feature — registering a
/// face, rendering a non-embedded font with it, and (from the Pass 21.0 GUI
/// slice) embedding it as a donor for new text — was unobservable from
/// `tools/gui-drive.ps1`. Every question about it had to be answered by
/// reading the code, which standing rule R86 says is not an answer.
///
/// This is the same argument [`Step::Redact`] and [`Step::Groups`] make: it
/// isolates "does the feature work" from "is the picker that feeds it wired
/// up". The picker still has to be verified by hand; everything downstream of
/// it no longer does.
///
/// Off unless asked, like every other member of this module, and never
/// load-bearing: with the variable unset this returns an empty vector and the
/// application starts with no supplied folders, exactly as before.
#[must_use]
pub fn font_dirs() -> Vec<std::path::PathBuf> {
    let Ok(raw) = std::env::var("PDFCE_DIAG_FONT_DIR") else {
        return Vec::new();
    };
    // `;` rather than `:` — these are Windows paths and `C:onts` contains a
    // colon. Splitting on `:` would turn one real path into two broken ones
    // and report neither as an error, which is the silent-misconfiguration
    // failure this module exists to avoid producing.
    raw.split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_parse_and_bad_ones_are_skipped_rather_than_fatal() {
        let steps: Vec<Step> = "move:10,20;down:10,20;up:10,20;delete;wait;nonsense;up:oops"
            .split(';')
            .filter_map(parse_step)
            .collect();
        assert_eq!(
            steps,
            vec![
                Step::Move(10.0, 20.0),
                Step::Down(10.0, 20.0),
                Step::Up(10.0, 20.0),
                Step::Delete,
                Step::Wait,
            ]
        );
    }

    /// Each step is handed out exactly once, and the script then reports
    /// exhaustion by returning `None` — which is what tells the harness to
    /// close the window, so a run lasts exactly as long as its script rather
    /// than a guessed timeout.
    #[test]
    fn a_script_hands_out_each_step_once_then_runs_dry() {
        let mut s = Script {
            steps: vec![Step::Wait, Step::Delete],
            next: 0,
        };
        assert_eq!(s.advance(), Some(Step::Wait));
        assert_eq!(s.advance(), Some(Step::Delete));
        assert_eq!(s.advance(), None);
        assert_eq!(s.advance(), None, "exhaustion must be stable, not a cycle");
    }
}
