# P0-1: Activate Memory Hub as Real Service

## Priority

P0 - Immediate

## Key Files / Modules

- `memory-hub/src/main.rs`
- `memory-hub/src/layered_memory.rs`
- `memory-hub/src/recall_enhancer.rs`
- `memory-hub/src/graph_memory.rs`

## Goals

将 `memory-hub` 从占位符变为可用的记忆服务，接入已实现的 `LayeredMemory`、`decay` 等模块。

## Specific Requirements

1.  **Initialize LayeredMemory instance** - 在 `main.rs` 中初始化 `LayeredMemory` 实例。

2.  **Implement `/memories` API (POST)**
   - 将记忆写入分层存储（热/温/冷）
   - 支持字段: `content`, `type`, `importance` (0-1), `metadata` (optional)

3.  **Implement `/memories/search` API (GET)**
   - 调用 `recall_enhancer` 进行向量检索
   - 返回 TOP-K 相关记忆
   - 支持参数: `q` (query), `k` (count, default 5)

4.  **Persistence**
   - 存储路径可配置
   - 重启后数据不丢失

## Acceptance Criteria

- [ ] 能通过 API 添加记忆
- [ ] 能检索到之前添加的记忆
- [ ] 服务重启后数据仍存在

## Implementation Hints

```rust
// main.rs 核心改动
use memory_hub::{LayeredMemory, RecallEnhancer};

async fn add_memory(Json(payload): Json<MemoryRequest>) -> Json<Value> {
    let memory = Memory::new(
        payload.content,
        payload.memory_type,
        payload.importance,
    );
    memory_hub.add(memory).await;
    Json(json!({ "id": memory.id }))
}

async fn search_memories(Query(params): Query<SearchParams>) -> Json<Value> {
    let results = memory_hub.search(&params.q, params.k).await;
    Json(json!({ "memories": results }))
}
```