@echo off
rem Builds the Rust runtime (qbrt) as a static library for the MinGW toolchain
rem in internal\c\c_compiler and copies it to this directory as libqbrt.a.
rem Requires cargo/rustup on PATH. End users get the prebuilt libqbrt.a in release bundles.
setlocal
set "QBRT_TARGET=x86_64-pc-windows-gnu"
for /f "delims=" %%i in ('..\..\..\c_compiler\bin\g++ -dumpmachine 2^>nul') do set "QBRT_MACHINE=%%i"
echo %QBRT_MACHINE% | findstr /i "i686" >nul && set "QBRT_TARGET=i686-pc-windows-gnu"
where rustup >nul 2>nul && rustup target add %QBRT_TARGET% >nul 2>nul
cargo build --release --locked --manifest-path ..\..\Cargo.toml --target %QBRT_TARGET% --target-dir ..\..\..\..\..\target
if errorlevel 1 exit /b 1
copy /y ..\..\..\..\..\target\%QBRT_TARGET%\release\libqbrt.a libqbrt.a >nul
if errorlevel 1 exit /b 1
echo %QBRT_TARGET% %DATE% %TIME% %RANDOM%> libqbrt.stamp
endlocal
