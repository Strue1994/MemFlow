# P2-1: Memory Hub Layered Persistence

## Priority

P2 - Medium-term

## Key Files / Modules

- `memory-hub/src/layered_memory.rs`
- `memory-hub/src/decay.rs`

## Goals

将记忆分层存储到不同的后端，实现真正的持久化。

## Specific Requirements

1.  **Hot Layer (Memory)**
   - 保持 LRU 缓存用于高频访问
   - 最大容量可配置 (默认 1000 条)

2.  **Warm Layer (Redis)**
   - 短期记忆存储
   - 设置 TTL (默认 7 天)
   - 用于冷热数据迁移

3.  **Cold Layer (PostgreSQL/ClickHouse)**
   - 长期记忆存储
   - 持久化保存
   - 支持向量索引

4.  **Garbage Collection**
   - 后台协程定期清理过期记忆
   - 层间迁移 (热 -> 温 -> 冷)

5.  **Data Load on Restart**
   - 服务重启时从温/冷层加载历史记忆

## Acceptance Criteria

- [ ] 服务重启后，温层和冷层中的记忆能被正确加载
- [ ] 检索时能返回跨层记忆

## Implementation

```rust
pub struct LayeredMemory {
    hot: HotLayer,
    warm: RedisLayer,
    cold: ColdLayer,
    gc_interval: Duration,
}

pub struct HotLayer {
    cache: LruCache<String, Memory>,
    max_size: usize,
}

pub struct RedisLayer {
    client: redis::Client,
    ttl_days: u32,
}

pub struct ColdLayer {
    pool: SqlPool,
}

impl LayeredMemory {
    pub async fn new() -> Self {
        let hot = HotLayer::new(1000);
        let warm = RedisLayer::new("redis://localhost:6379", 7);
        let cold = ColdLayer::new("postgres://localhost/memflow").await;
        Self { hot, warm, cold, gc_interval: Duration::from_hours(1) }
    }
    
    pub async fn add(&self, memory: &Memory) {
        // 热层
        self.hot.put(&memory.id, memory.clone());
        // 温层 (异步)
        tokio::spawn(self.warm.put_async(memory));
        // 冷层 (异步)
        tokio::spawn(self.cold.put_async(memory));
    }
    
    pub async fn search(&self, query: &str, k: usize) -> Vec<Memory> {
        // 合并三层结果并排序
        let mut results = Vec::new();
        results.extend(self.hot.search(query, k));
        results.extend(self.warm.search(query, k).await);
        results.extend(self.cold.search(query, k).await);
        // 按相关性排序，取 top-k
        sort_by_similarity(&mut results, query);
        results.truncate(k);
        results
    }
}
```