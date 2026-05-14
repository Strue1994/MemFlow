# 任务 CC2-03：子 Agent 与多 Agent 协作

## 目标
实现类似 Claude Code 的 `AgentTool`，允许一个工作流异步创建子 Agent 执行子任务，并支持多个 Agent 并行协作。

## 文件
- `executor/src/sub_agent.rs`（新建）
- `compiler/src/ir.rs`（添加 `SpawnAgent` 和 `JoinAgent` 指令）

## 具体要求

### 1. 子 Agent 定义
- 子 Agent 拥有独立的上下文和工具集（可限制）。
- 主 Agent 传递任务描述和期望的输出格式。
- 子 Agent 执行完毕后返回结果，主 Agent 继续。

### 2. 异步执行
- `SpawnAgent` 不阻塞主工作流，立即返回 `agent_id`。
- 主工作流可在后续节点等待子 Agent 完成（`JoinAgent` 指令）。

### 3. 团队协作
- 提供 `CreateTeam` 指令，创建多个 Agent 并行处理不同子任务。
- 所有子任务完成后，聚合结果。

### 4. 资源管理
- 限制最大并发子 Agent 数量（可配置）。
- 子 Agent 执行超时自动终止。

## 测试用例
- 主工作流拆分为两个子 Agent：一个抓取网页，一个分析情感，最后合并结果。

## 验收标准
- 子 Agent 执行结果正确。
- 并行执行时间小于串行执行时间。
- 错误隔离：子 Agent 崩溃不影响主 Agent。

## 预估 token
3200