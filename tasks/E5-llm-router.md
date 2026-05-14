# 任务 E5：LLM Router

## 目标
实现智能路由器，根据任务复杂度、预算、延迟要求，自动选择最合适的 LLM（GPT-4o、Claude、Gemini、本地模型），并缓存常见生成结果。

## 文件
- `agent-service/src/llm_router.ts`（新建）
- `agent-service/src/llm_clients/`（封装各厂商 API）
- `agent-service/src/cache.ts`（Redis 缓存）
- `agent-service/src/index.ts`（集成路由器）

## 具体要求

### 1. 路由策略
- **简单任务**（如模式匹配、表达式生成）→ 本地 7B 模型（OpenCode / Llama）
- **中等任务**（如单步工作流生成）→ Claude 3 Haiku / GPT-3.5
- **复杂任务**（如多节点、条件分支、错误修复）→ GPT-4o / Claude 3 Opus
- 支持用户设置 `max_budget` 和 `max_latency` 偏好。

### 2. 缓存机制
- 对相同的 `(task_type, input_hash)` 缓存结果，TTL 24 小时。
- 使用 Redis 存储，降低重复调用成本。

### 3. 成本追踪
- 记录每次调用的模型、token 数、成本，写入 ClickHouse。
- 提供 `/cost/breakdown` 接口，展示每日/每周成本。

### 4. 故障转移
- 若首选模型超时或出错，自动切换到备用模型（如 GPT-4o → Claude）。

## 验收标准
- 路由器能根据预设规则选择正确模型。
- 缓存命中率 > 40%。
- 成本追踪数据准确。

## 预估 token
2500