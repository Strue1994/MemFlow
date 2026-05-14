# 任务 A6：并发执行限制

## 目标
限制同时执行的工作流数量，防止资源耗尽（如文件句柄、数据库连接）。

## 文件
- `executor/src/concurrency.rs`（新建）
- `executor/src/http_server.rs`（集成限流器）
- `executor/src/lib.rs`（执行前获取许可）

## 具体要求

### 1. 并发限制器
使用 `tokio::sync::Semaphore` 实现：
- 全局静态 `Semaphore::new(max_concurrent)`。
- 每个工作流执行前 `acquire()`，完成后 `release()`。
- 最大并发数可配置（环境变量 `MAX_CONCURRENT_WORKFLOWS`，默认 10）。

### 2. HTTP 层队列
当所有许可被占用时，新请求应等待（有超时）或立即返回 429（Too Many Requests）。采用等待策略，设置等待超时 5 秒。

### 3. 指标暴露
增加 Gauge 指标 `concurrent_workflows` 显示当前并发数。

## 验收标准
- 同时发起超过限制的请求，只有限制数量的请求立即执行，其余排队或超时。
- 并发数不超过配置值。
- 压力测试下系统稳定。