# Builds the QBNex native kit (D15 final): qb_native.lib (crate qb-native),
# rust-lld from the active Rust toolchain and import libraries generated from
# the system DLLs. Result: <Out>\kit.txt etc. Use it with
#   $env:QBNEX_NATIVE_KIT = <Out>   or copy <Out> to native\ next to qb-exec.exe
param(
    [string]$Out = "target\release\native"
)
$ErrorActionPreference = "Stop"
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = "1" }
cargo build --release -p qb-native -p qb-exec
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$sysroot = (rustc --print sysroot).Trim()
$host_triple = ((rustc -vV | Select-String '^host: ').Line -replace '^host: ', '').Trim()
$lld = Join-Path $sysroot "lib\rustlib\$host_triple\bin\rust-lld.exe"
if (-not (Test-Path $lld)) { throw "rust-lld not found: $lld (rustup component add llvm-tools / rust-lld ships with the toolchain)" }
New-Item -ItemType Directory -Force $Out | Out-Null
& target\release\qb-exec.exe native-kit --staticlib target\release\qb_native.lib --lld $lld -o $Out
exit $LASTEXITCODE
