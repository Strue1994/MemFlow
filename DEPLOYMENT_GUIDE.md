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
# 编辑 .env 填写至少一个 LLM API key

# 启动所有服务
docker compose up -d

# 检查状态
docker compose ps
```

### 2. 配置 LLM Provider

```bash
# 检查服务健康
curl http://localhost:3000/health

# 添加 OpenAI provider
curl -X POST http://localhost:3000/providers \
  -H "Content-Type: application/json" \
  -d '{"id":"openai","apiKey":"sk-...","enabled":true}'

# 或者添加 Anthropic
curl -X POST http://localhost:3000/providers \
  -H "Content-Type: application/json" \
  -d '{"id":"anthropic","apiKey":"sk-ant-...","enabled":true}'

# 验证设置
curl http://localhost:3000/providers
```

### 3. 运行第一个任务

```bash
curl -X POST http://localhost:3000/agent/execute \
  -H "Content-Type: application/json" \
  -d '{"text":"Hello, what can you do?"}'
```

## 服务架构

| 服务 | 端口 | 说明 |
|------|:----:|------|
| **agent-service** | 3000 | 主 API 网关，LLM 路由，技能系统 |
| **executor** | 8082 | Rust 工作流执行引擎 |
| **memory-hub** | 8081 | 持久化存储 + 语义搜索 |
| **web-ui** | 5273 | React 可视化编辑器 |
| **learning-engine** | 8083 | 分析与优化引擎 |
| **gateway** | 8084 | 消息通道网关 |

## 环境变量

### 必需
| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` | OpenAI API key |
| 或 `ANTHROPIC_API_KEY` | Anthropic API key |
| 或 `GROQ_API_KEY` | Groq API key |
| 或 `DEEPSEEK_API_KEY` | DeepSeek API key |

### 安全
| 变量 | 默认值 | 说明 |
|------|--------|------|
| `AUTH_ENABLED` | `true` | 启用 API 认证 |
| `API_KEYS` | — | 逗号分隔的 API keys |
| `RATE_LIMIT_RPM` | `100` | 每 IP 每分钟请求数 |
| `CORS_ORIGIN` | `http://localhost:3000` | 允许的跨域来源 |
| `ENCRYPTION_KEY` | — | AES-256-GCM 加密密钥 |

### 运行时
| 变量 | 默认值 | 说明 |
|------|--------|------|
| `PORT` | `3000` | agent-service 端口 |
| `EXECUTOR_URL` | `http://127.0.0.1:8082` | Executor 地址 |
| `MEMORY_HUB_URL` | `http://127.0.0.1:8081` | Memory hub 地址 |
| `EXECUTOR_API_KEY` | `memflow-local-dev-key` | 内部通信密钥 |
| `LOG_LEVEL` | `info` | 日志级别 |
| `RUST_LOG` | `info` | Rust 日志级别 |

## Docker 部署

### 构建镜像

```bash
# 构建所有服务
docker compose build

# 或单独构建
docker build -f Dockerfile.executor -t memflow/executor .
docker build -f Dockerfile.agent -t memflow/agent-service .
```

### 生产部署

```bash
# 使用生产配置
docker compose -f docker-compose.yml up -d

# 查看日志
docker compose logs -f agent-service

# 更新服务
docker compose pull && docker compose up -d
```

### 数据持久化

Docker Compose 使用两个 volume：
- `memflow_data` — executor + learning-engine 数据
- `memflow_memory` — memory-hub 数据

## Kubernetes 部署

项目提供 Helm chart：

```bash
helm upgrade --install memflow ./deploy/helm/memflow \
  --set agent.service.port=3000 \
  --set secrets.apiKey=your-key
```

配置项见 `deploy/helm/memflow/values.yaml`

## 验证部署

```bash
# 健康检查
curl http://localhost:3000/health
# → {"status":"ok","uptime_s":123}

# 依赖检查
curl http://localhost:3000/ready
# → {"ready":true,"executor":true,"memory_hub":true}

# 存活检查
curl http://localhost:3000/live
# → {"live":true}

# 指标
curl http://localhost:3000/metrics
# → Prometheus 格式指标

# 安全扫描
curl -X POST http://localhost:3000/security/scan
# → {"totalFindings":...,"findings":[...]}

# 备份
curl -X POST http://localhost:3000/backup
# → {"path":".memflow-runtime/backups/2026-05-15-..."}
```

## Chat 通道配置

### Telegram
```bash
curl -X POST http://localhost:3000/channels \
  -H "Content-Type: application/json" \
  -d '{"id":"telegram","enabled":true,"config":{"botToken":"123:ABC"}}'
```

### Discord
```bash
curl -X POST http://localhost:3000/channels \
  -d '{"id":"discord","enabled":true,"config":{"botToken":"MT23_..."}}'
```

### Slack
```bash
curl -X POST http://localhost:3000/channels \
  -d '{"id":"slack","enabled":true,"config":{"token":"xoxb-..."}}'
```

## 运维命令

```bash
# 查看所有 API 端点
curl http://localhost:3000/health

# 查看路由配置
curl http://localhost:3000/router/config

# 触发 curator 技能整理
curl -X POST http://localhost:3000/curator/run

# 查看 curator 状态
curl http://localhost:3000/curator/status

# 保存 checkpoint
curl -X POST http://localhost:3000/checkpoints/save \
  -d '{"sessionId":"my-session","messages":[]}'

# 查看 metrics
curl http://localhost:3000/metrics

# 查看 traces
curl http://localhost:3000/traces
```

## 故障排查

| 症状 | 可能原因 | 修复 |
|------|----------|------|
| agent-service 启动失败 | 端口 3000 占用或依赖缺失 | `netstat -an \| findstr :3000` 检查端口 |
| executor 连接失败 | executor 未启动或端口不对 | 检查 `EXECUTOR_URL` 配置 |
| LLM 调用返回空 | API key 未配置或额度不足 | `GET /providers` 检查配置 |
| Auth 返回 401 | 未配置 API key 且 AUTH_ENABLED=true | 设置 `API_KEYS` 或 `AUTH_ENABLED=false` |
| Rate limit 429 | 请求太频繁 | 等待 60 秒或调高 `RATE_LIMIT_RPM` |

## 安全建议

1. **修改默认 API key**：设置 `EXECUTOR_API_KEY` 和 `API_KEYS` 环境变量
2. **启用 HTTPS**：使用反向代理（nginx/caddy）或 Helm Ingress 配置 TLS
3. **限制 CORS**：设置 `CORS_ORIGIN` 为具体域名，不要使用 `*`
4. **加密敏感数据**：设置 `ENCRYPTION_KEY` 环境变量
5. **定期备份**：使用 `POST /backup` 端点
6. **安全扫描**：定期运行 `POST /security/scan`
