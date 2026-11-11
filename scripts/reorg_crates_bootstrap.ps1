#Requires -Version 7
<#
.SYNOPSIS
    Rebuilds bootstrap/qbnex.qir until it is a fixpoint (Windows).

.DESCRIPTION
    Run after scripts/reorg_crates.ps1: the compiler holds the runner path
    (crates\engine\exec\bin\...) and the runtime path as string constants, so
    the bootstrap QIR has to come from the new sources.

    Each round:
      1. scripts\setup_win.cmd: builds qb-exec, packs bootstrap\qbnex.qir into
         qb-stage0.exe, which compiles compiler\qbnex.bas into qb.exe;
      2. qb.exe compiles compiler\qbnex.bas again; its QIR is
         target\qb-temp*\program.qir;
      3. that QIR equal to bootstrap\qbnex.qir: fixpoint, done. Else it
         replaces bootstrap\qbnex.qir and the next round starts.

    A step that fails is retried (low-memory builds on this machine sometimes die
    with STATUS_STACK_BUFFER_OVERRUN). Uses CARGO_BUILD_JOBS=2 unless set.

.PARAMETER MaxRounds
    Rounds before giving up (default 4; two are normally enough).

.PARAMETER Retries
    Attempts per step (default 3).
#>
[CmdletBinding()]
param(
    [int]$MaxRounds = 4,
    [int]$Retries = 3
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $IsWindows) {
    throw 'Windows only (setup_win.cmd). On Linux/macOS run the same loop with scripts/setup_lnx.sh or scripts/setup_osx.command.'
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root

if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
$env:QBNEX_NO_ERROR_DIALOG = '1'
$env:QBNEX_HEADLESS = '1'
$env:QBNEX_CI = '1'

$Bootstrap = Join-Path $Root 'bootstrap\qbnex.qir'
$OutDir = Join-Path $Root 'target\reorg-bootstrap'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Invoke-Step([string]$name, [scriptblock]$step, [scriptblock]$ok) {
    for ($i = 1; $i -le $Retries; $i++) {
        Write-Host "-- $name (attempt $i/$Retries)"
        & $step
        $code = $LASTEXITCODE
        if ($code -eq 0 -and (& $ok)) { return }
        Write-Warning ("{0} failed (exit code {1}, 0x{1:X8})." -f $name, $code)
    }
    throw "$name failed $Retries times."
}

for ($round = 1; $round -le $MaxRounds; $round++) {
    Write-Host "`n== Round $round"
    $before = (Get-FileHash $Bootstrap -Algorithm SHA256).Hash

    Invoke-Step 'setup_win.cmd' { cmd /c scripts\setup_win.cmd } { Test-Path (Join-Path $Root 'qb.exe') }

    $start = Get-Date
    $exe = Join-Path $OutDir 'qbnex-check.exe'
    Invoke-Step 'qb.exe compiler\qbnex.bas' { & (Join-Path $Root 'qb.exe') compiler\qbnex.bas -o $exe } {
        $script:qir = Get-ChildItem (Join-Path $Root 'target') -Directory -Filter 'qb-temp*' |
            ForEach-Object { Get-Item -LiteralPath (Join-Path $_.FullName 'program.qir') -ErrorAction SilentlyContinue } |
            Where-Object { $_ -and $_.LastWriteTime -ge $start } |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        [bool]$script:qir
    }

    $after = (Get-FileHash $qir.FullName -Algorithm SHA256).Hash
    if ($after -eq $before) {
        Write-Host "`nFixpoint after round ${round}: bootstrap\qbnex.qir is what qb.exe produces from compiler\qbnex.bas ($after)."
        exit 0
    }
    Copy-Item -LiteralPath $qir.FullName -Destination $Bootstrap -Force
    Write-Host "bootstrap\qbnex.qir replaced from $($qir.FullName) ($before -> $after)"
}
throw "No fixpoint after $MaxRounds rounds."
