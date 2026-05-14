$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$dbPath = Join-Path $env:TEMP "memflow-direct-check.db"
$stdoutPath = Join-Path $env:TEMP "memflow-direct-executor.log"
$stderrPath = Join-Path $env:TEMP "memflow-direct-executor.err.log"
$freshTargetRoot = Join-Path $env:TEMP "memflow-executor-fresh-target"
$exePath = Join-Path $freshTargetRoot "release\executor.exe"

if (-not (Test-Path $exePath)) {
    throw "Missing executor binary: $exePath"
}

$proc = Start-Process `
    -FilePath $exePath `
    -ArgumentList "--db", $dbPath, "serve", "--addr", "127.0.0.1:8092" `
    -WorkingDirectory $projectRoot `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput $stdoutPath `
    -RedirectStandardError $stderrPath

Start-Sleep -Seconds 6

$portOk = Test-NetConnection -ComputerName 127.0.0.1 -Port 8092 -InformationLevel Quiet -ErrorAction SilentlyContinue

try {
    $health = Invoke-RestMethod "http://127.0.0.1:8092/health" -Headers @{ "X-API-Key" = "memflow-local-dev-key" } -TimeoutSec 8
    $healthJson = $health | ConvertTo-Json -Depth 5
} catch {
    if ($_.ErrorDetails) {
        $healthJson = $_.ErrorDetails.Message
    } else {
        $healthJson = $_.Exception.Message
    }
}

[pscustomobject]@{
    Pid = $proc.Id
    Port8092 = $portOk
    Health = $healthJson
    Stdout = (Get-Content $stdoutPath -Tail 20 -ErrorAction SilentlyContinue) -join "`n"
    Stderr = (Get-Content $stderrPath -Tail 20 -ErrorAction SilentlyContinue) -join "`n"
} | Format-List
