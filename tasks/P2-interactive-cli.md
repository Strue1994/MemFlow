# 任务 P2：交互式 CLI

## 目标
提供命令行工具 `memflow-cli`，支持 REPL 环境、快速测试工作流、查看日志、手动触发学习循环。

## 文件
- `cli/`（新建 Rust 项目，使用 `clap` 和 `rustyline`）
- `cli/src/main.rs`
- `cli/src/commands.rs`
- `cli/src/repl.rs`
- `scripts/install-cli.sh`

## 具体要求

### 1. REPL 环境
- 启动后显示提示符 `memflow>`
- 支持命令：
  - `execute <workflow_id> [params_json]`：执行工作流并显示结果。
  - `create "自然语言描述"`：调用 Agent 创建工作流，返回 ID。
  - `list`：列出所有工作流。
  - `logs <workflow_id> [limit]`：查看最近执行日志。
  - `learn`：手动触发学习引擎运行一次闭环。
  - `metrics`：显示 Prometheus 关键指标（缓存命中率、平均延迟）。
  - `exit` 或 `Ctrl+D`：退出。

### 2. 集成现有 API
- 通过 HTTP 调用 Agent 服务（默认 `http://localhost:3000`）。
- 支持环境变量 `MEMFLOW_API_URL` 和 `MEMFLOW_API_KEY`。

### 3. 输出格式化
- JSON 结果自动高亮（使用 `bat` 或 `colored`）。
- 错误信息用红色显示。

### 4. 离线模式（可选）
- 支持直接加载本地工作流 JSON 文件执行，不依赖 Agent 服务。

## 测试用例
- 在 REPL 中输入 `create "fetch https://httpbin.org/get"`，应返回 workflow_id。
- 输入 `execute <workflow_id>`，应显示 HTTP 响应内容。

## 验收标准
- 所有命令可用，响应时间 < 2 秒。
- 安装脚本可一键安装（`curl -fsSL https://memflow.io/install.sh | sh`）。

## 预估 token
2500