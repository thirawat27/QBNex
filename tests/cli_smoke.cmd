@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_OK=%ROOT%fixtures\label_recompile_success.bas"
set "SRC_CONSOLE=%ROOT%fixtures\cli_console_output.bas"
set "BIN_XMODE=%REPO_ROOT%\cli_console_output.exe"
set "BIN_ZMODE=%REPO_ROOT%\label_recompile_success.exe"
set "STRINGS=%REPO_ROOT%\internal\c\c_compiler\bin\strings.exe"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC_OK%" (
    echo [FAIL] Fixture source not found at "%SRC_OK%"
    exit /b 2
)

if not exist "%SRC_CONSOLE%" (
    echo [FAIL] Console fixture source not found at "%SRC_CONSOLE%"
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
set "OUT_QUIET=%TMPDIR%\quiet.txt"
set "OUT_SETTINGS=%TMPDIR%\settings.txt"
set "OUT_ZMODE=%TMPDIR%\zmode.txt"
set "OUT_XMODE=%TMPDIR%\xmode.txt"
set "OUT_EXTERNAL=%TMPDIR%\external_cwd.txt"
set "OUT_CACHE_A=%TMPDIR%\cache_a.txt"
set "OUT_CACHE_B=%TMPDIR%\cache_b.txt"
set "OUT_CACHE_A_STRINGS=%TMPDIR%\cache_a_strings.txt"
set "OUT_CACHE_B_STRINGS=%TMPDIR%\cache_b_strings.txt"
set "BIN_EXTERNAL=%TMPDIR%\external_cwd.exe"
set "SRC_CACHE_A=%TMPDIR%\cache_a.bas"
set "SRC_CACHE_B=%TMPDIR%\cache_b.bas"
set "BIN_CACHE_A=%TMPDIR%\cache_a.exe"
set "BIN_CACHE_B=%TMPDIR%\cache_b.exe"

if exist "%BIN_XMODE%" del /f /q "%BIN_XMODE%" >nul 2>&1
if exist "%BIN_ZMODE%" del /f /q "%BIN_ZMODE%" >nul 2>&1

"%QB%" --help > "%OUT_HELP%" 2>&1
set "EC_HELP=%ERRORLEVEL%"

"%QB%" --version > "%OUT_VERSION%" 2>&1
set "EC_VERSION=%ERRORLEVEL%"

"%QB%" --definitely-invalid > "%OUT_INVALID%" 2>&1
set "EC_INVALID=%ERRORLEVEL%"

"%QB%" "%SRC_OK%" -o "%TMPDIR%\missing-dir\out.exe" > "%OUT_BADOUT%" 2>&1
set "EC_BADOUT=%ERRORLEVEL%"

"%QB%" "%SRC_OK%" -q -o "%TMPDIR%\quiet.exe" > "%OUT_QUIET%" 2>&1
set "EC_QUIET=%ERRORLEVEL%"

"%QB%" -s > "%OUT_SETTINGS%" 2>&1
set "EC_SETTINGS=%ERRORLEVEL%"

"%QB%" "%SRC_OK%" -z > "%OUT_ZMODE%" 2>&1
set "EC_ZMODE=%ERRORLEVEL%"

"%QB%" "%SRC_CONSOLE%" -x > "%OUT_XMODE%" 2>&1
set "EC_XMODE=%ERRORLEVEL%"

pushd "%TMPDIR%" >nul
"%QB%" "%SRC_OK%" -o "%BIN_EXTERNAL%" > "%OUT_EXTERNAL%" 2>&1
set "EC_EXTERNAL=%ERRORLEVEL%"
popd >nul

> "%SRC_CACHE_A%" echo PRINT "CACHE_A_UNIQUE"
> "%SRC_CACHE_B%" echo PRINT "CACHE_B_UNIQUE"

"%QB%" "%SRC_CACHE_A%" -q -o "%BIN_CACHE_A%" > "%OUT_CACHE_A%" 2>&1
set "EC_CACHE_A=%ERRORLEVEL%"

"%QB%" "%SRC_CACHE_B%" -q -o "%BIN_CACHE_B%" > "%OUT_CACHE_B%" 2>&1
set "EC_CACHE_B=%ERRORLEVEL%"

if exist "%STRINGS%" (
    if exist "%BIN_CACHE_A%" "%STRINGS%" "%BIN_CACHE_A%" > "%OUT_CACHE_A_STRINGS%" 2>&1
    if exist "%BIN_CACHE_B%" "%STRINGS%" "%BIN_CACHE_B%" > "%OUT_CACHE_B_STRINGS%" 2>&1
)

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

if not "%EC_QUIET%"=="0" (
    echo [FAIL] Quiet compile should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"Compiling program..." "%OUT_QUIET%" >nul && (
    echo [FAIL] Quiet compile should suppress progress banner.
    set "FAIL=1"
)
if not exist "%TMPDIR%\quiet.exe" (
    echo [FAIL] Quiet compile should still emit an executable.
    set "FAIL=1"
)

if not "%EC_SETTINGS%"=="0" (
    echo [FAIL] -s should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"debuginfo" "%OUT_SETTINGS%" >nul || (
    echo [FAIL] -s output is missing debuginfo setting.
    set "FAIL=1"
)
findstr /L /C:"exewithsource" "%OUT_SETTINGS%" >nul || (
    echo [FAIL] -s output is missing exewithsource setting.
    set "FAIL=1"
)

if not "%EC_ZMODE%"=="0" (
    echo [FAIL] -z should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"Compiling program..." "%OUT_ZMODE%" >nul && (
    echo [FAIL] -z should not invoke native executable build output.
    set "FAIL=1"
)

if not "%EC_XMODE%"=="0" (
    echo [FAIL] -x should exit successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_XMODE%" >nul || (
    echo [FAIL] -x output is missing successful build output.
    set "FAIL=1"
)
if not exist "%BIN_XMODE%" (
    echo [FAIL] -x should emit the default output executable.
    set "FAIL=1"
)

if not "%EC_EXTERNAL%"=="0" (
    echo [FAIL] Running qb.exe from outside the repo root should still compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_EXTERNAL%" >nul || (
    echo [FAIL] External-cwd compile is missing successful build output.
    set "FAIL=1"
)
if not exist "%BIN_EXTERNAL%" (
    echo [FAIL] External-cwd compile should emit the requested executable.
    set "FAIL=1"
)

if not exist "%STRINGS%" (
    echo [FAIL] strings.exe not found at "%STRINGS%".
    set "FAIL=1"
)

if not "%EC_CACHE_A%"=="0" (
    echo [FAIL] Cache regression compile A should exit successfully.
    set "FAIL=1"
)
if not "%EC_CACHE_B%"=="0" (
    echo [FAIL] Cache regression compile B should exit successfully.
    set "FAIL=1"
)
if not exist "%BIN_CACHE_A%" (
    echo [FAIL] Cache regression compile A should emit an executable.
    set "FAIL=1"
)
if not exist "%BIN_CACHE_B%" (
    echo [FAIL] Cache regression compile B should emit an executable.
    set "FAIL=1"
)
if exist "%OUT_CACHE_A_STRINGS%" (
    findstr /L /C:"CACHE_A_UNIQUE" "%OUT_CACHE_A_STRINGS%" >nul || (
        echo [FAIL] Cache regression binary A is missing its unique string.
        set "FAIL=1"
    )
    findstr /L /C:"CACHE_B_UNIQUE" "%OUT_CACHE_A_STRINGS%" >nul && (
        echo [FAIL] Cache regression binary A contains binary B's string.
        set "FAIL=1"
    )
)
if exist "%OUT_CACHE_B_STRINGS%" (
    findstr /L /C:"CACHE_B_UNIQUE" "%OUT_CACHE_B_STRINGS%" >nul || (
        echo [FAIL] Cache regression binary B is missing its unique string.
        set "FAIL=1"
    )
    findstr /L /C:"CACHE_A_UNIQUE" "%OUT_CACHE_B_STRINGS%" >nul && (
        echo [FAIL] Cache regression binary B contains binary A's string.
        set "FAIL=1"
    )
)

if "%FAIL%"=="0" (
    echo CLI_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    set "RESULT=0"
    goto :cleanup
)

echo CLI_SMOKE_FAIL
echo Inspect outputs:
echo   Help: "%OUT_HELP%"
echo   Version: "%OUT_VERSION%"
echo   Invalid switch: "%OUT_INVALID%"
echo   Bad output path: "%OUT_BADOUT%"
echo   Quiet: "%OUT_QUIET%"
echo   Settings: "%OUT_SETTINGS%"
echo   Z mode: "%OUT_ZMODE%"
echo   X mode: "%OUT_XMODE%"
echo   External cwd: "%OUT_EXTERNAL%"
echo   Cache A: "%OUT_CACHE_A%"
echo   Cache B: "%OUT_CACHE_B%"
set "RESULT=1"
goto :cleanup

:cleanup
if exist "%BIN_XMODE%" del /f /q "%BIN_XMODE%" >nul 2>&1
if exist "%BIN_ZMODE%" del /f /q "%BIN_ZMODE%" >nul 2>&1
exit /b %RESULT%
