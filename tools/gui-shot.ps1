<#
.SYNOPSIS
    Launch pdfce-gui on the real screen, drive it with a diag script, and
    capture a PNG of its window.

.DESCRIPTION
    The counterpart to tools/gui-drive.ps1. That script exists for when the
    operator is at the machine and the screen must be left alone; this one is
    for when the screen IS available and the question is visual -- "does the
    selection outline actually draw", "does the page jump", "what does the
    toolbar look like" -- which no amount of stderr tracing can answer.

    The window is placed at a known position and driven by the same
    PDFCE_DIAG_SCRIPT mechanism, so a screenshot is reproducible rather than
    dependent on whatever state a hand-driven session happened to reach.

.PARAMETER Script
    A PDFCE_DIAG_SCRIPT step list. End it with a long run of `wait` steps so
    the window is still up when the capture happens.

.PARAMETER Shot
    Where the PNG is written.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Pdf,
    [string]$Script = "wait",
    [string]$Shot = "$env:TEMP\pdfce-shot.png",
    [int]$CaptureAfterSeconds = 12,
    [string]$Exe = "$PSScriptRoot\..\target\release\pdfce-gui.exe",
    [string]$Log = "$env:TEMP\pdfce-shot-trace.txt",
    [int]$X = 40, [int]$Y = 40, [int]$W = 1760, [int]$H = 1150,
    # Capture region, defaulting to the whole window. Kept SEPARATE from the
    # window rect because conflating them silently resizes the application to
    # crop a screenshot -- which changes the very layout being inspected, so the
    # picture answers a different question from the one asked.
    [int]$CropX = -1, [int]$CropY = -1, [int]$CropW = -1, [int]$CropH = -1
)


# ROBUST $Exe DEFAULT -- do not fold this back into the param block.
#
# The param-block default above is correct PowerShell and works under
# PowerShell 7 (pwsh). Under Windows PowerShell 5.1 invoked as
# `powershell -File <this>`, this file's $Exe default came back WITHOUT
# the $PSScriptRoot prefix, resolving to `\..\target\release\pdfce-gui.exe`
# and throwing "no binary at" before doing anything.
#
# A minimal script with the same `param([string]$P = "$PSScriptRoot\x")`
# shape does NOT reproduce it -- $PSScriptRoot expands correctly there
# under both hosts -- so the cause is specific to this file and is NOT
# the general "PSScriptRoot is unavailable in param defaults" claim it
# first looked like. Rather than ship an explanation that testing had
# already contradicted, the value is recomputed here, in the BODY, where
# $PSScriptRoot is verified to work under both hosts.
#
# Only when the caller did not pass -Exe: an explicit path must win.
if (-not $PSBoundParameters.ContainsKey('Exe') -and $PSScriptRoot) {
    $Exe = Join-Path $PSScriptRoot '..\target\release\pdfce-gui.exe'
}

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$env:PDFCE_DIAG = "1"
$env:PDFCE_DIAG_VIEWPORT = "$X,$Y,$W,$H"
# Pad the script so the window outlives the capture no matter how fast the
# steps run -- a script that ends closes the window (see diag::Script).
$env:PDFCE_DIAG_SCRIPT = $Script + (";wait" * 4000)

$proc = Start-Process -FilePath $Exe -ArgumentList $Pdf -PassThru -RedirectStandardError $Log
Start-Sleep -Seconds $CaptureAfterSeconds

# RAISE THE WINDOW BEFORE CAPTURING. Added 2026-08-05 (Pass 34.2) after a
# capture returned a pixel-perfect screenshot of SOLIDWORKS: pdfce had started,
# run its whole script and traced correctly, but its window was created behind
# an already-running maximised application, so `CopyFromScreen` at the window's
# nominal rect photographed whatever owned those pixels instead.
#
# This is the same failure class the comment below warns about -- an image that
# looks like real evidence and is not -- so it gets the same treatment: make it
# impossible rather than remember to check. `SetForegroundWindow` is best-effort
# by Windows' own rules (a process without foreground rights may be refused),
# which is why the capture is still verified by eye and not trusted blindly.
if (-not ('Win32Fg' -as [type])) {
    Add-Type -Namespace Win32Fg -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
'@ -PassThru | Out-Null
}
$proc.Refresh()
if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
    [Win32Fg.U]::ShowWindow($proc.MainWindowHandle, 5) | Out-Null   # SW_SHOW
    [Win32Fg.U]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    # A short settle after the raise. 700 ms, measured -- see the CORRECTION
    # below before changing it.
    #
    # CORRECTION 2026-08-05, same day, and the correction matters more than the
    # number. This line briefly read 2500 ms with a comment asserting that at
    # 700 ms "the raise had happened but DWM had not recomposited the GPU
    # surface yet". THAT CAUSE WAS INVENTED. It was a plausible story told
    # about a symptom (a uniform WHITE client area under a correct pdfce title
    # bar) without testing it, and raising the sleep appeared to fix the
    # problem because the real cause happened to go away at the same time.
    #
    # The real cause was the DISPLAY POWER STATE. The operator's monitor had
    # gone to sleep; `CopyFromScreen` reads the composited desktop, and there
    # is nothing to read from a powered-down display. The operator identified
    # it ("I set the display to always stay on so screenshots should stay
    # working now") -- which is also why the blanks came back later at 20 s,
    # something a recomposite race could not explain and which should have
    # falsified the story at the time.
    #
    # Re-measured after the fact: three consecutive captures at 700 ms, each
    # with identical non-blank content. The longer sleep bought nothing and
    # cost 1.8 s per capture.
    #
    # The lesson kept, because this project has now recorded it five times: a
    # comment stating a CAUSE is a claim, and a change that appears to fix
    # something is not evidence for the claim. If the cause was not tested,
    # say so in the comment instead of naming one.
    Start-Sleep -Milliseconds 700
} else {
    Write-Warning "gui-shot: no main window handle -- the capture may photograph another app."
}

# Capture the screen region the window occupies rather than the window's own
# device context: a GPU-composited (wgpu/glow) surface is frequently blank in a
# PrintWindow/BitBlt of the window DC, which would silently produce an empty
# image and "prove" a rendering bug that does not exist.
$cx = if ($CropX -ge 0) { $CropX } else { $X }
$cy = if ($CropY -ge 0) { $CropY } else { $Y }
$cw = if ($CropW -gt 0) { $CropW } else { $W }
$ch = if ($CropH -gt 0) { $CropH } else { $H }
$bmp = New-Object System.Drawing.Bitmap $cw, $ch
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($cx, $cy, 0, 0, (New-Object System.Drawing.Size $cw, $ch))
$bmp.Save($Shot, [System.Drawing.Imaging.ImageFormat]::Png)

# REFUSE A UNIFORM CAPTURE, loudly. Added 2026-08-05 after a run of blank
# screenshots got a WRONG CAUSE invented for them (see the CORRECTION above).
#
# This is the fix that actually matters, and it is not the sleep. A blank
# capture is indistinguishable from a real one at the call site: the file
# exists, the command succeeds, and the only thing that says "this is not
# evidence" is a human looking at it. That is exactly how a plausible cause
# gets attached to an unexamined symptom -- the failure was silent, so it got
# a story instead of a diagnosis.
#
# `observe-gui.ps1` was hardened the same way on 2026-08-03, for the same
# reason, and gui-shot simply never got the same treatment.
#
# Sampled on a grid rather than every pixel: a 1760x1150 capture is 2M
# GetPixel calls in PowerShell, which is slower than the capture itself, and a
# uniform image is uniform at any sampling density.
$distinct = @{}
for ($sy = 0; $sy -lt $bmp.Height; $sy += 17) {
    for ($sx = 0; $sx -lt $bmp.Width; $sx += 17) {
        $distinct[$bmp.GetPixel($sx, $sy).ToArgb()] = $true
        if ($distinct.Count -gt 4) { break }
    }
    if ($distinct.Count -gt 4) { break }
}
$g.Dispose(); $bmp.Dispose()
if ($distinct.Count -le 4) {
    Write-Warning @"
gui-shot: the capture is (near-)UNIFORM -- $($distinct.Count) distinct colour(s) sampled.
This is almost certainly NOT a picture of pdfce. Do not treat it as evidence.
Known causes, in the order they have actually occurred:
  1. The DISPLAY IS ASLEEP or powered off. CopyFromScreen reads the composited
     desktop and there is nothing there to read. Wake it / set it to stay on.
  2. The window was not raised, so the capture region belongs to another app
     (this one usually looks like a screenshot OF THAT APP, not a blank).
  3. pdfce died before the capture -- check $Log.
Re-run after ruling those out; do not raise the settle sleep and assume it fixed
it, which is what happened the first time.
"@
}

$proc | Stop-Process -Force
Write-Host "shot=$Shot"
Get-Content $Log -ErrorAction SilentlyContinue | Select-String "vector-click|plain-click|commit-" | Select-Object -Last 5
