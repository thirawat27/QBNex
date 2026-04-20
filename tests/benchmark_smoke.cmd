@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
set "REPO_ROOT=%ROOT%.."
set "QB=%REPO_ROOT%\qb.exe"
set "SRC_TINY=%ROOT%fixtures\label_recompile_success.bas"
set "SRC_WARN=%ROOT%fixtures\unused_variable_warning.bas"
set "SRC_MEDIUM=%ROOT%fixtures\benchmark_large.bas"

if not exist "%QB%" (
    echo [FAIL] qb.exe not found at "%QB%"
    echo Build QBNex first, then run this benchmark again.
    exit /b 2
)

if not exist "%SRC_TINY%" (
    echo [FAIL] Tiny benchmark fixture not found at "%SRC_TINY%"
    exit /b 2
)

if not exist "%SRC_WARN%" (
    echo [FAIL] Warning benchmark fixture not found at "%SRC_WARN%"
    exit /b 2
)

if not exist "%SRC_MEDIUM%" (
    echo [FAIL] Medium benchmark fixture not found at "%SRC_MEDIUM%"
    exit /b 2
)

set "TMPDIR=%TEMP%\qbnex_benchmark_%RANDOM%_%RANDOM%"
mkdir "%TMPDIR%" >nul 2>&1
if errorlevel 1 (
    echo [FAIL] Could not create temp directory "%TMPDIR%"
    exit /b 2
)

set "BIN_TINY=%TMPDIR%\tiny.exe"
set "BIN_WARN=%TMPDIR%\warn.exe"
set "BIN_MEDIUM=%TMPDIR%\medium.exe"
set "RESULTS=%TMPDIR%\benchmark_results.txt"

powershell -NoProfile -Command ^
  "$ErrorActionPreference = 'Stop';" ^
  "$cases = @(" ^
  "  @{ Name = 'tiny-success'; Iterations = 5; Source = $env:SRC_TINY; Output = $env:BIN_TINY; Args = @() }," ^
  "  @{ Name = 'warning-path'; Iterations = 5; Source = $env:SRC_WARN; Output = $env:BIN_WARN; Args = @('-w') }," ^
  "  @{ Name = 'medium-parse'; Iterations = 3; Source = $env:SRC_MEDIUM; Output = $env:BIN_MEDIUM; Args = @() }" ^
  ");" ^
  "$lines = @('BENCHMARK_SMOKE_OK', 'Case           Iterations  AvgMs');" ^
  "foreach ($case in $cases) {" ^
  "  $total = 0.0;" ^
  "  for ($i = 0; $i -lt $case.Iterations; $i++) {" ^
  "    Remove-Item -LiteralPath $case.Output -Force -ErrorAction SilentlyContinue;" ^
  "    $sw = [System.Diagnostics.Stopwatch]::StartNew();" ^
  "    & $env:QB $case.Source @($case.Args) -o $case.Output *> $null;" ^
  "    $exitCode = $LASTEXITCODE;" ^
  "    $sw.Stop();" ^
  "    if ($exitCode -ne 0) { throw ('benchmark case failed: ' + $case.Name + ' exit=' + $exitCode) }" ^
  "    if (-not (Test-Path -LiteralPath $case.Output)) { throw ('benchmark output missing: ' + $case.Name) }" ^
  "    $total += $sw.Elapsed.TotalMilliseconds;" ^
  "  }" ^
  "  $avg = [math]::Round($total / $case.Iterations, 2);" ^
  "  $lines += ('{0,-14} {1,10} {2,7}' -f $case.Name, $case.Iterations, $avg);" ^
  "}" ^
  "Set-Content -LiteralPath $env:RESULTS -Value $lines;"

if errorlevel 1 (
    echo [FAIL] Benchmark harness failed.
    echo Inspect results path: "%RESULTS%"
    exit /b 1
)

type "%RESULTS%"
rmdir /s /q "%TMPDIR%" >nul 2>&1
exit /b 0
