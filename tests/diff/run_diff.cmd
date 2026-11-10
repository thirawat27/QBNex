@echo off
rem Builds and runs the legacy-vs-Rust differential tests on Windows (MinGW in
rem internal\c\c_compiler) against crates\qb-runtime\os\windows\libqb_runtime.a.
rem Usage: tests\diff\run_diff.cmd [--quick]
setlocal
cd /d "%~dp0"
set "CXX=..\..\internal\c\c_compiler\bin\g++"
set "LIBS=..\..\crates\qb-runtime\os\windows\libqb_runtime.a -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -luser32 -lgdi32 -lole32 -loleaut32 -lshell32 -limm32 -ldwmapi -luxtheme -lcomctl32 -ladvapi32 -lxinput -static-libgcc -static-libstdc++"
set "OUT=%TEMP%\qb_diff_%RANDOM%"
mkdir "%OUT%" >nul 2>&1
rem host_stub.cpp: the host symbols libqb_runtime.a imports from libqb.cpp (diff_qbs defines its own).
for %%t in (diff_msbin diff_round diff_val diff_str diff_timer diff_mem diff_files diff_sys diff_gfx diff_audio diff_keymap diff_keybuf diff_layout diff_systime diff_net diff_screen diff_printer) do (
    "%CXX%" -O2 -std=c++17 -w %%t.cpp host_stub.cpp %LIBS% -o "%OUT%\%%t.exe" || exit /b 1
    "%OUT%\%%t.exe" %1 || exit /b 1
)
"%CXX%" -O2 -std=c++17 -w diff_qbs.cpp %LIBS% -o "%OUT%\diff_qbs.exe" || exit /b 1
"%OUT%\diff_qbs.exe" %1 || exit /b 1

rem R10: _DEFLATE/_INFLATE vs the system zlib they linked before (MinGW libz.a, reference only).
"%CXX%" -O2 -std=c++17 -w diff_zlib.cpp host_stub.cpp %LIBS% -l:libz.a -o "%OUT%\diff_zlib.exe" || exit /b 1
"%OUT%\diff_zlib.exe" %1 || exit /b 1

rem R11d: software x87 float (crates/qb-float80) vs real long double. Own staticlib with
rem the diff-hooks feature; the oracle is MinGW libm, __mingw_snprintf and __mingw_strtold.
cargo rustc -q -p qb-float80 --release --target x86_64-pc-windows-gnu --features diff-hooks --crate-type staticlib || exit /b 1
"%CXX%" -O2 -std=c++17 -w diff_float80.cpp ..\..\target\x86_64-pc-windows-gnu\release\libqb_float80.a -lntdll -luserenv -lws2_32 -o "%OUT%\diff_float80.exe" || exit /b 1
"%OUT%\diff_float80.exe" %1 || exit /b 1

rem R7: FreeType, stb_image and dr_pcx were deleted from the tree; the references are
rem taken from git history (the last commit that still had them).
set "LEGACY_REV=4aa6f1f037277585c381fda9c956e1ddcc411033"
set "LEG=%OUT%\legacy"
mkdir "%LEG%" >nul 2>&1
set "HAVE_LEGACY=1"
for %%f in (image/stb_image.h image/dr_pcx.h font/ttf/src/freetypeamalgam.c font/ttf/src/freetypeamalgam.h) do (
    git show %LEGACY_REV%:internal/c/parts/video/%%f > "%LEG%\%%~nxf" 2>nul || set "HAVE_LEGACY="
)
if not defined HAVE_LEGACY echo diff_image/diff_font skipped: legacy sources not in git history, shallow clone?
rem diff_font runs on the committed OFL test fonts plus common Windows fonts when present
rem (those are not redistributed). No parentheses in comments inside the block below.
if defined HAVE_LEGACY (
    "%CXX%" -O2 -std=c++17 -w -I"%LEG%" diff_image.cpp host_stub.cpp %LIBS% -o "%OUT%\diff_image.exe" || exit /b 1
    "%OUT%\diff_image.exe" data\images %1 || exit /b 1
    "%CXX%" -O1 -c -w "%LEG%\freetypeamalgam.c" -o "%OUT%\freetype.o" || exit /b 1
    "%CXX%" -O2 -std=c++17 -w -I"%LEG%" diff_font.cpp "%OUT%\freetype.o" host_stub.cpp %LIBS% -o "%OUT%\diff_font.exe" || exit /b 1
    set "FONTS=data\fonts\NotoSans-Subset.ttf data\fonts\NotoSansSymbols2-Subset.ttf data\fonts\NotoSansThai-Regular.ttf"
    for %%f in (arial.ttf cour.ttf lucon.ttf times.ttf tahoma.ttf verdana.ttf consola.ttf segoeui.ttf) do (
        if exist "%WINDIR%\Fonts\%%f" call set "FONTS=%%FONTS%% %WINDIR%\Fonts\%%f"
    )
    call "%OUT%\diff_font.exe" %1 %%FONTS%% || exit /b 1
)
rd /s /q "%OUT%" >nul 2>&1
endlocal
