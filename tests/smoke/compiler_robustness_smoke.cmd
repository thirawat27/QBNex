@echo off
setlocal EnableExtensions
rem R14: inputs that crashed the compiler or stopped it with an internal error
rem must give a clean diagnostic (exit 1, the expected message, a line number,
rem no "INTERNAL COMPILER ERROR"); the valid ones must compile. Front end only
rem (-z): no C++ compiler needed.
set "QBNEX_NO_ERROR_DIALOG=1"
set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%..\.."
set "QB=%REPO_ROOT%\qb.exe"
set "FIX=%ROOT%..\fixtures\robustness"
if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    exit /b 2
)
set "TMPDIR=%TEMP%\qbnex_robust_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
set FAILED=0

call :bad deep_parens "Expression is nested too deeply" "deep_parens.bas(2)"
call :bad deep_blocks "Too many nested blocks" "deep_blocks.bas(1002)"
call :bad deep_for "Too many nested blocks" "deep_for.bas(1002)"
call :bad ifdef_unclosed "$IF without $END IF" "ifdef_unclosed.bas(2)"
call :bad endif_alone "$END IF without $IF" "endif_alone.bas(2)"
call :bad ifdef_too_deep "Too many nested $IF blocks" "ifdef_too_deep.bas(257)"
call :good ifdef_valid
call :good deep_parens_ok
call :good long_expr

rd /s /q "%TMPDIR%" >nul 2>&1
if "%FAILED%"=="1" exit /b 1
echo [PASS] compiler robustness smoke
exit /b 0

:bad
"%QB%" "%FIX%\%~1.bas" -z -m < NUL > "%TMPDIR%\%~1.txt" 2>&1
if not "%ERRORLEVEL%"=="1" (
    echo [FAIL] %~1: exit code %ERRORLEVEL%, expected 1
    set FAILED=1
    exit /b 0
)
findstr /L /c:"INTERNAL COMPILER ERROR" "%TMPDIR%\%~1.txt" >nul && (
    echo [FAIL] %~1: internal compiler error
    set FAILED=1
)
findstr /L /c:"%~2" "%TMPDIR%\%~1.txt" >nul || (
    echo [FAIL] %~1: message "%~2" missing
    set FAILED=1
)
findstr /L /c:"%~3" "%TMPDIR%\%~1.txt" >nul || (
    echo [FAIL] %~1: location "%~3" missing
    set FAILED=1
)
exit /b 0

:good
"%QB%" "%FIX%\%~1.bas" -z -m < NUL > "%TMPDIR%\%~1.txt" 2>&1
if not "%ERRORLEVEL%"=="0" (
    echo [FAIL] %~1: exit code %ERRORLEVEL%, expected 0
    type "%TMPDIR%\%~1.txt"
    set FAILED=1
)
exit /b 0
