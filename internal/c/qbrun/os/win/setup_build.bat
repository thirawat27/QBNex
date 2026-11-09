@echo off
rem R11: builds the QIR runner (qbrun) used by `qb --ir` and copies it to
rem internal\c\qbrun\bin\win\qbrun.exe. Same Rust target as qbrt (MinGW ABI),
rem so the executable has no MSVC runtime dependency.
rem Requires cargo/rustup on PATH. End users get the prebuilt runner in release bundles.
setlocal
set "QBRUN_TARGET=x86_64-pc-windows-gnu"
for /f "delims=" %%i in ('..\..\..\c_compiler\bin\g++ -dumpmachine 2^>nul') do set "QBRUN_MACHINE=%%i"
echo %QBRUN_MACHINE% | findstr /i "i686" >nul && set "QBRUN_TARGET=i686-pc-windows-gnu"
where rustup >nul 2>nul && rustup target add %QBRUN_TARGET% >nul 2>nul
cargo build --release --locked --manifest-path ..\..\Cargo.toml --target %QBRUN_TARGET% --target-dir ..\..\..\..\..\target
if errorlevel 1 exit /b 1
if not exist ..\..\bin\win mkdir ..\..\bin\win
copy /y ..\..\..\..\..\target\%QBRUN_TARGET%\release\qbrun.exe ..\..\bin\win\qbrun.exe >nul
if errorlevel 1 exit /b 1
endlocal
