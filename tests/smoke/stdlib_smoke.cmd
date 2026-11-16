@echo off
setlocal EnableExtensions
rem Runtime errors go to stderr (exit code 1) instead of a modal dialog.
set "QBNEX_NO_ERROR_DIALOG=1"

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%..\.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_QBNEX=%ROOT%..\fixtures\stdlib_import_success.bas"
set "SRC_QBNEX_INCLUDE=%ROOT%..\fixtures\qbnex_include_success.bas"
set "SRC_URL=%ROOT%..\fixtures\url_import_success.bas"

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
    echo [FAIL] qbnex include fixture not found at "%SRC_QBNEX_INCLUDE%"
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
set "RUN_QBNEX=%TMPDIR%\stdlib_qbnex_run.txt"
set "RUN_QBNEX_INCLUDE=%TMPDIR%\stdlib_qbnex_include_run.txt"
set "RUN_URL=%TMPDIR%\stdlib_url_run.txt"

"%QB%" "%SRC_QBNEX%" -o "%BIN_QBNEX%" > "%OUT_QBNEX%" 2>&1
set "EC_QBNEX=%ERRORLEVEL%"

"%QB%" "%SRC_QBNEX_INCLUDE%" -o "%BIN_QBNEX_INCLUDE%" > "%OUT_QBNEX_INCLUDE%" 2>&1
set "EC_QBNEX_INCLUDE=%ERRORLEVEL%"

"%QB%" "%SRC_URL%" -o "%BIN_URL%" > "%OUT_URL%" 2>&1
set "EC_URL=%ERRORLEVEL%"

set "EC_RUN_QBNEX=999"
if exist "%BIN_QBNEX%" (
    "%BIN_QBNEX%" < NUL > "%RUN_QBNEX%" 2>&1
    call set "EC_RUN_QBNEX=%%ERRORLEVEL%%"
)
set "EC_RUN_QBNEX_INCLUDE=999"
if exist "%BIN_QBNEX_INCLUDE%" (
    "%BIN_QBNEX_INCLUDE%" < NUL > "%RUN_QBNEX_INCLUDE%" 2>&1
    call set "EC_RUN_QBNEX_INCLUDE=%%ERRORLEVEL%%"
)
set "EC_RUN_URL=999"
if exist "%BIN_URL%" (
    "%BIN_URL%" < NUL > "%RUN_URL%" 2>&1
    call set "EC_RUN_URL=%%ERRORLEVEL%%"
)

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
if not "%EC_RUN_QBNEX%"=="0" (
    echo [FAIL] qbnex stdlib executable should run successfully.
    set "FAIL=1"
)
findstr /L /C:"QBNex" "%RUN_QBNEX%" >nul 2>&1 || (
    echo [FAIL] qbnex stdlib runtime output is missing JSON text.
    set "FAIL=1"
)
findstr /L /C:"a+b+c" "%RUN_QBNEX%" >nul 2>&1 || (
    echo [FAIL] qbnex stdlib runtime output is missing list text.
    set "FAIL=1"
)

if not "%EC_QBNEX_INCLUDE%"=="0" (
    echo [FAIL] direct qbnex.bas include should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_QBNEX_INCLUDE%" >nul || (
    echo [FAIL] direct qbnex.bas include compile output is missing success text.
    set "FAIL=1"
)
if not exist "%BIN_QBNEX_INCLUDE%" (
    echo [FAIL] direct qbnex.bas include should emit an executable.
    set "FAIL=1"
)
if not "%EC_RUN_QBNEX_INCLUDE%"=="0" (
    echo [FAIL] direct qbnex.bas include executable should run successfully.
    set "FAIL=1"
)
findstr /L /C:"QBNex include" "%RUN_QBNEX_INCLUDE%" >nul 2>&1 || (
    echo [FAIL] direct qbnex.bas include runtime output is missing list text.
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
if not "%EC_RUN_URL%"=="0" (
    echo [FAIL] url stdlib executable should run successfully.
    set "FAIL=1"
)
findstr /L /C:"example.com" "%RUN_URL%" >nul 2>&1 || (
    echo [FAIL] url stdlib runtime output is missing hostname.
    set "FAIL=1"
)
findstr /L /C:"/path/file.txt" "%RUN_URL%" >nul 2>&1 || (
    echo [FAIL] url stdlib runtime output is missing pathname.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo STDLIB_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo STDLIB_SMOKE_FAIL
echo Inspect outputs:
echo   QBNex compile:         "%OUT_QBNEX%"
echo   QBNex run:             "%RUN_QBNEX%"
echo   QBNex include compile: "%OUT_QBNEX_INCLUDE%"
echo   QBNex include run:     "%RUN_QBNEX_INCLUDE%"
echo   URL compile:           "%OUT_URL%"
echo   URL run:               "%RUN_URL%"
exit /b 1
