# MemFlow 部署指南

## 环境要求

| 组件 | 最低版本 | 推荐版本 |
|------|---------|---------|
| Docker | 20.10+ | 24.0+ |
| Docker Compose | 2.0+ | 2.24+ |
| 内存 | 4GB | 8GB+ |
| CPU | 2 核 | 4 核 |

## 快速部署

### 1. 一键启动

```bash
# 克隆项目
git clone https://github.com/your-repo/MemFlow.git
cd MemFlow

# 创建环境配置文件
cp .env.example .env

# 启动所有服务
docker compose up -d

# 检查状态
docker compose ps
```

### 2. 访问服务

| 服务 | 地址 | 说明 |
|------|------|------|
| Executor | http://localhost:8080 | API 服务 |
| 前端 | http://localhost:80 | Web UI |
| Redis | localhost:6379 | 缓存 |
| PostgreSQL | localhost:5432 | 数据库 |

## 详细配置

### 环境变量

在 `.env` 文件中配置：

```bash
# ─── 必需配置 ─────────────────────────────────────
# 至少选择一个 LLM Provider
OPENAI_API_KEY=sk-xxx      # OpenAI
# ANTHROPIC_API_KEY=xxx    # Claude (二选一)

# ─── 可选配置 ─────────────────────────────────
PORT=8080                 # API 端口
EXECUTOR_API_KEY=your-key  # 管理密钥

# ─── 数据库 (使用 SQLite 默认) ─────────────────
DATABASE_URL=sqlite:///data/memflow.db
```

### 启动模式

```bash
# 完整模式 (executor + agent + frontend + redis + postgres)
docker compose up -d

# 无外部依赖 (仅 executor + sqlite)
docker compose up -d --profile minimal

# 开发模式 (含前端热重载)
docker compose --profile dev up
```

## 云平台部署

### Zeabur (推荐)

1. 推送镜像到 GitHub Container Registry
2. 在 Zeabur 控制台创建新服务
3. 选择 Docker 容器
4. 配置环境变量:
   - `OPENAI_API_KEY`
   - `EXECUTOR_API_KEY`

### Railway

1. 连接 GitHub 仓库
2. 设置环境变量
3. 点击 Deploy

### Render

1. 新建 Web Service
2. 选择 Docker
3. 配置 `DATABASE_URL` (PostgreSQL)

### 自签Docker

```bash
# 构建镜像
docker build -f Dockerfile.executor -t memflow-executor .

# 运行
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  -e OPENAI_API_KEY=sk-xxx \
  -e EXECUTOR_API_KEY=your-key \
  memflow-executor
```

## 验证部署

```bash
# 健康检查
curl http://localhost:8080/health

# 响应示例:
# {"status":"ok","version":"0.1.0"}

# Prometheus 指标
curl http://localhost:8080/metrics
```

## 常用命令

```bash
# 查看日志
docker compose logs -f executor

# 重启服务
docker compose restart executor

# 更新服务
docker compose pull
docker compose up -d

# 停止
docker compose down

# 数据备份
cp data/memflow.db data/memflow.db.backup
```

## 故障排查

### 服务无法启动

```bash
# 查看日志
docker compose logs executor

# 检查端口占用
netstat -an | grep 8080
```

### 数据库问题

```bash
# 重置数据库
rm data/memflow.db
docker compose restart executor
```

### API Key 错误

确保 `.env` 文件中的 `OPENAI_API_KEY` 正确，且有足够额度。

## 安全建议

1. 修改默认 `EXECUTOR_API_KEY`
2. 使用 HTTPS (配置 SSL 证书)
3. 限制 CORS 来源
4. 定期备份数据

## 监控

健康检查端点: `GET /health`
指标端点: `GET /metrics` (Prometheus 格式)