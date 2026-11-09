@echo off
setlocal EnableExtensions

rem Self-host fixpoint check (legacy defect L-00).
rem qb.exe (gen1) builds qb-gen2.exe from source\qbnex.bas, qb-gen2 builds qb-gen3.exe.
rem gen2 and gen3 must emit byte-identical C++ for source\qbnex.bas, and both must pass
rem the conformance corpus. Pass --no-conformance to skip the corpus runs.

set "ROOT=%~dp0"
pushd "%ROOT%.." || exit /b 2
set "QB=.\qb.exe"
set "WORK=temp\fixpoint"
set "RC=0"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found. Build QBNex first.
    popd & exit /b 2
)
if exist "%WORK%" rmdir /s /q "%WORK%"
mkdir "%WORK%" || (popd & exit /b 2)

echo [fixpoint] gen1 -^> qb-gen2.exe
"%QB%" source\qbnex.bas -q -o qb-gen2.exe > "%WORK%\gen2.log" 2>&1
if errorlevel 1 (type "%WORK%\gen2.log" & echo [FAIL] gen1 could not build gen2 & set "RC=1" & goto done)

echo [fixpoint] gen2 -^> qb-gen3.exe
.\qb-gen2.exe source\qbnex.bas -q -o qb-gen3.exe > "%WORK%\gen3.log" 2>&1
if errorlevel 1 (type "%WORK%\gen3.log" & echo [FAIL] gen2 could not build gen3 & set "RC=1" & goto done)

for %%G in (gen2 gen3) do (
    .\qb-%%G.exe source\qbnex.bas -q -z > "%WORK%\%%G-z.log" 2>&1
    if errorlevel 1 (type "%WORK%\%%G-z.log" & echo [FAIL] %%G -z failed & set "RC=1" & goto done)
    xcopy /e /i /q /y internal\temp "%WORK%\%%G-c" > nul
)
git diff --no-index --quiet --stat "%WORK%\gen2-c" "%WORK%\gen3-c"
if errorlevel 1 (
    git diff --no-index --stat "%WORK%\gen2-c" "%WORK%\gen3-c"
    echo [FAIL] gen2 and gen3 emit different C++ for source\qbnex.bas
    set "RC=1" & goto done
)
echo [ok] gen2 and gen3 emit identical C++

if /i "%~1"=="--no-conformance" goto done
for %%G in (gen2 gen3) do (
    echo [fixpoint] conformance with qb-%%G.exe
    cargo run --release -q -p qbnex-conformance -- --qb .\qb-%%G.exe --work "%WORK%\conf-%%G" --timeout 60
    if errorlevel 1 (echo [FAIL] conformance with qb-%%G.exe & set "RC=1" & goto done)
)

:done
if "%RC%"=="0" (
    del /q qb-gen2.exe qb-gen3.exe 2> nul
    echo [PASS] self-host fixpoint
)
popd
exit /b %RC%
