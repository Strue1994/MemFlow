# Restart the persistent local MemFlow agent on port 3000.

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_RUNTIME_ROOT)) {
    Join-Path $projectRoot ".memflow-runtime"
} else {
    $env:MEMFLOW_RUNTIME_ROOT
}

$logDir = Join-Path $runtimeRoot "logs"
$configDir = Join-Path $runtimeRoot "config"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
New-Item -ItemType Directory -Force -Path $configDir | Out-Null

$agentLog = Join-Path $logDir "agent.log"
$agentErrLog = Join-Path $logDir "agent.err.log"
$agentPidFile = Join-Path $logDir "agent.pid"
$agentPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_AGENT_PORT)) { "3000" } else { $env:MEMFLOW_AGENT_PORT }
$executorPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_EXECUTOR_PORT)) { "8082" } else { $env:MEMFLOW_EXECUTOR_PORT }
$executorKey = if ([string]::IsNullOrWhiteSpace($env:EXECUTOR_API_KEY)) { "memflow-local-dev-key" } else { $env:EXECUTOR_API_KEY }

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

Write-Host "=== Restart MemFlow live agent ===" -ForegroundColor Cyan
Write-Host "Project root: $projectRoot"
Write-Host "Runtime root: $runtimeRoot"
Write-Host "Agent port: $agentPort"
Write-Host "Executor URL: http://127.0.0.1:$executorPort"

Stop-PortOwner -Port ([int]$agentPort)

$env:MEMFLOW_RUNTIME_ROOT = $runtimeRoot
$env:MEMFLOW_CRON_CONFIG_PATH = Join-Path $configDir "cron-workflows.json"
$env:MEMFLOW_LLM_SETTINGS_PATH = Join-Path $configDir "llm-settings.json"
$env:EXECUTOR_URL = "http://127.0.0.1:$executorPort"
$env:EXECUTOR_API_KEY = $executorKey
$env:PORT = $agentPort
$env:AUTONOMY_ENABLED = if ([string]::IsNullOrWhiteSpace($env:AUTONOMY_ENABLED)) { "true" } else { $env:AUTONOMY_ENABLED }
$env:AUTO_CRON_WORKFLOWS_ENABLED = if ([string]::IsNullOrWhiteSpace($env:AUTO_CRON_WORKFLOWS_ENABLED)) { "true" } else { $env:AUTO_CRON_WORKFLOWS_ENABLED }

$agentWorkingDirectory = Join-Path $projectRoot "agent-service"
$agentProcess = Start-Process `
    -FilePath "node" `
    -WorkingDirectory $agentWorkingDirectory `
    -ArgumentList "dist/index.js" `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput $agentLog `
    -RedirectStandardError $agentErrLog

$agentProcess.Id | Out-File -FilePath $agentPidFile -Encoding ascii

Wait-ForHttp -Name "Agent settings" -Url "http://127.0.0.1:$agentPort/llm-settings/catalog" -Attempts 20
Wait-ForHttp -Name "Autonomy status" -Url "http://127.0.0.1:$agentPort/autonomy/status" -Attempts 20

Write-Host "Agent restarted successfully."
Write-Host "  PID: $($agentProcess.Id)"
Write-Host "  Log: $agentLog"
Write-Host "  Err: $agentErrLog"
