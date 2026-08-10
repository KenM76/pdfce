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
    /// The Create-Field tool (decision 020 F5) — placing a new form field.
    PlaceField,
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
    /// A primary press/release with CTRL held (`ctrldown:X,Y` / `ctrlup:X,Y`).
    ///
    /// Added to prove additive selection: the harness could send Alt (the
    /// click-through cycle) but not Ctrl, so the modifier an operator actually
    /// reaches for to multi-select was the one gesture that could not be
    /// driven.
    CtrlClick(bool, f32, f32),
    /// Press (`true`) or release (`false`) the MIDDLE button at a position —
    /// the pan gesture, which the primary-button steps cannot express.
    Middle(bool, f32, f32),
    /// A Ctrl+wheel zoom step by this factor, as `egui::Event::Zoom`.
    ///
    /// Exists so zoom-to-cursor can be checked in the LIVE application and not
    /// only in `canvas::zoom_anchor_offset`'s unit tests — the unit tests prove
    /// the solve, this proves it is wired to the wheel and to the scroll area.
    Zoom(f32),
    /// A plain wheel scroll by this many points — positive scrolls the
    /// content DOWN the list (the direction a wheel-away gesture moves it).
    ///
    /// # Why this exists, and what it unblocked
    ///
    /// [`Self::Zoom`] was the only wheel step, and it is Ctrl+wheel. Every
    /// panel in this application is a `ScrollArea`, so any control far
    /// enough down one was **unreachable by the harness** — and R86 makes
    /// "verified in the running application" a shipping condition, so an
    /// unreachable control is an unverifiable one.
    ///
    /// Found the first time it mattered: the rich-text summary on the
    /// Forms panel's `Notes` row traced at y=1121 in a 1000-high window,
    /// below the fold, with no way to bring it into view. The screenshot
    /// showed the panel ending three fields above the one under test.
    ///
    /// Points, not "lines" or "notches": egui's own
    /// [`egui::Event::MouseWheel`] takes points in
    /// [`egui::MouseWheelUnit::Point`], and converting through a
    /// notion of line height here would put a second, disagreeing
    /// definition of a scroll notch in the codebase.
    Scroll(f32),
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
    /// Open the settings window, through the same action the ribbon
    /// button pushes.
    ///
    /// Exists so R86 can be discharged for a window WITHOUT taking the
    /// operator's screen. `gui-shot.ps1` answers visual questions but
    /// raises a real window and grabs the foreground; this harness runs
    /// off-screen and never touches focus, which is the only option when
    /// someone is working at the machine.
    Settings,
    /// Open the Export-DXF window (`export:dxf`), through the same action
    /// the ribbon button pushes (Pass 52.2).
    ///
    /// # Why this step exists and the export needs it more than most
    ///
    /// The export's whole point is a decision made BEFORE a file is written
    /// — the scale — and the scale is inferred from the ce dimensions on the
    /// pages being exported. There is exactly one moment when that inference
    /// happens, and it is the moment this step reproduces. Without it, the
    /// only way to reach the inference is to click a ribbon button by pixel
    /// coordinate, and
    /// `only_the_active_tab_is_emitted_so_scripted_harnesses_cannot_reach_
    /// other_tabs` plus
    /// `scripted_click_coordinates_go_stale_when_a_dock_width_changes` both
    /// say what that costs.
    ///
    /// It also splits the two questions that would otherwise be one failure:
    /// *"does the scale inference reach the dialog"* and *"is the ribbon
    /// button wired up"*. The second still has to be checked by eye; the
    /// first no longer does.
    ExportDxf,
    /// Commit the open Export-DXF draft (`export:dxf-go`), Pass 52.2.
    ///
    /// Pair it with a preceding `export:dxf` and, for a destination,
    /// `PDFCE_DIAG_EXPORT_DIR` — see [`export_dir`].
    ///
    /// # A commit that does nothing is a RESULT here, not a failed step
    ///
    /// On an uncalibrated page with nothing typed, the draft resolves no
    /// scale and this writes no file. That is the feature working: the
    /// Export button is disabled in exactly that state, and this step
    /// reaches the same guard the button does. A script asserting "no
    /// `dxf-export` trace line appeared" is asserting the gate held, which
    /// is the single most valuable thing about this feature to be able to
    /// check from outside.
    ExportDxfCommit,
    /// Dump every editable text LINE's screen rectangle, once
    /// (`text:lines`), Pass 24.0 follow-up.
    ///
    /// # Why the harness could not test canvas text without this
    ///
    /// Enter now commits an Edit Text draft, and that path could not be
    /// driven at all: a script has to CLICK a text run — both to place the
    /// caret and because typing is gated on `image_response.has_focus()`,
    /// which only a real click grants — and nothing emitted where a run
    /// was. R172 forbids guessing coordinates, and the two harnesses use
    /// different window sizes, so a screenshot's pixels are not
    /// transferable either. That left the feature verifiable only by hand.
    ///
    /// **One-shot, not per-frame.** A real drawing puts every label on a
    /// sheet in one text object — measured at 237 runs — and a script runs
    /// for hundreds of frames, so tracing this every frame would emit tens
    /// of thousands of lines to say something that does not change. The
    /// step sets a flag; the tool's own draw consumes it, emits once, and
    /// clears it.
    DumpTextLines,
    /// Choose the Create Field tool's field TYPE
    /// (`field-kind:text|check|radio|choice`).
    ///
    /// Added by Pass 47.4 for the reason every other `Step` in this enum was
    /// added: **a control the observation harness cannot reach is a control
    /// whose defects only the operator finds.** The type selector is a
    /// `selectable_label` in the Tool Options pane, and driving it by
    /// coordinate proved unreliable — which is itself the argument, since a
    /// harness that can only reach a control by guessing its pixel position
    /// re-breaks every time the pane's layout changes.
    ///
    /// Set directly rather than by clicking the selector, exactly as `tool:`
    /// is, so a script isolates *"does a choice field get its options"* from
    /// the separate question *"is the type selector wired up"*.
    FieldKind(&'static str),
    /// Drop a FILE on the canvas (`drop:D:/path/to/image.png`).
    ///
    /// Injects a `dropped_files` entry into the frame's `RawInput`, which is
    /// the same seam a real OS drag-and-drop arrives through — winit fills
    /// that field and egui hands it to the app untouched.
    ///
    /// Added by Pass 47.7 for the reason every other step here exists: an OS
    /// drag-and-drop cannot be scripted from outside the process, so without
    /// this the whole drop path — the only route to placing an image in the
    /// GUI — would be reachable **only by the operator dragging a file with
    /// his own hand**, which is precisely the arrangement this harness exists
    /// to end.
    ///
    /// Pair it with a preceding `move:X,Y` — the placement centres on the
    /// pointer, so a drop with the pointer off-canvas exercises the
    /// centred-on-page fallback instead.
    Drop(String),
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
    /// Open the comments/markup list (`panel:comments`).
    ///
    /// Added by `Pass 38.5`, when that panel's rows stopped being read-only
    /// and grew a Delete control. The same argument as the two variants
    /// above applies with more force now: a panel the harness cannot open
    /// is a panel whose defects only the operator finds, and this one can
    /// now change the document.
    ///
    /// `Action::ToggleCommentsPanel` already existed and the ribbon button
    /// already pushed it — the only thing missing was a way for a script to
    /// say so, which is exactly the shape of gap that leaves a surface
    /// unverified without anybody deciding to leave it unverified.
    Comments,
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
    /// Select a ribbon tab (`tab:edit`, `tab:view`, …).
    ///
    /// Added because the ribbon was entirely undrivable: only the ACTIVE tab's
    /// band is emitted (R125), the default tab is `File`, and every control on
    /// any other tab was therefore unreachable from the harness — including the
    /// master edit switch on `Edit`. A control the observation harness cannot
    /// reach is one whose defects only the operator finds.
    Tab(&'static str),

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
    /// a typo mid-investigation wastes a whole run.
    ///
    /// # But the skip is ANNOUNCED, and it did not used to be
    ///
    /// This doc comment used to end "...and the trace shows which steps
    /// actually ran", offering that as the mitigation. It was not one. An
    /// absent trace line is indistinguishable from a step that ran and
    /// produced no output, so a typo presented as **a feature failing to
    /// respond** rather than as a step that never executed.
    ///
    /// That cost a real investigation on 2026-08-07: a `placefield` step
    /// (correct spelling was `tool:placefield`) was dropped here, and the
    /// resulting silence was read as a defect in the tool-arming code. It was
    /// caught only by running a known-good sibling step and noticing the
    /// difference — which is luck, not method.
    ///
    /// So every dropped step now emits a trace line naming it. The
    /// non-fatal posture is unchanged and still correct; what changed is that
    /// the harness no longer stays quiet about disobeying its instructions.
    /// This is the earliest question in the "green is not evidence" family —
    /// **did my instruction ever execute at all?** — and it has to be
    /// answerable before any later reading of the trace means anything.
    pub fn from_env() -> Option<Self> {
        // ui-text-exempt: environment variable name, never displayed
        let raw = std::env::var("PDFCE_DIAG_SCRIPT").ok()?;
        let (steps, rejected) = parse_script(&raw);
        for bad in &rejected {
            // NOT silent. See this function's doc comment: the whole point is
            // that a dropped step announces itself, because the failure it
            // produces otherwise looks like a bug in the feature under test.
            // ui-text-exempt: diagnostic trace, never displayed in the UI
            trace(|| format!("script-step-UNPARSEABLE step={bad:?} skipped=1"));
        }
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

/// Split a raw script into the steps that parsed and the ones that did NOT.
///
/// Returns `(steps, rejected)` rather than just the steps, so the rejects are
/// a **value the caller must deal with** instead of a side effect it may
/// forget. [`Script::from_env`] traces every reject; a future caller that
/// ignores the second element is at least doing so visibly.
///
/// Empty segments are not rejects — a trailing `;`, or the empty string from
/// `"a;;b"`, is punctuation rather than a typo, and reporting those as
/// unparseable would be noise that trains the reader to ignore the warning.
/// That distinction is the difference between a signal and a nag.
///
/// Kept a free function, like [`parse_step`], so it is unit-testable without
/// an egui context, an environment variable, or captured stderr.
fn parse_script(raw: &str) -> (Vec<Step>, Vec<String>) {
    let mut steps = Vec::new();
    let mut rejected = Vec::new();
    for segment in raw.split(';') {
        match parse_step(segment) {
            Some(step) => steps.push(step),
            None if segment.trim().is_empty() => {}
            None => rejected.push(segment.trim().to_owned()),
        }
    }
    (steps, rejected)
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
        "scroll" => rest.trim().parse().ok().map(Step::Scroll),
        "altdown" => xy().map(|(x, y)| Step::AltClick(true, x, y)),
        "ctrldown" => xy().map(|(x, y)| Step::CtrlClick(true, x, y)),
        "ctrlup" => xy().map(|(x, y)| Step::CtrlClick(false, x, y)),
        "altup" => xy().map(|(x, y)| Step::AltClick(false, x, y)),
        "mdown" => xy().map(|(x, y)| Step::Middle(true, x, y)),
        "mup" => xy().map(|(x, y)| Step::Middle(false, x, y)),
        "delete" => Some(Step::Delete),
        "escape" => Some(Step::Escape),
        "wait" => Some(Step::Wait),
        "panel" if rest.trim() == "groups" => Some(Step::Groups),
        "panel" if rest.trim() == "redact" => Some(Step::Redact),
        "panel" if rest.trim() == "forms" => Some(Step::Forms),
        "panel" if rest.trim() == "comments" => Some(Step::Comments),
        "tab" => match rest.trim() {
            "file" => Some(Step::Tab("file")),
            "edit" => Some(Step::Tab("edit")),
            "review" => Some(Step::Tab("review")),
            "measure" => Some(Step::Tab("measure")),
            "tools" => Some(Step::Tab("tools")),
            "view" => Some(Step::Tab("view")),
            _ => None,
        },
        "key" => match rest.trim() {
            "left" => Some(Step::NavKey("left")),
            "right" => Some(Step::NavKey("right")),
            "up" => Some(Step::NavKey("up")),
            "down" => Some(Step::NavKey("down")),
            "home" => Some(Step::NavKey("home")),
            "end" => Some(Step::NavKey("end")),
            // `key:enter` (Pass 24.0's deferred half). Carried by `NavKey`
            // rather than a variant of its own because the injector is the
            // thing that matters — a real press/release pair through
            // `raw_input.events`, the same seam a physical key arrives on —
            // and Enter needs exactly that and nothing else. The variant is
            // named for its first use, not for a taxonomy.
            //
            // Added because the feature was NOT DRIVABLE WITHOUT IT: Enter
            // now commits an Edit Text draft, and the same keystroke is what
            // the Forms panel's value editors and the field-rename editor
            // rely on for their `lost_focus()` commit. All three are exactly
            // the interaction a global `consume_key` would have broken
            // silently, so the harness has to be able to press it.
            "enter" => Some(Step::NavKey("enter")),
            _ => None,
        },
        // A top-level step, NOT `tool:settings`: settings are not a tool,
        // and filing them under one would make the harness vocabulary lie
        // about the application's own structure.
        "settings" => Some(Step::Settings),
        // A top-level verb like `settings`, and NOT `tool:dxf`: an export is
        // not a canvas tool and arms nothing, so filing it under `tool:`
        // would make the harness vocabulary lie about the application's
        // structure — the objection `settings` records just above.
        "text" if rest.trim() == "lines" => Some(Step::DumpTextLines),
        "export" if rest.trim() == "dxf" => Some(Step::ExportDxf),
        "export" if rest.trim() == "dxf-go" => Some(Step::ExportDxfCommit),
        "view" if rest.trim() == "points" => Some(Step::ShowPoints),
        // NOT `rest.trim()`: leading and trailing spaces are legitimate text.
        "type" if !rest.is_empty() => Some(Step::Text(rest.to_owned())),
        // NOT `rest.trim()` on the payload's interior: a path may legitimately
        // contain spaces, and only the ends are safe to strip.
        "drop" if !rest.trim().is_empty() => Some(Step::Drop(rest.trim().to_owned())),
        "field-kind" => match rest.trim() {
            "text" => Some(Step::FieldKind("text")),
            "check" => Some(Step::FieldKind("check")),
            "radio" => Some(Step::FieldKind("radio")),
            "choice" => Some(Step::FieldKind("choice")),
            _ => None,
        },
        "tool" => match rest.trim() {
            "none" => Some(Step::Tool(ScriptTool::None)),
            "obj" => Some(Step::Tool(ScriptTool::Obj)),
            "measure" => Some(Step::Tool(ScriptTool::Measure)),
            "text" => Some(Step::Tool(ScriptTool::Text)),
            "addtext" => Some(Step::Tool(ScriptTool::AddText)),
            "placefield" => Some(Step::Tool(ScriptTool::PlaceField)),
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

/// Where a scripted DXF export writes, from `PDFCE_DIAG_EXPORT_DIR`.
///
/// # Why this exists
///
/// `commit_dxf_export` asks for its destination through a **native file or
/// folder dialog**, and a native modal is exactly what the scripted-input
/// harness cannot drive. That is the identical wall [`font_dirs`] was built
/// to get past, and it has the identical consequence if left standing:
/// everything downstream of the picker — the page loop, the decompose
/// against the session view, the atomic write, the per-page naming, and
/// every rule-4 disclosure the outcome carries — would be observable only
/// by the operator clicking through a dialog by hand.
///
/// When set, both branches use it: a single-page export writes
/// `<dir>/<stem>.dxf` and a multi-page export writes into `<dir>` exactly as
/// a picked folder would. So the harness exercises the SAME code path,
/// substituting only the answer the dialog would have returned — which is
/// the property that makes this a diagnostic seam rather than a second
/// implementation.
///
/// The picker itself still has to be verified by hand. Everything after it
/// no longer does.
///
/// Off unless asked, like every other member of this module: unset, the
/// dialog opens exactly as it always has.
#[must_use]
pub fn export_dir() -> Option<std::path::PathBuf> {
    // ui-text-exempt: environment variable name, never displayed
    let raw = std::env::var("PDFCE_DIAG_EXPORT_DIR").ok()?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| std::path::PathBuf::from(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `export:dxf` parses, and the near-misses do not silently become it.
    #[test]
    fn the_export_step_parses_and_its_near_misses_are_rejected() {
        assert_eq!(parse_step("export:dxf"), Some(Step::ExportDxf));
        assert_eq!(parse_step("export: dxf "), Some(Step::ExportDxf));
        assert_eq!(parse_step("export:dxf-go"), Some(Step::ExportDxfCommit));
        assert_eq!(parse_step("key:enter"), Some(Step::NavKey("enter")));
        assert_eq!(parse_step("text:lines"), Some(Step::DumpTextLines));
        assert_eq!(parse_step("text"), None);
        // Rejected, not silently coerced — and a reject is TRACED by
        // `Script::from_env`, which is the whole point of the 2026-08-07
        // `placefield` lesson recorded on this module.
        assert_eq!(parse_step("export"), None);
        assert_eq!(parse_step("export:pdf"), None);
        assert_eq!(parse_step("tool:dxf"), None);
    }

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

    /// The real typo, from the real investigation it cost.
    ///
    /// On 2026-08-07 a script step was written `placefield` when the verb is
    /// `tool:placefield`. It was dropped silently, and the resulting absence
    /// of tool-arming traces was read as a defect in the tool-arming code —
    /// caught only by running a known-good sibling step and noticing the
    /// difference.
    ///
    /// This asserts the two halves that make that impossible to repeat: the
    /// bad step is REPORTED, and the good steps around it still run (the
    /// non-fatal posture, which is deliberate and must not regress into
    /// aborting the whole script).
    #[test]
    fn an_unparseable_step_is_reported_and_does_not_stop_the_script() {
        let (steps, rejected) = parse_script("wait;placefield;wait");
        assert_eq!(
            rejected,
            vec!["placefield".to_owned()],
            "a step the parser cannot read must come back as a REJECT the \
             caller has to handle, never be dropped on the floor — an absent \
             trace line is indistinguishable from a step that ran and did \
             nothing, which is exactly how this typo was misread as a bug in \
             the feature under test",
        );
        assert_eq!(
            steps.len(),
            2,
            "the surrounding steps must still run: dying on a typo \
             mid-investigation wastes a whole run, which is why the skip is \
             non-fatal and only its SILENCE was the defect",
        );
    }

    /// The correctly-spelled verb parses — which is what makes the test above
    /// mean something.
    ///
    /// R162: without this, `an_unparseable_step_is_reported…` would pass
    /// identically if `tool:` had never been a valid verb at all, and would
    /// be asserting nothing about the typo it names.
    #[test]
    fn the_correctly_spelled_verb_parses_so_the_typo_test_is_not_vacuous() {
        let (steps, rejected) = parse_script("tool:placefield");
        assert!(
            rejected.is_empty(),
            "`tool:placefield` is the CORRECT spelling; if it rejects here \
             then the typo test above proves nothing about spelling",
        );
        assert_eq!(steps.len(), 1);
    }

    /// Punctuation is not a typo.
    ///
    /// A trailing `;` and the empty segment in `a;;b` are both routine. If
    /// those were reported the warning would fire on well-formed scripts,
    /// and a warning that cries wolf is worse than no warning — it teaches
    /// the reader to skip exactly the line that matters.
    #[test]
    fn empty_segments_are_punctuation_not_rejects() {
        let (steps, rejected) = parse_script("wait;;wait;");
        assert!(rejected.is_empty(), "empty segments must not be reported");
        assert_eq!(steps.len(), 2);
    }
}
