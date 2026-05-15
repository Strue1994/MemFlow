# MemFlow 升级路线图 — Phase A/B/C 任务计划

## TL;DR
> **核心目标**: 基于 AI Agent 生态调研，将 MemFlow 升级为自学习 AI Agent 平台
> **交付物**: 14 个任务分 3 个 Phase
> **估计工作量**: Phase A ~1周 / Phase B ~2周 / Phase C ~3周

## Context

### 调研发现 (2026年5月)
| 项目 | ⭐ | 核心模式 | 对 MemFlow 的启发 |
|------|:--:|----------|-------------------|
| Superpowers | 190K | SKILL.md 方法论体系 | 技能文件标准化,跨平台兼容 |
| Everything CC | 182K | 55 agent + 208 skill | Agent 即配置,市场模式 |
| DeerFlow (字节) | 67K | 长时超 Agent | 9 层中间件 + 子代理隔离 |
| Hermes Curator | 150K | 自学习循环 | 自动技能整理 + 剪枝 |
| Activepieces | 22K | 400+ MCP 服务器 | MCP 生态构建 |
| Cherry Studio | 44K | CherryClaw | Soul 人格 + 内置调度器 |

### 当前 MemFlow 状态
- Rust 工作流引擎(独特优势)
- 6 平台消息通道
- 基础路由 + setup 流程(已完成)
- SkillManager + learning-engine crate
- MCP server(仅有 server 端)
- 循环修复完毕

## 执行策略

### Wave 分组
```
Phase A — Wave 1 (Foundation, 4 任务并行):
  A1: SKILL.md 兼容层 + 技能编排 [quick]
  A2: MCP 客户端 + 自动发现 [unspecified-high]
  A3: 自学习闭环 [deep]
  A4: 上下文压缩系统 [unspecified-high]

Phase B — Wave 2 (Reliability, 4 任务并行):
  B1: 子代理本地实现 [deep]
  B2: 沙箱执行环境 [unspecified-high]
  B3: Checkpoints v2 状态持久化 [unspecified-high]
  B4: DeerFlow 式中件链 [unspecified-high]

Phase C — Wave 3 (Ecosystem, 6 任务并行):
  C1: 通道扩展 6→20+ [unspecified-high]
  C2: 技能市场 + agentskills.io 兼容 [quick]
  C3: Agent 安全扫描器 [unspecified-high]
  C4: TypeScript Agent SDK [unspecified-high]
  C5: Agent 定义配置化 [writing]
  C6: 可观测性 + 追踪 [unspecified-high]
```

依赖: C2 依赖 A1, C3 依赖 B2, C5 依赖 A1

---

## Phase A — 基础补全(速赢, ~1周)

### A1. SKILL.md 兼容层 — 借用 Superpowers 方法论体系
- 创建 skill-loader.ts: 递归扫描 skill 目录,解析 YAML frontmatter
- 扩展 SkillManager: importSkill(path) 导入外部 SKILL.md
- 安装 Superpowers 核心技能: brainstorming, writing-plans, TDD, debugging, verification
- 修改 assembleSystemPrompt() 注入匹配技能
- 添加 POST /skills/import 端点
- 引用: github.com/obra/superpowers/tree/main/skills, agent-service/src/skill-system.ts

**QA 场景**:
1. curl POST /skills/import url=superpowers TDD skill → 200 + skills 列表包含 TDD
2. curl POST /agent/execute → 不 crash

### A2. MCP 客户端 — 让 Agent 调用外部 MCP 工具
- 创建 mcp-client.ts: 支持 stdio 和 SSE 传输
- 配置文件自动发现 ~/.memflow/mcp-config.json
- 将 MCP 工具暴露为 Tool 接口
- 在 agent-loop 中注入 MCP 工具
- 添加 GET/POST /mcp/servers API
- 引用: spec.modelcontextprotocol.io, agent-service/src/mcp-server.ts

**QA 场景**:
1. POST /mcp/servers 注册 test MCP server → 工具列表可见
2. agent 执行时能发现 MCP 工具

### A3. 自学习闭环 — Hermes Curator 精简版
- 在 learning-engine 创建 curator.rs: 监听 agent 执行完成事件
- 每次执行提取: 任务类型,成功/失败,耗时,输出摘要
- 写入 skills/learned/ 目录
- curator.ts: 定时扫描 + LLM 评分 + 合并相似 + 剪枝
- 添加 POST /curator/run, GET /curator/status, GET /curator/report
- 引用: learning-engine/src/, agent-service/src/skill-system.ts

**QA 场景**:
1. POST /curator/run → 返回处理记录数
2. GET /curator/report → 返回 JSON 报告

### A4. 上下文压缩 — DeerFlow 式三层递减
- 创建 context-compressor.ts
- Level 1 Tool Output Budget: >2000 tokens 外移
- Level 2 Microcompact: >8000 tokens 压缩旧轮次
- Level 3 Full Compact: >15000 tokens LLM 压缩
- 添加 POST /compact API
- 引用: github.com/bytedance/deer-flow/pull/1844

**QA 场景**:
1. POST /compact → 200 OK
2. 压缩后对话上下文保持连贯

---

## Phase B — 可靠性(~2周)

### B1. 子代理本地实现
- executor/src/subagent.rs: 独立 workflow 实例
- spawn/wait/cancel 三接口
- agent-service/src/subagent-tools.ts: 通过 executor HTTP API 调用
- 并发 max 3, 超时 15 分钟
- GET /subagents/status

**QA 场景**: 启动子代理,等待结果,查看状态

### B2. 沙箱执行环境
- Sandbox trait: LocalSandbox + DockerSandbox
- 规则引擎: 命令白名单,路径限制,网络限制
- 集成到子代理执行

**QA 场景**: 沙箱阻止危险命令

### B3. Checkpoints v2
- checkpoint.rs: 每次迭代后保存状态快照
- 自动恢复: 启动时检测未完成 session
- 磁盘保护: 1GB 上限

**QA 场景**: 创建 checkpoint, 重启, 自动恢复

### B4. 中件链
- middleware-chain.ts: before/after 钩子
- 7 层中间件(参考 DeerFlow)
- 每层可独立启用/禁用

**QA 场景**: 列出和配置中间件

---

## Phase C — 生态(~3周)

### C1. 通道扩展 6→20+
- 新增 Discord, Signal, Google Chat, Email, Teams, Matrix 等
- 每次 2-3 个 adapter

### C2. 技能市场
- agentskills.io 兼容
- GitHub repo → SKILL.md + metadata

### C3. 安全扫描器
- 5 类规则: 密钥泄露,权限越界,Hook 注入,MCP 风险,配置审计

### C4. TypeScript Agent SDK
- 从 sdk/typescript 扩展

### C5. Agent 定义配置化
- 外置 agent 行为为 YAML/TOML

### C6. 可观测性
- OpenTelemetry 全链路追踪

---

## Verification Strategy
- 每个 task 完成后: bash/curl QA 场景
- 每个 Phase 完成后: 全链路集成测试
- 零人工干预,全部 agent 执行

## Commit Strategy
- 每个任务独立 atomic commit
- Phase A 完成 → tag phase-a-complete
- Phase B 完成 → tag phase-b-complete
- Phase C 完成 → tag phase-c-complete
