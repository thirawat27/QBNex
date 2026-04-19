@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_OK=%ROOT%fixtures\label_recompile_success.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC_OK%" (
    echo [FAIL] Fixture source not found at "%SRC_OK%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_cli_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "OUT_HELP=%TMPDIR%\help.txt"
set "OUT_VERSION=%TMPDIR%\version.txt"
set "OUT_INVALID=%TMPDIR%\invalid.txt"
set "OUT_BADOUT=%TMPDIR%\bad_output.txt"

"%QB%" --help > "%OUT_HELP%" 2>&1
set "EC_HELP=%ERRORLEVEL%"

"%QB%" --version > "%OUT_VERSION%" 2>&1
set "EC_VERSION=%ERRORLEVEL%"

"%QB%" --definitely-invalid > "%OUT_INVALID%" 2>&1
set "EC_INVALID=%ERRORLEVEL%"

"%QB%" "%SRC_OK%" -o "%TMPDIR%\missing-dir\out.exe" > "%OUT_BADOUT%" 2>&1
set "EC_BADOUT=%ERRORLEVEL%"

set "FAIL=0"

if not "%EC_HELP%"=="0" (
    echo [FAIL] --help should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"Usage: qb <file> [switches]" "%OUT_HELP%" >nul || (
    echo [FAIL] --help output is missing usage text.
    set "FAIL=1"
)

if not "%EC_VERSION%"=="0" (
    echo [FAIL] --version should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"QBNex Compiler " "%OUT_VERSION%" >nul || (
    echo [FAIL] --version output is missing compiler version text.
    set "FAIL=1"
)

if "%EC_INVALID%"=="0" (
    echo [FAIL] Unknown switch should fail.
    set "FAIL=1"
)
findstr /L /C:"Unknown switch: --definitely-invalid" "%OUT_INVALID%" >nul || (
    echo [FAIL] Unknown switch output is missing the switch error.
    set "FAIL=1"
)
findstr /L /C:"Run 'qb --help' for usage." "%OUT_INVALID%" >nul || (
    echo [FAIL] Unknown switch output is missing usage guidance.
    set "FAIL=1"
)

if "%EC_BADOUT%"=="0" (
    echo [FAIL] Invalid output path should fail.
    set "FAIL=1"
)
findstr /L /C:"Can't create output executable - path not found:" "%OUT_BADOUT%" >nul || (
    echo [FAIL] Invalid output path output is missing the path error.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_BADOUT%" >nul && (
    echo [FAIL] Invalid output path should not report a successful build.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo CLI_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo CLI_SMOKE_FAIL
echo Inspect outputs:
echo   Help: "%OUT_HELP%"
echo   Version: "%OUT_VERSION%"
echo   Invalid switch: "%OUT_INVALID%"
echo   Bad output path: "%OUT_BADOUT%"
exit /b 1
