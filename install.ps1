#!/usr/bin/env pwsh
# MemFlow Windows 一键安装脚本
# 支持: Windows 10/11, Windows Server 2019/2022
# 使用: .\install.ps1 [-Mode docker|binary] [-DataDir C:\memflow-data]

param(
    [ValidateSet('docker', 'binary')]
    [string]$Mode = 'docker',
    [string]$DataDir = "$env:USERPROFILE\memflow-data",
    [switch]$SkipRust,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

function Write-Step([string]$msg) { Write-Host "`n>>> $msg" -ForegroundColor Cyan }
function Write-OK([string]$msg)   { Write-Host "    [OK] $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "    [!!] $msg" -ForegroundColor Yellow }
function Write-Fail([string]$msg) { Write-Host "    [ERR] $msg" -ForegroundColor Red; exit 1 }

function Test-Command([string]$cmd) {
    return $null -ne (Get-Command $cmd -ErrorAction SilentlyContinue)
}

Write-Host @"
╔══════════════════════════════════════╗
║   MemFlow 安装向导 (Windows)         ║
║   模式: $($Mode.PadRight(27))║
╚══════════════════════════════════════╝
"@ -ForegroundColor Cyan

# ─── 环境检测 ──────────────────────────────────────────────
Write-Step "检测系统环境"
$os = [System.Environment]::OSVersion.Version
Write-OK "Windows $($os.Major).$($os.Minor)"

if ($Mode -eq 'docker') {
    # Docker 模式
    Write-Step "检查 Docker"
    if (-not (Test-Command 'docker')) {
        Write-Warn "未找到 Docker，正在跳转下载页..."
        Start-Process "https://docs.docker.com/desktop/windows/install/"
        Write-Fail "请先安装 Docker Desktop，然后重新运行此脚本"
    }
    $dockerVer = docker version --format '{{.Server.Version}}' 2>$null
    Write-OK "Docker $dockerVer"

    if (-not (Test-Command 'docker-compose') -and -not (docker compose version 2>$null)) {
        Write-Fail "未找到 docker compose，请升级 Docker Desktop"
    }

} else {
    # Binary 模式
    Write-Step "检查 Rust 工具链"
    if (-not (Test-Command 'cargo')) {
        if ($SkipRust) {
            Write-Fail "需要 Rust，请先安装: https://rustup.rs"
        }
        Write-Warn "Rust 未安装，正在安装..."
        $rustupUrl = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
        $rustupExe = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe -UseBasicParsing
        & $rustupExe -y --default-toolchain stable --profile minimal
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    }
    $rustVer = rustc --version
    Write-OK $rustVer

    Write-Step "检查 Node.js"
    if (-not (Test-Command 'node')) {
        Write-Warn "Node.js 未安装，正在跳转下载页..."
        Start-Process "https://nodejs.org/en/download/"
        Write-Fail "请先安装 Node.js 18+，然后重新运行此脚本"
    }
    $nodeVer = node --version
    Write-OK "Node.js $nodeVer"
}

# ─── 数据目录 ──────────────────────────────────────────────
Write-Step "准备数据目录: $DataDir"
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
New-Item -ItemType Directory -Force -Path "$DataDir\sqlite" | Out-Null
New-Item -ItemType Directory -Force -Path "$DataDir\logs" | Out-Null
Write-OK "目录创建完成"

# ─── 环境变量文件 ──────────────────────────────────────────
$envFile = Join-Path (Get-Location) ".env"
if (-not (Test-Path $envFile) -or $Force) {
    Write-Step "生成 .env 配置文件"
    $adminKey = [System.Guid]::NewGuid().ToString()
    $pgPass    = [System.Guid]::NewGuid().ToString("N").Substring(0, 16)
    $exampleFile = Join-Path (Get-Location) ".env.example"
    if (Test-Path $exampleFile) {
        Copy-Item $exampleFile $envFile
        # 注入生成的值
        (Get-Content $envFile) `
            -replace 'EXECUTOR_API_KEY=.*',        "EXECUTOR_API_KEY=$adminKey" `
            -replace 'POSTGRES_PASSWORD=.*',       "POSTGRES_PASSWORD=$pgPass" `
            -replace 'DATABASE_URL=.*postgres.*',  "DATABASE_URL=postgres://postgres:$pgPass@localhost:5432/memflow" `
            -replace 'DATA_DIR=.*',                "DATA_DIR=$($DataDir -replace '\\', '/')" |
            Set-Content $envFile
    } else {
        @"
EXECUTOR_API_KEY=$adminKey
POSTGRES_PASSWORD=$pgPass
DATABASE_URL=postgres://postgres:$pgPass@localhost:5432/memflow
REDIS_URL=redis://localhost:6379
DATA_DIR=$($DataDir -replace '\\', '/')
RUST_LOG=info
MAX_CONCURRENT_WORKFLOWS=20
LOG_LEVEL=info
"@ | Set-Content $envFile
    }
    Write-OK ".env 已生成，Admin Key: $adminKey"
    Write-Warn "请妥善保存 Admin Key！"
} else {
    Write-OK ".env 已存在，跳过生成 (使用 -Force 覆盖)"
}

# ─── 构建 / 安装 ───────────────────────────────────────────
if ($Mode -eq 'docker') {
    Write-Step "构建 Docker 镜像"
    docker compose build
    Write-OK "镜像构建完成"
} else {
    Write-Step "编译 Rust 后端"
    $env:RUSTFLAGS = "-C target-cpu=native"
    cargo build --release -p executor
    Write-OK "后端编译完成 -> target\release\executor.exe"

    Write-Step "安装前端依赖并构建"
    Push-Location web-ui
    npm install
    npm run build
    Pop-Location
    Write-OK "前端构建完成 -> web-ui\dist\"
}

# ─── 完成 ──────────────────────────────────────────────────
Write-Host @"

╔══════════════════════════════════════╗
║   安装完成！                         ║
╠══════════════════════════════════════╣
║ 启动服务:  .\start.ps1               ║
║ 停止服务:  .\start.ps1 -Stop         ║
║ 查看日志:  .\start.ps1 -Logs         ║
╚══════════════════════════════════════╝
"@ -ForegroundColor Green
