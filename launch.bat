@echo off
setlocal
cd /d "%~dp0"

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ======================================================================
    echo [KilJoy] Running in standard user mode.
    echo [KilJoy] TIP: For direct hardware fan control via HP BIOS ACPI/EC,
    echo         right-click this file or kiljoy.exe and choose:
    echo         "Run as administrator"
    echo ======================================================================
) else (
    echo [KilJoy] Running with Administrator privileges: HP EC LINK ACTIVE.
)

if exist "target\release\kiljoy.exe" (
    start "" "target\release\kiljoy.exe"
) else (
    echo [KilJoy] Compiling release binary...
    cargo build --release
    start "" "target\release\kiljoy.exe"
)
