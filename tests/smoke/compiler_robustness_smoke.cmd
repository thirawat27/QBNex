@echo off
setlocal EnableExtensions EnableDelayedExpansion
rem R14: inputs that crashed the compiler or stopped it with an internal error
rem must give a clean diagnostic (exit 1, "[!!]", no "INTERNAL COMPILER ERROR");
rem the valid ones must compile. Front end only (-z): no C++ compiler needed.
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

for %%f in (deep_parens deep_blocks deep_for ifdef_unclosed endif_alone ifdef_too_deep) do (
    "%QB%" "%FIX%\%%f.bas" -z -m < NUL > "%TMPDIR%\%%f.txt" 2>&1
    set "EC=!ERRORLEVEL!"
    if not "!EC!"=="1" (
        echo [FAIL] %%f: exit code !EC!, expected 1
        set FAILED=1
    ) else (
        findstr /L /c:"INTERNAL COMPILER ERROR" "%TMPDIR%\%%f.txt" >nul && (
            echo [FAIL] %%f: internal compiler error
            set FAILED=1
        )
        findstr /L /c:"[!!]" "%TMPDIR%\%%f.txt" >nul || (
            echo [FAIL] %%f: no diagnostic
            set FAILED=1
        )
    )
)

for %%f in (ifdef_valid deep_parens_ok) do (
    "%QB%" "%FIX%\%%f.bas" -z -m < NUL > "%TMPDIR%\%%f.txt" 2>&1
    set "EC=!ERRORLEVEL!"
    if not "!EC!"=="0" (
        echo [FAIL] %%f: exit code !EC!, expected 0
        type "%TMPDIR%\%%f.txt"
        set FAILED=1
    )
)

rd /s /q "%TMPDIR%" >nul 2>&1
if "%FAILED%"=="1" exit /b 1
echo [PASS] compiler robustness smoke
exit /b 0
