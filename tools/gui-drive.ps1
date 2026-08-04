<#
.SYNOPSIS
    Drive a pdfce-gui window with synthesized window messages, without ever
    touching the operator's screen, mouse, keyboard or focus.

.DESCRIPTION
    The only honest oracle for a GUI defect is the running application
    (standing rule R86) — but the operator's desktop is not available for that
    whenever they are using the machine, which is most of the time. This script
    is the way to have both.

    It launches pdfce-gui with PDFCE_DIAG_VIEWPORT set, which places the window
    far outside the visible desktop and marks it inactive, then posts
    WM_MOUSEMOVE / WM_LBUTTONDOWN / WM_LBUTTONUP directly to that window's
    HWND. PostMessage delivers to one window's queue only: the physical cursor
    never moves, no foreground window changes, and nothing the operator is
    doing is disturbed.

    The application's own PDFCE_DIAG trace goes to stderr, redirected to a
    file, which is the actual result of the run.

.PARAMETER Pdf
    Document to open. Anything under D:\Dev\temp\pdfce is the operator's
    scratch area and is never committed (docs/LEGAL.md §5).

.PARAMETER Clicks
    One or more "x,y" CLIENT-space points to left-click, in order.

.PARAMETER Exe
    The binary to drive. Defaults to the release build.

.PARAMETER Log
    Where the trace is written.

.EXAMPLE
    pwsh tools/gui-drive.ps1 -Pdf D:\Dev\temp\pdfce\SW41177.pdf -Clicks "800,550","400,300"

.NOTES
    Why not SendInput / keybd_event (as tools/gui-click.ps1 uses)? Those inject
    at the system level: they move the real cursor and act on whatever window
    is in front. That is correct for driving a window the operator can see, and
    completely wrong here.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Pdf,
    [string[]]$Clicks = @("800,550"),
    [string]$Exe = "$PSScriptRoot\..\target\release\pdfce-gui.exe",
    [string]$Log = "$env:TEMP\pdfce-diag.txt",
    # Where to park the window. Far enough negative to be off every plausible
    # multi-monitor arrangement, while still a real on-screen-coordinate window
    # that lays out and interacts normally.
    [int]$OffscreenX = -4000,
    [int]$OffscreenY = -4000,
    [int]$Width = 1600,
    [int]$Height = 1000
)

$ErrorActionPreference = 'Stop'

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Win32 {
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder s, int max);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    public static IntPtr Found = IntPtr.Zero;
    public static uint Want = 0;
    public static bool Cb(IntPtr h, IntPtr l) {
        uint pid; GetWindowThreadProcessId(h, out pid);
        if (pid == Want && IsWindowVisible(h)) {
            StringBuilder sb = new StringBuilder(256);
            GetClassName(h, sb, 256);
            // winit's top-level window class; skip the message-only and
            // tooltip helpers a windowing stack also creates for the process.
            if (sb.ToString().Contains("Window")) { Found = h; return false; }
        }
        return true;
    }
}
"@

if (-not (Test-Path $Exe)) { throw "no binary at $Exe" }
if (-not (Test-Path $Pdf)) { throw "no document at $Pdf" }
Remove-Item $Log -ErrorAction SilentlyContinue

$env:PDFCE_DIAG = "1"
$env:PDFCE_DIAG_VIEWPORT = "$OffscreenX,$OffscreenY,$Width,$Height"
$proc = Start-Process -FilePath $Exe -ArgumentList $Pdf -PassThru `
    -RedirectStandardError $Log -RedirectStandardOutput "$Log.out" -WindowStyle Hidden

# Wait for the window to exist. It has to be found by PID rather than by title
# because the title is operator-facing copy that may change.
[Win32]::Want = $proc.Id
$deadline = (Get-Date).AddSeconds(30)
$cb = [Win32+EnumWindowsProc]{ param($h, $l) [Win32]::Cb($h, $l) }
while ((Get-Date) -lt $deadline -and [Win32]::Found -eq [IntPtr]::Zero) {
    Start-Sleep -Milliseconds 250
    [void][Win32]::EnumWindows($cb, [IntPtr]::Zero)
}
$hwnd = [Win32]::Found
if ($hwnd -eq [IntPtr]::Zero) { $proc | Stop-Process -Force; throw "window never appeared" }
Write-Host "hwnd=$hwnd pid=$($proc.Id)"

# Give the document time to parse and the first frame to lay out; clicking
# before the page exists would prove nothing.
Start-Sleep -Seconds 3

$WM_MOUSEMOVE = 0x0200; $WM_LBUTTONDOWN = 0x0201; $WM_LBUTTONUP = 0x0202; $WM_CLOSE = 0x0010
$MK_LBUTTON = 1

foreach ($c in $Clicks) {
    $p = $c.Split(','); $x = [int]$p[0]; $y = [int]$p[1]
    $lp = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
    Write-Host "click $x,$y"
    # Each button message is preceded IMMEDIATELY by a move, with no sleep
    # between them.
    #
    # The reason is specific and cost a run to find: winit calls
    # TrackMouseEvent on WM_MOUSEMOVE, and because the physical cursor is not
    # over this window Windows answers with WM_MOUSELEAVE almost at once. That
    # becomes egui's `PointerGone`, which clears the known pointer position —
    # and egui-winit only emits a `PointerButton` event when it HAS a position.
    # A move, a pause, then a press therefore delivers the press into a state
    # where egui has already forgotten where the pointer is, and the press is
    # silently dropped. Observed exactly that: an event list of
    # `[PointerMoved, PointerGone]` and no button event at all.
    #
    # Posting move+button back to back puts both in the queue ahead of the
    # system's leave notification, so the position is live when the button
    # arrives. The sleeps go BETWEEN the press pair and the release pair, where
    # a re-established position makes them harmless.
    [void][Win32]::PostMessage($hwnd, $WM_MOUSEMOVE, [IntPtr]::Zero, $lp)
    [void][Win32]::PostMessage($hwnd, $WM_LBUTTONDOWN, [IntPtr]$MK_LBUTTON, $lp)
    Start-Sleep -Milliseconds 150
    [void][Win32]::PostMessage($hwnd, $WM_MOUSEMOVE, [IntPtr]$MK_LBUTTON, $lp)
    [void][Win32]::PostMessage($hwnd, $WM_LBUTTONUP, [IntPtr]::Zero, $lp)
    Start-Sleep -Milliseconds 600
}

[void][Win32]::PostMessage($hwnd, $WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
if (-not $proc.WaitForExit(8000)) { $proc | Stop-Process -Force }
Write-Host "--- trace ($Log) ---"
Get-Content $Log -ErrorAction SilentlyContinue
