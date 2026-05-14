#!/usr/bin/env bash
# E2E: agent-service → executor → memory-hub
set -e

BASE="http://localhost:3300"
EXEC="http://localhost:8082"
MEM="http://localhost:8081"
PASS=0
FAIL=0

green() { echo -e "\033[32m✓ $1\033[0m"; ((PASS++)); }
red() { echo -e "\033[31m✗ $1 — $2\033[0m"; ((FAIL++)); }

echo "=== MemFlow E2E Tests ==="

# 1. Health check
echo "--- Service Health ---"
HEALTH=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/llm-settings" 2>/dev/null)
[ "$HEALTH" = "200" ] && green "Agent service reachable" || red "Agent service" "HTTP $HEALTH"

EXEC_HEALTH=$(curl -s -o /dev/null -w "%{http_code}" "$EXEC/health" 2>/dev/null)
[ "$EXEC_HEALTH" = "200" ] && green "Executor reachable" || red "Executor" "HTTP $EXEC_HEALTH"

MEM_HEALTH=$(curl -s -o /dev/null -w "%{http_code}" "$MEM/stats" 2>/dev/null)
[ "$MEM_HEALTH" = "200" ] && green "Memory hub reachable" || red "Memory hub" "HTTP $MEM_HEALTH"

# 2. Memory store + search
echo "--- Memory ---"
STORE=$(curl -s -X POST "$MEM/memories" -H "Content-Type: application/json" \
  -d '{"content":"User prefers dark mode","type":"UserPreference","importance":0.8}')
[ -n "$STORE" ] && green "Memory stored" || red "Memory store" "$STORE"

SEARCH=$(curl -s "$MEM/memories/search?q=dark+mode&k=1")
echo "$SEARCH" | grep -q "dark" && green "Memory search works" || red "Memory search" "$SEARCH"

# 3. Workflow list
echo "--- Workflows ---"
WFS=$(curl -s "$EXEC/workflows" -H "X-API-Key: test" 2>/dev/null)
[ -n "$WFS" ] && green "Workflow list OK" || red "Workflow list" "$WFS"

# 4. Skills list
echo "--- Skills ---"
SKILLS=$(curl -s "$BASE/skills" 2>/dev/null)
[ -n "$SKILLS" ] && green "Skills list OK" || red "Skills list" "$SKILLS"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && echo "ALL PASSED ✅" || echo "SOME FAILED ❌"
