<#
.SYNOPSIS
    Send a real mouse click (or a click sequence) to the running pdfce-gui
    window, in WINDOW-RELATIVE coordinates.

.DESCRIPTION
    The other half of the observation harness that `tools/observe-gui.ps1`
    starts. Observation alone can only confirm static state; most of pdfce's
    operator-facing behavior is a GESTURE — pick two points, then Accept — and
    cannot be reached without driving the pointer.

    This exists because of standing rule R86 (decision 018 §11.2): a Pass that
    adds or changes operator-facing behavior does not ship until that behavior
    has been observed working in the running application. Ken accepted that rule
    on 2026-08-02 conditioned on the ENGINEER doing the observing — "I don't want
    to be a beta tester." Verifying a two-click authoring gesture therefore
    requires synthesizing the two clicks.

    WHY WINDOW-RELATIVE COORDINATES
    -------------------------------
    Callers reason about positions by looking at a screenshot from
    `observe-gui.ps1`, whose origin is the window's top-left. Making this script
    take the same coordinate space means a pixel measured in the screenshot can
    be clicked directly, with no mental offset arithmetic — the single most
    likely source of a confidently-wrong automated click. The script resolves
    the window rect itself and adds the offset.

    WHY SendInput RATHER THAN PostMessage
    -------------------------------------
    egui/winit consume real input events from the OS queue. Posting synthetic
    WM_LBUTTONDOWN to the window handle does not reliably drive winit's event
    loop and, worse, can appear to work while bypassing the hit-testing path the
    operator actually exercises — an observation that proves nothing. SendInput
    injects at the same layer a physical mouse does, so what is verified is the
    real path.

    LIMITATION, STATED PLAINLY: this moves the actual system cursor and steals
    focus. It is a development verification aid for an interactive session, not
    something to run unattended while the operator is using the machine.

.PARAMETER Clicks
    One or more "X,Y" window-relative points to click in order. Example:
    -Clicks "1235,45","1200,120"

.PARAMETER ProcessName
    Target process. Defaults to pdfce-gui.

.PARAMETER DelayMs
    Pause after each click, letting egui process the event and repaint. egui is
    immediate-mode and only repaints on demand; clicking again too quickly can
    land before the UI has reacted to the previous click.

.PARAMETER MoveOnly
    Move the cursor to the single given point without clicking — used to park
    the pointer somewhere harmless before a screenshot, so a hover highlight or
    tooltip does not contaminate the observation.

.PARAMETER Modifiers
    Modifier keys to HOLD DOWN for the whole click sequence: any of Shift,
    Ctrl, Alt. Added because several canvas gestures are only reachable with a
    modifier — Shift+click toggles selection membership, and Alt+click steps
    click-through cycling to the next object under the pointer — and R86 says
    those behaviors do not ship until they have been observed in the running
    app. Without this the modifier paths could only ever be tested headlessly,
    which is exactly the gap the harness exists to close.

    The keys are pressed before the first point and released in a `finally`, so
    an exception mid-sequence (a refused out-of-window point, a foreground
    steal) cannot leave Alt latched down across the operator's desktop — a
    stuck modifier is a far nastier side effect than a failed observation.

    Uses `keybd_event` for the same reason the clicks use `mouse_event`: it
    injects at the OS layer winit reads, so `egui::Modifiers` sees the real
    state rather than a synthesized message the event loop may ignore.

.EXAMPLE
    pwsh -File tools/gui-click.ps1 -Clicks "1235,45"
    # click the Measure menu

.EXAMPLE
    pwsh -File tools/gui-click.ps1 -Clicks "700,300","900,300" -DelayMs 500
    # pick two canvas points for a linear dimension
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]] $Clicks,
    [string]   $ProcessName = 'pdfce-gui',
    [int]      $DelayMs     = 400,
    [switch]   $MoveOnly,
    [ValidateSet('Shift', 'Ctrl', 'Alt')]
    [string[]] $Modifiers   = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not ('PdfceInput.Win32' -as [type])) {
    Add-Type -Namespace PdfceInput -Name Win32 -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)]
public struct RECT { public int Left, Top, Right, Bottom; }

[DllImport("user32.dll", SetLastError = true)]
public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

[DllImport("user32.dll")]
public static extern bool SetForegroundWindow(IntPtr hWnd);

[DllImport("user32.dll")]
public static extern bool SetCursorPos(int X, int Y);

[DllImport("user32.dll")]
public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);

[DllImport("user32.dll")]
public static extern IntPtr GetForegroundWindow();

[DllImport("user32.dll")]
public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
'@
}

$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP   = 0x0004

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } |
        Select-Object -First 1
if (-not $proc) { throw "No running '$ProcessName' window to click." }

[void][PdfceInput.Win32]::SetForegroundWindow($proc.MainWindowHandle)
Start-Sleep -Milliseconds 400

# VERIFY the raise took before injecting any click. This is more serious here
# than in observe-gui.ps1: a screenshot of the wrong window is merely useless,
# but a CLICK delivered to the wrong window is an unintended action on someone
# else's application. SendInput goes to whatever is under the cursor, and the
# per-point rect check below only proves the coordinates fall inside pdfce's
# rectangle — not that pdfce is the window actually on top of that rectangle.
#
# This bit on 2026-08-02: an unrelated terminal window occupying the same screen
# region came to the foreground mid-sequence, so synthesized canvas picks landed
# in it. Refuse instead.
$fg = [PdfceInput.Win32]::GetForegroundWindow()
if ($fg -ne $proc.MainWindowHandle) {
    throw "REFUSING TO CLICK: '$ProcessName' (handle $($proc.MainWindowHandle)) is not the foreground window (foreground is $fg). Clicks would be delivered to whatever window is on top — possibly another application. Raise '$ProcessName' and retry."
}

$rect = New-Object PdfceInput.Win32+RECT
if (-not [PdfceInput.Win32]::GetWindowRect($proc.MainWindowHandle, [ref] $rect)) {
    throw "GetWindowRect failed for '$ProcessName'."
}

# Virtual-key codes for the held modifiers, and the KEYEVENTF_KEYUP flag.
$VK = @{ Shift = 0x10; Ctrl = 0x11; Alt = 0x12 }
$KEYEVENTF_KEYUP = 0x0002

$held = @($Modifiers | Select-Object -Unique)
foreach ($m in $held) {
    [PdfceInput.Win32]::keybd_event([byte]$VK[$m], 0, 0, [UIntPtr]::Zero)
}
Start-Sleep -Milliseconds 80

try {
    foreach ($c in $Clicks) {
        $parts = $c -split ','
        if ($parts.Count -ne 2) { throw "Bad point '$c' — expected 'X,Y'." }
        $x = [int]$parts[0].Trim()
        $y = [int]$parts[1].Trim()

        $sx = $rect.Left + $x
        $sy = $rect.Top  + $y

        # Guard against clicking outside the target window: a click that lands
        # on another application is not merely a failed observation, it is an
        # action taken on something the operator did not intend.
        if ($sx -lt $rect.Left -or $sx -gt $rect.Right -or $sy -lt $rect.Top -or $sy -gt $rect.Bottom) {
            throw "Point ($x,$y) resolves to ($sx,$sy), outside the '$ProcessName' window rect [$($rect.Left),$($rect.Top) .. $($rect.Right),$($rect.Bottom)]. Refusing to click outside the target window."
        }

        [void][PdfceInput.Win32]::SetCursorPos($sx, $sy)
        Start-Sleep -Milliseconds 120

        $with = if ($held.Count -gt 0) { " with $($held -join '+')" } else { "" }
        if (-not $MoveOnly) {
            [PdfceInput.Win32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 60
            [PdfceInput.Win32]::mouse_event($MOUSEEVENTF_LEFTUP,   0, 0, 0, [UIntPtr]::Zero)
            Write-Output "clicked window($x,$y) -> screen($sx,$sy)$with"
        } else {
            Write-Output "moved to window($x,$y) -> screen($sx,$sy)$with"
        }

        Start-Sleep -Milliseconds $DelayMs
    }
}
finally {
    # ALWAYS release, even on a refusal above: a modifier left latched down
    # affects every subsequent keystroke on the machine, which is a far worse
    # side effect than the failed observation that caused it.
    foreach ($m in $held) {
        [PdfceInput.Win32]::keybd_event([byte]$VK[$m], 0, $KEYEVENTF_KEYUP, [UIntPtr]::Zero)
    }
}
