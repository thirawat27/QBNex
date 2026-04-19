@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC=%ROOT%fixtures\unused_variable_warning.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC%" (
    echo [FAIL] Warning fixture source not found at "%SRC%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_warnings_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "BIN_WARN=%TMPDIR%\warn.exe"
set "BIN_WERROR=%TMPDIR%\werror.exe"
set "OUT_WARN=%TMPDIR%\warn.txt"
set "OUT_WERROR=%TMPDIR%\werror.txt"

"%QB%" "%SRC%" -w -o "%BIN_WARN%" > "%OUT_WARN%" 2>&1
set "EC_WARN=%ERRORLEVEL%"

"%QB%" "%SRC%" --warnings-as-errors -o "%BIN_WERROR%" > "%OUT_WERROR%" 2>&1
set "EC_WERROR=%ERRORLEVEL%"

set "FAIL=0"

if not "%EC_WARN%"=="0" (
    echo [FAIL] Warning fixture with -w should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"warning: " "%OUT_WARN%" >nul || (
    echo [FAIL] Warning fixture with -w is missing warning output.
    set "FAIL=1"
)
findstr /L /C:"unused variable" "%OUT_WARN%" >nul || (
    echo [FAIL] Warning fixture with -w is missing the warning header.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_WARN%" >nul || (
    echo [FAIL] Warning fixture with -w is missing successful build output.
    set "FAIL=1"
)
if not exist "%BIN_WARN%" (
    echo [FAIL] Warning fixture with -w should emit an executable.
    set "FAIL=1"
)

if "%EC_WERROR%"=="0" (
    echo [FAIL] Warning fixture with --warnings-as-errors should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"warning promoted to blocking diagnostic" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing the promotion note.
    set "FAIL=1"
)
findstr /L /C:"unused variable" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing the warning header.
    set "FAIL=1"
)
findstr /L /C:"[x] QBNex :: Error [W" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing structured diagnostic headline.
    set "FAIL=1"
)
findstr /L /C:"  [@] " "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing location marker [@].
    set "FAIL=1"
)
findstr /L /C:"  [#] source" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing source marker [#].
    set "FAIL=1"
)
findstr /L /C:"  [>] next" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing suggestion marker [>].
    set "FAIL=1"
)
findstr /L /C:"  [::] flow" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing flow marker [::].
    set "FAIL=1"
)
findstr /L /C:"  [!] cause" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing cause marker [!].
    set "FAIL=1"
)
findstr /L /C:"  [+] example" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing example marker [+].
    set "FAIL=1"
)
findstr /L /C:"[x] QBNex :: Build Halted" "%OUT_WERROR%" >nul || (
    echo [FAIL] Warnings-as-errors output is missing build halt summary.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_WERROR%" >nul && (
    echo [FAIL] Warnings-as-errors run should not report a successful build.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo WARNINGS_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo WARNINGS_SMOKE_FAIL
echo Inspect outputs:
echo   Warn:   "%OUT_WARN%"
echo   Werror: "%OUT_WERROR%"
exit /b 1
