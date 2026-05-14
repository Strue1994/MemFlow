# 任务 CC1：实现 memflow-cli 工具

## 目标
开发一个 Rust CLI 工具 `memflow-cli`，提供与 Agent 服务交互的命令行接口，支持环境检查、工作流执行、日志查看等核心操作，并输出结构化 JSON。

## 文件
- `cli/`（新建 Rust 项目，使用 `clap`、`reqwest`、`serde_json`、`colored`、`tokio`）
- `cli/src/main.rs`（入口，子命令分发）
- `cli/src/commands/doctor.rs`（环境检查）
- `cli/src/commands/run.rs`（执行工作流）
- `cli/src/commands/logs.rs`（查看日志）
- `cli/src/commands/list.rs`（列出工作流）
- `cli/src/config.rs`（读取 `~/.memflow/config.toml`）
- `scripts/install-cli.sh`（一键安装脚本）

## 具体要求

### 1. 全局配置
- 支持配置文件 `~/.memflow/config.toml`：
  ```toml
  api_url = "http://localhost:3000"
  api_key = "your-api-key"
  output = "json"  # 或 "pretty"
  ```
- 环境变量 `MEMFLOW_API_URL`、`MEMFLOW_API_KEY` 可覆盖。

### 2. `doctor` 命令
- 检查 `api_url` 是否可访问（发送健康检查请求）。
- 验证 `api_key` 是否有效（调用 `/workflows` 看是否返回 401）。
- 检查本地依赖（如 Docker、`jq` 可选）并报告版本。
- 输出彩色表格或 JSON。

### 3. `run` 命令
- 用法：`memflow-cli run <workflow_id> [--params '{"key":"value"}'] [--json]`
- 调用 `POST /execute`，等待完成，打印结果。
- 支持 `--async` 模式，只返回 `request_id`。

### 4. `logs` 命令
- 用法：`memflow-cli logs <workflow_id> [--limit 20] [--follow]`
- 调用 `GET /logs`（需执行引擎或 log-collector 提供该接口），实时输出日志流。

### 5. 输出格式
- 默认输出为彩色可读格式（`colored` 库）。
- `--json` 标志输出纯 JSON，便于脚本解析。

## 验收标准
- 安装脚本可在 Ubuntu / macOS / Windows (Git Bash) 上一键安装。
- `doctor` 命令能正确检测常见配置错误。
- `run` 命令执行工作流并返回结果，与 `curl` 调用 Agent API 一致。
- 所有命令支持 `--help`。

## 预估 token
3000