@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_OK=%ROOT%fixtures\label_recompile_success.bas"
set "SRC_FAIL=%ROOT%fixtures\label_missing_failure.bas"
set "SRC_SCOPE=%ROOT%fixtures\label_scope_conflict.bas"
set "SRC_DATA=%ROOT%fixtures\label_ambiguous_data.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC_OK%" (
    echo [FAIL] Success fixture source not found at "%SRC_OK%"
    exit /b 2
)

if not exist "%SRC_FAIL%" (
    echo [FAIL] Failure fixture source not found at "%SRC_FAIL%"
    exit /b 2
)

if not exist "%SRC_SCOPE%" (
    echo [FAIL] Scope-conflict fixture source not found at "%SRC_SCOPE%"
    exit /b 2
)

if not exist "%SRC_DATA%" (
    echo [FAIL] Ambiguous-data fixture source not found at "%SRC_DATA%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_labels_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "BIN_OK=%TMPDIR%\label_recompile_success.exe"
set "BIN_FAIL=%TMPDIR%\label_missing_failure.exe"
set "BIN_SCOPE=%TMPDIR%\label_scope_conflict.exe"
set "BIN_DATA=%TMPDIR%\label_ambiguous_data.exe"
set "BIN_STALE=%TMPDIR%\label_stale_failure.exe"
set "OUT_OK=%TMPDIR%\label_recompile_success.txt"
set "OUT_FAIL=%TMPDIR%\label_missing_failure.txt"
set "OUT_SCOPE=%TMPDIR%\label_scope_conflict.txt"
set "OUT_DATA=%TMPDIR%\label_ambiguous_data.txt"
set "OUT_STALE=%TMPDIR%\label_stale_failure.txt"

"%QB%" "%SRC_OK%" -o "%BIN_OK%" > "%OUT_OK%" 2>&1
set "EC_OK=%ERRORLEVEL%"

"%QB%" "%SRC_FAIL%" -o "%BIN_FAIL%" > "%OUT_FAIL%" 2>&1
set "EC_FAIL=%ERRORLEVEL%"

"%QB%" "%SRC_SCOPE%" -o "%BIN_SCOPE%" > "%OUT_SCOPE%" 2>&1
set "EC_SCOPE=%ERRORLEVEL%"

"%QB%" "%SRC_DATA%" -o "%BIN_DATA%" > "%OUT_DATA%" 2>&1
set "EC_DATA=%ERRORLEVEL%"

"%QB%" "%SRC_OK%" -o "%BIN_STALE%" >nul 2>&1
"%QB%" "%SRC_FAIL%" -o "%BIN_STALE%" > "%OUT_STALE%" 2>&1
set "EC_STALE=%ERRORLEVEL%"

set "FAIL=0"

if not "%EC_OK%"=="0" (
    echo [FAIL] Label recompile fixture should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_OK%" >nul || (
    echo [FAIL] Label recompile fixture is missing successful build output.
    set "FAIL=1"
)
findstr /L /C:"not defined" "%OUT_OK%" >nul && (
    echo [FAIL] Label recompile fixture should not report undefined labels.
    set "FAIL=1"
)
if not exist "%BIN_OK%" (
    echo [FAIL] Label recompile fixture should emit an executable.
    set "FAIL=1"
)

if "%EC_FAIL%"=="0" (
    echo [FAIL] Missing-label fixture should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Build Halted" "%OUT_FAIL%" >nul || (
    echo [FAIL] Missing-label fixture is missing blocking diagnostic output.
    set "FAIL=1"
)
findstr /L /C:"Label 'MissingLabel' not defined" "%OUT_FAIL%" >nul || findstr /L /C:"Unknown statement" "%OUT_FAIL%" >nul || (
    echo [FAIL] Missing-label fixture should report a blocking label-path failure.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_FAIL%" >nul && (
    echo [FAIL] Missing-label fixture should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_FAIL%" (
    echo [FAIL] Missing-label fixture should not emit an executable.
    set "FAIL=1"
)

if "%EC_SCOPE%"=="0" (
    echo [FAIL] Scope-conflict fixture should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Build Halted" "%OUT_SCOPE%" >nul || (
    echo [FAIL] Scope-conflict fixture is missing blocking diagnostic output.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_SCOPE%" >nul && (
    echo [FAIL] Scope-conflict fixture should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_SCOPE%" (
    echo [FAIL] Scope-conflict fixture should not emit an executable.
    set "FAIL=1"
)

if "%EC_DATA%"=="0" (
    echo [FAIL] Ambiguous-data fixture should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Build Halted" "%OUT_DATA%" >nul || (
    echo [FAIL] Ambiguous-data fixture is missing blocking diagnostic output.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_DATA%" >nul && (
    echo [FAIL] Ambiguous-data fixture should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_DATA%" (
    echo [FAIL] Ambiguous-data fixture should not emit an executable.
    set "FAIL=1"
)

if "%EC_STALE%"=="0" (
    echo [FAIL] Missing-label stale-output fixture should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Warning: Existing output was not updated because compilation failed." "%OUT_STALE%" >nul || (
    echo [FAIL] Missing-label stale-output fixture is missing the stale executable warning.
    set "FAIL=1"
)
if not exist "%BIN_STALE%" (
    echo [FAIL] Missing-label stale-output fixture should keep the previous executable.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo LABELS_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo LABELS_SMOKE_FAIL
echo Inspect outputs:
echo   Success: "%OUT_OK%"
echo   Failure: "%OUT_FAIL%"
echo   Scope:   "%OUT_SCOPE%"
echo   Data:    "%OUT_DATA%"
echo   Stale:   "%OUT_STALE%"
exit /b 1
