@echo off
setlocal EnableExtensions
rem CHAIN runs another program and passes it the COMMON variables by position.
set "QBNEX_NO_ERROR_DIALOG=1"

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%..\.."
set "QB=%REPO_ROOT%\qb.exe"
set "FIX=%ROOT%..\fixtures"
if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_chain_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1 || (echo [FAIL] Could not create "%TMPDIR%" & exit /b 2)

"%QB%" "%FIX%\chain_common_main.bas" -o "%TMPDIR%\chain_common_main.exe" > "%TMPDIR%\main.log" 2>&1
if errorlevel 1 (type "%TMPDIR%\main.log" & echo [FAIL] chain_common_main.bas does not compile & exit /b 1)
"%QB%" "%FIX%\chain_common_child.bas" -o "%TMPDIR%\chain_common_child.exe" > "%TMPDIR%\child.log" 2>&1
if errorlevel 1 (type "%TMPDIR%\child.log" & echo [FAIL] chain_common_child.bas does not compile & exit /b 1)

pushd "%TMPDIR%"
.\chain_common_main.exe < nul > out.txt 2>&1
popd
findstr /x /c:" 7  2.5 text abcd 3 -4 " "%TMPDIR%\out.txt" >nul || (
    type "%TMPDIR%\out.txt"
    echo [FAIL] the chained program did not get the COMMON values
    exit /b 1
)
rem COMMON arrays: static INTEGER, TYPE and STRING * 3 arrays, then dynamic STRING and DOUBLE arrays with their bounds
set "ARRAYS_OK=1"
findstr /l /x /c:" 10  20  30  5  6  abcxyz" "%TMPDIR%\out.txt" >nul || set "ARRAYS_OK="
findstr /l /x /c:"-1  1  one||three" "%TMPDIR%\out.txt" >nul || set "ARRAYS_OK="
findstr /l /x /c:" 2  1  2  1.5 " "%TMPDIR%\out.txt" >nul || set "ARRAYS_OK="
if not defined ARRAYS_OK (
    type "%TMPDIR%\out.txt"
    echo [FAIL] the chained program did not get the COMMON arrays
    exit /b 1
)
findstr /c:"not reached" "%TMPDIR%\out.txt" >nul && (
    echo [FAIL] the program went on after CHAIN
    exit /b 1
)
echo CHAIN_SMOKE_OK
rmdir /s /q "%TMPDIR%" >nul 2>&1
exit /b 0
