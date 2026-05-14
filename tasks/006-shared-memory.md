# 任务 006：共享内存优化（可选）

## 目标
优化 Agent 与执行引擎之间的参数传递，使用共享内存代替 JSON 序列化，减少开销。

## 文件
- `executor/src/shmem.rs`（新建）
- `agent-service/src/shmemClient.ts`（新建）

## 具体要求

### 1. Rust 端实现
使用 `shared_memory` crate。
- 函数 `fn create_shared_memory(name: &str, size: usize) -> Result<Shmem, ShmemError>`
- 函数 `fn write_params(shmem: &mut Shmem, params: &Value) -> Result<(), ShmemError>`
- 函数 `fn read_result(shmem: &Shmem) -> Result<Value, ShmemError>`

### 2. TypeScript 端实现
使用 Node.js `ffi-napi` 或 `node-shared-memory`（如果没有合适的库，可使用子进程传递文件描述符的 fallback）。
简化方案：使用 UNIX 域 socket 传递文件描述符，Rust 端监听 socket。

### 3. 修改 CLI 支持共享内存模式
添加 `--shmem <name>` 参数，从共享内存读取参数，结果写回共享内存。

## 验收标准
- 性能测试：传递 1KB 参数 1000 次，平均耗时 < 1ms（不含执行时间）。
- 与 JSON 传参方式结果一致。

## 预期输出格式
##FILE:executor/src/shmem.rs
```rust
// 共享内存实现
```
##FILE:executor/src/main.rs
```rust
// 增加 --shmem 处理
```
##FILE:agent-service/src/shmemClient.ts
```typescript
// 共享内存客户端（可选，若复杂可先跳过）
```