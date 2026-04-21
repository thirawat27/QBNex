@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_QBNEX=%ROOT%fixtures\stdlib_import_success.bas"
set "SRC_QBNEX_INCLUDE=%ROOT%fixtures\qbnex_stdlib_include_success.bas"
set "SRC_URL=%ROOT%fixtures\url_import_success.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC_QBNEX%" (
    echo [FAIL] qbnex stdlib fixture not found at "%SRC_QBNEX%"
    exit /b 2
)

if not exist "%SRC_QBNEX_INCLUDE%" (
    echo [FAIL] qbnex stdlib include fixture not found at "%SRC_QBNEX_INCLUDE%"
    exit /b 2
)

if not exist "%SRC_URL%" (
    echo [FAIL] url stdlib fixture not found at "%SRC_URL%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_stdlib_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "BIN_QBNEX=%TMPDIR%\stdlib_qbnex.exe"
set "BIN_QBNEX_INCLUDE=%TMPDIR%\stdlib_qbnex_include.exe"
set "BIN_URL=%TMPDIR%\stdlib_url.exe"
set "OUT_QBNEX=%TMPDIR%\stdlib_qbnex.txt"
set "OUT_QBNEX_INCLUDE=%TMPDIR%\stdlib_qbnex_include.txt"
set "OUT_URL=%TMPDIR%\stdlib_url.txt"

"%QB%" "%SRC_QBNEX%" -o "%BIN_QBNEX%" > "%OUT_QBNEX%" 2>&1
set "EC_QBNEX=%ERRORLEVEL%"

"%QB%" "%SRC_QBNEX_INCLUDE%" -o "%BIN_QBNEX_INCLUDE%" > "%OUT_QBNEX_INCLUDE%" 2>&1
set "EC_QBNEX_INCLUDE=%ERRORLEVEL%"

"%QB%" "%SRC_URL%" -o "%BIN_URL%" > "%OUT_URL%" 2>&1
set "EC_URL=%ERRORLEVEL%"

set "FAIL=0"

if not "%EC_QBNEX%"=="0" (
    echo [FAIL] qbnex stdlib import should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_QBNEX%" >nul || (
    echo [FAIL] qbnex stdlib compile output is missing success text.
    set "FAIL=1"
)
if not exist "%BIN_QBNEX%" (
    echo [FAIL] qbnex stdlib import should emit an executable.
    set "FAIL=1"
)

if not "%EC_QBNEX_INCLUDE%"=="0" (
    echo [FAIL] direct qbnex_stdlib include should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_QBNEX_INCLUDE%" >nul || (
    echo [FAIL] direct qbnex_stdlib include compile output is missing success text.
    set "FAIL=1"
)
if not exist "%BIN_QBNEX_INCLUDE%" (
    echo [FAIL] direct qbnex_stdlib include should emit an executable.
    set "FAIL=1"
)

if not "%EC_URL%"=="0" (
    echo [FAIL] url stdlib import should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_URL%" >nul || (
    echo [FAIL] url stdlib compile output is missing success text.
    set "FAIL=1"
)
if not exist "%BIN_URL%" (
    echo [FAIL] url stdlib import should emit an executable.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo STDLIB_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo STDLIB_SMOKE_FAIL
echo Inspect outputs:
echo   QBNex compile: "%OUT_QBNEX%"
echo   QBNex include compile: "%OUT_QBNEX_INCLUDE%"
echo   URL compile:   "%OUT_URL%"
exit /b 1
