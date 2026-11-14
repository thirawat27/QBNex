@echo off
setlocal EnableExtensions
rem A window program (no $CONSOLE) is a GUI executable, so Windows opens no empty
rem console next to its window; $CONSOLE and $CONSOLE:ONLY programs stay console
rem executables (issue #7). The build here is the runner with the QIR appended.
set "QBNEX_NO_ERROR_DIALOG=1"
set "ROOT=%~dp0"
set "QB=%ROOT%..\..\qb.exe"
if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    exit /b 2
)
set "TMPDIR=%TEMP%\qbnex_pe_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
> "%TMPDIR%\win.bas" echo SCREEN 12: PRINT "hi"
(echo $CONSOLE& echo PRINT "hi")> "%TMPDIR%\con.bas"
(echo $CONSOLE:ONLY& echo PRINT "hi")> "%TMPDIR%\only.bas"
for %%n in (win con only) do (
    "%QB%" "%TMPDIR%\%%n.bas" -o "%TMPDIR%\%%n.exe" > "%TMPDIR%\%%n.txt" 2>&1
    if not exist "%TMPDIR%\%%n.exe" (
        type "%TMPDIR%\%%n.txt"
        echo [FAIL] pe subsystem smoke: build of %%n.bas
        exit /b 1
    )
)
powershell -NoProfile -Command "$ErrorActionPreference = 'Stop'; function Sub($f) { $b = [IO.File]::ReadAllBytes($f); $pe = [BitConverter]::ToInt32($b, 0x3C); [BitConverter]::ToUInt16($b, $pe + 92) }; $w = Sub '%TMPDIR%\win.exe'; $c = Sub '%TMPDIR%\con.exe'; $o = Sub '%TMPDIR%\only.exe'; Write-Host ('PE subsystem: window ' + $w + ', $CONSOLE ' + $c + ', $CONSOLE:ONLY ' + $o); if ($w -ne 2 -or $c -ne 3 -or $o -ne 3) { exit 1 }"
if errorlevel 1 (
    echo [FAIL] pe subsystem smoke: expected window 2 ^(GUI^), console 3
    exit /b 1
)
rmdir /s /q "%TMPDIR%" >nul 2>&1
echo [PASS] pe subsystem smoke: window programs are GUI executables, console programs console ones
exit /b 0
