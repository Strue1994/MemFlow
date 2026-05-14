# Restart the persistent local MemFlow executor on port 8082.

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_RUNTIME_ROOT)) {
    Join-Path $projectRoot ".memflow-runtime"
} else {
    $env:MEMFLOW_RUNTIME_ROOT
}

$logDir = Join-Path $runtimeRoot "logs"
$stateDir = Join-Path $runtimeRoot "state"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
New-Item -ItemType Directory -Force -Path $stateDir | Out-Null

$executorLog = Join-Path $logDir "executor.log"
$executorErrLog = Join-Path $logDir "executor.err.log"
$executorPidFile = Join-Path $logDir "executor.pid"
$executorPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_EXECUTOR_PORT)) { "8082" } else { $env:MEMFLOW_EXECUTOR_PORT }
$executorDbPath = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_EXECUTOR_DB_PATH)) {
    Join-Path $env:TEMP "memflow-live-workflows.db"
} else {
    $env:MEMFLOW_EXECUTOR_DB_PATH
}
$executorBinaryCandidates = @()
if (-not [string]::IsNullOrWhiteSpace($env:MEMFLOW_EXECUTOR_BINARY)) {
    $executorBinaryCandidates += $env:MEMFLOW_EXECUTOR_BINARY
}
$executorBinaryCandidates += @(
    (Join-Path $env:TEMP "memflow-cargo-target-v6\release\executor.exe"),
    (Join-Path $projectRoot "target\release\executor.exe"),
    (Join-Path (Split-Path -Parent $projectRoot) "MemFlow\target\release\executor.exe")
)
$executorBinary = $executorBinaryCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

function Get-ListeningPid {
    param([int]$Port)

    $line = netstat -ano -p TCP | Select-String ":$Port" | Where-Object { $_.Line -match "LISTENING" } | Select-Object -First 1
    if (-not $line) {
        return $null
    }

    $parts = ($line.Line -replace "\s+", " ").Trim().Split(" ")
    if ($parts.Length -lt 5) {
        return $null
    }

    return $parts[-1]
}

function Stop-PortOwner {
    param([int]$Port)

    $portOwnerPid = Get-ListeningPid -Port $Port
    if (-not $portOwnerPid) {
        return
    }

    if ($portOwnerPid -match "^\d+$") {
        taskkill /PID $portOwnerPid /T /F | Out-Null
        Write-Host "Stopped existing port owner PID $portOwnerPid on port $Port"
    }
}

function Wait-ForHttp {
    param(
        [string]$Name,
        [string]$Url,
        [int]$Attempts
    )

    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        Start-Sleep -Seconds 2
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 5
            if ($response.StatusCode -eq 200) {
                Write-Host "$Name ready at $Url"
                return
            }
        } catch {
        }
    }

    throw "$Name did not become ready: $Url"
}

if (-not $executorBinary) {
    throw "Executor binary not found. Checked: $($executorBinaryCandidates -join ', ')"
}

Write-Host "=== Restart MemFlow live executor ===" -ForegroundColor Cyan
Write-Host "Project root: $projectRoot"
Write-Host "Runtime root: $runtimeRoot"
Write-Host "Executor binary: $executorBinary"
Write-Host "Executor port: $executorPort"

Stop-PortOwner -Port ([int]$executorPort)
Write-Host "Executor DB: $executorDbPath"

$executorProcess = Start-Process `
    -FilePath $executorBinary `
    -WorkingDirectory $projectRoot `
    -ArgumentList @("--db", $executorDbPath, "serve", "--addr", "127.0.0.1:$executorPort") `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput $executorLog `
    -RedirectStandardError $executorErrLog

$executorProcess.Id | Out-File -FilePath $executorPidFile -Encoding ascii

Wait-ForHttp -Name "Executor root" -Url "http://127.0.0.1:$executorPort/" -Attempts 20

Write-Host "Executor restarted successfully."
Write-Host "  PID: $($executorProcess.Id)"
Write-Host "  Log: $executorLog"
Write-Host "  Err: $executorErrLog"
