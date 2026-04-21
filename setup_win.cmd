@rem This batch script downloads a standalone MinGW distribution from WinLibs:
@rem https://winlibs.com/
@rem
@rem The downloaded archive is extracted with PowerShell's Expand-Archive support.
@rem
@rem The archive is downloaded using 'curl', extracted to the correct location,
@rem and then the temporary archive is deleted.
@rem
@rem Copyright (c) 2022, Samuel Gomes
@rem https://github.com/a740g
@rem
@echo off

rem Enable cmd extensions and exit if not present
setlocal enableextensions
if errorlevel 1 (
    echo Error: Command Prompt extensions not available!
    goto end
)

echo QBNex Setup
echo.

rem Change to the correct drive letter
%~d0

rem Change to the correct path
cd %~dp0

call findcurl.cmd
if errorlevel 1 (
    echo Error: curl is required to download the Windows toolchain.
    goto end
)

set "ARCHIVE_STAGE=%CD%\toolchain_stage"
set "ARCHIVE_FILE=%CD%\temp.zip"

rem Ensure the compiler temp staging folder exists before cleaning/copying files into it
if not exist internal\temp mkdir internal\temp

del /q /s internal\c\libqb\*.o >nul 2>nul
del /q /s internal\c\libqb\*.a >nul 2>nul
del /q /s internal\c\parts\*.o >nul 2>nul
del /q /s internal\c\parts\*.a >nul 2>nul
del /q /s internal\temp\*.* >nul 2>nul
if exist "%ARCHIVE_FILE%" del /q "%ARCHIVE_FILE%" >nul 2>nul
if exist "%ARCHIVE_STAGE%" rd /s /q "%ARCHIVE_STAGE%" >nul 2>nul
if exist "mingw32" rd /s /q "mingw32" >nul 2>nul
if exist "mingw64" rd /s /q "mingw64" >nul 2>nul

rem Check if the C++ compiler is there and skip downloading if the runtime headers are present too
if exist internal\c\c_compiler\bin\c++.exe if exist internal\c\c_compiler\include\windows.h goto skipccompsetup
if exist internal\c\c_compiler\bin\c++.exe if exist internal\c\c_compiler\x86_64-w64-mingw32\include\windows.h goto skipccompsetup
if exist internal\c\c_compiler\bin\c++.exe if exist internal\c\c_compiler\i686-w64-mingw32\include\windows.h goto skipccompsetup

if exist internal\c\c_compiler rd /s /q internal\c\c_compiler >nul 2>nul
mkdir internal\c\c_compiler
mkdir "%ARCHIVE_STAGE%"

rem Check the processor type and then set the correct WinLibs archive URL

rem reg Query "HKLM\Hardware\Description\System\CentralProcessor\0" | find /i "x86" > NUL && set MINGW=mingw32 || set MINGW=mingw64
rem 
rem rem Set the correct file to download based on processor type
rem if "%MINGW%"=="mingw64" (
rem 	set url="https://github.com/niXman/mingw-builds-binaries/releases/download/12.2.0-rt_v10-rev0/x86_64-12.2.0-release-win32-seh-rt_v10-rev0.7z"
rem ) else (
rem 	set url="https://github.com/niXman/mingw-builds-binaries/releases/download/12.2.0-rt_v10-rev0/i686-12.2.0-release-win32-sjlj-rt_v10-rev0.7z"
rem )

reg Query "HKLM\Hardware\Description\System\CentralProcessor\0" | find /i "x86" > NUL && goto chose32 || goto choose

:choose
if /I "%QBNEX_MINGW_ARCH%"=="x86" goto chose32
if /I "%QBNEX_MINGW_ARCH%"=="x64" goto chose64
choice /c 12 /M "Use (1) 64-bit or (2) 32-bit MINGW? "
if errorlevel 2 goto chose32
if errorlevel 1 goto chose64
goto chose32

:chose32
set url="https://github.com/brechtsanders/winlibs_mingw/releases/download/15.2.0posix-14.0.0-msvcrt-r7/winlibs-i686-posix-dwarf-gcc-15.2.0-mingw-w64msvcrt-14.0.0-r7.zip"
goto chosen

:chose64
set url="https://github.com/brechtsanders/winlibs_mingw/releases/download/15.2.0posix-14.0.0-msvcrt-r7/winlibs-x86_64-posix-seh-gcc-15.2.0-mingw-w64msvcrt-14.0.0-r7.zip"
goto chosen

:chosen

echo Downloading %url%...
curl -L %url% -o "%ARCHIVE_FILE%"

if not exist "%ARCHIVE_FILE%" (
    echo Error: Failed to download toolchain archive.
    goto end
)

echo Extracting C++ Compiler...
powershell -NoProfile -Command "Expand-Archive -Path '%ARCHIVE_FILE%' -DestinationPath '%ARCHIVE_STAGE%' -Force"

set "TOOLCHAIN_ROOT="
for /f "delims=" %%A in ('dir /b /s "%ARCHIVE_STAGE%\g++.exe" ^| findstr /i /r "\\bin\\g++\.exe$"') do (
    if not defined TOOLCHAIN_ROOT (
        for %%B in ("%%~dpA..") do set "TOOLCHAIN_ROOT=%%~fB"
    )
)

if not defined TOOLCHAIN_ROOT (
    echo Error: Extracted toolchain root could not be located.
    goto end
)

echo Copying C++ compiler from "%TOOLCHAIN_ROOT%"...
xcopy /e /i /y "%TOOLCHAIN_ROOT%\*" "internal\c\c_compiler\" >nul

if not exist "internal\c\c_compiler\include\windows.h" if not exist "internal\c\c_compiler\x86_64-w64-mingw32\include\windows.h" if not exist "internal\c\c_compiler\i686-w64-mingw32\include\windows.h" (
    echo Error: Extracted toolchain is missing Windows runtime headers.
    goto end
)

echo Cleaning up..
if exist "%ARCHIVE_FILE%" del /q "%ARCHIVE_FILE%" >nul 2>nul
if exist "%ARCHIVE_STAGE%" rd /s /q "%ARCHIVE_STAGE%" >nul 2>nul
if exist "mingw32" rd /s /q "mingw32" >nul 2>nul
if exist "mingw64" rd /s /q "mingw64" >nul 2>nul

:skipccompsetup

echo Building library 'LibQB'
cd internal/c/libqb/os/win
if exist libqb_setup.o del libqb_setup.o
call setup_build.bat
cd ../../../../..

echo Building library 'FreeType'
cd internal/c/parts/video/font/ttf/os/win
if exist src.o del src.o
call setup_build.bat
cd ../../../../../../../..

echo Building library 'Core:FreeGLUT'
cd internal/c/parts/core/os/win
if exist src.a del src.a
call setup_build.bat
cd ../../../../../..

echo Building 'QBNex'
if not exist internal\temp mkdir internal\temp
copy internal\source\*.* internal\temp\ >nul
xcopy /e /i /y source\* internal\temp\ >nul
copy source\qbnex.ico internal\temp\ >nul
copy source\icon.rc internal\temp\ >nul
cd internal\c
c_compiler\bin\windres.exe -i ../temp/icon.rc -o ../temp/icon.o
c_compiler\bin\g++ -mconsole -s -Wfatal-errors -w -Wall qbx.cpp libqb\os\win\libqb_setup.o ..\temp\icon.o -D DEPENDENCY_LOADFONT  parts\video\font\ttf\os\win\src.o -D DEPENDENCY_SOCKETS -D DEPENDENCY_NO_PRINTER -D DEPENDENCY_ICON -D DEPENDENCY_NO_SCREENIMAGE parts\core\os\win\src.a -lopengl32 -lglu32 -static-libgcc -static-libstdc++ -D GLEW_STATIC -D FREEGLUT_STATIC -lws2_32 -lwinmm -lgdi32 -o "..\..\qb-stage0.exe"
cd ..\..

set "SELFHOST_EXIT="
if exist qb-stage0.exe (
    call :selfhost
) else (
    echo Stage0 compiler was not produced.
)

echo.
if exist qb.exe (
    if not defined QBNEX_KEEP_STAGE0 del qb-stage0.exe >nul 2>nul
    echo QBNex CLI compiler is ready:
    echo   qb yourfile.bas
) else (
    echo Final compiler build failed.
    if defined SELFHOST_EXIT echo Stage0 exit code: %SELFHOST_EXIT%
    for %%F in (internal\temp\compilelog.txt internal\temp\mainerr.txt internal\temp\ideerror.txt) do (
        if exist "%%F" if not "%%~zF"=="0" (
            echo.
            echo ===== %%F =====
            type "%%F"
        )
    )
    exit /b 1
)
if not defined QBNEX_CI pause

:end
endlocal

:selfhost
echo Self-hosting 'QBNex'
if exist qb.exe del /q qb.exe >nul 2>nul
qb-stage0.exe source\qbnex.bas -o qb.exe
set "SELFHOST_EXIT=%ERRORLEVEL%"
if not exist qb.exe if /I "%QBNEX_MINGW_ARCH%"=="x86" (
    echo Self-hosting did not produce qb.exe on Windows x86. Retrying once...
    qb-stage0.exe source\qbnex.bas -o qb.exe
    set "SELFHOST_EXIT=%ERRORLEVEL%"
)
exit /b 0
