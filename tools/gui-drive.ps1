<#
.SYNOPSIS
    Run pdfce-gui off-screen, drive it with a scripted input sequence, and
    print its diagnostic trace. Never touches the operator's screen, mouse,
    keyboard or focus.

.DESCRIPTION
    Standing rule R86 says a GUI defect is settled in the running application,
    not by reading the dispatch chain. But the operator is usually working at
    that machine, so the running application is usually unavailable. This
    closes that gap: the window is placed far outside the visible desktop and
    marked inactive, input is injected through eframe's `raw_input_hook`, and
    the result comes back as text on stderr.

    Use this when the question is behavioural ("did the click land", "what did
    the hit test return", "did the commit succeed"). Use tools/gui-shot.ps1
    when the question is visual ("is the outline drawn", "where did the panel
    end up").

.PARAMETER Script
    A PDFCE_DIAG_SCRIPT step list, semicolons between steps, one step per
    frame. The window closes when the script runs dry, so a run lasts exactly
    as long as its script.

      wait                 burn a frame (let a raster / provider rebuild settle)
      move:X,Y             move the pointer, screen points
      down:X,Y  up:X,Y     primary button
      mdown:X,Y mup:X,Y    middle button (the pan gesture)
      zoom:F               a Ctrl+wheel step by factor F
      delete               press and release Delete
      tool:none|obj|measure  arm a tool through the real toolbar action

.PARAMETER Filter
    Regex applied to the trace before printing. Default shows the lines that
    answer most questions; pass '.' for everything.

.EXAMPLE
    # Does a click at this point select anything, and what?
    pwsh tools/gui-drive.ps1 -Pdf cad-drawing-a.pdf `
      -Script (("wait;"*25) + "move:819,513;wait;down:819,513;up:819,513;wait;wait")

.EXAMPLE
    # Double-click to descend into an object, then click away to leave.
    pwsh tools/gui-drive.ps1 -Pdf cad-drawing-a.pdf `
      -Script (("wait;"*25) + "down:819,513;up:819,513;down:819,513;up:819,513;wait;wait;move:600,300;wait;down:600,300;up:600,300;wait")

.NOTES
    PICKING SCREEN COORDINATES. Do not guess them, and do not reuse them
    across a layout change — a hard-coded point silently stops hitting
    anything, which reads exactly like a broken feature. Take `rect=` and
    `zoom=` from one run's `canvas` trace line, then:

        canvas_y = page_height - pdf_y
        screen   = image_rect.min + canvas * zoom

    WHAT THIS SCRIPT USED TO DO, AND WHY IT DOES NOT. The first version drove
    the window with Win32 PostMessage(WM_MOUSEMOVE / WM_LBUTTONDOWN). That
    does not work for an off-screen window and the failure is silent: winit
    calls TrackMouseEvent on the move, Windows answers WM_MOUSELEAVE because
    the physical cursor is elsewhere, and egui-winit then drops the button
    entirely because it emits PointerButton only when it knows the pointer
    position. The observed event list was [PointerMoved, PointerGone] in every
    message ordering tried, including move and button posted back to back.
    Recorded here so nobody rebuilds it. (Full finding: D:\dev\rag\egui\.)

    SendInput / keybd_event — as tools/gui-click.ps1 uses — are a different
    thing again: they inject at the system level, moving the real cursor and
    acting on whatever window is in front. Correct for driving a window the
    operator can see; completely wrong here.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Pdf,
    [string]$Script = "wait",
    [string]$Filter = 'plain-click|vector-click|depth-click|commit-|canvas tool|refus|error',
    [string]$Exe = "$PSScriptRoot\..\target\release\pdfce-gui.exe",
    [string]$Log = "$env:TEMP\pdfce-diag.txt",
    [int]$TimeoutSeconds = 120,
    # Far enough negative to be off every plausible multi-monitor arrangement,
    # while still a real on-screen-coordinate window that lays out and
    # interacts normally.
    [int]$X = -4000, [int]$Y = -4000, [int]$W = 1600, [int]$H = 1000,
    # Run against a binary older than the sources. Escape hatch only —
    # see the staleness check below for why this defaults off.
    [switch]$AllowStaleBinary
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) { throw "no binary at $Exe — cargo build --release -p pdfce-gui" }
if (-not (Test-Path $Pdf)) { throw "no document at $Pdf" }

# ---------------------------------------------------------------------------
# STALENESS GATE — refuse to drive a binary older than the code.
#
# This script defaults to target\release\. A developer who has been running
# `cargo test` (debug) and then drives this harness is testing a build that
# predates everything they just wrote — and the failure mode is the worst
# kind: the traces they expect are simply ABSENT, which reads as "the feature
# does not work" rather than "the feature was never compiled".
#
# That is not hypothetical. On 2026-08-07 an agent building the field-deletion
# panel got zero `form-delete` traces and nearly concluded the controls did
# not render. The binary predated every change it had made.
#
# R163 (prefer a compile error over a rule asking a human to remember): the
# obligation "rebuild release before driving the GUI" is exactly the kind a
# mechanical gate can carry, so it is carried here instead of written down
# somewhere and forgotten. An absence is only evidence when the thing that
# would have produced it was actually built.
# ---------------------------------------------------------------------------
$exeTime = (Get-Item $Exe).LastWriteTimeUtc
$newestSrc = Get-ChildItem "$PSScriptRoot\..\crates" -Recurse -Include *.rs, Cargo.toml -File `
    -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if ($newestSrc -and $newestSrc.LastWriteTimeUtc -gt $exeTime) {
    # The verb differs per path, because a warning that says "refusing to
    # run" while it runs is itself a misleading instrument — which is the
    # entire class of defect this gate exists to prevent.
    $verb = if ($AllowStaleBinary) { "running anyway (-AllowStaleBinary)" } else { "refusing to run" }
    $msg = @"
STALE BINARY — $verb.

  binary : $Exe
           built $exeTime UTC
  newest : $($newestSrc.FullName)
           edited $($newestSrc.LastWriteTimeUtc) UTC

The traces you are about to collect would describe code that is NOT the code
you just wrote, and a missing trace would look exactly like a broken feature.

  cargo build --release -p pdfce-gui

Pass -AllowStaleBinary only if you intend to drive the older build.
"@
    if ($AllowStaleBinary) { Write-Warning $msg } else { throw $msg }
}
Remove-Item $Log -ErrorAction SilentlyContinue

$env:PDFCE_DIAG = "1"
$env:PDFCE_DIAG_VIEWPORT = "$X,$Y,$W,$H"
$env:PDFCE_DIAG_SCRIPT = $Script

$proc = Start-Process -FilePath $Exe -ArgumentList $Pdf -PassThru `
    -RedirectStandardError $Log -WindowStyle Hidden
if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
    Write-Warning "script did not run dry within ${TimeoutSeconds}s — killing"
    $proc | Stop-Process -Force
}

$trace = Get-Content $Log -ErrorAction SilentlyContinue
if (-not $trace) {
    # An empty trace is ambiguous, so the app emits an unconditional `start`
    # line. No output at all means the process never got as far as that.
    throw "no trace at $Log — the process produced nothing (bad binary? crash on open?)"
}
if (-not ($trace | Select-String -Pattern '^pdfce-diag start')) {
    throw "trace has no 'start' line — PDFCE_DIAG did not reach the process"
}
$trace | Select-String -Pattern $Filter
