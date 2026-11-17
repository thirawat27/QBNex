@echo off
setlocal DisableDelayedExpansion
rem QBNex: same compiler as qb.cmd, available under the product name. qb.exe also
rem runs the developer tools:
rem   qbnex fmt [options] [files]   formatter        (qb-fmt)
rem   qbnex lsp                     language server  (qb-lsp, stdio)
rem   qbnex dap                     debug adapter    (qb-dap, stdio)
rem
rem The arguments go to qb.exe as typed, in one piece: nothing here reads or compares
rem them, so & ( ) ! and quotes inside them (--x="a&b", f(2)) cannot break this script.
"%~dp0qb.exe" %*
rem an explicit code: a plain exit /b loses it when the script runs under cmd /c
exit /b %errorlevel%
