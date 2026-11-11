@echo off
setlocal EnableExtensions

rem Self-host fixpoint check (legacy defect L-00), QIR pipeline.
rem qb.exe (gen1) builds qb-gen2.exe from compiler\qbnex.bas, qb-gen2 builds qb-gen3.exe.
rem The QIR that gen2 and gen3 emit for compiler\qbnex.bas must be byte-identical, and
rem identical to bootstrap\qbnex.qir. Pass --conformance to run the corpus with gen3 too.

set "ROOT=%~dp0"
pushd "%ROOT%.." || exit /b 2
set "WORK=target\fixpoint"
set "RC=0"

if not exist qb.exe (
    echo [FAIL] qb.exe not found. Run scripts\setup_win.cmd first.
    popd & exit /b 2
)
if exist "%WORK%" rmdir /s /q "%WORK%"
mkdir "%WORK%" || (popd & exit /b 2)

echo [fixpoint] gen1 -^> qb-gen2.exe
.\qb.exe compiler\qbnex.bas -q -o qb-gen2.exe > "%WORK%\gen2.log" 2>&1
if not "%ERRORLEVEL%"=="0" (type "%WORK%\gen2.log" & echo [FAIL] gen1 could not build gen2 & set "RC=1" & goto done)
copy /y target\qb-temp\program.qir "%WORK%\gen1.qir" > nul

echo [fixpoint] gen2 -^> qb-gen3.exe
.\qb-gen2.exe compiler\qbnex.bas -q -o qb-gen3.exe > "%WORK%\gen3.log" 2>&1
if not "%ERRORLEVEL%"=="0" (type "%WORK%\gen3.log" & echo [FAIL] gen2 could not build gen3 & set "RC=1" & goto done)
copy /y target\qb-temp\program.qir "%WORK%\gen2.qir" > nul

.\qb-gen3.exe compiler\qbnex.bas -q -o "%WORK%\qb-gen4.exe" > "%WORK%\gen4.log" 2>&1
if not "%ERRORLEVEL%"=="0" (type "%WORK%\gen4.log" & echo [FAIL] gen3 could not build gen4 & set "RC=1" & goto done)
copy /y target\qb-temp\program.qir "%WORK%\gen3.qir" > nul

fc /b "%WORK%\gen2.qir" "%WORK%\gen3.qir" > nul
if errorlevel 1 (echo [FAIL] gen2 and gen3 emit different QIR for compiler\qbnex.bas & set "RC=1" & goto done)
echo [ok] gen2 and gen3 emit identical QIR
fc /b "%WORK%\gen3.qir" bootstrap\qbnex.qir > nul
if errorlevel 1 (echo [FAIL] the compiler's QIR differs from bootstrap\qbnex.qir ^(refresh it^) & set "RC=1" & goto done)
echo [ok] bootstrap\qbnex.qir is up to date

if /i not "%~1"=="--conformance" goto done
echo [fixpoint] conformance with qb-gen3.exe
cargo run --release -q -p qbnex-conformance -- --qb .\qb-gen3.exe --work "%WORK%\conf" --timeout 60
if errorlevel 1 (echo [FAIL] conformance with qb-gen3.exe & set "RC=1" & goto done)

:done
del /q qb-gen2.exe qb-gen3.exe 2> nul
if "%RC%"=="0" echo [PASS] self-host fixpoint
popd
exit /b %RC%
