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
call :has examples "qb run hello.bas"
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

rem ---- without -o the executable goes next to the source; --run starts it ----
del /q "%T%\clean.exe"
call :run beside 0 "%T%\clean.bas" -q
call :exists "%T%\clean.exe"
call :run run_after 0 "%T%\clean.bas" --run -q -o "%T%\run_after.exe"
call :has run_after "CLEAN_OK"

rem ---- qb run, program arguments after --, -wq, suggestions for typos ----
> "%T%\args.bas" echo PRINT "ARGS=[" + COMMAND$ + "]"
call :run run_sub 0 run "%T%\clean.bas" -q
call :has run_sub "CLEAN_OK"
call :run run_args 0 run "%T%\args.bas" -q -- one two
call :has run_args "ARGS=[one two]"
rem cmd /c must not replace %NAME% in an argument. %%%% is one % here: the batch
rem file halves it, and call halves it again
call :run run_pct 0 run "%T%\args.bas" -q -- "%%%%PATH%%%%" "100%%%%"
call :has run_pct "ARGS=[%%%%PATH%%%% 100%%%%]"
call :run joined 0 "%T%\warn.bas" -wq -o "%T%\joined.exe"
call :has joined "warning: unused variable"
call :hasnot joined "Build complete"
call :run typo_switch 1 "%T%\clean.bas" --werrror
call :has typo_switch "Did you mean -Werror?"
call :run typo_file 1 "%T%\clen.bas"
call :has typo_file "Did you mean"
call :has typo_file "clean.bas"
call :run hint 0 "%T%\clean.bas" -m
call :has hint "Run it:"
rem -o takes no switch for a name, and never the source file itself
call :run o_switch 1 "%T%\clean.bas" -o -q
call :has o_switch "is a switch"
call :run o_source 1 "%T%\clean.bas" -o "%T%\clean.bas"
call :has o_source "it is the source file"
call :missing "%T%\clean.bas.exe"

rem ---- paths: a relative name is relative to the folder qb was started in, and what a
rem source names ($INCLUDE, $EXEICON, DECLARE DYNAMIC LIBRARY) is next to it ----
mkdir "%T%\src\lib" >nul 2>&1
> "%T%\src\once.bi" echo '$INCLUDEONCE
>> "%T%\src\once.bi" echo DIM SHARED onceArr(3) AS LONG
> "%T%\src\once.bas" echo '$INCLUDE:'once.bi'
>> "%T%\src\once.bas" echo '$INCLUDE:'ONCE.BI'
>> "%T%\src\once.bas" echo PRINT "ONCE_OK"; UBOUND(onceArr)
call :run include_once 0 "%T%\src\once.bas" -q -o "%T%\once.exe"
"%T%\once.exe" < NUL > "%T%\once_run.txt" 2>&1
call :has once_run "ONCE_OK"
rem not QBNex's own README.md, from the folder qb works in (started in %T%)
> "%T%\src\readme.bas" echo '$INCLUDE:'README.md'
pushd "%T%"
call :run include_missing 1 "%T%\src\readme.bas" -q -o "%T%\readme.exe"
popd
call :has include_missing "File README.md not found (looked in"
call :hasnot include_missing "Metacommands must start with"
> "%T%\src\app.ico" echo not an icon: $EXEICON is accepted without effect
> "%T%\src\icon.bas" echo $EXEICON:'app.ico'
>> "%T%\src\icon.bas" echo PRINT "ICON_OK"
call :run exeicon 0 "%T%\src\icon.bas" -q -o "%T%\icon.exe"
copy /y "%SystemRoot%\System32\version.dll" "%T%\src\lib\mylib.dll" >nul
> "%T%\src\dynlib.bas" echo DECLARE DYNAMIC LIBRARY "lib\mylib"
>> "%T%\src\dynlib.bas" echo FUNCTION GetFileVersionInfoSizeA^& (BYVAL f AS _OFFSET, BYVAL h AS _OFFSET)
>> "%T%\src\dynlib.bas" echo END DECLARE
>> "%T%\src\dynlib.bas" echo PRINT "LIB_OK"; GetFileVersionInfoSizeA^&(0, 0)
call :run dynlib 0 "%T%\src\dynlib.bas" -q -o "%T%\dynlib.exe"
"%T%\dynlib.exe" < NUL > "%T%\dynlib_run.txt" 2>&1
call :has dynlib_run "LIB_OK"
> "%T%\src\nolib.bas" echo DECLARE DYNAMIC LIBRARY "lib\nolib"
>> "%T%\src\nolib.bas" echo FUNCTION NoLibFn^& ()
>> "%T%\src\nolib.bas" echo END DECLARE
call :run nolib 1 "%T%\src\nolib.bas" -q -o "%T%\nolib.exe"
call :has nolib "there is no nolib.dll in"
rem a relative source path is not looked for in the QBNex folder
pushd "%T%"
call :run not_in_qbnex 1 compiler\qbnex.bas -q
call :has not_in_qbnex "Cannot find the source file"
popd
rem the stale-output warning names the file -o writes: app.v2 is app.v2.exe
call :run stale_ok 0 "%T%\clean.bas" -q -o "%T%\app.v2"
call :exists "%T%\app.v2.exe"
call :run stale 1 "%T%\err.bas" -q -o "%T%\app.v2"
call :has stale "Existing executable may be stale"
call :has stale "app.v2.exe"
rem an output that cannot be written is said as such (cmd holds it open while qb runs)
call :run locked 1 "%T%\clean.bas" -q -o "%T%\app.v2" 9>>"%T%\app.v2.exe"
call :has locked "Cannot write"
call :hasnot locked "internal"
mkdir "%T%\folderout.exe" >nul 2>&1
> "%T%\folderout.bas" echo PRINT 1
call :run folderout 1 "%T%\folderout.bas" -q
call :has folderout "it is a folder"

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
call :run settings_one 0 -s:ignorewarnings
call :has settings_one "ignorewarnings = "
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
