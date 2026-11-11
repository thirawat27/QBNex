#Requires -Version 7
<#
.SYNOPSIS
    Moves the crates under crates/ into the grouped layout and rewrites every
    path that refers to them.

.DESCRIPTION
    Layout (package names stay qb-*):
        crates/compiler/{ir,syntax}
        crates/engine/{exec,jit,link,native,ffi}
        crates/runtime/{core,audio,gpu,float80,ext}
        crates/tools/{fmt,lsp,dap}
        crates/web

    Steps:
      A. inside each crate that is about to move, relative paths that leave the
         crate (Cargo path dependencies, include_str!/include_bytes!,
         CARGO_MANIFEST_DIR joins, ../../target in os/*/setup_build.*) are
         recomputed for the new depth;
      B. each crate is moved with `git mv`;
      C. every tracked text file is rewritten: crates/qb-x, crates\qb-x and
         crates\\qb-x become the new folder (same separator), and
         'crates', 'qb-x' segment lists become 'crates', '<group>', '<name>';
      D. string constants in bootstrap/qbnex.qir (hex encoded) get the same
         rewrite, so the stage0 compiler finds the runner at its new place.
         Rebuild the bootstrap afterwards with scripts/reorg_crates_bootstrap.ps1.

    Deterministic and re-runnable: a crate already at its new place is skipped,
    and the rewrites only match the old names, so a second run changes nothing.
    Files are read and written as bytes (Latin-1 round trip), so encodings and
    line endings are kept.

.PARAMETER DryRun
    Report what would change; write nothing and move nothing.

.PARAMETER AllowDirty
    Run even if the working tree has uncommitted changes.
#>
[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$AllowDirty
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root

$SelfFiles = @('scripts/reorg_crates.ps1', 'scripts/reorg_crates_bootstrap.ps1')

# Old folder name (under crates/) -> new folder (under crates/).
$Map = [ordered]@{
    'qb-ir'      = 'compiler/ir'
    'qb-syntax'  = 'compiler/syntax'
    'qb-exec'    = 'engine/exec'
    'qb-jit'     = 'engine/jit'
    'qb-link'    = 'engine/link'
    'qb-native'  = 'engine/native'
    'qb-ffi'     = 'engine/ffi'
    'qb-runtime' = 'runtime/core'
    'qb-audio'   = 'runtime/audio'
    'qb-gpu'     = 'runtime/gpu'
    'qb-float80' = 'runtime/float80'
    'qb-ext'     = 'runtime/ext'
    'qb-fmt'     = 'tools/fmt'
    'qb-lsp'     = 'tools/lsp'
    'qb-dap'     = 'tools/dap'
    'qb-web'     = 'web'
}
$Latin1 = [Text.Encoding]::Latin1
$NameAlt = ($Map.Keys | Sort-Object { - $_.Length }, { $_ } | ForEach-Object { [regex]::Escape($_.Substring(3)) }) -join '|'

# crates/qb-x with /, \ or \\ (escaped in a string literal) as separator.
$RxPath = [regex]::new("crates(/|\\\\|\\)qb-($NameAlt)(?![\w-])")
# 'crates', 'qb-x'  (path built from segments, e.g. join(here, '..', 'crates', 'qb-syntax'))
$RxSegs = [regex]::new("(['""])crates\1(\s*,\s*)(['""])qb-($NameAlt)\3(?![\w-])")
# crates/qb-* as a glob in prose.
$RxGlob = [regex]::new('crates(/|\\\\|\\)qb-\*(?![\w/\\])')
# "├── qb-x/   # comment" in folder trees (UTF-8 read as Latin-1: "──" ends in 0x80).
$RxTree = [regex]::new("(?<=\x80 )qb-($NameAlt)/( +)(?=#)")
# A relative path that starts with ../ or ..\ (or ..\\ in a string literal).
$RxRel = [regex]::new('(?:(?<=["''`(]/)|(?<![\w.\-%$/\\]))\.\.(?:/|\\)[\w.\-%$/\\]*')

$failed = $false
$Edits = [ordered]@{}        # final path -> number of edits
$RelLog = [System.Collections.Generic.List[string]]::new()
$Review = [System.Collections.Generic.List[string]]::new()

function Add-Edit([string]$path, [int]$n) {
    if ($n -le 0) { return }
    if ($Edits.Contains($path)) { $Edits[$path] += $n } else { $Edits[$path] = $n }
}

function Read-Text([string]$path) { $Latin1.GetString([IO.File]::ReadAllBytes((Join-Path $Root $path))) }
function Write-Text([string]$path, [string]$text) {
    if (-not $DryRun) { [IO.File]::WriteAllBytes((Join-Path $Root $path), $Latin1.GetBytes($text)) }
}

# Repo-relative segments, mapped from the old layout to the new one.
function Map-Segs([string[]]$segs) {
    $out = [System.Collections.Generic.List[string]]::new()
    if ($segs.Count -ge 2 -and $segs[0] -eq 'crates' -and $Map.Contains($segs[1])) {
        $out.Add('crates')
        foreach ($p in $Map[$segs[1]].Split('/')) { $out.Add($p) }
        for ($i = 2; $i -lt $segs.Count; $i++) { $out.Add($segs[$i]) }
    } else {
        foreach ($s in $segs) { $out.Add($s) }
    }
    return , $out.ToArray()
}

function Map-RepoPath([string]$path) { (Map-Segs ($path -split '/')) -join '/' }

# Resolves a ../ token against base segments; $null if it climbs above the repo.
function Resolve-Segs([string[]]$base, [string[]]$parts) {
    $cur = [System.Collections.Generic.List[string]]::new()
    foreach ($b in $base) { $cur.Add($b) }
    foreach ($p in $parts) {
        if ($p -eq '..') {
            if ($cur.Count -eq 0) { return $null }
            $cur.RemoveAt($cur.Count - 1)
        } elseif ($p -ne '.') {
            $cur.Add($p)
        }
    }
    return , $cur.ToArray()
}

function Test-RepoPath([string[]]$segs) {
    if ($segs.Count -eq 0) { return $true }
    Test-Path -LiteralPath (Join-Path $Root ($segs -join '/'))
}

function Starts-With([string[]]$segs, [string[]]$prefix) {
    if ($segs.Count -lt $prefix.Count) { return $false }
    for ($i = 0; $i -lt $prefix.Count; $i++) { if ($segs[$i] -ne $prefix[$i]) { return $false } }
    return $true
}

# ---------------------------------------------------------------- preconditions
if (-not (Test-Path (Join-Path $Root 'Cargo.toml')) -or -not (Test-Path (Join-Path $Root 'crates'))) {
    throw "Run from a QBNex checkout ($Root has no Cargo.toml / crates)."
}
if (-not $AllowDirty -and -not $DryRun) {
    $dirty = git status --porcelain --untracked-files=no
    if ($dirty) { throw "Working tree has uncommitted changes; commit them or pass -AllowDirty.`n$($dirty -join "`n")" }
}

$tracked = @(git ls-files -- crates)
$unknown = $tracked | ForEach-Object { ($_ -split '/')[1] } | Sort-Object -Unique |
    Where-Object { $_ -like 'qb-*' -and -not $Map.Contains($_) }
if ($unknown) { throw "Crates not in the layout map: $($unknown -join ', '). Add them to `$Map in $($SelfFiles[0])." }

$toMove = [System.Collections.Generic.List[string]]::new()
foreach ($old in $Map.Keys) {
    $oldDir = "crates/$old"; $newDir = "crates/$($Map[$old])"
    $oldTracked = @($tracked | Where-Object { $_.StartsWith("$oldDir/") })
    $newExists = Test-Path -LiteralPath (Join-Path $Root $newDir)
    if ($oldTracked.Count -gt 0) {
        if ($newExists -and @(git ls-files -- $newDir).Count -gt 0) {
            throw "Both $oldDir and $newDir have tracked files; resolve by hand."
        }
        $toMove.Add($old)
    } elseif (-not $newExists) {
        Write-Warning "$oldDir and $newDir are both missing (skipped)."
    }
}

# ------------------------------------------- A. relative paths inside moved crates
foreach ($old in $toMove) {
    $crateOld = @('crates', $old)
    $files = @(git grep -I -l -E '\.\.(/|\\)' -- "crates/$old")
    foreach ($f in $files) {
        if ($SelfFiles -contains $f) { continue }
        $text = Read-Text $f
        $fileDir = @(($f -split '/') | Select-Object -SkipLast 1)
        $isManifest = (Split-Path $f -Leaf) -eq 'Cargo.toml'
        $count = 0
        $new = $RxRel.Replace($text, [System.Text.RegularExpressions.MatchEvaluator] {
                param($m)
                $tok = $m.Value
                $sep = ([regex]::Match($tok, '\\\\|\\|/')).Value
                $trail = $tok -match '(/|\\)$'
                $parts = @($tok -split '\\\\|\\|/' | Where-Object { $_ -ne '' })

                # Base: crate root for Cargo.toml and CARGO_MANIFEST_DIR joins,
                # else the file's folder (include_str!, scripts, relative imports).
                $ctxStart = $m.Index
                for ($k = 0; $k -lt 4 -and $ctxStart -gt 0; $k++) {
                    # char overload: the string one is culture-aware and skips \n inside \r\n
                    $ctxStart = $text.LastIndexOf([char]10, [Math]::Max(0, $ctxStart - 1))
                    if ($ctxStart -lt 0) { $ctxStart = 0; break }
                }
                $ctx = $text.Substring($ctxStart, $m.Index - $ctxStart)
                $lineNo = 1 + ([regex]::Matches($text.Substring(0, $m.Index), "`n")).Count
                $fromFile = Resolve-Segs $fileDir $parts
                $fromCrate = Resolve-Segs $crateOld $parts
                if ($isManifest -or $ctx -match 'CARGO_MANIFEST_DIR') {
                    $base = $crateOld; $full = $fromCrate; $how = 'crate root'
                } elseif ($null -ne $fromFile -and (Test-RepoPath $fromFile)) {
                    $base = $fileDir; $full = $fromFile; $how = 'file dir'
                } elseif ($null -ne $fromCrate -and (Test-RepoPath $fromCrate)) {
                    $base = $crateOld; $full = $fromCrate; $how = 'crate root'
                } else {
                    $base = $fileDir; $full = $fromFile; $how = 'file dir, target missing'
                }
                if ($null -eq $full) {
                    $Review.Add("${f}:${lineNo}: '$tok' climbs above the repository; left as is")
                    return $tok
                }
                if (Starts-With $full $crateOld) { return $tok }   # stays inside the crate

                $newBase = Map-Segs $base
                $newFull = Map-Segs $full
                $c = 0
                while ($c -lt $newBase.Count -and $c -lt $newFull.Count -and $newBase[$c] -eq $newFull[$c]) { $c++ }
                $segs = [System.Collections.Generic.List[string]]::new()
                for ($i = $c; $i -lt $newBase.Count; $i++) { $segs.Add('..') }
                for ($i = $c; $i -lt $newFull.Count; $i++) { $segs.Add($newFull[$i]) }
                $out = $segs -join $sep
                if ($trail) { $out += $sep }
                if ($out -ceq $tok) { return $tok }
                $script:count++
                $RelLog.Add("$(Map-RepoPath $f):${lineNo}: $tok -> $out ($how)")
                # target/ is the cargo output folder: not tracked, often absent.
                if ($how -like '*missing*' -and $full[0] -ne 'target') {
                    $Review.Add("$(Map-RepoPath $f):${lineNo}: '$tok' -> '$out' (target $($full -join '/') does not exist; check)")
                }
                return $out
            })
        if ($new -cne $text) {
            Write-Text $f $new
            Add-Edit (Map-RepoPath $f) $count
        }
    }
}

# ------------------------------------------------------------------- B. git mv
$moved = [System.Collections.Generic.List[string]]::new()
foreach ($old in $toMove) {
    $oldDir = "crates/$old"; $newDir = "crates/$($Map[$old])"
    $parent = Split-Path $newDir -Parent
    if (-not $DryRun) {
        if (-not (Test-Path -LiteralPath (Join-Path $Root $parent))) {
            New-Item -ItemType Directory -Path (Join-Path $Root $parent) | Out-Null
        }
        if (Test-Path -LiteralPath (Join-Path $Root $newDir)) {
            # Only untracked leftovers (e.g. bin/ from an older build) are there.
            throw "$newDir exists (untracked files only); move them away and run again."
        }
        git mv -- $oldDir $newDir
        if ($LASTEXITCODE -ne 0) { throw "git mv $oldDir $newDir failed." }
    }
    $moved.Add("$oldDir -> $newDir")
}

# ------------------------------------------------- C. references in text files
$pattern = 'crates(/|\\)+qb-(' + $NameAlt + '|\*)|crates[''"]\s*,\s*[''"]qb-(' + $NameAlt + ')|qb-(' + $NameAlt + ')/ +#'
$candidates = @(git grep -I -l -E $pattern -- .)
foreach ($f in $candidates) {
    if ($SelfFiles -contains $f) { continue }
    $text = Read-Text $f
    $n = 0
    $new = $RxPath.Replace($text, [System.Text.RegularExpressions.MatchEvaluator] {
            param($m)
            $script:n++
            'crates' + $m.Groups[1].Value + $Map['qb-' + $m.Groups[2].Value].Replace('/', $m.Groups[1].Value)
        })
    $new = $RxSegs.Replace($new, [System.Text.RegularExpressions.MatchEvaluator] {
            param($m)
            $script:n++
            $q = $m.Groups[1].Value; $q2 = $m.Groups[3].Value; $j = $m.Groups[2].Value
            $q + 'crates' + $q + (($Map['qb-' + $m.Groups[4].Value].Split('/') | ForEach-Object { $j + $q2 + $_ + $q2 }) -join '')
        })
    # crates/qb-* (prose for "all the crates") -> crates/
    $new = $RxGlob.Replace($new, [System.Text.RegularExpressions.MatchEvaluator] {
            param($m)
            $script:n++
            'crates' + $m.Groups[1].Value
        })
    # Folder trees: keep the comment column where it was.
    $new = $RxTree.Replace($new, [System.Text.RegularExpressions.MatchEvaluator] {
            param($m)
            $script:n++
            $name = $Map['qb-' + $m.Groups[1].Value] + '/'
            $width = $m.Length
            $name + (' ' * [Math]::Max(1, $width - $name.Length))
        })
    if ($new -cne $text) {
        Write-Text $f $new
        Add-Edit (Map-RepoPath $f) $n
    }
}

# ---------------------------------------- D. hex string constants in bootstrap QIR
$qirPath = 'bootstrap/qbnex.qir'
if (Test-Path -LiteralPath (Join-Path $Root $qirPath)) {
    $qir = Read-Text $qirPath
    $qn = 0
    $rxHex = [regex]::new('(?m)^(\s*const\.str\s+%\S+\s+x)([0-9A-Fa-f]+)')
    $newQir = $rxHex.Replace($qir, [System.Text.RegularExpressions.MatchEvaluator] {
            param($m)
            $hex = $m.Groups[2].Value
            $bytes = [Convert]::FromHexString($hex)
            $s = $Latin1.GetString($bytes)
            if ($s -notmatch 'crates') { return $m.Value }
            $t = $RxPath.Replace($s, [System.Text.RegularExpressions.MatchEvaluator] {
                    param($mm)
                    'crates' + $mm.Groups[1].Value + $Map['qb-' + $mm.Groups[2].Value].Replace('/', $mm.Groups[1].Value)
                })
            if ($t -ceq $s) { return $m.Value }
            $script:qn++
            $m.Groups[1].Value + [Convert]::ToHexString($Latin1.GetBytes($t))
        })
    if ($newQir -cne $qir) {
        Write-Text $qirPath $newQir
        Add-Edit $qirPath $qn
    }
}

# ------------------------------------------------------------------- summary
$mode = if ($DryRun) { ' (dry run: nothing written)' } else { '' }
Write-Host "`n== Moves$mode"
if ($moved.Count) { $moved | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (none: already in the new layout)' }

Write-Host "`n== Relative paths recomputed inside moved crates"
if ($RelLog.Count) { $RelLog | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (none)' }

Write-Host "`n== Edits per file"
$total = 0
foreach ($k in ($Edits.Keys | Sort-Object)) { Write-Host ('  {0,5}  {1}' -f $Edits[$k], $k); $total += $Edits[$k] }
Write-Host ('  {0,5}  total in {1} files' -f $total, $Edits.Count)

if (-not $DryRun) {
    $exclude = $SelfFiles | ForEach-Object { ":(exclude)$_" }
    $left = @(git grep -n -I -E $pattern -- . @exclude)
    $hexOld = $Map.Keys | ForEach-Object {
        foreach ($s in '/', '\') { [Convert]::ToHexString($Latin1.GetBytes("crates$s$_")) }
    }
    if (Test-Path -LiteralPath (Join-Path $Root $qirPath)) {
        $qirText = Read-Text $qirPath
        foreach ($h in $hexOld) { if ($qirText.Contains($h)) { $left += "${qirPath}: hex string still holds $($Latin1.GetString([Convert]::FromHexString($h)))" } }
    }
    Write-Host "`n== Old paths left (must be empty)"
    if ($left.Count) { $left | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (none)' }
    if ($left.Count) { $Review.Add("$($left.Count) old path reference(s) left; see above"); $script:failed = $true }
}

Write-Host "`n== Needs a look by hand"
if ($Review.Count) { $Review | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (none)' }

if (-not $DryRun) {
    Write-Host "`nNext: scripts\reorg_crates_bootstrap.ps1 (rebuild bootstrap/qbnex.qir to a fixpoint), then cargo build --workspace."
}
if ($failed) { exit 1 }
exit 0
