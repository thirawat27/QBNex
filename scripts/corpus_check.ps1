<#
.SYNOPSIS
  Compiles and briefly runs every program of the local corpus and the examples,
  and reports the ones that fail to compile, crash or hit an internal error.

.DESCRIPTION
  tests/corpus-local holds real programs that are not committed (see its
  README); examples/ is the committed sample set. Each .bas file is compiled
  with qb (-q); a program that compiles is then started headless
  (QBNEX_HEADLESS=1, no window, stdin empty) in its own folder and stopped after
  -Seconds. Most corpus programs are games that wait for keys, so a program
  that is still running at the end is a pass.

  Results per program:
    ok        compiled, ended with code 0 or was still running
    compile   the compiler refused it (may be a limitation: see the log)
    error     ended with a BASIC run-time error (often only missing input)
    crash     an internal error (a Rust panic: "Internal Error"), or an exit
              code that no BASIC error gives (an abort or access violation)

  The summary goes to the console and to <Out>/summary.json; every compile and
  run log is kept under <Out>. Exit code: the number of crash results (0 = none);
  with -Strict, the number of results that are not ok (CI runs the examples so).
  Runs on Windows, Linux and macOS (pwsh 7).

.EXAMPLE
  pwsh scripts/corpus_check.ps1
  pwsh scripts/corpus_check.ps1 -Seconds 0      # compile only
  pwsh scripts/corpus_check.ps1 -Dirs examples -Strict
#>
param(
    [string]$Qb = '',
    [string[]]$Dirs = @('tests/corpus-local', 'examples'),
    [int]$Seconds = 3,
    [int]$CompileTimeout = 120,
    [string]$Out = 'target/corpus-check',
    [switch]$Strict
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $repo
if (-not $Qb) { $Qb = Join-Path $repo $(if ($IsWindows) { 'qb.exe' } else { 'qb' }) }
if (-not (Test-Path $Qb)) { throw "compiler not found: $Qb (run scripts/setup_* first)" }
$Qb = (Resolve-Path $Qb).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$exe = if ($IsWindows) { '.exe' } else { '' }
$empty = Join-Path $Out 'empty-stdin.txt'
Set-Content -LiteralPath $empty -Value '' -NoNewline

# Starts `file` with `arguments` in `dir`, stdin from an empty file; waits at most
# `seconds`. Returns @{ code; timedOut; ms; out }.
function Invoke-Bounded([string]$file, [string[]]$arguments, [string]$dir, [int]$seconds, [string]$log) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($file)
    foreach ($a in $arguments) { $psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $dir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment['QBNEX_HEADLESS'] = '1'
    $psi.Environment['QBNEX_NO_ERROR_DIALOG'] = '1'
    $psi.Environment['QB_AUDIO_DEVICE'] = 'none'
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    $stdout = $p.StandardOutput.ReadToEndAsync()
    $stderr = $p.StandardError.ReadToEndAsync()
    $done = $p.WaitForExit([Math]::Max(1, $seconds) * 1000)
    if (-not $done) {
        try { $p.Kill($true) } catch { }
        $p.WaitForExit()
    }
    $sw.Stop()
    $text = $stdout.Result + $stderr.Result
    Set-Content -LiteralPath $log -Value $text
    return @{ code = if ($done) { $p.ExitCode } else { $null }; timedOut = -not $done; ms = $sw.ElapsedMilliseconds; out = $text }
}

$files = foreach ($d in $Dirs) {
    if (Test-Path $d) { Get-ChildItem $d -Recurse -File | Where-Object { $_.Extension -ieq '.bas' } }
}
$results = [System.Collections.Generic.List[object]]::new()
$i = 0
foreach ($f in $files) {
    $i++
    $rel = [System.IO.Path]::GetRelativePath($repo, $f.FullName)
    $key = ($rel -replace '[\\/:]', '_') -replace '\.bas$', ''
    $bin = Join-Path $Out ($key + $exe)
    $r = [ordered]@{ program = $rel; result = 'ok'; compile_ms = 0; run_ms = 0; exit = $null; note = '' }
    Remove-Item -LiteralPath $bin -ErrorAction SilentlyContinue
    $c = Invoke-Bounded $Qb @($f.FullName, '-q', '-o', $bin) $f.DirectoryName $CompileTimeout (Join-Path $Out "$key.compile.log")
    $r.compile_ms = $c.ms
    if ($c.timedOut -or $c.code -ne 0 -or -not (Test-Path $bin)) {
        $r.result = if ($c.out -match 'Internal Error|panicked') { 'crash' } else { 'compile' }
        $r.note = if ($c.timedOut) { "compiler still running after ${CompileTimeout}s" } else { (($c.out -split "`n") | Where-Object { $_.Trim() } | Select-Object -First 2) -join ' | ' }
    } elseif ($Seconds -gt 0) {
        $run = Invoke-Bounded $bin @() $f.DirectoryName $Seconds (Join-Path $Out "$key.run.log")
        $r.run_ms = $run.ms
        $r.exit = $run.code
        if ($run.out -match 'Internal Error|panicked at') {
            $r.result = 'crash'
            $r.note = (($run.out -split "`n") | Where-Object { $_ -match 'Internal Error|panicked' } | Select-Object -First 1)
        } elseif (-not $run.timedOut -and $run.code -ne 0) {
            # BASIC errors end with code 1 (and print the error); other codes are aborts
            # (Windows 0xC0000005, 0xC0000409, Unix signals as 128+n)
            $r.result = if ($run.code -eq 1) { 'error' } else { 'crash' }
            $r.note = (($run.out -split "`n") | Where-Object { $_.Trim() } | Select-Object -Last 1)
        }
    }
    Remove-Item -LiteralPath $bin -ErrorAction SilentlyContinue
    $results.Add([pscustomobject]$r)
    Write-Host ("[{0,-7}] {1,4}/{2} {3}" -f $r.result, $i, $files.Count, $rel)
}

$results | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $Out 'summary.json')
Write-Host ''
$results | Group-Object result | Sort-Object Name | ForEach-Object { Write-Host ("{0,-8} {1}" -f $_.Name, $_.Count) }
$bad = @($results | Where-Object { $_.result -ne 'ok' })
if ($bad) {
    Write-Host ''
    $bad | Sort-Object result, program | ForEach-Object { Write-Host ("{0,-8} {1}  {2}" -f $_.result, $_.program, $_.note) }
}
$slow = $results | Sort-Object compile_ms -Descending | Select-Object -First 5
Write-Host ''
Write-Host 'slowest compiles:'
$slow | ForEach-Object { Write-Host ("  {0,7} ms  {1}" -f $_.compile_ms, $_.program) }
exit @($results | Where-Object { if ($Strict) { $_.result -ne 'ok' } else { $_.result -eq 'crash' } }).Count
