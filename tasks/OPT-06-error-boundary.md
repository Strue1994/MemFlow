# OPT-06: Add Error Boundary and Recovery

## 目标

添加错误边界和自动恢复机制，提高系统稳定性。

## 当前状态

错误处理分散，可能导致服务崩溃。

## 实现方案

1. **添加错误边界中间件**
   ```rust
   pub async fn error_boundary(
       f: impl Future<Output = Result<Response>>,
   ) -> Result<Response> {
       match f.await {
           Ok(rsp) => Ok(rsp),
           Err(e) => {
               log::error!("Error in request handler: {}", e);
               
               let error_response = ErrorResponse {
                   code: ErrorCode::InternalError,
                   message: e.to_string(),
                   request_id: Uuid::new_v4().to_string(),
               };
               
               Ok(Response::builder()
                   .status(StatusCode::INTERNAL_SERVER_ERROR)
                   .json(error_response)?)
           }
       }
   }
   ```

2. **添加重试机制**
   ```rust
   use tokio::time::{sleep, Duration};
   
   pub async fn retry_with_backoff<F, T, E>(
       mut f: F,
       max_retries: u32,
   ) -> Result<T>
   where
       F: FnMut() -> impl Future<Output = Result<T, E>,
       E: std::fmt::Debug,
   {
       let mut last_error = None;
       for attempt in 0..max_retries {
           match f().await {
               Ok(t) => return Ok(t),
               Err(e) => {
                   last_error = Some(e);
                   let delay = Duration::from_secs(2u64.pow(attempt));
                   sleep(delay).await;
               }
           }
       }
       Err(last_error.unwrap())
   }
   ```

3. **添加断路器**
   ```rust
   pub struct CircuitBreaker {
       failures: AtomicUsize,
       last_failure: AtomicU64,
       state: AtomicU8,
       threshold: usize,
   }
   
   impl CircuitBreaker {
       pub fn new(threshold: usize) -> Self { ... }
       
       pub fn call<T>(&self, f: impl FnOnce() -> T) -> Option<T> {
           if self.state.load(Ordering::SeqCst) == 2 {
               return None;  // Circuit open
           }
           let result = f();
           if is_error(&result) {
               self.record_failure();
           } else {
               self.record_success();
           }
           Some(result)
       }
   }
   ```

## 影响文件

- `executor/src/http_server.rs`
- `executor/src/lib.rs`

## 验证方法

模拟故障，确认自动恢复。

## 优先级

MEDIUM - 稳定性提升