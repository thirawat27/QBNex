<#
.SYNOPSIS
  VM / JIT / native timings of five compute-heavy programs of the local corpus.

.DESCRIPTION
  The corpus demos run until a key is pressed, so each one is copied and only its
  exit is changed to a fixed amount of work (see $Programs: the edit is a plain
  text replacement, everything else is the corpus source). Each program is
  compiled once with -Qb; the QIR the compiler wrote is also linked natively
  (qb-exec build --native-link, needs the native kit: scripts\native-kit.ps1).
  Every variant runs -Runs times headless (QBNEX_HEADLESS=1, no window); the
  median wall time is reported, and the final frame (QBNEX_FB_CAPTURE hash) of
  every variant must be the same as the VM's.

  Variants: vm (QB_JIT=off), jit (default: tier up after 1000 ticks),
  eager (QB_JIT=eager: whole module compiled before the first statement),
  native (standard executable linked by rust-lld, D15).

.EXAMPLE
  pwsh scripts\bench\corpus_perf.ps1 -Qb .\qb.exe -Runs 3
#>
param(
    [string]$Qb = (Join-Path $PSScriptRoot '..\..\qb.exe'),
    [string]$Corpus = (Join-Path $PSScriptRoot '..\..\tests\corpus-local'),
    [string]$Kit = $env:QBNEX_NATIVE_KIT,
    [int]$Runs = 3,
    [string]$Out = (Join-Path $PSScriptRoot '..\..\target\bench')
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Qb = (Resolve-Path $Qb).Path
$samples = Join-Path (Resolve-Path $Corpus).Path 'qb64-samples\programs\samples'
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path

# name, corpus path, replacements (old -> new), what the fixed workload is;
# every exit becomes SYSTEM (a window program's END waits for a key)
$Programs = @(
    @{ n = 'mandelb'; p = 'misc\mandelb.bas'; work = '64000 pixels (one 320x200 screen)'
       r = @(,@('8 IF LEN(INKEY$) = 0 THEN GOTO 1', '8 bench& = bench& + 1: IF bench& < 64000 THEN GOTO 1 ELSE SYSTEM')) },
    @{ n = 'npplasma'; p = 'pete\25lines\npplasma.bas'; work = '60 frames of 320x200 POKEs'
       r = @(,@('LOOP UNTIL INKEY$ <> ""', 'bench& = bench& + 1: LOOP UNTIL bench& >= 60: SYSTEM')) },
    @{ n = 'raytra1b'; p = 'pete\raytra1b.bas'; work = '20 ray-traced frames (corpus: 200)'
       r = @(@('FOR l = 0 TO 199', 'FOR l = 0 TO 19'), @('IF LEN(INKEY$) THEN EXIT FOR', "'"), @('COLOR 255: PRINT l / (TIMER - tt!)', "'"), @('KK$ = INPUT$(1)', "'")) },
    @{ n = 'cuberot'; p = 'misc\cuberot.bas'; work = '2000 rotations of 343 points (no WAIT for the retrace)'
       r = @(@('9  WAIT &H3DA, 8', "9  '"), @('11 IF LEN(INKEY$) = 0 THEN GOTO 1', '11 bench& = bench& + 1: IF bench& < 2000 THEN GOTO 1 ELSE SYSTEM')) },
    @{ n = '3dcube25'; p = 'pete\25lines\3dcube25.bas'; work = '300 frames of a filled rotating cube (LINE, PAINT, PCOPY)'
       r = @(,@('LOOP UNTIL INKEY$ <> ""', 'bench& = bench& + 1: LOOP UNTIL bench& >= 300: SYSTEM')) }
)

function Invoke-Timed([string]$exe, [string]$cwd, [hashtable]$envs) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($exe)
    $psi.WorkingDirectory = $cwd
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    foreach ($k in $envs.Keys) { if ($null -eq $envs[$k]) { [void]$psi.Environment.Remove($k) } else { $psi.Environment[$k] = [string]$envs[$k] } }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    $o = $p.StandardOutput.ReadToEndAsync(); $e = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit(300000)) { $p.Kill($true); return @{ ms = -1; code = 'timeout'; err = '' } }
    [void]$o.Wait(); [void]$e.Wait()
    return @{ ms = $sw.ElapsedMilliseconds; code = $p.ExitCode; err = $e.Result }
}

$rows = @()
foreach ($prog in $Programs) {
    $dir = Join-Path $Out $prog.n
    New-Item -ItemType Directory -Force $dir | Out-Null
    $src = [System.IO.File]::ReadAllText((Join-Path $samples $prog.p))
    foreach ($rep in $prog.r) {
        if (-not $src.Contains($rep[0])) { throw "$($prog.n): '$($rep[0])' not found in the corpus source" }
        $src = $src.Replace($rep[0], $rep[1])
    }
    $bas = Join-Path $dir "$($prog.n).bas"
    [System.IO.File]::WriteAllText($bas, $src)
    $exe = Join-Path $dir "$($prog.n).exe"
    Push-Location $repo
    $log = & $Qb $bas -o $exe -q 2>&1
    $qir = Join-Path $repo 'target\qb-temp\program.qir'
    if (-not (Test-Path $exe)) { Pop-Location; throw "$($prog.n): compile failed: $log" }
    Copy-Item $qir (Join-Path $dir "$($prog.n).qir") -Force
    Pop-Location
    $variants = [ordered]@{ vm = @($exe, 'off'); jit = @($exe, $null); eager = @($exe, 'eager') }
    $runner = Join-Path $repo 'crates\qb-exec\bin\windows\qb-exec.exe'
    if ($Kit -and (Test-Path (Join-Path $Kit 'kit.txt'))) {
        $nexe = Join-Path $dir "$($prog.n)_native.exe"
        $env:QBNEX_NATIVE_KIT = $Kit
        & $runner build (Join-Path $dir "$($prog.n).qir") -o $nexe --native-link 2>&1 | Out-Null
        if (Test-Path $nexe) { $variants.native = @($nexe, $null) }
    }
    $row = [ordered]@{ program = $prog.n; work = $prog.work }
    $vmHash = $null
    foreach ($v in $variants.Keys) {
        $times = @()
        $hash = ''
        for ($i = 1; $i -le $Runs; $i++) {
            $fb = Join-Path $dir "$v.fb"
            if (Test-Path $fb) { Remove-Item $fb }
            # the machine is shared with large builds: a run killed by the system commit
            # limit ("memory allocation of N bytes failed") is retried, not counted
            for ($try = 1; $try -le 5; $try++) {
                $r = Invoke-Timed $variants[$v][0] $dir @{ QBNEX_HEADLESS = '1'; QB_JIT = $variants[$v][1]; QBNEX_FB_CAPTURE = $fb; QBNEX_NO_ERROR_DIALOG = '1' }
                if ($r.err -notmatch 'memory allocation of \d+ bytes failed') { break }
                Start-Sleep -Seconds 20
            }
            if ($r.code -ne 0) { throw "$($prog.n) $v exit $($r.code): $($r.err)" }
            $times += $r.ms
            if (Test-Path $fb) { $hash = (Get-FileHash $fb -Algorithm SHA256).Hash.Substring(0, 16) }
        }
        if ($v -eq 'vm') { $vmHash = $hash } elseif ($hash -ne $vmHash) { throw "$($prog.n): $v frame $hash differs from the VM's $vmHash" }
        $row[$v] = ($times | Sort-Object)[[int][math]::Floor(($times.Count - 1) / 2)]
        Write-Host ("{0,-10} {1,-6} {2,7} ms  (runs: {3})" -f $prog.n, $v, $row[$v], ($times -join ', '))
    }
    $rows += [pscustomobject]$row
}

$md = [System.Text.StringBuilder]::new()
[void]$md.AppendLine('| program | workload | VM ms | JIT ms | eager ms | native ms | JIT speed-up | native speed-up |')
[void]$md.AppendLine('|---|---|---:|---:|---:|---:|---:|---:|')
foreach ($r in $rows) {
    $nat = if ($r.PSObject.Properties['native']) { $r.native } else { '-' }
    $su = { param($x) if ($x -is [long] -or $x -is [int]) { '{0:0.00}x' -f ($r.vm / [math]::Max(1, $x)) } else { '-' } }
    [void]$md.AppendLine("| $($r.program) | $($r.work) | $($r.vm) | $($r.jit) | $($r.eager) | $nat | $(& $su $r.jit) | $(& $su $nat) |")
}
$md.ToString() | Set-Content (Join-Path $Out 'results.md')
$md.ToString()
