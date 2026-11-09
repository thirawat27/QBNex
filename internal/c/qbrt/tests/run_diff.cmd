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
for %%t in (diff_msbin diff_round diff_val diff_str diff_timer) do (
    "%CXX%" -O2 -std=c++17 -w %%t.cpp %LIBS% -o "%OUT%\%%t.exe" || exit /b 1
    "%OUT%\%%t.exe" %1 || exit /b 1
)

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
    "%CXX%" -O2 -std=c++17 -w -I"%LEG%" diff_image.cpp %LIBS% -o "%OUT%\diff_image.exe" || exit /b 1
    "%OUT%\diff_image.exe" data\images %1 || exit /b 1
    "%CXX%" -O1 -c -w "%LEG%\freetypeamalgam.c" -o "%OUT%\freetype.o" || exit /b 1
    "%CXX%" -O2 -std=c++17 -w -I"%LEG%" diff_font.cpp "%OUT%\freetype.o" %LIBS% -o "%OUT%\diff_font.exe" || exit /b 1
    set "FONTS=data\fonts\NotoSans-Subset.ttf data\fonts\NotoSansSymbols2-Subset.ttf data\fonts\NotoSansThai-Regular.ttf"
    for %%f in (arial.ttf cour.ttf lucon.ttf times.ttf tahoma.ttf verdana.ttf consola.ttf segoeui.ttf) do (
        if exist "%WINDIR%\Fonts\%%f" call set "FONTS=%%FONTS%% %WINDIR%\Fonts\%%f"
    )
    call "%OUT%\diff_font.exe" %1 %%FONTS%% || exit /b 1
)
rd /s /q "%OUT%" >nul 2>&1
endlocal
