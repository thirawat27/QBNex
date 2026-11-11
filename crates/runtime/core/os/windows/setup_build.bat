@echo off
rem Builds the Rust runtime (qb-runtime) as a static library for the MinGW toolchain
rem in internal\c\c_compiler and copies it to this directory as libqb_runtime.a.
rem Requires cargo/rustup on PATH. End users get the prebuilt libqb_runtime.a in release bundles.
setlocal
cd /d "%~dp0"
set "QB_RUNTIME_TARGET=x86_64-pc-windows-gnu"
for /f "delims=" %%i in ('..\..\..\..\..\internal\c\c_compiler\bin\g++ -dumpmachine 2^>nul') do set "QB_RUNTIME_MACHINE=%%i"
echo %QB_RUNTIME_MACHINE% | findstr /i "i686" >nul && set "QB_RUNTIME_TARGET=i686-pc-windows-gnu"
where rustup >nul 2>nul && rustup target add %QB_RUNTIME_TARGET% >nul 2>nul
cargo build --release --locked --manifest-path ..\..\Cargo.toml --target %QB_RUNTIME_TARGET% --target-dir ..\..\..\..\..\target
if errorlevel 1 exit /b 1
copy /y ..\..\..\..\..\target\%QB_RUNTIME_TARGET%\release\libqb_runtime.a libqb_runtime.a >nul
if errorlevel 1 exit /b 1
echo %QB_RUNTIME_TARGET% %DATE% %TIME% %RANDOM%> qb_runtime.stamp
endlocal
