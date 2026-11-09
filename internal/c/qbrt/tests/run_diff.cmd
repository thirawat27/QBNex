@echo off
rem Builds and runs the legacy-vs-Rust differential tests on Windows (MinGW in
rem internal\c\c_compiler) against internal\c\qbrt\os\win\libqbrt.a.
rem Usage: internal\c\qbrt\tests\run_diff.cmd [--quick]
setlocal
cd /d "%~dp0"
set "CXX=..\..\c_compiler\bin\g++"
set "LIBS=..\os\win\libqbrt.a -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -static-libgcc -static-libstdc++"
set "OUT=%TEMP%\qbrt_diff_%RANDOM%"
mkdir "%OUT%" >nul 2>&1
for %%t in (diff_msbin diff_round diff_val diff_str diff_timer diff_gfx) do (
    "%CXX%" -O2 -std=c++17 -w %%t.cpp %LIBS% -o "%OUT%\%%t.exe" || exit /b 1
    "%OUT%\%%t.exe" %1 || exit /b 1
)
rd /s /q "%OUT%" >nul 2>&1
endlocal
