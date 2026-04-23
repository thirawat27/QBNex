@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC=%ROOT%fixtures\audio_synth_success.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this smoke test again.
    exit /b 2
)

if not exist "%SRC%" (
    echo [FAIL] Audio fixture source not found at "%SRC%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_audio_smoke_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "OUT_ZMODE=%TMPDIR%\audio_zmode.txt"
set "OUT_LINK=%TMPDIR%\audio_link.txt"
set "AUDIO_EXE=%TMPDIR%\audio_synth.exe"
set "SIMPLE_SRC=%TMPDIR%\simple_beep.bas"
set "SIMPLE_EXE=%TMPDIR%\simple_beep.exe"
set "OUT_SIMPLE=%TMPDIR%\simple_beep.txt"
set "TEXTUI_SRC=%TMPDIR%\text_ui.bas"
set "TEXTUI_EXE=%TMPDIR%\text_ui.exe"
set "OUT_TEXTUI=%TMPDIR%\text_ui.txt"

>"%SIMPLE_SRC%" echo PRINT "simple"
>>"%SIMPLE_SRC%" echo BEEP
>"%TEXTUI_SRC%" echo CLS
>>"%TEXTUI_SRC%" echo COLOR 14, 1
>>"%TEXTUI_SRC%" echo LOCATE 2, 4
>>"%TEXTUI_SRC%" echo PRINT "text ui"
>>"%TEXTUI_SRC%" echo BEEP

"%QB%" "%SRC%" -z > "%OUT_ZMODE%" 2>&1
set "EC_ZMODE=%ERRORLEVEL%"
"%QB%" "%SRC%" -o "%AUDIO_EXE%" > "%OUT_LINK%" 2>&1
set "EC_LINK=%ERRORLEVEL%"
"%QB%" "%SIMPLE_SRC%" -o "%SIMPLE_EXE%" > "%OUT_SIMPLE%" 2>&1
set "EC_SIMPLE=%ERRORLEVEL%"
"%QB%" "%TEXTUI_SRC%" -o "%TEXTUI_EXE%" > "%OUT_TEXTUI%" 2>&1
set "EC_TEXTUI=%ERRORLEVEL%"

if not "%EC_ZMODE%"=="0" (
    echo [FAIL] Audio synth fixture should compile in -z mode.
    echo AUDIO_SMOKE_FAIL
    echo Inspect output: "%OUT_ZMODE%"
    exit /b 1
)

if not "%EC_LINK%"=="0" (
    echo [FAIL] Audio synth fixture should link as an executable.
    echo AUDIO_SMOKE_FAIL
    echo Inspect output: "%OUT_LINK%"
    exit /b 1
)

if not "%EC_SIMPLE%"=="0" (
    echo [FAIL] Simple PRINT+BEEP program should link without the audio runtime.
    echo AUDIO_SMOKE_FAIL
    echo Inspect output: "%OUT_SIMPLE%"
    exit /b 1
)

if not "%EC_TEXTUI%"=="0" (
    echo [FAIL] Text UI program should link without GUI/audio runtime.
    echo AUDIO_SMOKE_FAIL
    echo Inspect output: "%OUT_TEXTUI%"
    exit /b 1
)

for %%F in ("%AUDIO_EXE%") do set "AUDIO_SIZE=%%~zF"
for %%F in ("%SIMPLE_EXE%") do set "SIMPLE_SIZE=%%~zF"
for %%F in ("%TEXTUI_EXE%") do set "TEXTUI_SIZE=%%~zF"
if %SIMPLE_SIZE% GEQ %AUDIO_SIZE% (
    echo [FAIL] Simple PRINT+BEEP executable should stay smaller than audio synth executable.
    echo AUDIO_SMOKE_FAIL
    echo simple=%SIMPLE_SIZE% audio=%AUDIO_SIZE%
    exit /b 1
)

if %TEXTUI_SIZE% GEQ %AUDIO_SIZE% (
    echo [FAIL] Text UI executable should stay smaller than audio synth executable.
    echo AUDIO_SMOKE_FAIL
    echo textui=%TEXTUI_SIZE% audio=%AUDIO_SIZE%
    exit /b 1
)

echo AUDIO_SMOKE_OK
rmdir /s /q "%TMPDIR%" >nul 2>&1
exit /b 0
