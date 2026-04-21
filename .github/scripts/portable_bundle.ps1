param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [Parameter(Mandatory = $false)]
    [string]$WrapperPath = "qb.cmd"
)

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")
$binarySource = Join-Path $repoRoot $BinaryPath
$bundleRoot = Join-Path $repoRoot $OutputDir

New-Item -ItemType Directory -Force -Path $bundleRoot | Out-Null

function Copy-IfExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Source,

        [Parameter(Mandatory = $true)]
        [string]$Destination
    )

    if (Test-Path -LiteralPath $Source) {
        $destinationDir = Split-Path -Parent $Destination
        if ($destinationDir) {
            New-Item -ItemType Directory -Force -Path $destinationDir | Out-Null
        }

        Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
    }
}

Copy-IfExists -Source $binarySource -Destination (Join-Path $bundleRoot (Split-Path -Leaf $BinaryPath))
Copy-IfExists -Source (Join-Path $repoRoot $WrapperPath) -Destination (Join-Path $bundleRoot (Split-Path -Leaf $WrapperPath))
Copy-IfExists -Source (Join-Path $repoRoot "internal") -Destination (Join-Path $bundleRoot "internal")
Copy-IfExists -Source (Join-Path $repoRoot "licenses") -Destination (Join-Path $bundleRoot "licenses")
Copy-IfExists -Source (Join-Path $repoRoot "README.md") -Destination (Join-Path $bundleRoot "README.md")
Copy-IfExists -Source (Join-Path $repoRoot "CHANGELOG.md") -Destination (Join-Path $bundleRoot "CHANGELOG.md")
