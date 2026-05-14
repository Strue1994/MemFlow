$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = if ([string]::IsNullOrWhiteSpace($env:MEMFLOW_RUNTIME_ROOT)) {
    Join-Path $projectRoot ".memflow-runtime"
} else {
    $env:MEMFLOW_RUNTIME_ROOT
}
$logDir = Join-Path $runtimeRoot "logs"

Write-Host "Stopping all MEMFLOW local services..." -ForegroundColor Cyan

foreach ($pidFile in @(
    (Join-Path $logDir "executor.pid"),
    (Join-Path $logDir "agent.pid"),
    (Join-Path $logDir "frontend.pid"),
    (Join-Path $logDir "learning.pid"),
    (Join-Path $logDir "cron-runner.pid")
)) {
    if (-not (Test-Path $pidFile)) {
        continue
    }

    $processId = (Get-Content $pidFile -Raw).Trim()
    if ($processId) {
        taskkill /PID $processId /T /F | Out-Null
        Write-Host "  Stopped PID $processId"
    }

    Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
}

Write-Host "Done." -ForegroundColor Green
