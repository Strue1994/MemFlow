# P1-2: HTTP Node Retry & Circuit Breaker

## Priority

P1 - Near-term

## Key Files / Modules

- `executor/src/http.rs`
- `compiler/src/ir.rs`

## Goals

提高工作流在瞬态网络故障下的健壮性，防止雪崩。

## Specific Requirements

1.  **Retry Configuration**
   - 节点配置支持 `retry` 字段
   - 默认重试 3 次，可配置
   - 指数退避策略 (1s, 2s, 4s)

2.  **Circuit Breaker**
   - 连续失败 N 次后打开熔断
   - 熔断期间直接标记失败
   - 自动恢复 (默认 30s)

3.  **Logging**
   - 熔断器状态变化记录日志
   - 重试次数和结果记录日志

## Acceptance Criteria

- [ ] 第一次请求超时，第二次成功，观察到重试生效
- [ ] 连续失败后跳过请求，返回熔断错误

## Implementation

```rust
pub struct HttpClient {
    client: reqwest::Client,
    retry_config: RetryConfig,
    circuit_breaker: CircuitBreaker,
}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    state: AtomicU8, // CLOSED=0, OPEN=1, HALF_OPEN=2
    last_failure: AtomicU64,
    threshold: u32,
    recovery_timeout: Duration,
}

impl HttpClient {
    pub async fn execute(&self, req: Request) -> Result<Response, HttpError> {
        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return Err(HttpError::CircuitOpen);
        }
        
        let mut last_error = None;
        for attempt in 0..self.retry_config.max_retries + 1 {
            match self.client.send(req.clone()).await {
                Ok(rsp) => {
                    self.circuit_breaker.record_success();
                    return Ok(rsp);
                }
                Err(e) => {
                    last_error = Some(e);
                    self.circuit_breaker.record_failure();
                    if attempt < self.retry_config.max_retries {
                        sleep(Duration::from_millis(
                            self.retry_config.backoff_ms * 2_u64.pow(attempt)
                        )).await;
                    }
                }
            }
        }
        Err(last_error.unwrap())
    }
}
```