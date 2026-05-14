# 任务 A5：执行超时控制

## 目标
为每个工作流执行设置最大超时时间，防止长时间运行或死循环耗尽资源。

## 文件
- `executor/src/lib.rs`（修改 `execute` 增加超时检查）
- `executor/src/http_server.rs`（支持请求级超时）
- `executor/src/main.rs`（CLI 添加 `--timeout` 参数）
- `agent-service/src/index.ts`（传递 timeout 参数）

## 具体要求

### 1. 超时机制
- 在执行开始时记录 `start_time`。
- 在每个指令执行前检查 `elapsed >= timeout`，若超时则返回 `ExecError::Timeout`。
- 默认超时 30 秒，可通过参数或工作流配置覆盖。

### 2. 异步支持
由于执行引擎目前是同步的，可以使用 `tokio::time::timeout` 将整个执行包裹为异步任务：
```rust
tokio::time::timeout(Duration::from_secs(timeout_secs), async {
    // 执行工作流
}).await.map_err(|_| ExecError::Timeout)?;
```

### 3. HTTP 层超时
在 Axum 中配置请求超时中间件，确保长时间请求被切断。

### 4. Agent 传递超时
`/execute` 请求体增加 `timeout_seconds` 字段，默认 30。

## 验收标准
- 超过超时时间的工作流被强制终止并返回超时错误。
- 超时不影响其他并发工作流。
- 资源被正确释放。