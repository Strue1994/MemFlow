# 任务 PERF-02：执行引擎内存池

## 目标
使用对象池复用 `Environment`，减少内存分配。

## 文件
- `executor/src/memory_pool.rs`（新建）

## 具体要求
- 使用 `lazy_static` + `Mutex` 实现
- 可配置池大小 (`ENV_POOL_SIZE`)
- 提供 `acquire()` 和 `release()` 方法

## 验收标准
- 内存分配减少 80%

## 预估 token
1800