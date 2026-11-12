# Packs a QIR program into a copy of the QIR runner (no native code, no linker):
#   out = runner + <QIR text> + <u64 LE length> + "QBNEXIR0"
# The runner (qb-exec) reads its own file at start-up and runs the payload.
# Used by scripts\setup_win.cmd when `qb-exec build` (native image) is not wanted
# or not available for the QIR version of the bootstrap.
param(
    [Parameter(Mandatory = $true)][string]$Runner,
    [Parameter(Mandatory = $true)][string]$Qir,
    [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
$runnerBytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Runner))
$qirBytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Qir))
$len = [System.BitConverter]::GetBytes([UInt64]$qirBytes.Length)
if (-not [System.BitConverter]::IsLittleEndian) { [Array]::Reverse($len) }
$magic = [System.Text.Encoding]::ASCII.GetBytes('QBNEXIR0')
$stream = [System.IO.File]::Create($Out)
try {
    $stream.Write($runnerBytes, 0, $runnerBytes.Length)
    $stream.Write($qirBytes, 0, $qirBytes.Length)
    $stream.Write($len, 0, 8)
    $stream.Write($magic, 0, 8)
} finally {
    $stream.Dispose()
}
