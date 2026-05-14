# MEMFLOW local development startup script for Windows PowerShell

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

$runtimeRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_RUNTIME_ROOT)) {
    Join-Path $projectRoot ".memflow-runtime"
} else {
    $env:MEMFLOW_RUNTIME_ROOT
}
$logDir = Join-Path $runtimeRoot "logs"
$configDir = Join-Path $runtimeRoot "config"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
New-Item -ItemType Directory -Force -Path $configDir | Out-Null

$executorLog = Join-Path $logDir "executor.log"
$agentLog = Join-Path $logDir "agent.log"
$frontendLog = Join-Path $logDir "frontend.log"
$learningLog = Join-Path $logDir "learning.log"
$cronSeedLog = Join-Path $logDir "cron-seed.log"
$cronRunnerLog = Join-Path $logDir "cron-runner.log"

$executorPidFile = Join-Path $logDir "executor.pid"
$agentPidFile = Join-Path $logDir "agent.pid"
$frontendPidFile = Join-Path $logDir "frontend.pid"
$learningPidFile = Join-Path $logDir "learning.pid"
$cronRunnerPidFile = Join-Path $logDir "cron-runner.pid"

$executorKey = if ([string]::IsNullOrWhiteSpace($env:EXECUTOR_API_KEY)) {
    "memflow-local-dev-key"
} else {
    $env:EXECUTOR_API_KEY
}

$env:EXECUTOR_API_KEY = $executorKey
$executorPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_EXECUTOR_PORT)) { "8082" } else { $env:MEMFLOW_EXECUTOR_PORT }
$agentPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_AGENT_PORT)) { "3300" } else { $env:MEMFLOW_AGENT_PORT }
$frontendPort = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_WEB_PORT)) { "5273" } else { $env:MEMFLOW_WEB_PORT }
$env:MEMFLOW_RUNTIME_ROOT = $runtimeRoot
$env:EXECUTOR_URL = "http://127.0.0.1:$executorPort"
$env:PORT = $agentPort
$env:MEMFLOW_AGENT_PROXY = "http://127.0.0.1:$agentPort"
$env:MEMFLOW_WEB_PORT = $frontendPort
$env:MEMFLOW_WEB_PROXY = "http://127.0.0.1:$executorPort"
$env:MEMFLOW_LLM_SETTINGS_PATH = Join-Path $configDir "llm-settings.json"
$env:MEMFLOW_CRON_CONFIG_PATH = Join-Path $configDir "cron-workflows.json"
$env:MEMFLOW_DISABLE_RATE_LIMIT = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_DISABLE_RATE_LIMIT)) { "true" } else { $env:MEMFLOW_DISABLE_RATE_LIMIT }
$env:AUTONOMY_ENABLED = if ([string]::IsNullOrWhiteSpace($env:AUTONOMY_ENABLED)) { "true" } else { $env:AUTONOMY_ENABLED }
$env:AUTO_LEARNING_ENABLED = if ([string]::IsNullOrWhiteSpace($env:AUTO_LEARNING_ENABLED)) { "false" } else { $env:AUTO_LEARNING_ENABLED }
$env:AUTO_CRON_WORKFLOWS_ENABLED = if ([string]::IsNullOrWhiteSpace($env:AUTO_CRON_WORKFLOWS_ENABLED)) { "true" } else { $env:AUTO_CRON_WORKFLOWS_ENABLED }
$env:LEARNING_INTERVAL_SECONDS = if ([string]::IsNullOrWhiteSpace($env:LEARNING_INTERVAL_SECONDS)) { "300" } else { $env:LEARNING_INTERVAL_SECONDS }
$reuseExecutor = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_REUSE_EXECUTOR)) { "true" } else { $env:MEMFLOW_REUSE_EXECUTOR }
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-msvc"
$cargoTargetRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_CARGO_TARGET_DIR)) {
    Join-Path $runtimeRoot "cargo-target"
} else {
    $env:MEMFLOW_CARGO_TARGET_DIR
}
$stateRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_STATE_DIR)) {
    Join-Path $runtimeRoot "state"
} else {
    $env:MEMFLOW_STATE_DIR
}
New-Item -ItemType Directory -Force -Path $cargoTargetRoot | Out-Null
New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
$env:CARGO_TARGET_DIR = $cargoTargetRoot
$executorDbPath = Join-Path $stateRoot "workflows.db"

function Start-LoggedProcess {
    param(
        [string]$Name,
        [string]$WorkingDirectory,
        [string]$Command,
        [string]$LogFile,
        [string]$PidFile
    )

    $childCommand = "& { $Command } *> '$LogFile'"
    $process = Start-Process `
        -FilePath "powershell.exe" `
        -WorkingDirectory $WorkingDirectory `
        -ArgumentList "-NoProfile", "-Command", $childCommand `
        -PassThru

    $process.Id | Out-File -FilePath $PidFile -Encoding ascii
    Write-Host "    $Name PID: $($process.Id)" -ForegroundColor DarkGray
    Write-Host "    Log: $LogFile" -ForegroundColor DarkGray
    return $process
}

function Wait-ForPort {
    param(
        [string]$Name,
        [int]$Port,
        [System.Diagnostics.Process]$Process,
        [int]$MaxAttempts,
        [string]$LogFile
    )

    Write-Host "    Waiting for $Name..." -ForegroundColor DarkGray

    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        Start-Sleep -Seconds 2

        try {
            $ready = Test-NetConnection -ComputerName 127.0.0.1 -Port $Port -InformationLevel Quiet -ErrorAction SilentlyContinue
            if ($ready) {
                Write-Host "    $Name is ready." -ForegroundColor Green
                return
            }
        } catch {
        }

        if (-not (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
            throw "$Name exited early. Check log: $LogFile"
        }

        if ($attempt % 10 -eq 0) {
            Write-Host "    Still waiting for $Name ($($attempt * 2)s elapsed)..." -ForegroundColor Gray
        }
    }

    throw "$Name did not start within $($MaxAttempts * 2) seconds. Check log: $LogFile"
}

function Test-PortListening {
    param([int]$Port)

    try {
        return Test-NetConnection -ComputerName 127.0.0.1 -Port $Port -InformationLevel Quiet -ErrorAction SilentlyContinue
    } catch {
        return $false
    }
}

function Start-RustBinaryOrCargo {
    param(
        [string]$Name,
        [string]$WorkingDirectory,
        [string]$BinaryPath,
        [string]$CargoCommand,
        [string]$LogFile,
        [string]$PidFile
    )

    if (Test-Path $BinaryPath) {
        return Start-LoggedProcess `
            -Name $Name `
            -WorkingDirectory $WorkingDirectory `
            -Command "& `"$BinaryPath`"" `
            -LogFile $LogFile `
            -PidFile $PidFile
    }

    return Start-LoggedProcess `
        -Name $Name `
        -WorkingDirectory $WorkingDirectory `
        -Command $CargoCommand `
        -LogFile $LogFile `
        -PidFile $PidFile
}

function Stop-IfRunning {
    param([System.Diagnostics.Process]$Process)

    if ($null -ne $Process -and (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
        taskkill /PID $Process.Id /T /F | Out-Null
    }
}

function Wait-ForStableProcess {
    param(
        [string]$Name,
        [System.Diagnostics.Process]$Process,
        [int]$Seconds,
        [string]$LogFile
    )

    Write-Host "    Waiting for $Name warmup..." -ForegroundColor DarkGray
    Start-Sleep -Seconds $Seconds
    if (-not (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
        throw "$Name exited early. Check log: $LogFile"
    }
    Write-Host "    $Name is running." -ForegroundColor Green
}

Write-Host "=== MEMFLOW local development startup ===" -ForegroundColor Cyan
Write-Host "Project root: $projectRoot"
Write-Host "Runtime root: $runtimeRoot"
Write-Host "Executor API key: $executorKey"
Write-Host "Cargo target dir: $cargoTargetRoot"
Write-Host "State dir: $stateRoot"
Write-Host "Executor URL: $($env:EXECUTOR_URL)"
Write-Host "Agent port: $agentPort"
Write-Host "Frontend port: $frontendPort"
Write-Host ""

$executorProcess = $null
$agentProcess = $null
$frontendProcess = $null
$learningProcess = $null
$cronRunnerProcess = $null

try {
    $executorReused = $false
    if ($reuseExecutor -eq "true" -and (Test-PortListening -Port ([int]$executorPort))) {
        $executorReused = $true
        Write-Host "[1/3] Reusing existing Executor on port $executorPort..." -ForegroundColor Yellow
    } else {
        Write-Host "[1/3] Starting Executor (Rust) on port $executorPort..." -ForegroundColor Yellow
        $executorBinary = Join-Path $cargoTargetRoot "debug\executor.exe"
        $executorProcess = Start-RustBinaryOrCargo `
            -Name "Executor" `
            -WorkingDirectory $projectRoot `
            -BinaryPath $executorBinary `
            -CargoCommand "cargo run --package executor -- --db `"$executorDbPath`" serve --addr 127.0.0.1:$executorPort" `
            -LogFile $executorLog `
            -PidFile $executorPidFile
        Write-Host "    First build can take 5-15 minutes. Please wait." -ForegroundColor DarkGray
        Wait-ForPort -Name "Executor" -Port ([int]$executorPort) -Process $executorProcess -MaxAttempts 450 -LogFile $executorLog
    }

    Write-Host "[2/3] Starting Agent service (Node.js) on port $agentPort..." -ForegroundColor Yellow
    $agentProcess = Start-LoggedProcess `
        -Name "Agent" `
        -WorkingDirectory (Join-Path $projectRoot "agent-service") `
        -Command "npm run build; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }; node dist/index.js" `
        -LogFile $agentLog `
        -PidFile $agentPidFile
    Wait-ForPort -Name "Agent service" -Port ([int]$agentPort) -Process $agentProcess -MaxAttempts 90 -LogFile $agentLog

    Write-Host "[3/3] Starting Frontend static server on port $frontendPort..." -ForegroundColor Yellow
    $frontendProcess = Start-LoggedProcess `
        -Name "Frontend" `
        -WorkingDirectory (Join-Path $projectRoot "web-ui") `
        -Command "node ..\scripts\serve-web-ui.js" `
        -LogFile $frontendLog `
        -PidFile $frontendPidFile
    Wait-ForPort -Name "Frontend" -Port ([int]$frontendPort) -Process $frontendProcess -MaxAttempts 45 -LogFile $frontendLog

    if ($env:AUTO_CRON_WORKFLOWS_ENABLED -eq "true") {
        Write-Host "[4/5] Seeding cron-backed workflows..." -ForegroundColor Yellow
        $cronSeedCommand = "node .\scripts\seed-cron-workflows.js"
        & powershell.exe -NoProfile -Command "& { $cronSeedCommand } *> '$cronSeedLog'"
        Write-Host "    Seed log: $cronSeedLog" -ForegroundColor DarkGray

        Write-Host "[5/5] Starting cron workflow runner..." -ForegroundColor Yellow
        $cronRunnerProcess = Start-LoggedProcess `
            -Name "CronRunner" `
            -WorkingDirectory $projectRoot `
            -Command "node .\scripts\cron-runner.js" `
            -LogFile $cronRunnerLog `
            -PidFile $cronRunnerPidFile
        Wait-ForStableProcess -Name "Cron workflow runner" -Process $cronRunnerProcess -Seconds 3 -LogFile $cronRunnerLog
    }

    if ($env:AUTO_LEARNING_ENABLED -eq "true") {
        Write-Host "[Learning] Starting Learning Engine..." -ForegroundColor Yellow
        $learningBinary = Join-Path $cargoTargetRoot "debug\learning-engine.exe"
        $learningProcess = Start-RustBinaryOrCargo `
            -Name "Learning" `
            -WorkingDirectory $projectRoot `
            -BinaryPath $learningBinary `
            -CargoCommand "cargo run --package learning-engine" `
            -LogFile $learningLog `
            -PidFile $learningPidFile
        Wait-ForStableProcess -Name "Learning engine" -Process $learningProcess -Seconds 5 -LogFile $learningLog
    }

    Write-Host ""
    Write-Host "==========================================" -ForegroundColor Cyan
    Write-Host "All services started." -ForegroundColor Green
    if ($executorReused) {
        Write-Host "  Executor: http://127.0.0.1:$executorPort (reused existing process)"
    } else {
        Write-Host "  Executor: http://127.0.0.1:$executorPort (log: $executorLog, PID: $($executorProcess.Id))"
    }
    Write-Host "  Agent:    http://127.0.0.1:$agentPort (log: $agentLog, PID: $($agentProcess.Id))"
    Write-Host "  Frontend: http://127.0.0.1:$frontendPort (log: $frontendLog, PID: $($frontendProcess.Id))"
    if ($learningProcess) {
        Write-Host "  Learning: background loop (log: $learningLog, PID: $($learningProcess.Id))"
    }
    if ($cronRunnerProcess) {
        Write-Host "  Cron:     background runner (log: $cronRunnerLog, PID: $($cronRunnerProcess.Id))"
    }
    Write-Host ""
    Write-Host "Press Ctrl+C to stop all services..." -ForegroundColor Yellow

    $waitIds = @()
    foreach ($process in @($executorProcess, $agentProcess, $frontendProcess, $learningProcess, $cronRunnerProcess)) {
        if ($null -ne $process -and $process.Id -gt 4) {
            $waitIds += $process.Id
        }
    }
    if ($waitIds.Count -gt 0) {
        Wait-Process -Id $waitIds
    }
} finally {
    Write-Host ""
    Write-Host "Stopping services..." -ForegroundColor Cyan
    Stop-IfRunning -Process $cronRunnerProcess
    Stop-IfRunning -Process $learningProcess
    Stop-IfRunning -Process $frontendProcess
    Stop-IfRunning -Process $agentProcess
    if (-not $executorReused) {
        Stop-IfRunning -Process $executorProcess
    }
    Write-Host "Cleanup complete." -ForegroundColor Green
}
