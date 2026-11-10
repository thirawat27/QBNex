#!/usr/bin/env pwsh
# Compiles every BASIC code block of the user documentation with the real
# compiler and checks the output where the docs give one.
#
#   pwsh tests/docs/check_docs.ps1 [-Compiler path\to\qb.exe] [-Backend cpp|ir]
#                                  [-Files a.md,b.md] [-Match text] [-List] [-Keep] [-CheckOnly]
#
# A block is a fence tagged ```basic (also ```qbasic / ```bas). An HTML comment
# on the line(s) directly above the fence controls it:
#
#   (no marker)                          compile only; must compile
#   <!-- docs-test: skip <reason> -->    not compiled (fragments, needs a GPU, ...)
#   <!-- docs-test: compile-error -->    must NOT compile (planned syntax); when it
#                                        starts compiling the doc is out of date
#   <!-- docs-test: run -->              compile, run, exit code 0
#   <!-- docs-test: output -->           compile, run, and compare stdout with the
#                                        next ```text block after the code block
#   <!-- docs-test: no-ir <reason> -->   with -Backend ir: must NOT compile yet (a QIR gap);
#                                        when it compiles the marker is out of date
#   <!-- docs-test: prepend NAME -->     prefix the block with the snippet NAME
#                                        defined by <!-- docs-test: define NAME --> above
#                                        another block in the same file
#
# Programs run headless (QBNEX_HEADLESS=1, QBNEX_NO_ERROR_DIALOG=1, empty stdin,
# 30 s timeout) in a scratch folder; `$CONSOLE:ONLY` is added when the block
# has no `$CONSOLE` line so PRINT reaches stdout. Identical blocks (the Thai and
# English pages share their examples) are compiled once.
#
# -CheckOnly runs the front end only (`qb -z`: parse and generate C++, no
# link, nothing run): fast, and usable while the runtime library is being rebuilt.
param(
    [string]$Compiler = $env:QB_COMPILER,
    [ValidateSet('cpp', 'ir')][string]$Backend = 'cpp',
    [string[]]$Files,
    [string]$Match,
    [switch]$List,
    [switch]$Keep,
    [switch]$CheckOnly
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Files = @($Files | ForEach-Object { $_ -split ',' } | Where-Object { $_ } | ForEach-Object { (Resolve-Path $_).Path })
if (-not $Files) {
    $Files = @((Join-Path $root 'README.md')) + (Get-ChildItem (Join-Path $root 'docs/manual') -Recurse -Filter *.md | Sort-Object FullName | ForEach-Object FullName)
}
if (-not $Compiler) {
    $Compiler = Join-Path $root $(if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'qb.exe' } else { 'qb' })
}

# ---- extract blocks ------------------------------------------------------
$blocks = [System.Collections.Generic.List[object]]::new()
foreach ($file in $Files) {
    $lines = [IO.File]::ReadAllLines($file)
    $rel = [IO.Path]::GetRelativePath($root, $file) -replace '\\', '/'
    $defines = @{}
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -notmatch '^\s*```\s*(basic|qbasic|bas)\s*$') { continue }
        $start = $i
        $body = [System.Collections.Generic.List[string]]::new()
        $i++
        while ($i -lt $lines.Count -and $lines[$i] -notmatch '^\s*```\s*$') { $body.Add($lines[$i]); $i++ }
        # markers: comment lines directly above the fence
        $markers = @()
        for ($j = $start - 1; $j -ge 0 -and $lines[$j] -match '^\s*<!--\s*docs-test:\s*(.*?)\s*-->\s*$'; $j--) { $markers += $matches[1] }
        $mode = 'compile'; $reason = ''; $prepend = $null; $define = $null; $noIr = $null
        foreach ($m in $markers) {
            switch -Regex ($m) {
                '^skip\b\s*(.*)$' { $mode = 'skip'; $reason = $matches[1] }
                '^compile-error\b' { $mode = 'compile-error' }
                '^run\b' { $mode = 'run' }
                '^output\b' { $mode = 'output' }
                '^prepend\s+(\S+)' { $prepend = $matches[1] }
                '^define\s+(\S+)' { $define = $matches[1] }
                '^no-ir\b\s*(.*)$' { $noIr = $matches[1] }
            }
        }
        $expected = $null
        if ($mode -eq 'output') {
            $k = $i + 1
            while ($k -lt $lines.Count -and $lines[$k] -notmatch '^\s*```\s*(text|output)\s*$') {
                if ($lines[$k] -match '^\s*```\s*(basic|qbasic|bas)\s*$') { break }
                $k++
            }
            if ($k -lt $lines.Count -and $lines[$k] -match '^\s*```\s*(text|output)\s*$') {
                $exp = [System.Collections.Generic.List[string]]::new()
                $k++
                while ($k -lt $lines.Count -and $lines[$k] -notmatch '^\s*```\s*$') { $exp.Add($lines[$k]); $k++ }
                $expected = $exp -join "`n"
            }
        }
        $code = $body -join "`n"
        if ($define) { $defines[$define] = $code }
        if ($prepend) {
            if (-not $defines.ContainsKey($prepend)) { throw "${rel}:$($start + 1): prepend $prepend is not defined above" }
            $code = $defines[$prepend] + "`n" + $code
        }
        $blocks.Add([pscustomobject]@{ File = $rel; Line = $start + 1; Mode = $mode; Reason = $reason; Code = $code; Expected = $expected; NoIr = $noIr })
    }
}
if ($Match) { $blocks = @($blocks | Where-Object { $_.Code -like "*$Match*" -or "$($_.File):$($_.Line)" -like "*$Match*" }) }

if ($List) {
    $blocks | ForEach-Object { '{0}:{1} {2} {3}' -f $_.File, $_.Line, $_.Mode, $_.Reason }
    "{0} blocks" -f $blocks.Count
    exit 0
}
if (-not (Test-Path $Compiler)) { Write-Error "compiler not found: $Compiler (pass -Compiler or set QB_COMPILER)"; exit 2 }

# ---- compile / run ------------------------------------------------------
$work = Join-Path $root 'target/docs-test'
New-Item -ItemType Directory -Force $work | Out-Null
$exeExt = if ($IsWindows -or $env:OS -eq 'Windows_NT') { '.exe' } else { '' }

function Invoke-Proc([string]$exe, [string[]]$argv, [string]$cwd, [int]$timeoutSec) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($exe)
    foreach ($a in $argv) { $psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $cwd
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.RedirectStandardInput = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.Environment['QBNEX_NO_ERROR_DIALOG'] = '1'
    $psi.Environment['QBNEX_HEADLESS'] = '1'
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    $out = $p.StandardOutput.ReadToEndAsync()
    $err = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit($timeoutSec * 1000)) {
        try { $p.Kill($true) } catch {}
        return [pscustomobject]@{ Code = -999; Out = ''; Err = "timeout after $timeoutSec s" }
    }
    $p.WaitForExit()
    [pscustomobject]@{ Code = $p.ExitCode; Out = $out.Result; Err = $err.Result }
}

function Normalize([string]$s) {
    (($s -replace "`r`n", "`n" -replace "`r", "`n") -split "`n" | ForEach-Object { $_.TrimEnd() }) -join "`n" -replace "`n+$", ''
}

$cache = @{}
$results = [System.Collections.Generic.List[object]]::new()
$n = 0
foreach ($b in $blocks) {
    $n++
    $tag = '{0}:{1}' -f $b.File, $b.Line
    if ($b.Mode -eq 'skip') { $results.Add([pscustomobject]@{ Tag = $tag; Status = 'SKIP'; Detail = $b.Reason }); continue }
    $code = $b.Code
    if ($b.Mode -in 'run', 'output' -and $code -notmatch '(?im)^\s*\$CONSOLE') { $code = "`$CONSOLE:ONLY`n" + $code }
    # END in a console program waits at "Press any key" even without a console;
    # SYSTEM ends it at once (only possible when no SUB/FUNCTION follows).
    if ($b.Mode -in 'run', 'output' -and $code -notmatch '(?im)^\s*(SUB|FUNCTION)\s') { $code += "`nSYSTEM" }
    $key = "$($b.Mode)|$Backend|$code|$($b.Expected)"
    if ($cache.ContainsKey($key)) {
        $r = $cache[$key]
        $results.Add([pscustomobject]@{ Tag = $tag; Status = $r.Status; Detail = "(same as $($r.Tag)) $($r.Detail)" })
        continue
    }
    $dir = Join-Path $work ('b{0:D3}' -f $n)
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force $dir | Out-Null
    $src = Join-Path $dir 'doc.bas'
    [IO.File]::WriteAllText($src, $code + "`n")
    $exe = Join-Path $dir ('doc' + $exeExt)
    $cargs = @($src, '-o', $exe, '-m')
    if ($Backend -eq 'ir') { $cargs += '--ir' }
    if ($CheckOnly) { $cargs += '-z' }
    Write-Host ('[{0}/{1}] {2} {3}' -f $n, $blocks.Count, $tag, $b.Mode)
    $c = Invoke-Proc $Compiler $cargs (Split-Path $Compiler) 600
    $compiled = ($c.Code -eq 0) -and ($CheckOnly -or (Test-Path $exe))
    $msg = ((($c.Out + $c.Err) -replace 'Preparing build files\.\.\. \[[#.]+\] \d+%', '') -split "`n" | Where-Object { $_ -match 'error|Error|ERROR|\[E\d+\]|Expected|Unknown|not|unsupported' } | Select-Object -First 3) -join ' | '
    $status = 'PASS'; $detail = ''
    if ($Backend -eq 'ir' -and $null -ne $b.NoIr -and $b.Mode -ne 'compile-error') {
        $works = $compiled
        if ($compiled -and $b.Mode -in 'run', 'output') { $rr = Invoke-Proc $exe @() $dir 30; $works = $rr.Code -eq 0; $msg = ($rr.Err.Trim() -split "`n")[0] }
        if ($works) { $status = 'FAIL'; $detail = 'works with --ir now: remove the no-ir marker' } else { $status = 'XFAIL'; $detail = "no-ir ($($b.NoIr)): $msg" }
        $res = [pscustomobject]@{ Tag = $tag; Status = $status; Detail = $detail }; $cache[$key] = $res; $results.Add($res)
        if (-not $Keep) { Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue }
        continue
    }
    switch ($b.Mode) {
        'compile' { if (-not $compiled) { $status = 'FAIL'; $detail = "does not compile: $msg" } }
        'compile-error' { if ($compiled) { $status = 'FAIL'; $detail = 'compiles now: the doc calls this planned; update the doc and the marker' } else { $status = 'XFAIL'; $detail = $msg } }
        default {
            if ($CheckOnly) { if (-not $compiled) { $status = 'FAIL'; $detail = "does not compile: $msg" }; break }
            if (-not $compiled) { $status = 'FAIL'; $detail = "does not compile: $msg"; break }
            $r = Invoke-Proc $exe @() $dir 30
            if ($r.Code -ne 0) { $status = 'FAIL'; $detail = "exit $($r.Code): $(($r.Err.Trim() -split "`n")[0])"; break }
            if ($b.Mode -eq 'output') {
                if ($null -eq $b.Expected) { $status = 'FAIL'; $detail = 'output marker without a ```text block'; break }
                $got = Normalize $r.Out; $want = Normalize $b.Expected
                if ($got -ne $want) { $status = 'FAIL'; $detail = "output differs:`n--- expected`n$want`n--- got`n$got" }
            }
        }
    }
    $res = [pscustomobject]@{ Tag = $tag; Status = $status; Detail = $detail }
    $cache[$key] = $res
    $results.Add($res)
    if (-not $Keep -and $status -ne 'FAIL') { Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue }
}

# ---- report ---------------------------------------------------------------
''
foreach ($r in $results) { if ($r.Status -ne 'PASS') { '{0,-5} {1} {2}' -f $r.Status, $r.Tag, $r.Detail } }
$sum = $results | Group-Object Status | ForEach-Object { "$($_.Name) $($_.Count)" }
''
"docs-test ($Backend$(if ($CheckOnly) { ', front end only' })): " + ($sum -join ', ')
if ($results | Where-Object Status -eq 'FAIL') { exit 1 }
exit 0
