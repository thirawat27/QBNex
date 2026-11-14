@echo off
setlocal
rem QBNex: same compiler as qb.cmd, available under the product name.
rem Subcommands for the developer tools (Rust):
rem   qbnex fmt [options] [files]   formatter        (qb-fmt)
rem   qbnex lsp                     language server  (qb-lsp, stdio)
rem   qbnex dap                     debug adapter    (qb-dap, stdio)
rem Everything else goes to the compiler.
set "QB_TOOL="
if /i "%~1"=="fmt" set "QB_TOOL=qb-fmt"
if /i "%~1"=="lsp" set "QB_TOOL=qb-lsp"
if /i "%~1"=="dap" set "QB_TOOL=qb-dap"
if not defined QB_TOOL (
    "%~dp0qb.exe" %*
    exit /b
)
rem The tool gets every argument and skips the subcommand itself (QBNEX_SUBCOMMAND):
rem cutting it off here would expose & ( ) inside the user's quotes to cmd.
set "QBNEX_SUBCOMMAND=%~1"
rem Next to this script, then a cargo build of this checkout, then PATH.
set "QB_EXE="
for %%d in ("%~dp0" "%~dp0target\release\" "%~dp0target\debug\") do (
    if not defined QB_EXE if exist "%%~d%QB_TOOL%.exe" set "QB_EXE=%%~d%QB_TOOL%.exe"
)
if not defined QB_EXE for %%x in (%QB_TOOL%.exe) do if not "%%~$PATH:x"=="" set "QB_EXE=%%~$PATH:x"
if not defined QB_EXE (
    echo qbnex: %QB_TOOL% was not found. Build it with: cargo build --release -p %QB_TOOL% 1>&2
    exit /b 9009
)
"%QB_EXE%" %*
