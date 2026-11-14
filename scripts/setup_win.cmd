@echo off
rem QBNex setup (QIR pipeline, D10/D15b): no MinGW, no C/C++ toolchain.
rem
rem   1. builds the QIR runner qb-exec with the Rust toolchain only
rem   2. turns the bootstrap compiler bootstrap\qbnex.qir into qb-stage0.exe
rem      (runner + QIR payload, D15b; QBNEX_QIR_PACK=native adds a native image
rem       with `qb-exec build`)
rem   3. stage0 compiles compiler\qbnex.bas into qb.exe through QIR
rem
rem Variables:
rem   QBNEX_QB_EXEC       use this prebuilt qb-exec.exe instead of building it
rem   QBNEX_RUST_TARGET   Rust target for qb-exec (default: rustc's host target)
rem   QBNEX_BOOTSTRAP     bootstrap QIR (default: bootstrap\qbnex.qir)
rem   QBNEX_QIR_PACK      ir (default) or native
rem   QBNEX_STAGE1_ARGS   extra arguments for stage0 when building qb.exe
rem   QBNEX_SKIP_SELFHOST 1 = stop after qb-stage0.exe
rem   QBNEX_KEEP_STAGE0   1 = keep qb-stage0.exe
rem   QBNEX_CI            1 = no pause at the end

setlocal enableextensions
cd /d "%~dp0.."

echo QBNex Setup ^(QIR pipeline, no C/C++ toolchain^)
echo.

rem defaults first: the QBNEX_QB_EXEC branch below skips the runner build
if not defined QBNEX_BOOTSTRAP set "QBNEX_BOOTSTRAP=bootstrap\qbnex.qir"
if not defined QBNEX_QIR_PACK set "QBNEX_QIR_PACK=ir"

if not exist crates\engine\exec\bin\windows mkdir crates\engine\exec\bin\windows
set "RUNNER=crates\engine\exec\bin\windows\qb-exec.exe"
if defined QBNEX_QB_EXEC (
    copy /y "%QBNEX_QB_EXEC%" %RUNNER% >nul
    if errorlevel 1 (
        echo Error: cannot copy "%QBNEX_QB_EXEC%".
        exit /b 1
    )
    goto haverunner
)

where cargo >nul 2>nul
if errorlevel 1 (
    echo Error: cargo was not found. Install Rust from https://rustup.rs and run this script again.
    exit /b 1
)

if not defined QBNEX_RUST_TARGET (
    for /f "tokens=2" %%t in ('rustc -vV ^| findstr /b /c:"host:"') do set "QBNEX_RUST_TARGET=%%t"
)
if not defined QBNEX_RUST_TARGET (
    echo Error: could not determine the Rust host target.
    exit /b 1
)

echo Building the QIR runner 'qb-exec' for %QBNEX_RUST_TARGET%
where rustup >nul 2>nul && rustup target add %QBNEX_RUST_TARGET% >nul 2>nul
cargo build --release --locked -p qb-exec --target %QBNEX_RUST_TARGET% --target-dir target
if errorlevel 1 (
    echo Error: failed to build qb-exec.
    exit /b 1
)
copy /y target\%QBNEX_RUST_TARGET%\release\qb-exec.exe %RUNNER% >nul
if errorlevel 1 (
    echo Error: failed to copy the runner to crates\engine\exec\bin\windows.
    exit /b 1
)
:haverunner

if not exist "%QBNEX_BOOTSTRAP%" (
    echo Error: "%QBNEX_BOOTSTRAP%" is missing.
    echo It is produced from compiler\qbnex.bas by a QIR-capable compiler ^(qb --ir^);
    echo see CONTRIBUTING.md ^(bootstrap^).
    exit /b 1
)

echo Building 'qb-stage0.exe' from %QBNEX_BOOTSTRAP% ^(%QBNEX_QIR_PACK%^)
if exist qb-stage0.exe del /q qb-stage0.exe >nul 2>nul
if /i not "%QBNEX_QIR_PACK%"=="native" (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\qir_pack.ps1 -Runner "%RUNNER%" -Qir "%QBNEX_BOOTSTRAP%" -Out qb-stage0.exe
) else (
    .\%RUNNER% build "%QBNEX_BOOTSTRAP%" -o qb-stage0.exe
)
if not exist qb-stage0.exe (
    echo Error: could not build qb-stage0.exe.
    exit /b 1
)

if "%QBNEX_SKIP_SELFHOST%"=="1" (
    echo qb-stage0.exe is ready ^(self-host skipped^).
    exit /b 0
)

echo Self-hosting 'QBNex' through QIR
if exist qb.exe del /q qb.exe >nul 2>nul
.\qb-stage0.exe compiler\qbnex.bas -o qb.exe %QBNEX_STAGE1_ARGS%
if not exist qb.exe (
    echo Final compiler build failed.
    exit /b 1
)

echo Smoke test
> "%TEMP%\qbnex_qir_smoke.bas" echo PRINT "QBNex QIR OK"
.\qb.exe "%TEMP%\qbnex_qir_smoke.bas" -o "%TEMP%\qbnex_qir_smoke.exe"
if not exist "%TEMP%\qbnex_qir_smoke.exe" (
    echo Error: qb.exe could not compile the smoke test.
    exit /b 1
)
"%TEMP%\qbnex_qir_smoke.exe" | findstr /c:"QBNex QIR OK" >nul
if errorlevel 1 (
    echo Error: the smoke test program did not print the expected text.
    exit /b 1
)
del /q "%TEMP%\qbnex_qir_smoke.bas" "%TEMP%\qbnex_qir_smoke.exe" >nul 2>nul

if not "%QBNEX_KEEP_STAGE0%"=="1" del /q qb-stage0.exe >nul 2>nul
echo.
echo QBNex CLI compiler is ready
echo   qb yourfile.bas
echo   qbnex yourfile.bas
if not "%QBNEX_CI%"=="1" pause
endlocal
