# MemFlow Phase 2: 架构重构 + 产品化

## TL;DR

> 从"能跑的代码"升级为"可维护的产品"。核心：代码清理 → Rust 迁移 → SDK + 可观测性 → 架构进化。
>
> **关键交付**: 统一 Agent Core(Rust) | gRPC 通信 | SDK | 可观测性 | Webhook 服务 | 技能市场 CLI
> **预估工作量**: L（18 核心任务，4 波次）

---

## 从 Phase 1 继承的现状

`
Phase 1 完成: 35+ 文件, 86/86 测试, cargo check ✅
问题:
  1. executor/lib.rs 有 400+ 行重复代码 (两个 execute_instructions)
  2. TS Agent Loop ↔ Rust Executor 通过 HTTP 通信 (~50ms 延迟)
  3. 无认证中间件 (只有 executor 有 X-API-Key)
  4. 无结构化 tracing / metrics
  5. 无 schema migration
  6. Webhook 注册了但无真实 HTTP 监听器
  7. WASM 插件是 placeholder
`

---

## 执行策略: 4 波次

`
Wave 1 — 代码清理 + 基础设施 (3 并行):
├── P1.1 合并 executor/lib.rs 重复函数 (两个 execute_instructions → 一个)
├── P1.2 统一认证中间件 (agent-service + gateway)
├── P1.3 数据库 migration 框架

Wave 2 — Rust Agent Core + gRPC (4 并行):
├── P2.1 将 Agent Loop 从 TS 迁移到 Rust (agent-core crate)
├── P2.2 gRPC 替换 REST (内部服务间通信)
├── P2.3 Agent Core 直接内嵌 Executor (免 HTTP 延迟)
├── P2.4 Gateway 统一入口 (所有外部请求走 gateway)

Wave 3 — SDK + 可观测性 + Webhook (4 并行):
├── P3.1 TypeScript SDK (npm package)
├── P3.2 Python SDK (pip package)
├── P3.3 OpenTelemetry 可观测性 (traces + metrics + logs)
├── P3.4 真实 Webhook HTTP 监听器

Wave 4 — 产品化 (4 并行):
├── P4.1 WASM 插件真实运行时 (wasmtime)
├── P4.2 skill publish CLI 命令
├── P4.3 结构化 tracing (tracing crate + Jaeger/Zipkin)
├── P4.4 事件驱动异步队列 (NATS/RabbitMQ)

Wave FINAL — 4 路并行审查:
├── F1 架构合规 | F2 代码质量 | F3 端到端 QA | F4 范围检查
`

---

## TODOs

### Wave 1: 代码清理 + 基础设施

- [ ] **P1.1: 合并 executor/lib.rs 重复函数**

  **What**: 当前 xecute_instructions 和 xecute_instructions_with_timeout 是完全重复的 200+ 行。合并为单一函数 + 默认 timeout 参数。

  **文件**: xecutor/src/lib.rs
  **方法**: 保留 xecute_instructions_with_timeout，让 xecute_instructions 调它传 None
  **TDD**: 现有 7 个测试必须全部保持通过

- [ ] **P1.2: 统一认证中间件**

  **What**: 当前 executor 有 X-API-Key，agent-service 没有认证。创建统一 auth 层：
  - gateway 入口检查 JWT 或 API Key
  - 服务间通信使用内部 token
  - RBAC 集成 (security.rs 已有)

- [ ] **P1.3: 数据库 Migration 框架**

  **What**: 创建 migrations/ 目录 + Rust 迁移引擎
  - 支持 SQLite schema 版本管理
  - 自动迁移 (启动时检查并执行)
  - 回滚支持

### Wave 2: Rust Agent Core + gRPC (核心架构变更)

- [ ] **P2.1: Agent Loop 从 TS 迁移到 Rust**

  **What**: 创建 gent-core/ Rust crate
  - 将 gent-loop.ts 的 Think-Act-Observe 逻辑迁移到 Rust
  - 多模型 provider (OpenAI/Anthropic/Ollama) 作为 Rust trait
  - 工具调用的 Rust trait (不再走 HTTP)
  - 内嵌 Executor 直接调用 (零延迟)

  **原因**: 消除 agent-loop.ts ↔ executor 的 HTTP 往返。当前 Agent 每次工具调用都要:
  `
  TS Agent → HTTP → Rust Executor → HTTP → TS Agent
  `
  迁移后:
  `
  Rust Agent → 直接函数调用 → Rust Executor
  `

- [ ] **P2.2: gRPC 替代 REST (服务间通信)**

  **What**: 内部服务间通信从 REST 切到 gRPC
  - proto 定义: proto/memflow.proto
  - gateway → agent-core: gRPC
  - agent-core → memory-hub: gRPC
  - agent-core → learning-engine: gRPC

- [ ] **P2.3: Agent Core 内嵌 Executor**

  **What**: agent-core crate 直接依赖 executor crate，通过函数调用执行工作流，不走 HTTP

- [ ] **P2.4: Gateway 统一入口**

  **What**: 所有外部请求走 gateway (端口 8084):
  - REST API (向前兼容现有 agent-service 路由)
  - WebSocket (实时事件推送)
  - gRPC (内部转发到 agent-core)
  - HTTP 80/443 (Web UI 静态资源)

### Wave 3: SDK + 可观测性

- [ ] **P3.1: TypeScript SDK**

  **What**: sdk/typescript/ — npm package @memflow/sdk
  `	s
  import { MemFlow } from '@memflow/sdk'
  const mf = new MemFlow({ apiKey: '...' })
  await mf.workflows.execute('wf_123')
  await mf.memory.search('dark mode')
  await mf.skills.list()
  `

- [ ] **P3.2: Python SDK**

  **What**: sdk/python/ — pip package memflow-sdk
  同等功能的 Python 版

- [ ] **P3.3: OpenTelemetry 可观测性**

  **What**: 用 	racing-opentelemetry + opentelemetry-otlp 替代零散 logging
  - 所有 crate 统一 tracing
  - Jaeger/Zipkin 导出
  - 自定义 metrics (请求数、延迟、错误率)
  - Grafana 面板

- [ ] **P3.4: 真实 Webhook HTTP 监听器**

  **What**: 当前 webhook.rs 只注册了路径 → 工作流映射，没有真实 HTTP 服务器。
  在 executor 启动一个额外 HTTP listener 接收 webhook 请求并触发工作流

### Wave 4: 产品化

- [ ] **P4.1: WASM 插件真实运行时**

  **What**: 用 wasmtime 替换 plugin.rs 的 stub
  - 安全沙箱 (内存/CPU 限制)
  - 插件注册 API
  - 插件市场格式

- [ ] **P4.2: skill publish CLI 命令**

  `ash
  memflow skill create --from "执行记录"
  memflow skill publish ./skills/my-skill.json
  memflow skill search "web scraping"
  `

- [ ] **P4.3: 结构化 Tracing**

  **What**: 替换零散的 	racing::info!() 调用为结构化 span
  - 每个请求一个 trace
  - 每个 Agent 迭代一个 span
  - 每个工具调用有明细事件

- [ ] **P4.4: 事件驱动异步队列**

  **What**: NATS/RabbitMQ 集成
  - 工作流完成事件
  - 学习周期事件
  - 技能生成事件
  - 解耦 agent-core 和 learning-engine

---

## 差异化定位（推荐 README）

> **MemFlow 不是又一个 AI 聊天助手。**
>
> MemFlow 是 **工作流优先的 AI Agent 平台**——你定义流程，AI 帮你填参数、做决策、自动迭代。
>
> - 不是"对话式 agent"——是"工作流 + agent"混合引擎
> - 不是 Python 胶水——是 Rust 原生性能
> - 不是黑盒——每个决策都可追溯、可优化、可复用

---

## 成功标准

- [ ] executor/lib.rs 0 行重复代码
- [ ] Agent Loop 完全在 Rust 中运行，0 HTTP 往返
- [ ] gRPC 替代内部 REST，延迟降低 10x
- [ ] TS + Python SDK 可安装使用
- [ ] Jaeger/Grafana 可视化所有请求
- [ ] Webhook 真实可用
- [ ] WASM 插件可正常运行
- [ ] skill CLI 可用
- [ ] 86/86 测试继续通过
