param(
    [string]$Workspace = "D:\QBNex",
    [int]$TimeoutSeconds = 90,
    [string]$Filter = "",
    [switch]$IncludeIgnored
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Get-ShellCliTestBinary {
    param([string]$WorkspaceRoot)

    Push-Location $WorkspaceRoot
    try {
        $noRunOutput = & cmd /c "cargo test -p cli_tool --test shell_cli --no-run 2>&1"
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test --no-run failed.`n$($noRunOutput -join [Environment]::NewLine)"
        }

        $match = $noRunOutput | Select-String -Pattern 'Executable .*?\((.+shell_cli-[^\\)]+\.exe)\)'
        if (-not $match) {
            throw "could not locate shell_cli test binary in cargo output"
        }

        $relative = $match.Matches[0].Groups[1].Value
        return [System.IO.Path]::GetFullPath((Join-Path $WorkspaceRoot $relative))
    }
    finally {
        Pop-Location
    }
}

function Get-ShellCliTests {
    param(
        [string]$BinaryPath,
        [string]$NameFilter
    )

    $listed = ((& $BinaryPath --list 2>&1) -join [Environment]::NewLine) -split "\r?\n"
    if ($LASTEXITCODE -ne 0) {
        throw "failed to list tests from $BinaryPath`n$($listed -join [Environment]::NewLine)"
    }

    $tests = foreach ($line in $listed) {
        if ($line -match '^(.*): test$') {
            $name = $Matches[1].Trim()
            if ([string]::IsNullOrWhiteSpace($NameFilter) -or $name -like "*$NameFilter*") {
                $name
            }
        }
    }

    return ,@($tests)
}

function Stop-ProcessTree {
    param([int]$RootId)

    $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
    $childrenByParent = @{}
    foreach ($proc in $all) {
        if (-not $childrenByParent.ContainsKey($proc.ParentProcessId)) {
            $childrenByParent[$proc.ParentProcessId] = New-Object System.Collections.Generic.List[int]
        }
        $childrenByParent[$proc.ParentProcessId].Add([int]$proc.ProcessId)
    }

    $stack = New-Object System.Collections.Generic.Stack[int]
    $stack.Push($RootId)
    $ordered = New-Object System.Collections.Generic.List[int]
    while ($stack.Count -gt 0) {
        $id = $stack.Pop()
        $ordered.Add($id)
        if ($childrenByParent.ContainsKey($id)) {
            foreach ($childId in $childrenByParent[$id]) {
                $stack.Push($childId)
            }
        }
    }

    for ($i = $ordered.Count - 1; $i -ge 0; $i--) {
        $id = $ordered[$i]
        try {
            Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
        } catch {
        }
    }
}

function Stop-LeakedQbnexProcesses {
    param([string]$WorkspaceRoot)

    $qbPath = (Join-Path $WorkspaceRoot "target\debug\qb.exe").ToLowerInvariant()
    $tempMarker = "\qbnex_cli_"
    $procs = Get-CimInstance Win32_Process | Where-Object {
        $path = if ($_.ExecutablePath) { $_.ExecutablePath.ToLowerInvariant() } else { "" }
        $cmd = if ($_.CommandLine) { $_.CommandLine.ToLowerInvariant() } else { "" }
        $path -eq $qbPath -or
        $path.Contains($tempMarker) -or
        $cmd.Contains($tempMarker)
    }

    foreach ($proc in $procs) {
        try {
            Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
        } catch {
        }
    }
}

function Invoke-ShellCliTest {
    param(
        [string]$BinaryPath,
        [string]$TestName,
        [int]$TimeoutSecs,
        [switch]$RunIgnored
    )

    $stdout = Join-Path $env:TEMP ("qbnex_shell_cli_" + [guid]::NewGuid().ToString("N") + ".out.log")
    $stderr = Join-Path $env:TEMP ("qbnex_shell_cli_" + [guid]::NewGuid().ToString("N") + ".err.log")
    $args = @("--exact", $TestName, "--nocapture")
    if ($RunIgnored) {
        $args = @("--ignored") + $args
    }

    try {
        Stop-LeakedQbnexProcesses -WorkspaceRoot $Workspace

        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $BinaryPath
        $psi.WorkingDirectory = $Workspace
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.Arguments = ($args | ForEach-Object {
            if ($_ -match '\s') {
                '"' + ($_ -replace '"', '\"') + '"'
            } else {
                $_
            }
        }) -join ' '

        $process = New-Object System.Diagnostics.Process
        $process.StartInfo = $psi
        [void]$process.Start()

        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()

        if (-not $process.WaitForExit($TimeoutSecs * 1000)) {
            try {
                Stop-ProcessTree -RootId $process.Id
            } catch {
            }
            return [pscustomobject]@{
                Name = $TestName
                Status = "timeout"
                ExitCode = $null
                Stdout = if ($stdoutTask.IsCompleted) { $stdoutTask.Result } else { "" }
                Stderr = if ($stderrTask.IsCompleted) { $stderrTask.Result } else { "" }
            }
        }

        $process.WaitForExit()
        $resultStdout = $stdoutTask.Result
        $resultStderr = $stderrTask.Result
        Stop-LeakedQbnexProcesses -WorkspaceRoot $Workspace
        $status = if ($resultStdout -match '\.\.\. ignored') {
            "ignored"
        } elseif ($process.ExitCode -eq 0) {
            "passed"
        } else {
            "failed"
        }

        return [pscustomobject]@{
            Name = $TestName
            Status = $status
            ExitCode = $process.ExitCode
            Stdout = $resultStdout
            Stderr = $resultStderr
        }
    }
    finally {
        Remove-Item $stdout, $stderr -ErrorAction SilentlyContinue
    }
}

function Invoke-ShellCliTestWithRetry {
    param(
        [string]$BinaryPath,
        [string]$TestName,
        [int]$TimeoutSecs,
        [switch]$RunIgnored
    )

    $result = Invoke-ShellCliTest -BinaryPath $BinaryPath -TestName $TestName -TimeoutSecs $TimeoutSecs -RunIgnored:$RunIgnored
    if ($result.Status -ne "timeout") {
        return $result
    }

    Stop-LeakedQbnexProcesses -WorkspaceRoot $Workspace
    Start-Sleep -Milliseconds 500
    $retry = Invoke-ShellCliTest -BinaryPath $BinaryPath -TestName $TestName -TimeoutSecs $TimeoutSecs -RunIgnored:$RunIgnored
    if ($retry.Status -eq "passed" -or $retry.Status -eq "ignored") {
        return $retry
    }

    return $retry
}

$binary = Get-ShellCliTestBinary -WorkspaceRoot $Workspace
$tests = Get-ShellCliTests -BinaryPath $binary -NameFilter $Filter

if (-not $tests -or $tests.Count -eq 0) {
    throw "no shell_cli tests matched the current filter"
}

Write-Output "shell_cli binary: $binary"
Write-Output "tests selected: $($tests.Count)"
Write-Output "timeout per test: ${TimeoutSeconds}s"
if ($IncludeIgnored) {
    Write-Output "mode: include ignored tests"
}

$failures = @()
$timeouts = @()
$ignored = 0
$passed = 0

for ($i = 0; $i -lt $tests.Count; $i++) {
    $name = $tests[$i]
    Write-Output ("[{0}/{1}] {2}" -f ($i + 1), $tests.Count, $name)
    $result = Invoke-ShellCliTestWithRetry -BinaryPath $binary -TestName $name -TimeoutSecs $TimeoutSeconds -RunIgnored:$IncludeIgnored

    switch ($result.Status) {
        "passed" {
            $passed++
            Write-Output "  PASS"
        }
        "ignored" {
            $ignored++
            Write-Output "  IGNORED"
        }
        "timeout" {
            $timeouts += $result
            Write-Output "  TIMEOUT"
        }
        default {
            $failures += $result
            Write-Output "  FAIL"
        }
    }
}

Write-Output ""
Write-Output ("summary: passed={0} ignored={1} failed={2} timed_out={3}" -f $passed, $ignored, $failures.Count, $timeouts.Count)

if ($failures.Count -gt 0) {
    Write-Output ""
    Write-Output "failed tests:"
    foreach ($failure in $failures) {
        Write-Output ("- {0} (exit={1})" -f $failure.Name, $failure.ExitCode)
        if ($failure.Stdout) {
            Write-Output "  stdout:"
            Write-Output ($failure.Stdout.TrimEnd())
        }
        if ($failure.Stderr) {
            Write-Output "  stderr:"
            Write-Output ($failure.Stderr.TrimEnd())
        }
    }
}

if ($timeouts.Count -gt 0) {
    Write-Output ""
    Write-Output "timed out tests:"
    foreach ($timeout in $timeouts) {
        Write-Output ("- {0}" -f $timeout.Name)
    }
}

if ($failures.Count -gt 0 -or $timeouts.Count -gt 0) {
    exit 1
}
