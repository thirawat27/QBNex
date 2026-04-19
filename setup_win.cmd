@rem This batch script has been updated to download and get the latest copy of mingw binaries from:
@rem https://github.com/niXman/mingw-builds-binaries/releases
@rem So the filenames in 'url' variable should be updated to the latest stable builds as and when they are available
@rem
@rem This also grabs a copy of 7-Zip command line extraction utility from https://www.7-zip.org/a/7zr.exe
@rem to extact the 7z mingw binary archive
@rem
@rem Both files are downloaded using 'curl'. Once downloaded, the archive is extracted to the correct location
@rem and then both the archive and 7zr.exe are deleted
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

set "MINGW_STAGE="
set "ARCHIVE_FILE=%CD%\temp.7z"
set "SEVENZIP_EXE=%CD%\7zr.exe"

rem Ensure the compiler temp staging folder exists before cleaning/copying files into it
if not exist internal\temp mkdir internal\temp

del /q /s internal\c\libqb\*.o >nul 2>nul
del /q /s internal\c\libqb\*.a >nul 2>nul
del /q /s internal\c\parts\*.o >nul 2>nul
del /q /s internal\c\parts\*.a >nul 2>nul
del /q /s internal\temp\*.* >nul 2>nul
if exist "%ARCHIVE_FILE%" del /q "%ARCHIVE_FILE%" >nul 2>nul
if exist "%SEVENZIP_EXE%" del /q "%SEVENZIP_EXE%" >nul 2>nul
if exist "mingw32" rd /s /q "mingw32" >nul 2>nul
if exist "mingw64" rd /s /q "mingw64" >nul 2>nul

rem Check if the C++ compiler is there and skip downloading if it exists
if exist internal\c\c_compiler\bin\c++.exe goto skipccompsetup

rem Create the c_compiler directory that should contain the MINGW binaries
mkdir internal\c\c_compiler

rem Check the processor type and then set the MINGW variable to correct MINGW filename

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
set url="https://github.com/niXman/mingw-builds-binaries/releases/download/12.2.0-rt_v10-rev0/i686-12.2.0-release-win32-sjlj-rt_v10-rev0.7z"
set MINGW=mingw32
set "MINGW_STAGE=%CD%\mingw32"
goto chosen

:chose64
set url="https://github.com/niXman/mingw-builds-binaries/releases/download/12.2.0-rt_v10-rev0/x86_64-12.2.0-release-win32-seh-rt_v10-rev0.7z"
set MINGW=mingw64
set "MINGW_STAGE=%CD%\mingw64"
goto chosen

:chosen

echo Downloading %url%...
curl -L %url% -o "%ARCHIVE_FILE%"

echo Downloading 7zr.exe...
curl -L https://www.7-zip.org/a/7zr.exe -o "%SEVENZIP_EXE%"

echo Extracting C++ Compiler...
"%SEVENZIP_EXE%" x "%ARCHIVE_FILE%" -y

echo Moving C++ compiler...
for /f %%a in ('dir /b "%MINGW%"') do move /y "%MINGW%\%%a" "internal\c\c_compiler\" >nul

echo Cleaning up..
if defined MINGW_STAGE if exist "%MINGW_STAGE%" rd /s /q "%MINGW_STAGE%" >nul 2>nul
if exist "%SEVENZIP_EXE%" del /q "%SEVENZIP_EXE%" >nul 2>nul
if exist "%ARCHIVE_FILE%" del /q "%ARCHIVE_FILE%" >nul 2>nul
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

if exist qb-stage0.exe (
    if /I "%QBNEX_BOOTSTRAP%"=="1" (
        echo Bootstrapping compiler from source\qbnex.bas...
        qb-stage0.exe source\qbnex.bas -o qb.exe
    ) else if not exist qb.exe (
        echo Bootstrapping compiler from source\qbnex.bas...
        qb-stage0.exe source\qbnex.bas -o qb.exe
    )
)

echo.
if exist qb.exe (
    if not defined QBNEX_KEEP_STAGE0 del qb-stage0.exe >nul 2>nul
    echo QBNex CLI compiler is ready:
    echo   qb yourfile.bas
) else (
    echo Final self-hosted compiler build failed.
    exit /b 1
)
if not defined QBNEX_CI pause

:end
endlocal
