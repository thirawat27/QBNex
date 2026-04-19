@echo off
setlocal enabledelayedexpansion

rem Check if curl is already installed in the system
echo Checking for curl in the system...
curl --version >nul 2>&1

if %ERRORLEVEL% EQU 0 (
    echo [OK] Found curl, continuing.
    exit /b 0
)

echo [INFO] curl not found. Proceeding with installation...

rem Check system architecture (32-bit or 64-bit) to download the correct version
reg Query "HKLM\Hardware\Description\System\CentralProcessor\0" | find /i "x86" > NUL && set ARCH=win32|| set ARCH=win64

rem Set download link to the "latest" version from the official website
if "%ARCH%"=="win64" (
    set CURL_URL=https://curl.se/windows/latest.cgi?p=win64-mingw.zip
) else (
    set CURL_URL=https://curl.se/windows/latest.cgi?p=win32-mingw.zip
)

rem Set working directory (relative to the script's location %~dp0)
set WORK_DIR=%~dp0internal
set DOWNLOAD_DEST=%WORK_DIR%\curl_latest.zip
set EXTRACT_DEST=%WORK_DIR%\curl

rem Create directory (with error handling)
if not exist "%WORK_DIR%" (
    mkdir "%WORK_DIR%" >NUL
    if !ERRORLEVEL! NEQ 0 (
        echo [ERROR] Failed to create directory: "%WORK_DIR%"
        exit /b 1
    )
)

rem Use PowerShell to download the file synchronously (wait until complete)
echo [INFO] Fetching latest curl from %CURL_URL% ...
powershell -Command "(New-Object Net.WebClient).DownloadFile('%CURL_URL%', '%DOWNLOAD_DEST%')"

if not exist "%DOWNLOAD_DEST%" (
    echo [ERROR] Failed to download curl.
    exit /b 1
)

rem Extract Zip file using PowerShell (not using expand.exe because we downloaded a .zip instead of .cab)
echo [INFO] Extracting files...
if exist "%EXTRACT_DEST%" rmdir /s /q "%EXTRACT_DEST%"
powershell -Command "Expand-Archive -Path '%DOWNLOAD_DEST%' -DestinationPath '%EXTRACT_DEST%' -Force"

rem Delete the zip file to save space
del /q "%DOWNLOAD_DEST%"

rem Find the actual location of curl.exe in the extracted folder (since the subfolder name changes by version)
set CURL_EXE_DIR=
for /f "delims=" %%A in ('dir /b /s "%EXTRACT_DEST%\curl.exe"') do (
    set CURL_EXE_DIR=%%~dpA
)

if "%CURL_EXE_DIR%"=="" (
    echo [ERROR] Could not find curl.exe after extraction.
    exit /b 1
)

rem Add temporary path to make it usable immediately in this session
set PATH=%PATH%;%CURL_EXE_DIR%
echo [SUCCESS] curl has been installed and added to PATH temporarily.
echo [INFO] Path: %CURL_EXE_DIR%

rem Test the execution
curl --version | findstr /i "curl"