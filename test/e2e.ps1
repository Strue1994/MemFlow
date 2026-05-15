<#
.E2E Verification Script for MemFlow
Tests: setup, provider, MCP, curator, checkpoints, security, middleware, skills, backup
Requires: services running on localhost:3000
#>

$BASE = "http://localhost:3000"
$passed = 0
$failed = 0

function Test-Endpoint($name, $method, $path, $body, $expectStatus = 200, $expectKey = $null) {
    try {
        $params = @{ Uri = "$BASE$path"; Method = $method; UseBasicParsing = $true }
        if ($body) { $params.Headers = @{ "Content-Type" = "application/json" }; $params.Body = ($body | ConvertTo-Json -Compress) }
        $r = Invoke-WebRequest @params -TimeoutSec 10
        if ($r.StatusCode -eq $expectStatus) {
            if ($expectKey) {
                $data = $r.Content | ConvertFrom-Json
                if ($null -ne $data.$expectKey) { Write-Host "  ✅ $name"; $script:passed++ }
                else { Write-Host "  ❌ $name (missing key '$expectKey')"; $script:failed++ }
            } else { Write-Host "  ✅ $name"; $script:passed++ }
        } else { Write-Host "  ❌ $name (status $($r.StatusCode))"; $script:failed++ }
    } catch { Write-Host "  ❌ $name ($($_.Exception.Message))"; $script:failed++ }
}

Write-Host "`n=== MemFlow E2E Verification ===" -ForegroundColor Cyan

# 1. Health
Test-Endpoint "GET /health" "GET" "/health" $null 200 "status"
Test-Endpoint "GET /live" "GET" "/live" $null 200 "live"

# 2. Setup
Test-Endpoint "GET /setup/status" "GET" "/setup/status" $null 200 "needsSetup"

# 3. Skills
Test-Endpoint "GET /skills" "GET" "/skills" $null 200 "skills"

# 4. SKILL.md import
Test-Endpoint "POST /skills/import" "POST" "/skills/import" @{dir="."} 200 "imported"

# 5. Marketplace
Test-Endpoint "GET /marketplace/list" "GET" "/marketplace/list" $null 200 "listings"

# 6. Middleware
Test-Endpoint "GET /middleware/config" "GET" "/middleware/config" $null 200 "middlewares"

# 7. Router
Test-Endpoint "GET /router/config" "GET" "/router/config" $null 200 "config"
Test-Endpoint "POST /router/config" "POST" "/router/config" @{mode="manual"; manualTier="expert"} 200 "config"

# 8. Curator
Test-Endpoint "GET /curator/status" "GET" "/curator/status" $null 200 "totalRecords"

# 9. Checkpoints
Test-Endpoint "POST /checkpoints/save" "POST" "/checkpoints/save" @{sessionId="e2e-test"; messages=@(@{role="user";content="e2e"})} 200 "checkpoint"
Test-Endpoint "GET /checkpoints/latest" "GET" "/checkpoints/latest" $null 200 "checkpoint"

# 10. Backup
Test-Endpoint "POST /backup" "POST" "/backup" $null 200 "path"

# 11. Security scan
Test-Endpoint "POST /security/scan" "POST" "/security/scan" @{} 200 "totalFindings"

# 12. Tracing
Test-Endpoint "GET /traces" "GET" "/traces" $null 200 "sessions"

# 13. Metrics
Test-Endpoint "GET /metrics" "GET" "/metrics" $null 200

# 14. Agent Config
Test-Endpoint "GET /agents/config" "GET" "/agents/config" $null 200 "agents"

# 15. Channels
Test-Endpoint "GET /channels" "GET" "/channels" $null 200 "channels"

# 16. Providers
Test-Endpoint "GET /providers" "GET" "/providers" $null 200 "providers"

# 17. MCP servers
Test-Endpoint "GET /mcp/servers" "GET" "/mcp/servers" $null 200 "configs"

# 18. Router stats
Test-Endpoint "GET /router/stats" "GET" "/router/stats" $null 200 "totalCalls"

# Summary
Write-Host "`n=== Results ===" -ForegroundColor Cyan
Write-Host "Passed: $passed" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor Red
if ($failed -eq 0) { exit 0 } else { exit 1 }
