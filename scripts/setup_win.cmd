@echo off
rem QBNex setup (QIR pipeline).
rem
rem   1. builds the QIR runner qb-exec with the Rust toolchain only, and packs it with
rem      brotli behind the launcher qb-stub (qb-exec.pack, and qb-exec-lite.pack: the
rem      runner without the GPU feature, for console programs without 3D); qb puts the
rem      packed runner in front of every program it builds
rem   2. turns the bootstrap compiler bootstrap\qbnex.qir into qb-stage0.exe
rem      (runner + QIR payload; QBNEX_QIR_PACK=native adds a native image
rem       with `qb-exec build`)
rem   3. stage0 compiles compiler\qbnex.bas into qb.exe through QIR
rem
rem Variables:
rem   QBNEX_QB_EXEC       use this prebuilt qb-exec.exe instead of building it
rem   QBNEX_RUST_TARGET   Rust target for qb-exec (default: rustc's host target)
rem   QBNEX_GPU           0 = build qb-exec without the GPU feature (no wgpu, no GPU 3D)
rem   QBNEX_PACK          0 = no packed runners: every program carries a copy of qb-exec
rem   QBNEX_BOOTSTRAP     bootstrap QIR (default: bootstrap\qbnex.qir)
rem   QBNEX_QIR_PACK      ir (default) or native
rem   QBNEX_STAGE1_ARGS   extra arguments for stage0 when building qb.exe
rem   QBNEX_SKIP_SELFHOST 1 = stop after qb-stage0.exe
rem   QBNEX_KEEP_STAGE0   1 = keep qb-stage0.exe
rem   QBNEX_CI            1 = no pause at the end or after an error

setlocal enableextensions
cd /d "%~dp0.."

echo QBNex Setup ^(QIR pipeline^)
echo.

rem defaults first: the QBNEX_QB_EXEC branch below skips the runner build
if not defined QBNEX_BOOTSTRAP set "QBNEX_BOOTSTRAP=bootstrap\qbnex.qir"
if not defined QBNEX_QIR_PACK set "QBNEX_QIR_PACK=ir"

if not exist crates\engine\exec\bin\windows mkdir crates\engine\exec\bin\windows
set "RUNNER=crates\engine\exec\bin\windows\qb-exec.exe"
rem packed runners of an earlier build would not match the runner built now
del /f /q crates\engine\exec\bin\windows\*.pack >nul 2>nul
if exist crates\engine\exec\bin\windows\*.pack (
    echo Error: an old packed runner in crates\engine\exec\bin\windows cannot be deleted.
    goto fail
)
if defined QBNEX_QB_EXEC (
    copy /y "%QBNEX_QB_EXEC%" %RUNNER% >nul
    if errorlevel 1 (
        echo Error: cannot copy "%QBNEX_QB_EXEC%".
        goto fail
    )
    goto haverunner
)

where cargo >nul 2>nul
if errorlevel 1 (
    echo Error: cargo was not found. Install Rust from https://rustup.rs and run this script again.
    goto fail
)

if not defined QBNEX_RUST_TARGET (
    for /f "tokens=2" %%t in ('rustc -vV ^| findstr /b /c:"host:"') do set "QBNEX_RUST_TARGET=%%t"
)
if not defined QBNEX_RUST_TARGET (
    echo Error: could not determine the Rust host target.
    goto fail
)

rem The runner draws 3D on the GPU (wgpu); without a usable adapter it falls back to software.
set "RUNNER_FEATURES=--features gpu"
if "%QBNEX_GPU%"=="0" set "RUNNER_FEATURES="

echo Building the QIR runner 'qb-exec' for %QBNEX_RUST_TARGET%
where rustup >nul 2>nul && rustup target add %QBNEX_RUST_TARGET% >nul 2>nul
cargo build --release --locked -p qb-exec %RUNNER_FEATURES% --target %QBNEX_RUST_TARGET% --target-dir target
if errorlevel 1 (
    echo Error: failed to build qb-exec.
    goto fail
)
copy /y target\%QBNEX_RUST_TARGET%\release\qb-exec.exe %RUNNER% >nul
if errorlevel 1 (
    echo Error: failed to copy the runner to crates\engine\exec\bin\windows.
    goto fail
)
if "%QBNEX_PACK%"=="0" goto haverunner

echo Packing the runner behind the launcher 'qb-stub'
cargo build --release --locked -p qb-stub --target %QBNEX_RUST_TARGET% --target-dir target
if errorlevel 1 (
    echo Error: failed to build qb-stub.
    goto fail
)
set "PACKER=target\%QBNEX_RUST_TARGET%\release\qb-pack.exe"
set "STUB=target\%QBNEX_RUST_TARGET%\release\qb-stub.exe"
%PACKER% %RUNNER% --stub %STUB% -o crates\engine\exec\bin\windows\qb-exec.pack
if errorlevel 1 (
    echo Error: failed to pack the runner.
    goto fail
)
rem already the runner without the GPU feature
if "%QBNEX_GPU%"=="0" goto haverunner
echo Building the runner without the GPU feature for console programs
cargo build --release --locked -p qb-exec --target %QBNEX_RUST_TARGET% --target-dir target
if errorlevel 1 (
    echo Error: failed to build the runner without the GPU feature.
    goto fail
)
%PACKER% target\%QBNEX_RUST_TARGET%\release\qb-exec.exe --stub %STUB% -o crates\engine\exec\bin\windows\qb-exec-lite.pack
if errorlevel 1 (
    echo Error: failed to pack the runner without the GPU feature.
    goto fail
)
:haverunner

if not exist "%QBNEX_BOOTSTRAP%" (
    echo Error: "%QBNEX_BOOTSTRAP%" is missing.
    echo It is produced from compiler\qbnex.bas by a QIR-capable compiler ^(qb --ir^);
    echo see CONTRIBUTING.md ^(bootstrap^).
    goto fail
)

echo Building 'qb-stage0.exe' from %QBNEX_BOOTSTRAP% ^(%QBNEX_QIR_PACK%^)
if exist qb-stage0.exe del /f /q qb-stage0.exe >nul 2>nul
rem the check below takes an existing file for a new build: an old one must be gone
if exist qb-stage0.exe (
    echo Error: the old qb-stage0.exe cannot be deleted. Close the program that uses it and run this script again.
    goto fail
)
if /i not "%QBNEX_QIR_PACK%"=="native" (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\qir_pack.ps1 -Runner "%RUNNER%" -Qir "%QBNEX_BOOTSTRAP%" -Out qb-stage0.exe
) else (
    .\%RUNNER% build "%QBNEX_BOOTSTRAP%" -o qb-stage0.exe
)
if not exist qb-stage0.exe (
    echo Error: could not build qb-stage0.exe.
    goto fail
)

if "%QBNEX_SKIP_SELFHOST%"=="1" (
    echo qb-stage0.exe is ready ^(self-host skipped^).
    exit /b 0
)

echo Self-hosting 'QBNex' through QIR
if exist qb.exe del /f /q qb.exe >nul 2>nul
rem An old qb.exe that is still running (a terminal, the editor's language server)
rem cannot be deleted. Building on would leave it in place and report it as new.
if exist qb.exe (
    echo Error: the old qb.exe cannot be deleted; it is probably still running.
    echo Close the programs that use it and run this script again.
    goto fail
)
.\qb-stage0.exe compiler\qbnex.bas -o qb.exe %QBNEX_STAGE1_ARGS%
if errorlevel 1 (
    echo Error: the final compiler build failed.
    goto fail
)
if not exist qb.exe (
    echo Error: the final compiler build failed ^(no qb.exe was written^).
    goto fail
)

echo Smoke test
del /q "%TEMP%\qbnex_qir_smoke.exe" >nul 2>nul
> "%TEMP%\qbnex_qir_smoke.bas" echo PRINT "QBNex QIR OK"
.\qb.exe "%TEMP%\qbnex_qir_smoke.bas" -o "%TEMP%\qbnex_qir_smoke.exe"
if not exist "%TEMP%\qbnex_qir_smoke.exe" (
    echo Error: qb.exe could not compile the smoke test.
    goto fail
)
"%TEMP%\qbnex_qir_smoke.exe" | findstr /c:"QBNex QIR OK" >nul
if errorlevel 1 (
    echo Error: the smoke test program did not print the expected text.
    goto fail
)
del /q "%TEMP%\qbnex_qir_smoke.bas" "%TEMP%\qbnex_qir_smoke.exe" >nul 2>nul

if not "%QBNEX_KEEP_STAGE0%"=="1" del /q qb-stage0.exe >nul 2>nul
echo.
echo QBNex CLI compiler is ready
echo   qb yourfile.bas
echo   qbnex yourfile.bas
if not "%QBNEX_CI%"=="1" pause
endlocal
exit /b 0

rem Started by a double click, the window closes with the script: the error stays
rem on screen until a key is pressed (not under CI, where nobody presses one).
:fail
if not "%QBNEX_CI%"=="1" pause
endlocal
exit /b 1
