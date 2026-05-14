# 任务 PERF-01：工作流编译缓存

## 目标
对工作流编译结果（n8n JSON → IR）增加 Redis 缓存，相同 JSON 直接复用，避免重复解析和编译。

## 文件
- `compiler/src/cache.rs`（新建）
- `executor/src/workflow_registry.rs`（集成缓存）

## 具体要求

### 1. 缓存键设计
- 键名：`workflow:compiled:{sha256(n8n_json)}`
- 值：序列化后的 `Workflow` IR
- TTL：7 天

### 2. 缓存流程
- 在 `parse_n8n_workflow` 之前，查询 Redis
- 若命中，直接返回；否则编译后存入
- 环境变量 `WORKFLOW_CACHE_ENABLED` 开关

### 3. 失效策略
- 工作流被修改时，删除对应缓存

## 验收标准
- 同一工作流第二次编译速度提升 10 倍
- 缓存命中率 > 60%

## 预估 token
2000