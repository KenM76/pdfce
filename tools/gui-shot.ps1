<#
.SYNOPSIS
    Launch pdfce-gui on the real screen, drive it with a diag script, and
    capture a PNG of its window.

.DESCRIPTION
    The counterpart to tools/gui-drive.ps1. That script exists for when the
    operator is at the machine and the screen must be left alone; this one is
    for when the screen IS available and the question is visual — "does the
    selection outline actually draw", "does the page jump", "what does the
    toolbar look like" — which no amount of stderr tracing can answer.

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
    # crop a screenshot — which changes the very layout being inspected, so the
    # picture answers a different question from the one asked.
    [int]$CropX = -1, [int]$CropY = -1, [int]$CropW = -1, [int]$CropH = -1
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$env:PDFCE_DIAG = "1"
$env:PDFCE_DIAG_VIEWPORT = "$X,$Y,$W,$H"
# Pad the script so the window outlives the capture no matter how fast the
# steps run — a script that ends closes the window (see diag::Script).
$env:PDFCE_DIAG_SCRIPT = $Script + (";wait" * 4000)

$proc = Start-Process -FilePath $Exe -ArgumentList $Pdf -PassThru -RedirectStandardError $Log
Start-Sleep -Seconds $CaptureAfterSeconds

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
$g.Dispose(); $bmp.Dispose()

$proc | Stop-Process -Force
Write-Host "shot=$Shot"
Get-Content $Log -ErrorAction SilentlyContinue | Select-String "vector-click|plain-click|commit-" | Select-Object -Last 5
