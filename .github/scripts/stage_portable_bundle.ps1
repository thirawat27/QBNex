param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [string]$WrapperPath = ""
)

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptRoot "..\.."))

function Resolve-BundlePath {
    param([string]$PathValue)

    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return [System.IO.Path]::GetFullPath($PathValue)
    }

    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $PathValue))
}

$bundleRoot = Resolve-BundlePath $OutputDir

if (-not $bundleRoot.StartsWith($repoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Output directory must stay inside the repository: $bundleRoot"
}

if (Test-Path $bundleRoot) {
    Remove-Item -LiteralPath $bundleRoot -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $bundleRoot | Out-Null

$binarySource = Resolve-BundlePath $BinaryPath
if (-not (Test-Path -LiteralPath $binarySource)) {
    throw "Binary not found: $binarySource"
}
Copy-Item -LiteralPath $binarySource -Destination $bundleRoot -Force

if ($WrapperPath) {
    $wrapperSource = Resolve-BundlePath $WrapperPath
    if (Test-Path -LiteralPath $wrapperSource) {
        Copy-Item -LiteralPath $wrapperSource -Destination $bundleRoot -Force
    }
}

foreach ($fileName in @("README.md", "CHANGELOG.md", "LICENSE")) {
    $sourcePath = Join-Path $repoRoot $fileName
    if (Test-Path -LiteralPath $sourcePath) {
        Copy-Item -LiteralPath $sourcePath -Destination $bundleRoot -Force
    }
}

foreach ($directoryName in @("licenses", "source", "internal")) {
    $sourcePath = Join-Path $repoRoot $directoryName
    if (Test-Path -LiteralPath $sourcePath) {
        Copy-Item -LiteralPath $sourcePath -Destination $bundleRoot -Recurse -Force
    }
}

$tempPath = Join-Path $bundleRoot "internal\\temp"
if (Test-Path -LiteralPath $tempPath) {
    Get-ChildItem -LiteralPath $tempPath -Force | Remove-Item -Recurse -Force
} else {
    New-Item -ItemType Directory -Force -Path $tempPath | Out-Null
}
