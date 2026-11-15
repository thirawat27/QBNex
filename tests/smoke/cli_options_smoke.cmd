@echo off
setlocal EnableExtensions
rem Every command and option of `qb --help` does what its help line says, and each
rem one prints something of its own: a switch that is accepted but changes nothing
rem is caught here. (-m: output to a file is uncoloured anyway, so only its exit
rem code is checked; the colours themselves need a terminal.)
set "QBNEX_NO_ERROR_DIALOG=1"
set "NO_COLOR="

set "ROOT=%~dp0"
set "QB=%ROOT%..\..\qb.exe"
if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

set "T=%TEMP%\qbnex_cli_options_%RANDOM%_%RANDOM%"
mkdir "%T%" >nul 2>&1 || (
    echo [FAIL] Could not create "%T%"
    exit /b 2
)
> "%T%\clean.bas" echo PRINT "CLEAN_OK"
> "%T%\warn.bas" echo DIM unusedVar AS INTEGER
>> "%T%\warn.bas" echo PRINT "WARN_OK"
> "%T%\err.bas" echo PRINT "before"
>> "%T%\err.bas" echo x = (1 + 2
>> "%T%\err.bas" echo PRINT "after the error"
> "%T%\undecl.bas" echo total = 5
>> "%T%\undecl.bas" echo PRINT total

set "FAIL=0"

rem ---- commands: four different screens ----
call :run help 0 -h
call :has help "Usage: qb <file> [switches]"
call :run version 0 --version
call :has version "QBNex Compiler "
call :hasnot version "Usage:"
call :hasnot version "Repository:"
call :run info 0 -i
call :has info "Settings:"
call :has info "Repository:"
call :hasnot info "Usage:"
call :run examples 0 -g
call :has examples "Examples:"
call :has examples "qb -s:exewithsource=true"
call :hasnot examples "Usage:"

rem ---- building: <file>, -o; the QB64 switches -c -x -p -z (not in the help) still build ----
call :run clean 0 "%T%\clean.bas" -o "%T%\clean.exe"
call :has clean "Build complete:"
call :exists "%T%\clean.exe"
for %%s in (c x p z) do (
    call :run clean_%%s 0 "%T%\clean.bas" -%%s -o "%T%\clean_%%s.exe"
    call :has clean_%%s "Build complete:"
    call :exists "%T%\clean_%%s.exe"
)
"%T%\clean.exe" < NUL > "%T%\clean_run.txt" 2>&1
call :has clean_run "CLEAN_OK"

rem ---- -q: no banner or progress ----
call :run quiet 0 "%T%\clean.bas" -q -o "%T%\quiet.exe"
call :hasnot quiet "Build complete"
call :hasnot quiet "QBNex Compiler"
call :exists "%T%\quiet.exe"

rem ---- -m ----
call :run mono 0 "%T%\clean.bas" -m -o "%T%\mono.exe"
call :exists "%T%\mono.exe"

rem ---- warnings: counted, -w shows them, -Werror blocks ----
call :run warn 0 "%T%\warn.bas" -o "%T%\warn.exe"
call :has warn "warning(s) not shown; add -w"
call :hasnot warn "warning: unused variable"
call :run warn_w 0 "%T%\warn.bas" -w -o "%T%\warn_w.exe"
call :has warn_w "warning: unused variable"
call :has warn_w "warning(s) shown above"
call :run clean_w 0 "%T%\clean.bas" -w -o "%T%\clean_w.exe"
call :has clean_w "No warnings."
call :run werror 1 "%T%\warn.bas" -Werror -o "%T%\werror.exe"
call :has werror "warning promoted to blocking diagnostic"
call :missing "%T%\werror.exe"
call :run werror_long 1 "%T%\warn.bas" --warnings-as-errors -k -o "%T%\werror.exe"
call :hasnot werror_long "[!] cause"

rem ---- diagnostics: default, -d (lines around), -k (compact) ----
call :run err 1 "%T%\err.bas" -o "%T%\err.exe"
call :has err "[!] cause"
call :hasnot err "after the error"
call :run err_d 1 "%T%\err.bas" -d -o "%T%\err.exe"
call :has err_d "[!] cause"
call :has err_d "after the error"
call :run err_k 1 "%T%\err.bas" --compact-errors -o "%T%\err.exe"
call :has err_k "Missing )"
call :hasnot err_k "[!] cause"

rem ---- -e: OPTION _EXPLICIT ----
call :run undecl 0 "%T%\undecl.bas" -o "%T%\undecl.exe"
call :run undecl_e 1 "%T%\undecl.bas" -e -o "%T%\undecl_e.exe"
call :has undecl_e "'total' is not defined"
call :missing "%T%\undecl_e.exe"

rem ---- --emit-ir and -o name.qir: QIR text, no executable ----
call :run emit 0 "%T%\clean.bas" --emit-ir -o "%T%\emit.exe"
call :exists "%T%\emit.qir"
call :missing "%T%\emit.exe"
call :run qir 0 "%T%\clean.bas" -o "%T%\named.qir"
call :exists "%T%\named.qir"

rem ---- -s: list, read one, write one (debuginfo has no effect), reject others ----
call :run settings 0 -s
call :has settings "Settings file:"
call :has settings "ignorewarnings"
call :run settings_one 0 -s:exewithsource
call :has settings_one "exewithsource = "
rem quoted: call takes = for a separator
call :run settings_set 0 "-s:debuginfo=false"
call :has settings_set "debuginfo = false"
call :hasnot settings_set "Usage:"
call :run settings_bad 1 -s:no-such-setting
call :has settings_bad "Invalid settings switch"

if "%FAIL%"=="0" (
    echo CLI_OPTIONS_SMOKE_OK
    rmdir /s /q "%T%" >nul 2>&1
    exit /b 0
)
echo CLI_OPTIONS_SMOKE_FAIL
echo Outputs are in "%T%"
exit /b 1

rem :run name expected-exit args... -> %T%\name.txt; expected exit 1 means "not 0"
:run
set "RUN_NAME=%~1"
set "RUN_WANT=%~2"
shift
shift
set "RUN_ARGS="
:run_args
if "%~1"=="" goto run_go
set RUN_ARGS=%RUN_ARGS% %1
shift
goto run_args
:run_go
"%QB%"%RUN_ARGS% > "%T%\%RUN_NAME%.txt" 2>&1
set "RUN_EC=%ERRORLEVEL%"
if "%RUN_WANT%"=="0" if not "%RUN_EC%"=="0" (
    echo [FAIL] qb%RUN_ARGS% exited with %RUN_EC%, expected 0
    set "FAIL=1"
)
if not "%RUN_WANT%"=="0" if "%RUN_EC%"=="0" (
    echo [FAIL] qb%RUN_ARGS% exited with 0, expected a failure
    set "FAIL=1"
)
exit /b 0

:has
findstr /L /C:"%~2" "%T%\%~1.txt" >nul || (
    echo [FAIL] %~1: missing "%~2"
    set "FAIL=1"
)
exit /b 0

:hasnot
findstr /L /C:"%~2" "%T%\%~1.txt" >nul && (
    echo [FAIL] %~1: should not contain "%~2"
    set "FAIL=1"
)
exit /b 0

:exists
if not exist "%~1" (
    echo [FAIL] expected file "%~1"
    set "FAIL=1"
)
exit /b 0

:missing
if exist "%~1" (
    echo [FAIL] unexpected file "%~1"
    set "FAIL=1"
)
exit /b 0
