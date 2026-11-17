@echo off
setlocal DisableDelayedExpansion
rem QBNex compiler (qb.exe). qb.exe also runs the developer tools:
rem   qb fmt [options] [files]   formatter        (qb-fmt)
rem   qb lsp                     language server  (qb-lsp, stdio)
rem   qb dap                     debug adapter    (qb-dap, stdio)
rem
rem The arguments go to qb.exe as typed, in one piece: nothing here reads or compares
rem them, so & ( ) ! and quotes inside them (--x="a&b", f(2)) cannot break this script.
"%~dp0qb.exe" %*
rem an explicit code: a plain exit /b loses it when the script runs under cmd /c
exit /b %errorlevel%
