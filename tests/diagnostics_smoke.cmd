@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC=%ROOT%fixtures\diagnostics_compile_error.bas"
set "SRC_INCLUDE=%ROOT%fixtures\diagnostics_include_chain_main.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC%" (
    echo [FAIL] Fixture source not found at "%SRC%"
    exit /b 2
)

if not exist "%SRC_INCLUDE%" (
    echo [FAIL] Include-chain fixture source not found at "%SRC_INCLUDE%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_diag_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "OUT_DEFAULT=%TMPDIR%\default_diagnostics.txt"
set "OUT_COMPACT=%TMPDIR%\compact_diagnostics.txt"
set "OUT_INCLUDE=%TMPDIR%\include_chain_diagnostics.txt"

"%QB%" "%SRC%" > "%OUT_DEFAULT%" 2>&1
set "EC_DEFAULT=%ERRORLEVEL%"

"%QB%" "%SRC%" --compact-errors > "%OUT_COMPACT%" 2>&1
set "EC_COMPACT=%ERRORLEVEL%"

"%QB%" "%SRC_INCLUDE%" > "%OUT_INCLUDE%" 2>&1
set "EC_INCLUDE=%ERRORLEVEL%"

set "FAIL=0"

if "%EC_DEFAULT%"=="0" (
    echo [FAIL] Default diagnostics run should fail for invalid source.
    set "FAIL=1"
)

findstr /L /C:"[!] cause" "%OUT_DEFAULT%" >nul || (
    echo [FAIL] Default diagnostics output is missing [!] cause.
    set "FAIL=1"
)
findstr /L /C:"[+] example" "%OUT_DEFAULT%" >nul || (
    echo [FAIL] Default diagnostics output is missing [+] example.
    set "FAIL=1"
)
findstr /L /C:"[::] flow" "%OUT_DEFAULT%" >nul || (
    echo [FAIL] Default diagnostics output is missing [::] flow.
    set "FAIL=1"
)

if "%EC_COMPACT%"=="0" (
    echo [FAIL] Compact diagnostics run should fail for invalid source.
    set "FAIL=1"
)

findstr /L /C:"[!] cause" "%OUT_COMPACT%" >nul && (
    echo [FAIL] Compact diagnostics output should not include [!] cause.
    set "FAIL=1"
)
findstr /L /C:"[+] example" "%OUT_COMPACT%" >nul && (
    echo [FAIL] Compact diagnostics output should not include [+] example.
    set "FAIL=1"
)

if "%EC_INCLUDE%"=="0" (
    echo [FAIL] Include-chain diagnostics run should fail for missing nested include.
    set "FAIL=1"
)
findstr /L /C:"diagnostics_include_chain_missing.bas" "%OUT_INCLUDE%" >nul || (
    echo [FAIL] Include-chain diagnostics are missing the nested include filename.
    set "FAIL=1"
)
findstr /L /C:"diagnostics_include_chain_mid.bas" "%OUT_INCLUDE%" >nul || (
    echo [FAIL] Include-chain diagnostics are missing the parent include filename.
    set "FAIL=1"
)
findstr /L /C:"Triggered while compiling an included module." "%OUT_INCLUDE%" >nul || (
    echo [FAIL] Include-chain diagnostics are missing the location note.
    set "FAIL=1"
)
for /f %%C in ('findstr /L /C:"File diagnostics_include_chain_missing.bas not found" "%OUT_INCLUDE%" ^| find /c /v ""') do set "COUNT_INCLUDE=%%C"
if not "%COUNT_INCLUDE%"=="1" (
    echo [FAIL] Include-chain diagnostics should render the blocking error once.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo DIAGNOSTICS_SMOKE_OK
    echo Source fixture: "%SRC%"
    echo Output samples:
    echo   Default: "%OUT_DEFAULT%"
    echo   Compact: "%OUT_COMPACT%"
    echo   Include: "%OUT_INCLUDE%"
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo DIAGNOSTICS_SMOKE_FAIL
echo Source fixture: "%SRC%"
echo Inspect outputs:
echo   Default: "%OUT_DEFAULT%"
echo   Compact: "%OUT_COMPACT%"
echo   Include: "%OUT_INCLUDE%"
exit /b 1
