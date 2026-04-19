@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_encoding_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "SRC_UTF16=%TMPDIR%\utf16_source.bas"
set "SRC_INVALID=%TMPDIR%\invalid_utf8_source.bas"
set "SRC_INVALID_MID=%TMPDIR%\invalid_utf8_mid_source.bas"
set "SRC_BOM=%TMPDIR%\utf8_bom_source.bas"
set "SRC_EMPTY=%TMPDIR%\empty_source.bas"
set "SRC_SPACE_DIR=%TMPDIR%\source with spaces"
set "SRC_SPACE=%SRC_SPACE_DIR%\hello world.bas"
set "BIN_UTF16=%TMPDIR%\utf16_output.exe"
set "BIN_INVALID=%TMPDIR%\invalid_utf8_output.exe"
set "BIN_INVALID_MID=%TMPDIR%\invalid_utf8_mid_output.exe"
set "BIN_BOM=%TMPDIR%\utf8_bom_output.exe"
set "BIN_EMPTY=%TMPDIR%\empty_output.exe"
set "BIN_SPACE=%TMPDIR%\output with spaces\hello world.exe"
set "BIN_STALE=%TMPDIR%\stale_output.exe"
set "OUT_UTF16=%TMPDIR%\utf16_output.txt"
set "OUT_INVALID=%TMPDIR%\invalid_utf8_output.txt"
set "OUT_INVALID_MID=%TMPDIR%\invalid_utf8_mid_output.txt"
set "OUT_BOM=%TMPDIR%\utf8_bom_output.txt"
set "OUT_EMPTY=%TMPDIR%\empty_output.txt"
set "OUT_SPACE=%TMPDIR%\space_path_output.txt"
set "OUT_STALE=%TMPDIR%\stale_output.txt"

powershell -NoProfile -Command "[System.IO.File]::WriteAllBytes($env:SRC_UTF16, [byte[]](255,254,80,0,82,0,73,0,78,0,84,0,32,0,34,0,104,0,105,0,34,0,13,0,10,0))" >nul
if errorlevel 1 (
    echo [FAIL] Could not create UTF-16 fixture.
    exit /b 2
)

powershell -NoProfile -Command "[System.IO.File]::WriteAllBytes($env:SRC_INVALID, [byte[]](80,82,73,78,84,32,34,255,34,13,10))" >nul
if errorlevel 1 (
    echo [FAIL] Could not create invalid UTF-8 fixture.
    exit /b 2
)

powershell -NoProfile -Command "[System.IO.File]::WriteAllBytes($env:SRC_INVALID_MID, [byte[]](80,82,73,78,84,32,34,111,107,34,13,10,80,82,73,78,84,32,34,255,34,13,10))" >nul
if errorlevel 1 (
    echo [FAIL] Could not create mid-file invalid UTF-8 fixture.
    exit /b 2
)

powershell -NoProfile -Command "[System.IO.File]::WriteAllBytes($env:SRC_BOM, [byte[]](239,187,191,80,82,73,78,84,32,34,104,105,34,13,10))" >nul
if errorlevel 1 (
    echo [FAIL] Could not create UTF-8 BOM fixture.
    exit /b 2
)

type nul > "%SRC_EMPTY%"
if errorlevel 1 (
    echo [FAIL] Could not create empty-source fixture.
    exit /b 2
)

mkdir "%SRC_SPACE_DIR%" >nul 2>&1
mkdir "%TMPDIR%\output with spaces" >nul 2>&1
(
    echo PRINT "hi"
) > "%SRC_SPACE%"
if errorlevel 1 (
    echo [FAIL] Could not create spaced-path fixture.
    exit /b 2
)

"%QB%" "%SRC_UTF16%" -o "%BIN_UTF16%" > "%OUT_UTF16%" 2>&1
set "EC_UTF16=%ERRORLEVEL%"

"%QB%" "%SRC_INVALID%" -o "%BIN_INVALID%" > "%OUT_INVALID%" 2>&1
set "EC_INVALID=%ERRORLEVEL%"

"%QB%" "%SRC_INVALID_MID%" -o "%BIN_INVALID_MID%" > "%OUT_INVALID_MID%" 2>&1
set "EC_INVALID_MID=%ERRORLEVEL%"

"%QB%" "%SRC_BOM%" -o "%BIN_BOM%" > "%OUT_BOM%" 2>&1
set "EC_BOM=%ERRORLEVEL%"

"%QB%" "%SRC_EMPTY%" -o "%BIN_EMPTY%" > "%OUT_EMPTY%" 2>&1
set "EC_EMPTY=%ERRORLEVEL%"

"%QB%" "%SRC_SPACE%" -o "%BIN_SPACE%" > "%OUT_SPACE%" 2>&1
set "EC_SPACE=%ERRORLEVEL%"

"%QB%" "%SRC_BOM%" -o "%BIN_STALE%" >nul 2>&1
"%QB%" "%SRC_INVALID%" -o "%BIN_STALE%" > "%OUT_STALE%" 2>&1
set "EC_STALE=%ERRORLEVEL%"

set "FAIL=0"

if "%EC_UTF16%"=="0" (
    echo [FAIL] UTF-16 source should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"UTF-16 LE encoding detected" "%OUT_UTF16%" >nul || (
    echo [FAIL] UTF-16 diagnostics are missing the encoding error.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_UTF16%" >nul && (
    echo [FAIL] UTF-16 diagnostics should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_UTF16%" (
    echo [FAIL] UTF-16 compilation should not emit an executable.
    set "FAIL=1"
)

if "%EC_INVALID%"=="0" (
    echo [FAIL] Invalid UTF-8 source should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Invalid UTF-8 byte sequence detected in source file" "%OUT_INVALID%" >nul || (
    echo [FAIL] Invalid UTF-8 diagnostics are missing the fatal error.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_INVALID%" >nul && (
    echo [FAIL] Invalid UTF-8 diagnostics should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_INVALID%" (
    echo [FAIL] Invalid UTF-8 compilation should not emit an executable.
    set "FAIL=1"
)

if "%EC_INVALID_MID%"=="0" (
    echo [FAIL] Mid-file invalid UTF-8 source should fail compilation.
    set "FAIL=1"
)
findstr /L /C:"Invalid UTF-8 byte sequence detected in source file" "%OUT_INVALID_MID%" >nul || (
    echo [FAIL] Mid-file invalid UTF-8 diagnostics are missing the fatal error.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_INVALID_MID%" >nul && (
    echo [FAIL] Mid-file invalid UTF-8 diagnostics should not report a successful build.
    set "FAIL=1"
)
if exist "%BIN_INVALID_MID%" (
    echo [FAIL] Mid-file invalid UTF-8 compilation should not emit an executable.
    set "FAIL=1"
)

if not "%EC_BOM%"=="0" (
    echo [FAIL] UTF-8 BOM source should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_BOM%" >nul || (
    echo [FAIL] UTF-8 BOM fixture is missing successful build output.
    set "FAIL=1"
)
findstr /L /C:"encoding detected" "%OUT_BOM%" >nul && (
    echo [FAIL] UTF-8 BOM fixture should not report an encoding failure.
    set "FAIL=1"
)
if not exist "%BIN_BOM%" (
    echo [FAIL] UTF-8 BOM compilation should emit an executable.
    set "FAIL=1"
)

if not "%EC_EMPTY%"=="0" (
    echo [FAIL] Empty source should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_EMPTY%" >nul || (
    echo [FAIL] Empty-source fixture is missing successful build output.
    set "FAIL=1"
)
if not exist "%BIN_EMPTY%" (
    echo [FAIL] Empty-source compilation should emit an executable.
    set "FAIL=1"
)

if not "%EC_SPACE%"=="0" (
    echo [FAIL] Source and output paths with spaces should compile successfully.
    set "FAIL=1"
)
findstr /L /C:"Build complete:" "%OUT_SPACE%" >nul || (
    echo [FAIL] Spaced-path fixture is missing successful build output.
    set "FAIL=1"
)
if not exist "%BIN_SPACE%" (
    echo [FAIL] Spaced-path compilation should emit an executable.
    set "FAIL=1"
)

if "%EC_STALE%"=="0" (
    echo [FAIL] Recompiling invalid UTF-8 over an existing output should fail.
    set "FAIL=1"
)
findstr /L /C:"Warning: Existing output was not updated because compilation failed." "%OUT_STALE%" >nul || (
    echo [FAIL] Stale-output run is missing the stale executable warning.
    set "FAIL=1"
)
if not exist "%BIN_STALE%" (
    echo [FAIL] Stale-output run should keep the existing executable in place.
    set "FAIL=1"
)

if "%FAIL%"=="0" (
    echo ENCODING_SMOKE_OK
    rmdir /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)

echo ENCODING_SMOKE_FAIL
echo Inspect outputs:
echo   UTF-16: "%OUT_UTF16%"
echo   Invalid UTF-8: "%OUT_INVALID%"
echo   Mid invalid UTF-8: "%OUT_INVALID_MID%"
echo   UTF-8 BOM: "%OUT_BOM%"
echo   Empty: "%OUT_EMPTY%"
echo   Spaced path: "%OUT_SPACE%"
echo   Stale output: "%OUT_STALE%"
exit /b 1
