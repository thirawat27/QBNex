# The one-file installer on Windows: install into a folder that already holds a
# file of the user's and an old copy of qb.exe, compile and run a program with
# the installed compiler, install again over it, then uninstall.
param(
  [Parameter(Mandatory = $true)][string]$Setup,
  [Parameter(Mandatory = $true)][string]$Bundle,
  [Parameter(Mandatory = $true)][string]$Work
)
$ErrorActionPreference = 'Stop'
if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$pkg = Join-Path $Work 'qbnex-setup-test.exe'
& $Setup --pack $Bundle --out $pkg
if ($LASTEXITCODE) { throw 'pack failed' }
$dir = Join-Path $Work 'QBNex (Install) & Co'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Set-Content -Path (Join-Path $dir 'notes.txt') -Value 'mine'
Set-Content -Path (Join-Path $dir 'qb.exe') -Value 'old qb'

function Invoke-Hello {
  $src = Join-Path $Work 'hello.bas'
  $exe = Join-Path $Work 'hello.exe'
  Set-Content -Path $src -Value 'PRINT "hello from"; 1 + 1'
  if (Test-Path $exe) { Remove-Item -Force $exe }
  & (Join-Path $dir 'qb.exe') $src -o $exe
  if ($LASTEXITCODE) { throw 'compile failed' }
  $env:QBNEX_HEADLESS = '1'; $env:QBNEX_NO_ERROR_DIALOG = '1'
  $out = (& $exe | Out-String).TrimEnd("`r", "`n")
  if ($out -ne 'hello from 2 ') { throw "unexpected output: [$out]" }
}

function Assert-Notes {
  if ((Get-Content (Join-Path $dir 'notes.txt')) -ne 'mine') { throw "the user's file changed" }
}

& $pkg --yes --no-path --no-menu --dir $dir
if ($LASTEXITCODE) { throw 'install failed' }
Assert-Notes
& (Join-Path $dir 'qb.exe') --version
if ($LASTEXITCODE) { throw 'qb --version failed' }
Invoke-Hello
Write-Host 'first install: ok'

& $pkg --yes --no-path --no-menu --dir $dir
if ($LASTEXITCODE) { throw 'second install failed' }
Invoke-Hello
Write-Host 'install over the old one: ok'

& (Join-Path $dir 'qbnex-setup.exe') --uninstall --yes --dir $dir
if ($LASTEXITCODE) { throw 'uninstall failed' }
# the uninstaller removes its own exe after it has ended
Start-Sleep -Seconds 5
if (Test-Path (Join-Path $dir 'qb.exe')) { throw 'uninstall left qb.exe behind' }
Assert-Notes
Write-Host 'uninstall: ok'
