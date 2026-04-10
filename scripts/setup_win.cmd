@echo off
REM =============================================================================
REM QBNex Windows Setup Script
REM Copyright © 2026 thirawat27
REM Version: 1.0.0
REM Description: Builds and configures QBNex compiler for Windows systems
REM =============================================================================

setlocal enabledelayedexpansion

REM Configuration
set QBNEX_VERSION=1.0.0
set QBNEX_OWNER=thirawat27
set QBNEX_YEAR=2026
set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%~dp0..

REM Print banner
echo ╔═══════════════════════════════════════════════════════════╗
echo ║                  QBNex Setup for Windows                  ║
echo ║              Modern BASIC to Native Compiler              ║
echo ╚═══════════════════════════════════════════════════════════╝
echo.
echo Version: %QBNEX_VERSION%
echo Owner: %QBNEX_OWNER%
echo Year: %QBNEX_YEAR%
echo.

REM Check for MinGW
echo [?] Checking for MinGW installation...
where g++ >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [!] MinGW not found in PATH
    echo Please install MinGW-w64 and add it to your PATH
    echo Download from: https://www.mingw-w64.org/
    pause
    exit /b 1
) else (
    echo [✓] MinGW found
)

REM Create directories
echo [✓] Creating directory structure...
if not exist "%PROJECT_ROOT%\bin" mkdir "%PROJECT_ROOT%\bin"
if not exist "%PROJECT_ROOT%\tmp" mkdir "%PROJECT_ROOT%\tmp"
if not exist "%PROJECT_ROOT%\logs" mkdir "%PROJECT_ROOT%\logs"
if not exist "%PROJECT_ROOT%\cache" mkdir "%PROJECT_ROOT%\cache"

REM Build compiler
echo [✓] Building QBNex compiler...
cd /d "%PROJECT_ROOT%\internal\c"

if exist "libqb.cpp" (
    echo Compiling C++ runtime libraries...
    g++ -O2 -std=c++11 -c libqb.cpp -o libqb.o ^
        -DQB64_WINDOWS ^
        -DDEPENDENCY_CONSOLE_ONLY ^
        -I. ^
        -Iparts/core/gl_headers
    
    if %ERRORLEVEL% equ 0 (
        echo [✓] Runtime library compiled successfully
    ) else (
        echo [✗] Failed to compile runtime library
        pause
        exit /b 1
    )
) else (
    echo [!] Source directory not found, skipping build
)

cd /d "%PROJECT_ROOT%"

REM Create version file
echo From git %QBNEX_VERSION% > "%PROJECT_ROOT%\internal\version.txt"

echo.
echo ═══════════════════════════════════════════════════════════
echo QBNex setup completed successfully!
echo ═══════════════════════════════════════════════════════════
echo.
echo To use QBNex, run:
echo   bin\qb yourfile.bas
echo.
echo Or add to your PATH:
echo   set PATH=%%PATH%%;%PROJECT_ROOT%\bin
echo.
pause
