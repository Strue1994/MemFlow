# 任务 CC3：容器优先部署与开发环境

## 目标
提供官方 Docker 镜像和 `docker-compose` 开发环境，使 MemFlow 能在 5 分钟内启动完整服务栈（执行引擎、Agent、PostgreSQL、Redis、ClickHouse、Grafana）。

## 文件
- `Dockerfile.executor`（多阶段构建，优化镜像大小）
- `Dockerfile.agent`
- `Dockerfile.webui`
- `docker-compose.yml`（生产级配置，包含所有依赖）
- `docker-compose.dev.yml`（开发环境，带热重载）
- `Containerfile`（兼容 podman）
- `scripts/quick-start.sh`

## 具体要求

### 1. 镜像优化
- 执行引擎镜像：使用 `rust:1.75-alpine` 构建，最终镜像基于 `alpine:3.18`，体积 < 50MB。
- Agent 镜像：基于 `node:20-alpine`，体积 < 150MB。
- Web UI 镜像：基于 `nginx:alpine`，提供静态文件。

### 2. 开发环境
- `docker-compose.dev.yml` 包含：
  - 执行引擎（挂载源码，使用 `cargo watch` 热重载）
  - Agent 服务（挂载源码，使用 `nodemon`）
  - PostgreSQL、Redis、ClickHouse、Grafana、Prometheus
- 提供 `make dev` 或 `./scripts/dev.sh` 一键启动。

### 3. 健康检查与依赖等待
- 使用 `wait-for-it.sh` 或 `dockerize` 确保服务启动顺序。
- 所有服务配置 `healthcheck`。

### 4. 文档
- 在 `README.md` 添加"使用 Docker 快速启动"章节。
- 提供 `docs/deployment.md` 详细说明生产部署（包括反向代理、HTTPS）。

## 验收标准
- 执行 `docker-compose up -d` 后，访问 `http://localhost:3000` 可看到 Agent 服务响应。
- 执行 `docker-compose -f docker-compose.dev.yml up` 后，修改 Rust 代码可自动重启执行引擎。
- 镜像总大小 < 500MB。

## 预估 token
2500