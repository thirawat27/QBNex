@echo off
setlocal EnableExtensions

rem Regenerates the stage0 bootstrap snapshot internal\source from source\qbnex.bas.
rem Only the generated C++ fragments (*.txt) and DATA (data.bin) are replaced; the
rem hand-kept helper files (icon.rc, *.bat, manifest.h, manifests, icons, *.bin) stay
rem unchanged because the generated versions contain machine-specific paths.
rem Run tests\selfhost_fixpoint.cmd first: the snapshot must come from a fixpoint compiler.

set "ROOT=%~dp0"
pushd "%ROOT%.." || exit /b 2
if not exist ".\qb.exe" (
    echo [FAIL] qb.exe not found. Build QBNex first.
    popd & exit /b 2
)

if exist internal\temp rmdir /s /q internal\temp
mkdir internal\temp || (popd & exit /b 2)
.\qb.exe source\qbnex.bas -q -z > temp-refresh.log 2>&1
if errorlevel 1 (type temp-refresh.log & del temp-refresh.log & echo [FAIL] qb.exe -z failed & popd & exit /b 1)
del temp-refresh.log

del /q internal\source\*.txt
copy /y internal\temp\*.txt internal\source\ > nul || (popd & exit /b 1)
copy /y internal\temp\data.bin internal\source\data.bin > nul || (popd & exit /b 1)
echo [ok] internal\source refreshed from source\qbnex.bas
popd
exit /b 0
