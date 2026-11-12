param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [Parameter(Mandatory = $false)]
    [string]$WrapperPath = "qb.cmd",

    [Parameter(Mandatory = $false)]
    [string]$AliasWrapperPath = "qbnex.cmd"
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
Copy-IfExists -Source (Join-Path $repoRoot $AliasWrapperPath) -Destination (Join-Path $bundleRoot (Split-Path -Leaf $AliasWrapperPath))
# Files the compiler reads at run time next to itself: the $IMPORT standard library and
# the support includes ($COLOR, vwatch, the SUB _GL table compiler\support\opengl\gl_table.txt).
Copy-IfExists -Source (Join-Path $repoRoot "compiler\stdlib") -Destination (Join-Path $bundleRoot "compiler\stdlib")
Copy-IfExists -Source (Join-Path $repoRoot "compiler\support") -Destination (Join-Path $bundleRoot "compiler\support")
# QIR runner (every compiled program is a copy of it plus the program's QIR).
Copy-IfExists -Source (Join-Path $repoRoot "crates\engine\exec\bin") -Destination (Join-Path $bundleRoot "crates\engine\exec\bin")
# Developer tools behind `qb fmt|lsp|dap` (qb.cmd, qbnex.cmd), when a cargo release build
# made them; otherwise the launcher reports how to build them.
foreach ($tool in @("qb-fmt.exe", "qb-lsp.exe", "qb-dap.exe")) {
    Copy-IfExists -Source (Join-Path $repoRoot "target\release\$tool") -Destination (Join-Path $bundleRoot $tool)
}
Copy-IfExists -Source (Join-Path $repoRoot "licenses") -Destination (Join-Path $bundleRoot "licenses")
Copy-IfExists -Source (Join-Path $repoRoot "README.md") -Destination (Join-Path $bundleRoot "README.md")
Copy-IfExists -Source (Join-Path $repoRoot "CHANGELOG.md") -Destination (Join-Path $bundleRoot "CHANGELOG.md")
