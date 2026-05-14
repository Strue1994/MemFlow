# FIX-11: Fix Memory Leak in Workflow Execution

## 问题描述

Workflow 重复执行后，内存使用持续增长，可能是内存泄漏。

## 常见泄漏点

1. **HashMap/Map 不清理**
   ```rust
   // 问题: rated_items 持续增长
   let mut rated_items: HashMap<String, Rating> = HashMap::new();
   rated_items.insert(task_id, rating);
   // 永远不清理
   ```

2. **全局状态累积**
   ```rust
   lazy_static! {
       static ref CACHE: HashMap<String, Value> = HashMap::new();
   }
   // CACHE 持续增长
   ```

3. **事件监听器未移除**

## 修复方案

1. 添加 LRU 缓存限制:
   ```rust
   use lru::LruCache;
   
   let mut cache = LruCache::new(1000);  // 最大 1000 条
   cache.put(key, value);
   ```

2. 添加 TTL 过期:
   ```rust
   pub struct TtlCache<K, V> {
       cache: HashMap<K, (V, Instant)>,
       ttl: Duration,
   }
   
   impl<K: Eq + Hash, V> TtlCache<K, V> {
       pub fn get(&mut self, key: &K) -> Option<&V> {
           self.cache.get(key).and_then(|(v, time)| {
               if time.elapsed() > self.ttl {
                   self.cache.remove(key);
                   None
               } else {
                   Some(v)
               }
           })
       }
   }
   ```

3. 定期清理

## 影响文件

- `executor/src/cache.rs`
- `executor/src/rating.rs`
- `executor/src/memory_pool.rs`

## 验证方法

重复执行同一 workflow 100 次，监控内存使用。

## 优先级

HIGH - 稳定性问题