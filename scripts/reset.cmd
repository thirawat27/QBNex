@echo off
rem Removes what setup and the compiler produced: the compiler's build folders
rem (target\qb-temp*), its settings (target\qb-config.ini) and qb.exe.
rem Cargo's own output (target\release, target\debug) is left alone: cargo clean.

rem This script lives in scripts\; run everything from the repository root.
cd /d "%~dp0.."

echo Remove the compiler's build folders.
for /d %%D in (target\qb-temp*) do rmdir /s /q "%%D" 2>nul

echo Remove settings and version files.
if exist target\qb-config.ini del target\qb-config.ini
if exist target\qb-version.txt del target\qb-version.txt

echo Remove qb.exe
if exist qb.exe del qb.exe
if exist qb-stage0.exe del qb-stage0.exe

if not "%QBNEX_CI%"=="1" pause
