@echo off
rem Compiles and runs one program per stdlib module, each importing only that
rem module ('$IMPORT:'<key>'), so a module that does not stand on its own (for
rem example a broken metacommand in it) fails here.
setlocal EnableExtensions EnableDelayedExpansion
set "QBNEX_NO_ERROR_DIALOG=1"

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%..\.."
for %%i in ("%REPO_ROOT%") do set "REPO_ROOT=%%~fi"
set "QB=%REPO_ROOT%\qb.exe"
set "STDLIB=%REPO_ROOT%\compiler\stdlib"
if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_stdlib_modules_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
set "FAIL=0"
set "COUNT=0"

for /r "%STDLIB%" %%f in (*.bas) do (
    set "rel=%%f"
    set "rel=!rel:%STDLIB%\=!"
    set "key=!rel:~0,-4!"
    set "key=!key:\=.!"
    rem qbnex_stdlib.bas is the body of '$IMPORT:'qbnex' (stdlib.bas)
    if /i not "!key!"=="qbnex_stdlib" (
        set /a COUNT+=1
        set "SRC=%TMPDIR%\m!COUNT!.bas"
        > "!SRC!" echo $CONSOLE:ONLY
        >> "!SRC!" echo '$IMPORT:'!key!'
        >> "!SRC!" echo PRINT "ok"
        >> "!SRC!" echo SYSTEM
        "%QB%" "!SRC!" -o "%TMPDIR%\m!COUNT!.exe" > "%TMPDIR%\m!COUNT!.log" 2>&1
        if not exist "%TMPDIR%\m!COUNT!.exe" (
            echo [FAIL] import of !key! does not compile ^(log: %TMPDIR%\m!COUNT!.log^)
            set "FAIL=1"
        ) else (
            "%TMPDIR%\m!COUNT!.exe" < NUL > "%TMPDIR%\m!COUNT!.out" 2>&1
            findstr /x /c:"ok" "%TMPDIR%\m!COUNT!.out" >nul || (
                echo [FAIL] program importing !key! did not print ok
                set "FAIL=1"
            )
        )
    )
)

if "%FAIL%"=="0" (
    echo [PASS] !COUNT! stdlib modules import on their own
    rd /s /q "%TMPDIR%" >nul 2>&1
    exit /b 0
)
exit /b 1
