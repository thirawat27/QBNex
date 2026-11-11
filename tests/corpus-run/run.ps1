<#
.SYNOPSIS
  Compiles and RUNS every program of the local corpus (tests/corpus-local) headless,
  through every backend, and diffs the outputs.

.DESCRIPTION
  For each .bas file:
    cpp     qb.exe <file> -o cpp.exe              (C++ codegen + Rust qb-runtime)
    oracle  <oracle>\qb.exe <file> -o oracle.exe  (legacy QB64 C++ runtime; console-only programs only,
                                                    because the legacy runtime ignores QBNEX_HEADLESS and
                                                    would open a window)
    vm      qb.exe <file> --ir -o ir.exe, run with QB_JIT=off   (QIR on the typed VM)
    jit     the same ir.exe, run with QB_JIT=eager                (QIR through the Cranelift JIT)
  Every run gets a fresh copy of the program's directory as its working directory, stdin from
  inputs/<id>.in (default inputs/_default.in), keyboard/mouse events from inputs/<id>.input
  (default inputs/_default.input, GUI programs only), QBNEX_HEADLESS=1, QBNEX_NO_ERROR_DIALOG=1,
  QBNEX_FB_CAPTURE (framebuffer hash), and a timeout. The process tree is killed on timeout.

  Results: <Out>\results.json, <Out>\results.md (table), and per program <Out>\<id>\ with
  compile logs, stdout/stderr of each variant and the QIR (prog.qir).

  <id> = path below the corpus root without .bas, '\' -> '__', with the prefixes
  creationix-basic-games -> cx and qb64-samples\programs\samples -> qb64.

.EXAMPLE
  pwsh tests\corpus-run\run.ps1 -Filter 'cx__MISC__*' -Timeout 6
#>
param(
    [string]$Corpus = (Join-Path $PSScriptRoot '..\corpus-local'),
    [string]$Out = (Join-Path $PSScriptRoot '..\..\target\corpus-run'),
    [string]$Qb = (Join-Path $PSScriptRoot '..\..\qb.exe'),
    [string]$Oracle = $env:QBNEX_ORACLE,
    [string]$Filter = '*',
    [int]$Timeout = 8,
    [string[]]$Variants = @('cpp', 'oracle', 'vm', 'jit'),
    [switch]$KeepExe,
    # Keep the entries of an earlier results.json and only run programs it does not have.
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Corpus = (Resolve-Path $Corpus).Path
$Qb = (Resolve-Path $Qb).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$inputs = Join-Path $PSScriptRoot 'inputs'
if (-not $Oracle) {
    $cand = Join-Path $repo '.oracle\qb.exe'
    if (Test-Path $cand) { $Oracle = $cand }
}
if ($Oracle -and -not (Test-Path $Oracle)) { $Oracle = $null }
$oracleRoot = if ($Oracle) { Split-Path (Resolve-Path $Oracle).Path } else { $null }

function Get-Id([string]$full) {
    $rel = $full.Substring($Corpus.Length).TrimStart('\', '/')
    $rel = $rel -replace '^creationix-basic-games[\\/]', 'cx\' -replace '^qb64-samples[\\/]programs[\\/]samples[\\/]', 'qb64\'
    $rel = $rel -replace '\.bas$', ''
    return ($rel -replace '[\\/]', '__')
}

# Runs $exe with a timeout; stdin bytes are written asynchronously. Returns a hashtable.
function Invoke-Bounded([string]$exe, [string[]]$argv, [string]$cwd, [hashtable]$envs, [byte[]]$stdin, [int]$seconds) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new($exe)
    foreach ($a in $argv) { $psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $cwd
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    foreach ($k in $envs.Keys) {
        if ($null -eq $envs[$k]) { [void]$psi.Environment.Remove($k) } else { $psi.Environment[$k] = [string]$envs[$k] }
    }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # "The paging file is too small": the shared machine is out of commit; wait and retry.
    $p = $null
    for ($i = 0; $i -lt 10 -and -not $p; $i++) {
        try { $p = [System.Diagnostics.Process]::Start($psi) } catch { Start-Sleep -Seconds 30 }
    }
    if (-not $p) { return @{ code = -1; timedOut = $false; ms = 0; out = [byte[]]@(); err = [System.Text.Encoding]::ASCII.GetBytes('runner: could not start process (system memory)') } }
    $so = [System.IO.MemoryStream]::new(); $se = [System.IO.MemoryStream]::new()
    $t1 = $p.StandardOutput.BaseStream.CopyToAsync($so)
    $t2 = $p.StandardError.BaseStream.CopyToAsync($se)
    $wt = $null
    if ($stdin -and $stdin.Length) { $wt = $p.StandardInput.BaseStream.WriteAsync($stdin, 0, $stdin.Length) } else { $p.StandardInput.Close() }
    $stdinOpen = [bool]$wt
    $timedOut = $false
    $deadline = $seconds * 1000
    while (-not $p.WaitForExit(50)) {
        if ($stdinOpen -and $wt.IsCompleted) { try { $p.StandardInput.Close() } catch {}; $stdinOpen = $false }
        if ($sw.ElapsedMilliseconds -ge $deadline) {
            $timedOut = $true
            try { $p.Kill($true) } catch {}
            [void]$p.WaitForExit(5000)
            break
        }
    }
    if ($stdinOpen) { try { $p.StandardInput.Close() } catch {} }
    try { [void][System.Threading.Tasks.Task]::WaitAll(@($t1, $t2), 5000) } catch {}
    $code = if ($timedOut) { $null } else { $p.ExitCode }
    $p.Dispose()
    return @{ code = $code; timedOut = $timedOut; ms = $sw.ElapsedMilliseconds; out = $so.ToArray(); err = $se.ToArray() }
}

function Format-Code($code) {
    if ($null -eq $code) { return 'timeout' }
    if ($code -lt 0 -or $code -gt 0x7FFFFFFF) { return ('0x{0:X8}' -f ([int64]$code -band 0xFFFFFFFFL)) }
    return [string]$code
}

function Get-Normalised([byte[]]$b) {
    $s = [System.Text.Encoding]::Latin1.GetString($b) -replace "`r`n", "`n"
    return $s
}

function Get-InputFile([string]$id, [string]$ext) {
    $f = Join-Path $inputs "$id$ext"
    if (Test-Path $f) { return $f }
    $d = Join-Path $inputs "_default$ext"
    if (Test-Path $d) { return $d }
    return $null
}

function Get-ConsoleOnly([string]$tempDir) {
    $md = Join-Path $tempDir 'maindata.txt'
    if (-not (Test-Path $md)) { return $false }
    return [bool](Select-String -LiteralPath $md -SimpleMatch 'sub__dest(func__console());' -Quiet)
}

function Compile([string]$compiler, [string]$root, [string]$src, [string]$exe, [string[]]$extra, [string]$log) {
    if (Test-Path $exe) { Remove-Item $exe -Force }
    $argv = @($src) + $extra + @('-o', $exe, '-q', '-m')
    # "Build failed" (g++ step) on this shared machine is nearly always the commit limit
    # (cc1plus dies); a second try after a pause tells it apart from a real failure.
    for ($try = 1; $try -le 2; $try++) {
        $r = Invoke-Bounded $compiler $argv $root @{ QBNEX_NO_ERROR_DIALOG = '1' } $null 900
        $text = (Get-Normalised $r.out) + (Get-Normalised $r.err)
        # A failure with no output at all means the compiler never ran (machine out of
        # memory, or the app shutting down): retry it too.
        if ($text -notmatch 'Build failed' -and ($r.code -eq 0 -or $text -match '\S')) { break }
        Start-Sleep -Seconds 30
    }
    Set-Content -LiteralPath $log -Value $text -NoNewline
    $ok = ($r.code -eq 0) -and (Test-Path $exe)
    $first = ($text -split "`n" | Where-Object { $_ -match '\S' } | Select-Object -First 3) -join ' | '
    return @{ ok = $ok; msg = $first; ms = $r.ms }
}

# Fresh working copy of the program's folder in $cwd. QB64 samples refer to
# ".\programs\samples\<dir>" (CHDIR/OPEN; they expect to run from the QB64 root), so for them the
# folder goes to $cwd\programs\samples\<dir> and the exe stays at $cwd, the "QB64 root".
function New-RunDir($f, [string]$srcText, [string]$cwd) {
    $rel = $f.DirectoryName.Substring($Corpus.Length).TrimStart('\')
    $m = [regex]::Match($rel, '^qb64-samples\\(programs\\samples\\.*)$')
    if ($m.Success -and $srcText -match 'programs[\\/]samples') {
        $dest = Join-Path $cwd $m.Groups[1].Value
        New-Item -ItemType Directory -Force (Split-Path $dest) | Out-Null
        Copy-Item $f.DirectoryName $dest -Recurse
    } else {
        Copy-Item $f.DirectoryName $cwd -Recurse
    }
}

$interactiveRe = 'INKEY\$|\bINPUT\b|_KEYHIT|_KEYDOWN|\bINP\s*\(|_MOUSE|\bSLEEP\b\s*$|LINE\s+INPUT'
$nondetRe = 'RANDOMIZE\s+TIMER|\bTIMER\b|TIME\$|DATE\$|RANDOMIZE\s*$'

$files = Get-ChildItem $Corpus -Recurse -File -Filter *.bas | Sort-Object FullName
$results = [System.Collections.Generic.List[object]]::new()
$done = @{}
$prev = Join-Path $Out 'results.json'
if ($Resume -and (Test-Path $prev)) {
    foreach ($o in @(Get-Content $prev -Raw | ConvertFrom-Json -AsHashtable)) { $results.Add([pscustomobject]$o); $done[$o.id] = $true }
}
foreach ($f in $files) {
    $id = Get-Id $f.FullName
    if ($id -notlike $Filter -or $done[$id]) { continue }
    $w = Join-Path $Out $id
    if (Test-Path $w) { Remove-Item $w -Recurse -Force }
    New-Item -ItemType Directory -Force $w | Out-Null
    $srcBytes = [System.IO.File]::ReadAllBytes($f.FullName)
    $srcText = [System.Text.Encoding]::Latin1.GetString($srcBytes)
    $r = [ordered]@{
        id = $id; path = $f.FullName.Substring($Corpus.Length + 1); interactive = [bool]($srcText -match "(?im)$interactiveRe")
        nondeterministic = [bool]($srcText -match "(?im)$nondetRe"); consoleOnly = $false
        compile = [ordered]@{}; run = [ordered]@{}; status = ''; notes = [System.Collections.Generic.List[string]]::new()
    }
    Write-Host "== $id" -NoNewline
    if ($srcBytes.Length -gt 0 -and $srcBytes[0] -eq 0xFC) {
        $r.status = 'binary-format'; $r.notes.Add('QuickBASIC binary (fast-load) format, not source')
        $results.Add([pscustomobject]$r); Write-Host " binary-format"; continue
    }

    # --- compile -----------------------------------------------------------
    $exes = @{}
    if ('cpp' -in $Variants) {
        $c = Compile $Qb $repo $f.FullName (Join-Path $w 'cpp.exe') @() (Join-Path $w 'compile-cpp.log')
        $r.compile.cpp = $c
        if ($c.ok) { $exes.cpp = Join-Path $w 'cpp.exe'; $r.consoleOnly = Get-ConsoleOnly (Join-Path $repo 'internal\temp') }
    }
    if (('vm' -in $Variants) -or ('jit' -in $Variants)) {
        $c = Compile $Qb $repo $f.FullName (Join-Path $w 'ir.exe') @('--ir') (Join-Path $w 'compile-ir.log')
        $r.compile.ir = $c
        $qir = Join-Path $repo 'internal\temp\program.qir'
        if ($c.ok -and (Test-Path $qir)) {
            Copy-Item $qir (Join-Path $w 'prog.qir')
            $exes.vm = Join-Path $w 'ir.exe'; $exes.jit = $exes.vm
            if (-not $exes.cpp) { $r.consoleOnly = [bool](Select-String -LiteralPath $qir -Pattern '^meta console only' -Quiet) }
            # Compiler bug (corpus-run.md Q9): IrFinish writes the QIR before the automatic
            # console-only decision, so a console program's QIR has no `meta console only` and
            # qb-exec opens a window. Add the line the C++ build got and repack, so vm/jit are
            # compared on the same terms; the note keeps the bug visible.
            $pq = Join-Path $w 'prog.qir'
            if ($r.consoleOnly -and -not (Select-String -LiteralPath $pq -Pattern '^meta console' -Quiet)) {
                $t = [IO.File]::ReadAllText($pq) -replace '^QIR 1\r?\n', "QIR 1`nmeta console only`n"
                [IO.File]::WriteAllText($pq, $t)
                & (Join-Path $repo 'scripts\qir_pack.ps1') -Runner (Join-Path $repo 'crates\engine\exec\bin\windows\qb-exec.exe') -Qir $pq -Out $exes.vm
                $r.notes.Add('ir: meta console only added (Q9)')
            }
        }
    }
    if ('oracle' -in $Variants -and $Oracle -and $r.consoleOnly) {
        $c = Compile $Oracle $oracleRoot $f.FullName (Join-Path $w 'oracle.exe') @() (Join-Path $w 'compile-oracle.log')
        $r.compile.oracle = $c
        # Never run a legacy build that would open a window.
        if ($c.ok -and (Get-ConsoleOnly (Join-Path $oracleRoot 'internal\temp'))) { $exes.oracle = Join-Path $w 'oracle.exe' }
        elseif ($c.ok) { $r.notes.Add('oracle build not console-only: not run') }
    }

    # --- run ---------------------------------------------------------------
    $inFile = Get-InputFile $id '.in'
    $stdin = if ($inFile) { [System.IO.File]::ReadAllBytes($inFile) } else { $null }
    $script = if ($r.consoleOnly) { $f2 = Join-Path $inputs "$id.input"; if (Test-Path $f2) { $f2 } else { $null } } else { Get-InputFile $id '.input' }
    foreach ($v in @('cpp', 'oracle', 'vm', 'jit')) {
        if (-not ($v -in $Variants) -or -not $exes[$v]) { continue }
        $cwd = Join-Path $w "run-$v"
        New-RunDir $f $srcText $cwd
        $fb = Join-Path $w "$v.fbraw"
        $envs = @{ QBNEX_NO_ERROR_DIALOG = '1'; QBNEX_HEADLESS = '1'; QBNEX_FB_CAPTURE = $fb; QBNEX_INPUT_SCRIPT = $script; QB_JIT = $null }
        if ($v -eq 'vm') { $envs.QB_JIT = 'off' }
        if ($v -eq 'jit') { $envs.QB_JIT = 'eager' }
        # QB64-style executables chdir to their own folder at startup, so the exe sits next to
        # the program's data files, as it would for a user.
        $runExe = Join-Path $cwd "qbnex_corpus_$v.exe"
        Copy-Item $exes[$v] $runExe
        # The machine is shared with large builds: a run that dies because the system commit
        # limit is reached ("memory allocation of N bytes failed") is retried, not counted.
        for ($try = 1; $try -le 3; $try++) {
            $x = Invoke-Bounded $runExe @() $cwd $envs $stdin $Timeout
            if ((Get-Normalised $x.err) -notmatch 'memory allocation of \d+ bytes failed') { break }
            $r.notes.Add("$v hit the system memory limit (try $try)")
            Start-Sleep -Seconds 20
            Remove-Item $cwd -Recurse -Force -ErrorAction SilentlyContinue
            New-RunDir $f $srcText $cwd; Copy-Item $exes[$v] $runExe
        }
        [System.IO.File]::WriteAllBytes((Join-Path $w "$v.stdout"), $x.out)
        [System.IO.File]::WriteAllBytes((Join-Path $w "$v.stderr"), $x.err)
        $fbHash = ''
        if (Test-Path $fb) { $fbHash = (Get-FileHash $fb -Algorithm SHA256).Hash.Substring(0, 16); if (-not $KeepExe) { Remove-Item $fb } }
        $norm = Get-Normalised $x.out
        $r.run[$v] = [ordered]@{
            exit = Format-Code $x.code; ms = $x.ms; outBytes = $x.out.Length
            outHash = (Get-FileHash -InputStream ([System.IO.MemoryStream]::new([System.Text.Encoding]::Latin1.GetBytes($norm))) -Algorithm SHA256).Hash.Substring(0, 16)
            fbHash = $fbHash; stderr = ((Get-Normalised $x.err) -split "`n" | Where-Object { $_ -match '\S' } | Select-Object -First 2) -join ' | '
        }
        Remove-Item $cwd -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $KeepExe) { Get-ChildItem $w -Filter *.exe | Remove-Item -Force }

    # --- classify ----------------------------------------------------------
    $p = $r.run.cpp
    if (-not $r.compile.cpp -or -not $r.compile.cpp.ok) {
        $r.status = 'compile-fail'
        if ($exes.vm) { $p = $r.run.vm; $r.status = 'compile-fail(cpp)' }
    }
    if ($p) {
        $crashRe = '^0x|^(-|3221)'
        if ($p.exit -eq 'timeout') { $r.status = if ($r.interactive) { 'interactive-only' } else { 'timeout' } }
        elseif ($p.exit -match '^0x') { $r.status = 'crash' }
        elseif ($p.exit -ne '0' -or $p.stderr -match 'error|panick') { $r.status = 'rt-error' }
        else { $r.status = 'pass' }
        # Diffs: only between runs that both finished (a timeout cuts output at a random point).
        $base = $p
        foreach ($v in @('oracle', 'vm', 'jit')) {
            $o = $r.run[$v]
            if (-not $o -or $o -eq $base) { continue }
            # The framebuffer capture holds every _DISPLAY frame, so its count depends on timing:
            # a mismatch is noted, never counted as a diff.
            if ($o.fbHash -and $base.fbHash -and $o.fbHash -ne $base.fbHash -and $o.exit -ne 'timeout' -and $base.exit -ne 'timeout') { $r.notes.Add("$v framebuffer differs") }
            if ($o.exit -eq 'timeout' -or $base.exit -eq 'timeout') {
                if ($o.exit -ne $base.exit) { $r.notes.Add("$v exit $($o.exit) vs $($base.exit)") }
                continue
            }
            $same = ($o.outHash -eq $base.outHash) -and ($o.exit -eq $base.exit) 
            if (-not $same) {
                $what = @()
                if ($o.outHash -ne $base.outHash) { $what += 'stdout' }
                if ($o.exit -ne $base.exit) { $what += "exit $($o.exit) vs $($base.exit)" }
                $r.notes.Add("$v differs: $($what -join ', ')")
                if ($r.status -eq 'pass') { $r.status = 'diff' }
            }
        }
        if ($exes.vm -and -not $r.run.vm) { $r.notes.Add('vm not run') }
    }
    if ($r.compile.ir -and -not $r.compile.ir.ok) { $r.notes.Add("ir compile: $($r.compile.ir.msg)") }
    if ($r.status -eq 'diff' -and $r.nondeterministic) { $r.notes.Add('uses TIMER/RANDOMIZE: may be nondeterministic') }
    Write-Host " $($r.status) $($r.notes -join '; ')"
    $results.Add([pscustomobject]$r)
    $results | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $Out 'results.json')
}

# --- table -------------------------------------------------------------------
$md = [System.Text.StringBuilder]::new()
[void]$md.AppendLine('| program | status | console | cpp | oracle | vm | jit | notes |')
[void]$md.AppendLine('|---|---|---|---|---|---|---|---|')
foreach ($r in $results) {
    $cell = { param($v) $x = $r.run[$v]; if ($x) { "$($x.exit) $([math]::Round($x.ms/1000,1))s" } elseif ($r.compile[$v] -and -not $r.compile[$v].ok) { 'no compile' } elseif ($v -in 'vm', 'jit' -and $r.compile.ir -and -not $r.compile.ir.ok) { 'no compile' } else { '-' } }
    $notes = ($r.notes -join '; ') -replace '\|', '/'
    if ($r.status -like 'compile-fail*' -and $r.compile.cpp) { $notes = (($r.compile.cpp.msg -replace '\|', '/') + '; ' + $notes).Trim('; ') }
    [void]$md.AppendLine("| $($r.path -replace '\\','/') | $($r.status) | $(if($r.consoleOnly){'yes'}else{'no'}) | $(& $cell 'cpp') | $(& $cell 'oracle') | $(& $cell 'vm') | $(& $cell 'jit') | $notes |")
}
$md.ToString() | Set-Content (Join-Path $Out 'results.md')
$results | Group-Object status | Sort-Object Count -Descending | ForEach-Object { '{0,-20} {1}' -f $_.Name, $_.Count }
