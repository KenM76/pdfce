<#
.SYNOPSIS
    Capture a screenshot of a running pdfce-gui window, so the ENGINEER can
    verify operator-facing behavior instead of asking the operator to test it.

.DESCRIPTION
    This is the observation half of the project's operator-visible definition of
    done (decision 018 §11.2): *a Pass that adds or changes operator-facing
    behavior does not ship until that behavior has been observed working in the
    running application, not merely tested headlessly.*

    Ken accepted that rule on 2026-08-02 with an explicit condition — "only if
    you can do the observing. I don't want to be a beta tester." This script is
    what makes the rule executable. Without it, "observed working" degrades into
    "asked Ken to look," which is exactly what the condition forbids.

    WHY THIS EXISTS RATHER THAN AN egui_kittest HARNESS
    ---------------------------------------------------
    `egui_kittest` is the better long-term answer for *automated, asserted*
    snapshot testing in CI, and remains on the roadmap. It is not a substitute
    for this: kittest drives a synthetic egui context, so it proves what the
    widget tree *would* paint. This captures the REAL process, with the real
    wgpu/glow backend, the real font stack, the real window manager, and the
    real PDF raster — which is precisely the layer where the decision-018 defect
    lived (the widget tree was correct; the texture handed to it was of the
    wrong document revision). A kittest snapshot would have passed while the
    operator saw an unchanged page.

    Use both, for different questions. This answers "what is actually on Ken's
    screen right now."

.PARAMETER Path
    Output PNG path. Defaults to a timestamped file under tools/observations/,
    which is gitignored — these are diagnostic artifacts, not fixtures, and must
    never be committed (they are screenshots of whatever PDF happened to be
    open, which would bypass the LEGAL.md §5 test-corpus provenance rules).

.PARAMETER ProcessName
    Process to capture. Defaults to pdfce-gui.

.PARAMETER Foreground
    Bring the window to the front before capturing. Default true — a window
    that is behind another window captures the OCCLUDING window's pixels with
    the PrintWindow-free BitBlt path used here, which would produce a
    confidently wrong observation. Turning this off is only correct when the
    window is already known to be unobscured.

.PARAMETER SettleMs
    Milliseconds to wait after raising the window before capturing. egui is an
    immediate-mode GUI that only repaints on demand, and a raise triggers a
    repaint; capturing too early can catch a stale or partially-drawn frame.

.OUTPUTS
    The full path of the written PNG, so a caller can pipe it straight into an
    image-reading step.

.EXAMPLE
    pwsh -File tools/observe-gui.ps1
    # -> D:\Dev\pdfce\tools\observations\pdfce-gui-20260802-191530.png

.EXAMPLE
    pwsh -File tools/observe-gui.ps1 -Path D:\tmp\before.png -Foreground:$false
#>
[CmdletBinding()]
param(
    [string] $Path,
    [string] $ProcessName = 'pdfce-gui',
    # NOTE: deliberately NOT named $Pid — that is a PowerShell AUTOMATIC
    # variable holding the CURRENT process id, and shadowing it in a param
    # block is both an error and a genuinely confusing one to diagnose.
    [int]      $ProcessId   = 0,
    [bool]   $Foreground  = $true,
    [int]    $SettleMs    = 600
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

# Win32 interop. GetWindowRect gives the window's screen rectangle (including
# the frame); SetForegroundWindow + ShowWindow(SW_RESTORE) ensure we are not
# screenshotting a minimized or occluded window and silently reporting success.
if (-not ('PdfceNative.Win32' -as [type])) {
    Add-Type -Namespace PdfceNative -Name Win32 -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)]
public struct RECT { public int Left, Top, Right, Bottom; }

[DllImport("user32.dll", SetLastError = true)]
public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

[DllImport("user32.dll")]
public static extern bool SetForegroundWindow(IntPtr hWnd);

[DllImport("user32.dll")]
public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

[DllImport("user32.dll")]
public static extern bool IsIconic(IntPtr hWnd);

[DllImport("user32.dll")]
public static extern IntPtr GetForegroundWindow();

[DllImport("user32.dll")]
public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);

[DllImport("user32.dll")]
public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);

[StructLayout(LayoutKind.Sequential)]
public struct POINT { public int X, Y; }
'@
}

$SW_RESTORE = 9

$candidates = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 })

# Disambiguate BEFORE acting. Taking the first match by name was a real hazard:
# a build agent verifying its own binary found a second instance running from
# the shared checkout, and had to kill it — because a capture aimed at "the
# pdfce-gui window" could equally have hit the other one. Pass -ProcessId to
# name exactly which, or close the extras.
if ($ProcessId -gt 0) {
    $proc = $candidates | Where-Object { $_.Id -eq $ProcessId } | Select-Object -First 1
    if (-not $proc) {
        throw "No '$ProcessName' process with id $ProcessId and a visible main window. Running ids: $(($candidates | ForEach-Object { $_.Id }) -join ', ')"
    }
} elseif ($candidates.Count -gt 1) {
    throw "AMBIGUOUS TARGET: $($candidates.Count) '$ProcessName' processes have a visible window (ids: $(($candidates | ForEach-Object { $_.Id }) -join ', ')). Refusing to guess which one to capture — pass -ProcessId <id>, or close the others."
} else {
    $proc = $candidates | Select-Object -First 1
}

if (-not $proc) {
    throw "No running '$ProcessName' process with a visible main window. Launch it first — observing a process that is not running is the failure this script exists to make impossible to overlook."
}

$hwnd = $proc.MainWindowHandle

if ($Foreground) {
    # A minimized window has a valid handle but a meaningless rect, so restore
    # before measuring rather than after.
    if ([PdfceNative.Win32]::IsIconic($hwnd)) {
        [void][PdfceNative.Win32]::ShowWindow($hwnd, $SW_RESTORE)
        Start-Sleep -Milliseconds 300
    }
    [void][PdfceNative.Win32]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds $SettleMs

    # VERIFY the raise actually took. Windows refuses SetForegroundWindow in
    # several documented situations (another process owns the foreground and
    # has focus-steal protection, a menu is open elsewhere, the shell is busy),
    # and it reports failure only through the return value people ignore.
    #
    # This check exists because it ALREADY BIT: on 2026-08-02 an unrelated
    # terminal window came to the front between the raise and the capture, and
    # CopyFromScreen dutifully photographed THAT window. The screenshot was
    # returned as if it were pdfce. A capture of the wrong window is worse than
    # no capture at all — it is evidence that looks like evidence, and under
    # standing rule R86 it would be used to certify a Pass as observed-working.
    #
    # So: fail loudly rather than return a plausible-looking wrong image.
    $fg = [PdfceNative.Win32]::GetForegroundWindow()
    if ($fg -ne $hwnd) {
        throw "REFUSING TO CAPTURE: '$ProcessName' (handle $hwnd) is not the foreground window (foreground is $fg). Another window is on top, so a screen capture of this rect would photograph the WRONG window. Close/minimize the occluding window and retry, or pass -Foreground:`$false only if you have independently confirmed the target is unobscured."
    }
}

$rect = New-Object PdfceNative.Win32+RECT
if (-not [PdfceNative.Win32]::GetWindowRect($hwnd, [ref] $rect)) {
    throw "GetWindowRect failed for '$ProcessName' (handle $hwnd)."
}

$width  = $rect.Right  - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) {
    throw "Window rect for '$ProcessName' is degenerate (${width}x${height}) — the window is probably minimized or mid-transition."
}

if (-not $Path) {
    $dir = Join-Path $PSScriptRoot 'observations'
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
    $stamp = (Get-Date).ToString('yyyyMMdd-HHmmss')
    $Path = Join-Path $dir "$ProcessName-$stamp.png"
}

$bitmap   = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)

    # REFUSE A UNIFORM FRAME. A capture that is entirely one colour is not a
    # picture of the UI — it is a picture of nothing, and it is indistinguishable
    # from a real screenshot until someone looks closely.
    #
    # Three distinct causes have all produced one in this project:
    #   * the monitor slept          -> solid black
    #   * eframe had not yet presented a frame (it repaints reactively, only on
    #     input) -> solid white client area
    #   * a maximize/restore animation was mid-flight when the capture ran
    #
    # In every case the honest answer is "no observation was made". Returning
    # the file anyway invites exactly the error this harness exists to prevent:
    # certifying a Pass as observed-working (standing rule R86) on the strength
    # of an image containing no evidence. Sampling a sparse grid is enough --
    # any real UI has a toolbar, text or a page edge somewhere in it.
    # Sample the CLIENT area only, never the whole window.
    #
    # The title bar is painted by the shell, not by the app, so it always has
    # colour variation — an app icon, a caption, close buttons. Sampling the
    # full window therefore finds "several distinct colours" even when the
    # application surface underneath is entirely blank, and the guard passes on
    # a frame containing no evidence.
    #
    # That is not theoretical: it let through a capture with a fully painted
    # title bar and a completely white client area, because eframe had not yet
    # presented. The all-black sleeping-display case fired correctly on the very
    # next attempt, which is exactly the trap — a guard that catches the obvious
    # failure and misses the subtle one is more dangerous than no guard, because
    # it earns trust it does not deserve.
    $clientOrigin = New-Object PdfceNative.Win32+POINT
    $clientRect   = New-Object PdfceNative.Win32+RECT
    $sx0 = 0; $sy0 = 0; $sw = $width; $sh = $height
    if ([PdfceNative.Win32]::GetClientRect($hwnd, [ref] $clientRect) -and
        [PdfceNative.Win32]::ClientToScreen($hwnd, [ref] $clientOrigin)) {
        $cw = $clientRect.Right - $clientRect.Left
        $ch = $clientRect.Bottom - $clientRect.Top
        if ($cw -gt 0 -and $ch -gt 0) {
            $sx0 = $clientOrigin.X - $rect.Left
            $sy0 = $clientOrigin.Y - $rect.Top
            $sw  = [Math]::Min($cw, $width  - $sx0)
            $sh  = [Math]::Min($ch, $height - $sy0)
        }
    }

    $distinct = @{}
    $stepX = [Math]::Max(1, [int]($sw / 24))
    $stepY = [Math]::Max(1, [int]($sh / 24))
    for ($x = $sx0; $x -lt ($sx0 + $sw); $x += $stepX) {
        for ($y = $sy0; $y -lt ($sy0 + $sh); $y += $stepY) {
            $c = $bitmap.GetPixel($x, $y)
            $distinct["$($c.R),$($c.G),$($c.B)"] = $true
            if ($distinct.Count -gt 3) { break }
        }
        if ($distinct.Count -gt 3) { break }
    }
    if ($distinct.Count -le 1) {
        $only = ($distinct.Keys | Select-Object -First 1)
        throw "REFUSING TO RETURN A BLANK CAPTURE: every sampled pixel of '$ProcessName'`s CLIENT AREA is ($only). That is not a screenshot of the UI — the display is likely asleep, or eframe has not presented a frame yet (it repaints only on input; send a real mouse move or click first), or a window animation was mid-flight. No observation was made."
    }

    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}

Write-Output $Path
