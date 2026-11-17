@echo off
setlocal
rem QBNex compiler (qb.exe). qb.exe runs the developer tools itself, so `qb fmt`
rem works whether qb.exe or this script is found first; this script starts them
rem directly, without qb.exe in between:
rem   qb fmt [options] [files]   formatter        (qb-fmt)
rem   qb lsp                     language server  (qb-lsp, stdio)
rem   qb dap                     debug adapter    (qb-dap, stdio)
rem Everything else goes to the compiler.
rem
rem The arguments are only used outside parenthesised blocks: a ")" in them
rem (qb run calc.bas -- f(2) y) would end such a block early. The first argument
rem is compared through a variable with delayed expansion, so quotes inside it
rem ("--x="a b"") cannot break the comparison. Delayed expansion is on only for
rem the comparison: with it on, a "!" in the arguments would be lost.
set "QB_ARG1=%~1"
setlocal enabledelayedexpansion
set "QB_TOOL="
if /i "!QB_ARG1!"=="fmt" set "QB_TOOL=fmt"
if /i "!QB_ARG1!"=="lsp" set "QB_TOOL=lsp"
if /i "!QB_ARG1!"=="dap" set "QB_TOOL=dap"
endlocal & set "QB_TOOL=%QB_TOOL%"
set "QB_ARG1="
if defined QB_TOOL goto tool
"%~dp0qb.exe" %*
rem an explicit code: a plain exit /b loses it when the script runs under cmd /c
exit /b %errorlevel%

:tool
rem The tool gets every argument and skips the subcommand itself (QBNEX_SUBCOMMAND):
rem cutting it off here would expose & ( ) inside the user's quotes to cmd.
set "QBNEX_SUBCOMMAND=%QB_TOOL%"
set "QB_TOOL=qb-%QB_TOOL%"
rem Next to this script, then a cargo build of this checkout, then PATH.
set "QB_EXE="
for %%d in ("%~dp0" "%~dp0target\release\" "%~dp0target\debug\") do (
    if not defined QB_EXE if exist "%%~d%QB_TOOL%.exe" set "QB_EXE=%%~d%QB_TOOL%.exe"
)
if not defined QB_EXE for %%x in (%QB_TOOL%.exe) do if not "%%~$PATH:x"=="" set "QB_EXE=%%~$PATH:x"
if not defined QB_EXE (
    echo qb: %QB_TOOL% was not found. Build it with: cargo build --release -p %QB_TOOL% 1>&2
    exit /b 9009
)
"%QB_EXE%" %*
exit /b %errorlevel%
