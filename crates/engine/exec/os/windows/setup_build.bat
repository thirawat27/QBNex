@echo off
rem Builds the QIR runner (qb-exec) used by `qb` and copies it to
rem crates\engine\exec\bin\windows\qb-exec.exe, for the windows-gnu Rust target,
rem so the executable has no MSVC runtime dependency. scripts\setup_win.cmd is the
rem usual way to build the runner.
rem Requires cargo/rustup on PATH. End users get the prebuilt runner in release bundles.
setlocal
cd /d "%~dp0"
set "QB_EXEC_TARGET=x86_64-pc-windows-gnu"
where rustup >nul 2>nul && rustup target add %QB_EXEC_TARGET% >nul 2>nul
cargo build --release --locked --manifest-path ..\..\Cargo.toml --target %QB_EXEC_TARGET% --target-dir ..\..\..\..\..\target
if errorlevel 1 exit /b 1
if not exist ..\..\bin\windows mkdir ..\..\bin\windows
copy /y ..\..\..\..\..\target\%QB_EXEC_TARGET%\release\qb-exec.exe ..\..\bin\windows\qb-exec.exe >nul
if errorlevel 1 exit /b 1
endlocal
