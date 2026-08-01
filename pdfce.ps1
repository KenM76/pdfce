<#
.SYNOPSIS
    Launch the pdfce desktop GUI from the project root.

.DESCRIPTION
    Builds pdfce-gui in release mode (a fast freshness check when nothing
    changed, so you always launch the latest build), then starts the GUI.
    Optionally opens a PDF passed as the first argument.

.PARAMETER File
    Optional path to a .pdf to open on startup.

.EXAMPLE
    .\pdfce.ps1
    Build (if needed) and open the empty GUI.

.EXAMPLE
    .\pdfce.ps1 "C:\drawings\plan.pdf"
    ...and open that PDF on startup.

.NOTES
    Prefer this over `cargo run` when you want the console to detach.
    If PowerShell blocks the script, run it once as:
        powershell -ExecutionPolicy Bypass -File .\pdfce.ps1
#>
param(
    [Parameter(Position = 0)]
    [string]$File
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
Set-Location $root

Write-Host 'Building pdfce-gui (release)...' -ForegroundColor Cyan
& cargo build --release -p pdfce-gui
if ($LASTEXITCODE -ne 0) {
    Write-Host "`nBuild FAILED - see the errors above." -ForegroundColor Red
    exit 1
}

$exe = Join-Path $root 'target\release\pdfce-gui.exe'
if (-not (Test-Path $exe)) {
    Write-Host "Built, but $exe was not found." -ForegroundColor Red
    exit 1
}

Write-Host 'Launching pdfce...' -ForegroundColor Green
if ($File) {
    Start-Process -FilePath $exe -ArgumentList $File
} else {
    Start-Process -FilePath $exe
}
