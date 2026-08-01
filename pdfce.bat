@echo off
rem ===================================================================
rem  pdfce.bat - launch the pdfce desktop GUI from the project root.
rem
rem  Usage:
rem    pdfce.bat                 build (if needed) and open the empty GUI
rem    pdfce.bat "C:\some.pdf"   ...and open that PDF on startup
rem    (or drag a .pdf file onto this .bat in Explorer)
rem
rem  It always runs `cargo build --release -p pdfce-gui` first: that is a
rem  fast freshness check when nothing changed, and guarantees you launch
rem  the latest build. The GUI is then started detached so this console
rem  window closes immediately.
rem ===================================================================
setlocal
cd /d "%~dp0"

echo Building pdfce-gui (release)...
cargo build --release -p pdfce-gui
if errorlevel 1 (
    echo.
    echo Build FAILED - see the errors above.
    pause
    exit /b 1
)

echo Launching pdfce...
start "pdfce" "%~dp0target\release\pdfce-gui.exe" %*
endlocal
