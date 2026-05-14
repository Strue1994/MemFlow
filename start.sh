#!/usr/bin/env bash
# MemFlow Linux/macOS 启动/停止/日志脚本
# 支持: Docker 模式 / 本地 Binary 模式
# 使用:
#   ./start.sh                  # Docker 模式启动（默认）
#   ./start.sh --mode binary    # Binary 模式启动
#   ./start.sh --stop           # 停止所有服务
#   ./start.sh --logs           # 查看实时日志
#   ./start.sh --status         # 查看服务状态
#   ./start.sh --restart        # 重启服务
#   ./start.sh --rebuild        # 重新构建并启动（Docker 模式）

set -euo pipefail

MODE="docker"
STOP=false
SHOW_LOGS=false
STATUS=false
RESTART=false
REBUILD=false
PORT="${PORT:-8080}"
DATA_DIR="${DATA_DIR:-${HOME}/memflow-data}"

while [[ $# -gt 0 ]]; do
  case $1 in
    --mode)    MODE="$2"; shift 2 ;;
    --stop)    STOP=true; shift ;;
    --logs)    SHOW_LOGS=true; shift ;;
    --status)  STATUS=true; shift ;;
    --restart) RESTART=true; shift ;;
    --rebuild) REBUILD=true; shift ;;
    *) echo "未知参数: $1"; exit 1 ;;
  esac
done

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'; BOLD='\033[1m'
step() { echo -e "\n${CYAN}>>> $1${NC}"; }
ok()   { echo -e "    ${GREEN}[OK]${NC} $1"; }
warn() { echo -e "    ${YELLOW}[!!]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# ─── 加载 .env ────────────────────────────────────────────
if [[ -f "$SCRIPT_DIR/.env" ]]; then
  # shellcheck disable=SC2046
  export $(grep -v '^\s*#' "$SCRIPT_DIR/.env" | grep '=' | xargs) 2>/dev/null || true
  ok ".env 已加载"
else
  warn ".env 未找到，建议先运行: bash install.sh"
fi

# ─── docker compose 命令检测 ──────────────────────────────
COMPOSE_CMD="docker compose"
if ! docker compose version &>/dev/null; then
  if command -v docker-compose &>/dev/null; then
    COMPOSE_CMD="docker-compose"
  else
    [[ "$MODE" == "docker" ]] && { echo -e "${RED}[ERR]${NC} 未找到 docker compose，请先安装"; exit 1; }
  fi
fi

# ─── 停止 ─────────────────────────────────────────────────
if [[ "$STOP" == "true" ]]; then
  step "停止服务"
  if [[ "$MODE" == "docker" ]]; then
    $COMPOSE_CMD down
  else
    pkill -f "target/release/executor" 2>/dev/null && ok "executor 已停止" || warn "executor 未在运行"
    pkill -f "vite" 2>/dev/null && ok "frontend dev server 已停止" || true
  fi
  ok "服务已停止"
  exit 0
fi

# ─── 重启 ─────────────────────────────────────────────────
if [[ "$RESTART" == "true" ]]; then
  step "重启服务"
  if [[ "$MODE" == "docker" ]]; then
    $COMPOSE_CMD restart
  else
    "$0" --stop --mode binary 2>/dev/null || true
    exec "$0" --mode binary
  fi
  exit 0
fi

# ─── 日志 ─────────────────────────────────────────────────
if [[ "$SHOW_LOGS" == "true" ]]; then
  if [[ "$MODE" == "docker" ]]; then
    $COMPOSE_CMD logs -f --tail=100
  else
    LOG_FILE="$DATA_DIR/logs/executor.log"
    if [[ -f "$LOG_FILE" ]]; then
      tail -f -n 50 "$LOG_FILE"
    else
      warn "日志文件未找到: $LOG_FILE"
    fi
  fi
  exit 0
fi

# ─── 状态检查 ─────────────────────────────────────────────
if [[ "$STATUS" == "true" ]]; then
  step "服务状态"
  if [[ "$MODE" == "docker" ]]; then
    $COMPOSE_CMD ps
  else
    if pgrep -f "target/release/executor" &>/dev/null; then
      PID=$(pgrep -f "target/release/executor")
      ok "executor 运行中 (PID: $PID)"
    else
      warn "executor 未运行"
    fi
  fi
  # 健康检查
  if curl -sf "http://localhost:$PORT/health" -o /dev/null 2>/dev/null; then
    HEALTH=$(curl -sf "http://localhost:$PORT/health" 2>/dev/null)
    ok "API 健康: $HEALTH"
  else
    warn "API 无响应 (http://localhost:$PORT/health)"
  fi
  exit 0
fi

# ─── 启动 ─────────────────────────────────────────────────
echo -e "${BOLD}${CYAN}
╔══════════════════════════════════════╗
║   MemFlow 启动中 ($MODE 模式)
╚══════════════════════════════════════╝${NC}"

if [[ "$MODE" == "docker" ]]; then
  if [[ "$REBUILD" == "true" ]]; then
    step "重新构建镜像"
    $COMPOSE_CMD build --no-cache
  fi
  step "启动 Docker Compose 服务"
  $COMPOSE_CMD up -d

  step "等待服务就绪..."
  MAX_WAIT=60; WAITED=0
  while [[ $WAITED -lt $MAX_WAIT ]]; do
    if curl -sf "http://localhost:$PORT/health" -o /dev/null 2>/dev/null; then
      echo ""
      ok "服务就绪 (${WAITED}s)"
      break
    fi
    printf "."
    sleep 2; WAITED=$((WAITED + 2))
  done
  [[ $WAITED -ge $MAX_WAIT ]] && warn "启动超时，请检查日志: ./start.sh --logs"

else
  # Binary 模式
  mkdir -p "$DATA_DIR/sqlite" "$DATA_DIR/logs"

  EXEC_BIN="$SCRIPT_DIR/target/release/executor"
  if [[ ! -f "$EXEC_BIN" ]]; then
    step "二进制未找到，开始编译..."
    cargo build --release -p executor
  fi

  step "启动 Executor 后端"
  export DATABASE_PATH="$DATA_DIR/sqlite/memflow.db"
  export BIND_ADDR="0.0.0.0:$PORT"
  nohup "$EXEC_BIN" >> "$DATA_DIR/logs/executor.log" 2>> "$DATA_DIR/logs/executor-err.log" &
  EXEC_PID=$!
  ok "后端已启动 (PID: $EXEC_PID)"
  echo "$EXEC_PID" > "$DATA_DIR/executor.pid"

  # 前端
  if [[ -d "$SCRIPT_DIR/web-ui/dist" ]]; then
    ok "前端静态文件已内嵌到后端"
  elif [[ -f "$SCRIPT_DIR/web-ui/package.json" ]]; then
    step "启动前端开发服务器"
    (cd "$SCRIPT_DIR/web-ui" && nohup npm run dev >> "$DATA_DIR/logs/frontend.log" 2>&1 &)
    ok "前端开发服务器: http://localhost:5173"
  fi
fi

echo -e "${BOLD}${GREEN}
╔══════════════════════════════════════╗
║   MemFlow 已启动！                   ║
╠══════════════════════════════════════╣
║  Web UI:    http://localhost:$PORT
║  健康检查:  http://localhost:$PORT/health
║  Prometheus: http://localhost:$PORT/metrics
╠══════════════════════════════════════╣
║  停止:  ./start.sh --stop            ║
║  日志:  ./start.sh --logs            ║
║  状态:  ./start.sh --status          ║
╚══════════════════════════════════════╝${NC}"

