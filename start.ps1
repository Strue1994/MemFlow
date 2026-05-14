#!/usr/bin/env pwsh
# MemFlow Windows 启动脚本
# 支持: Docker 模式 / 本地 Binary 模式
# 使用:
#   .\start.ps1                  # Docker 模式启动
#   .\start.ps1 -Mode binary     # 本地 Binary 模式启动
#   .\start.ps1 -Stop            # 停止所有服务
#   .\start.ps1 -Logs            # 查看实时日志
#   .\start.ps1 -Status          # 查看服务状态

param(
    [ValidateSet('docker', 'binary')]
    [string]$Mode = 'docker',
    [switch]$Stop,
    [switch]$Logs,
    [switch]$Status,
    [switch]$Rebuild,
    [string]$Port = '8080',
    [string]$DataDir = "$env:USERPROFILE\memflow-data"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Write-Step([string]$msg) { Write-Host "`n>>> $msg" -ForegroundColor Cyan }
function Write-OK([string]$msg)   { Write-Host "    [OK] $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "    [!!] $msg" -ForegroundColor Yellow }

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

# ─── 加载 .env ────────────────────────────────────────────
$envFile = Join-Path $ScriptDir ".env"
if (Test-Path $envFile) {
    Get-Content $envFile | Where-Object { $_ -match '^\s*[^#].*=.*' } | ForEach-Object {
        $k, $v = $_ -split '=', 2
        if (-not [System.Environment]::GetEnvironmentVariable($k.Trim())) {
            [System.Environment]::SetEnvironmentVariable($k.Trim(), $v.Trim(), 'Process')
        }
    }
    Write-OK ".env 已加载"
} else {
    Write-Warn ".env 未找到，建议先运行 .\install.ps1"
}

# ─── 停止模式 ─────────────────────────────────────────────
if ($Stop) {
    Write-Step "停止服务"
    if ($Mode -eq 'docker') {
        docker compose down
    } else {
        Get-Process -Name "executor" -ErrorAction SilentlyContinue | Stop-Process -Force
        Get-Process -Name "node" -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like "*memflow*" } | Stop-Process -Force
    }
    Write-OK "服务已停止"
    exit 0
}

# ─── 日志模式 ─────────────────────────────────────────────
if ($Logs) {
    if ($Mode -eq 'docker') {
        docker compose logs -f --tail=100
    } else {
        $logFile = Join-Path $DataDir "logs\executor.log"
        if (Test-Path $logFile) { Get-Content $logFile -Wait -Tail 50 }
        else { Write-Warn "日志文件未找到: $logFile" }
    }
    exit 0
}

# ─── 状态模式 ─────────────────────────────────────────────
if ($Status) {
    Write-Step "服务状态"
    if ($Mode -eq 'docker') {
        docker compose ps
    } else {
        $proc = Get-Process -Name "executor" -ErrorAction SilentlyContinue
        if ($proc) { Write-OK "executor 运行中 (PID: $($proc.Id))" }
        else { Write-Warn "executor 未运行" }
    }
    # 健康检查
    try {
        $health = Invoke-RestMethod "http://localhost:$Port/health" -TimeoutSec 3
        Write-OK "API 健康: $($health.status)"
    } catch {
        Write-Warn "API 无响应"
    }
    exit 0
}

# ─── 启动 ─────────────────────────────────────────────────
Write-Host @"
╔══════════════════════════════════════╗
║   MemFlow 启动中 ($Mode 模式)
╚══════════════════════════════════════╝
"@ -ForegroundColor Cyan

if ($Mode -eq 'docker') {
    # Docker 模式
    if ($Rebuild) {
        Write-Step "重新构建镜像"
        docker compose build --no-cache
    }
    Write-Step "启动 Docker Compose 服务"
    docker compose up -d
    Write-Step "等待服务就绪..."
    $maxWait = 60
    $waited  = 0
    while ($waited -lt $maxWait) {
        try {
            $health = Invoke-RestMethod "http://localhost:$Port/health" -TimeoutSec 2
            if ($health.status -eq 'healthy') { break }
        } catch { }
        Start-Sleep -Seconds 2
        $waited += 2
        Write-Host "." -NoNewline
    }
    Write-Host ""

    if ($waited -ge $maxWait) {
        Write-Warn "服务启动超时，请检查日志: .\start.ps1 -Logs"
    } else {
        Write-OK "服务已就绪 ($waited 秒)"
    }

} else {
    # Binary 模式（本地开发/轻量部署）
    New-Item -ItemType Directory -Force -Path "$DataDir\sqlite" | Out-Null
    New-Item -ItemType Directory -Force -Path "$DataDir\logs"   | Out-Null

    # 检查二进制是否存在
    $exePath = Join-Path $ScriptDir "target\release\executor.exe"
    if (-not (Test-Path $exePath)) {
        Write-Step "二进制未找到，开始编译..."
        cargo build --release -p executor
    }

    Write-Step "启动 Executor 后端"
    $logFile = "$DataDir\logs\executor.log"
    $env:DATABASE_PATH = "$DataDir\sqlite\memflow.db"
    $env:BIND_ADDR     = "0.0.0.0:$Port"

    $procArgs = @{
        FilePath               = $exePath
        RedirectStandardOutput = $logFile
        RedirectStandardError  = "$DataDir\logs\executor-err.log"
        NoNewWindow            = $true
        PassThru               = $true
    }
    $proc = Start-Process @procArgs
    Write-OK "后端已启动 (PID: $($proc.Id))"

    # 前端 dev server（开发模式）
    if (Test-Path (Join-Path $ScriptDir "web-ui\dist")) {
        Write-OK "前端静态文件已内嵌到后端服务"
    } elseif (Test-Path (Join-Path $ScriptDir "web-ui\package.json")) {
        Write-Step "启动前端开发服务器"
        $webUiDir = Join-Path $ScriptDir "web-ui"
        Start-Process -FilePath "npm" -ArgumentList "run dev" -WorkingDirectory $webUiDir -NoNewWindow
        Write-OK "前端开发服务器启动: http://localhost:5173"
    }
}

# ─── 完成 ─────────────────────────────────────────────────
Write-Host @"

╔══════════════════════════════════════╗
║   MemFlow 已启动！                   ║
╠══════════════════════════════════════╣
║  Web UI:    http://localhost:$Port
║  API:       http://localhost:$Port/api
║  健康检查:  http://localhost:$Port/health
║  Prometheus: http://localhost:$Port/metrics
╠══════════════════════════════════════╣
║  停止:  .\start.ps1 -Stop            ║
║  日志:  .\start.ps1 -Logs            ║
║  状态:  .\start.ps1 -Status          ║
╚══════════════════════════════════════╝
"@ -ForegroundColor Green
