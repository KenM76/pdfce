<#
.SYNOPSIS
    Send a real mouse click (or a click sequence) to the running pdfce-gui
    window, in WINDOW-RELATIVE coordinates.

.DESCRIPTION
    The other half of the observation harness that `tools/observe-gui.ps1`
    starts. Observation alone can only confirm static state; most of pdfce's
    operator-facing behavior is a GESTURE -- pick two points, then Accept -- and
    cannot be reached without driving the pointer.

    This exists because of standing rule R86 (decision 018 sec. 11.2): a Pass that
    adds or changes operator-facing behavior does not ship until that behavior
    has been observed working in the running application. Ken accepted that rule
    on 2026-08-02 conditioned on the ENGINEER doing the observing -- "I don't want
    to be a beta tester." Verifying a two-click authoring gesture therefore
    requires synthesizing the two clicks.

    WHY WINDOW-RELATIVE COORDINATES
    -------------------------------
    Callers reason about positions by looking at a screenshot from
    `observe-gui.ps1`, whose origin is the window's top-left. Making this script
    take the same coordinate space means a pixel measured in the screenshot can
    be clicked directly, with no mental offset arithmetic -- the single most
    likely source of a confidently-wrong automated click. The script resolves
    the window rect itself and adds the offset.

    WHY SendInput RATHER THAN PostMessage
    -------------------------------------
    egui/winit consume real input events from the OS queue. Posting synthetic
    WM_LBUTTONDOWN to the window handle does not reliably drive winit's event
    loop and, worse, can appear to work while bypassing the hit-testing path the
    operator actually exercises -- an observation that proves nothing. SendInput
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
    Move the cursor to the single given point without clicking -- used to park
    the pointer somewhere harmless before a screenshot, so a hover highlight or
    tooltip does not contaminate the observation.

.PARAMETER Modifiers
    Modifier keys to HOLD DOWN for the whole click sequence: any of Shift,
    Ctrl, Alt. Added because several canvas gestures are only reachable with a
    modifier -- Shift+click toggles selection membership, and Alt+click steps
    click-through cycling to the next object under the pointer -- and R86 says
    those behaviors do not ship until they have been observed in the running
    app. Without this the modifier paths could only ever be tested headlessly,
    which is exactly the gap the harness exists to close.

    The keys are pressed before the first point and released in a `finally`, so
    an exception mid-sequence (a refused out-of-window point, a foreground
    steal) cannot leave Alt latched down across the operator's desktop -- a
    stuck modifier is a far nastier side effect than a failed observation.

    Uses `keybd_event` for the same reason the clicks use `mouse_event`: it
    injects at the OS layer winit reads, so `egui::Modifiers` sees the real
    state rather than a synthesized message the event loop may ignore.

.PARAMETER Type
    Text to type after the click sequence completes. Added 2026-08-03 because
    R86 kept being unsatisfiable for the whole in-place-text-editing family:
    every refusal in that family (R-INV-1's embedded-subset floor,
    `FormatError::CoverageFailure`, the encoding triggers) is reached only by
    SELECTING a run and then TYPING a replacement. Clicks alone get you to the
    property bar and no further, so those messages could never be read on
    screen -- which is exactly how two of them went several releases telling
    the operator to do something that could not work.

    WHY KEYEVENTF_UNICODE AND NOT VIRTUAL KEY CODES
    -----------------------------------------------
    Virtual-key codes are keyboard-LAYOUT dependent: sending VK_Z produces
    'z' on a US layout and 'w' on AZERTY. A harness that types different
    characters on different machines is worse than no harness, because it
    fails in a way that looks like an application bug.

    KEYEVENTF_UNICODE bypasses the layout entirely -- `wScan` carries the UTF-16
    code unit and Windows delivers exactly that character. It is also the only
    practical way to type the non-Latin text FF-C exists to support; a Greek or
    Cyrillic probe has no virtual-key code on a Latin keyboard at all.

    LIMIT, STATED: this sends one WM_CHAR-equivalent per UTF-16 code unit, so
    characters outside the BMP arrive as two surrogate halves. egui reassembles
    them correctly, but a test asserting "one keystroke" would be wrong.

.EXAMPLE
    pwsh -File tools/gui-click.ps1 -Clicks "1235,45"
    # click the Measure menu

.EXAMPLE
    pwsh -File tools/gui-click.ps1 -Clicks "487,277" -Type "ABZ"
    # select a text run, then type a replacement containing a character the
    # page's embedded subset font does not carry -- the R-INV-1 probe

.EXAMPLE
    pwsh -File tools/gui-click.ps1 -Clicks "700,300","900,300" -DelayMs 500
    # pick two canvas points for a linear dimension
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]] $Clicks,
    [string]   $ProcessName = 'pdfce-gui',
    # NOTE: deliberately NOT named $Pid -- that is a PowerShell AUTOMATIC
    # variable holding the CURRENT process id, and shadowing it in a param
    # block is both an error and a genuinely confusing one to diagnose.
    [int]      $ProcessId   = 0,
    [int]      $DelayMs     = 400,
    [switch]   $MoveOnly,
    [ValidateSet('Shift', 'Ctrl', 'Alt')]
    [string[]] $Modifiers   = @(),
    [string]   $Type        = '',
    [ValidateSet('Backspace', 'Delete', 'Enter', 'Tab', 'Escape', 'Home', 'End')]
    [string[]] $PressKeys   = @()
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

[StructLayout(LayoutKind.Sequential)]
public struct KEYBDINPUT {
    public ushort wVk; public ushort wScan; public uint dwFlags;
    public uint time; public IntPtr dwExtraInfo;
}

// INPUT is a tagged union. Only the keyboard arm is used here, but the
// struct must still be the full size the OS expects, hence the explicit
// layout with the 8-byte type tag followed by the union payload.
[StructLayout(LayoutKind.Explicit)]
public struct INPUT {
    [FieldOffset(0)]  public uint type;
    [FieldOffset(8)]  public KEYBDINPUT ki;
}

[DllImport("user32.dll", SetLastError = true)]
public static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
'@
}

$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP   = 0x0004

$candidates = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 })

# Disambiguate BEFORE acting. Taking the first match by name was a real hazard:
# a build agent verifying its own binary found a second instance running from
# the shared checkout, and had to kill it -- because a click aimed at "the
# pdfce-gui window" could equally have hit the other one. Pass -ProcessId to
# name exactly which, or close the extras.
if ($ProcessId -gt 0) {
    $proc = $candidates | Where-Object { $_.Id -eq $ProcessId } | Select-Object -First 1
    if (-not $proc) {
        throw "No '$ProcessName' process with id $ProcessId and a visible main window. Running ids: $(($candidates | ForEach-Object { $_.Id }) -join ', ')"
    }
} elseif ($candidates.Count -gt 1) {
    throw "AMBIGUOUS TARGET: $($candidates.Count) '$ProcessName' processes have a visible window (ids: $(($candidates | ForEach-Object { $_.Id }) -join ', ')). Refusing to guess which one to click -- pass -ProcessId <id>, or close the others."
} else {
    $proc = $candidates | Select-Object -First 1
}
if (-not $proc) { throw "No running '$ProcessName' window to click." }

[void][PdfceInput.Win32]::SetForegroundWindow($proc.MainWindowHandle)
Start-Sleep -Milliseconds 400

# VERIFY the raise took before injecting any click. This is more serious here
# than in observe-gui.ps1: a screenshot of the wrong window is merely useless,
# but a CLICK delivered to the wrong window is an unintended action on someone
# else's application. SendInput goes to whatever is under the cursor, and the
# per-point rect check below only proves the coordinates fall inside pdfce's
# rectangle -- not that pdfce is the window actually on top of that rectangle.
#
# This bit on 2026-08-02: an unrelated terminal window occupying the same screen
# region came to the foreground mid-sequence, so synthesized canvas picks landed
# in it. Refuse instead.
$fg = [PdfceInput.Win32]::GetForegroundWindow()
if ($fg -ne $proc.MainWindowHandle) {
    throw "REFUSING TO CLICK: '$ProcessName' (handle $($proc.MainWindowHandle)) is not the foreground window (foreground is $fg). Clicks would be delivered to whatever window is on top -- possibly another application. Raise '$ProcessName' and retry."
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
        if ($parts.Count -ne 2) { throw "Bad point '$c' -- expected 'X,Y'." }
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

# ---------------------------------------------------------------------------
# Typing stage. Runs AFTER the clicks and AFTER the modifiers are released,
# because every use so far is "click to focus something, then type into it" --
# and a modifier still held would turn the text into a shortcut sequence.
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# Control keys, pressed BEFORE any typing.
#
# WHY THESE ARE VIRTUAL-KEY CODES WHEN -Type IS DELIBERATELY NOT
# ---------------------------------------------------------------------------
# `-Type` uses KEYEVENTF_UNICODE precisely to AVOID virtual keys, because a
# VK is layout-dependent and would type a different character on AZERTY.
# Control keys are the exact inverse: Backspace, Enter, Tab and friends have
# fixed virtual-key codes on every layout and no character to deliver at all.
# Sending them as Unicode would either do nothing or insert a control
# character into the text.
#
# Added 2026-08-04 after failing twice to clear an egui text field: Ctrl+A
# typed a literal "a" (the modifier is held across a -Type sequence, so the
# character path wins), and three separate clicks are too far apart in time
# for Windows to register a triple-click. Neither was a product bug; both
# were the harness lacking a way to say "backspace".
if ($PressKeys.Count -gt 0) {
    $fgKeys = [PdfceInput.Win32]::GetForegroundWindow()
    if ($fgKeys -ne $proc.MainWindowHandle) {
        throw "REFUSING TO SEND KEYS: '$ProcessName' is not the foreground window (foreground is $fgKeys)."
    }
    $VKEY = @{
        Backspace = 0x08; Tab = 0x09; Enter = 0x0D; Escape = 0x1B
        End       = 0x23; Home = 0x24; Delete = 0x2E
    }
    foreach ($k in $PressKeys) {
        $code = [byte]$VKEY[$k]
        [PdfceInput.Win32]::keybd_event($code, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 30
        [PdfceInput.Win32]::keybd_event($code, 0, $KEYEVENTF_KEYUP, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 40
    }
    Write-Output "pressed $($PressKeys -join ', ')"
}

if ($Type) {
    # Re-verify the foreground window. The click loop may have taken a while,
    # and typing into the wrong window is worse than clicking into it: a click
    # is one stray event, a string is a stream of them, and some of those
    # characters are destructive in an editor.
    $fgNow = [PdfceInput.Win32]::GetForegroundWindow()
    if ($fgNow -ne $proc.MainWindowHandle) {
        throw "REFUSING TO TYPE: '$ProcessName' is no longer the foreground window (foreground is $fgNow). The text would be delivered to whatever now has focus."
    }

    $INPUT_KEYBOARD    = 1
    $KEYEVENTF_UNICODE = 0x0004
    $size = [System.Runtime.InteropServices.Marshal]::SizeOf([type]'PdfceInput.Win32+INPUT')

    # One UTF-16 code unit per event pair. `wVk` MUST be 0 when
    # KEYEVENTF_UNICODE is set -- a non-zero virtual key silently wins and the
    # layout-independence this exists for is lost.
    foreach ($unit in [System.Text.Encoding]::Unicode.GetBytes($Type) | ForEach-Object -Begin { $i = 0; $buf = @() } -Process { $buf += $_ } -End { for ($j = 0; $j -lt $buf.Count; $j += 2) { [uint16]($buf[$j] -bor ($buf[$j+1] -shl 8)) } }) {
        # keybd_event, NOT SendInput.
        #
        # SendInput is the modern API and the first implementation used it.
        # It returned 0 with ERROR_INVALID_PARAMETER (87) on every call,
        # despite `Marshal.SizeOf` confirming the expected x64 layout --
        # INPUT 32 bytes, KEYBDINPUT 24 -- so the declaration was right and
        # PowerShell's marshalling of the `INPUT[]` union array was not.
        # Chasing that is a PowerShell interop problem, not a pdfce problem.
        #
        # `keybd_event` is the legacy entry point that Windows implements in
        # terms of SendInput internally, it takes no union and no array, and
        # it is ALREADY PROVEN IN THIS SCRIPT -- the modifier press/release
        # above uses it and works. With `bVk = 0` and KEYEVENTF_UNICODE it is
        # layout-independent in exactly the same way, which was the whole
        # reason for preferring SendInput.
        #
        # Deliberately choosing the older API that demonstrably works here
        # over the newer one that does not.
        [PdfceInput.Win32]::keybd_event(0, [byte]($unit -band 0xFF), $KEYEVENTF_UNICODE, [UIntPtr]::Zero)
        [PdfceInput.Win32]::keybd_event(0, [byte]($unit -band 0xFF), $KEYEVENTF_UNICODE -bor $KEYEVENTF_KEYUP, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 40
    }
    Write-Output "typed $($Type.Length) character(s)"
    Start-Sleep -Milliseconds $DelayMs
}
