# P1-3: Structured Error Code System

## Priority

P1 - Near-term

## Key Files / Modules

- `executor/src/error.rs`
- `compiler/src/error.rs`
- `agent-service/src/middleware/errorHandler.ts`

## Goals

统一 API 错误响应格式，便于客户端程序化处理。

## Specific Requirements

1.  **Error Response Schema**
   ```json
   {
     "code": "ERR_CODE",
     "message": "Human readable",
     "details": {}
   }
   ```

2.  **Error Codes**
   | Code | Description |
   |------|-------------|
   | WORKFLOW_NOT_FOUND | 指定的工作流不存在 |
   | NODE_EXECUTION_FAILED | 节点执行失败 |
   | PARSE_ERROR | JSON 解析失败 |
   | SSRF_BLOCKED | 请求被 SSRF 防护阻止 |
   | RATE_LIMIT_EXCEEDED | 速率限制 |
   | TIMEOUT | 执行超时 |
   | CYCLE_DETECTED | 检测到循环调用 |
   | INVALID_CONFIG | 配置无效 |

3.  **Implementation**
   - Rust: 在 `error.rs` 中定义 `ApiError` 枚举
   - TypeScript: 统一错误中间件转换

## Acceptance Criteria

- [ ] 触发错误的 API 响应符合新格式
- [ ] 错误码文档生成

## Implementation

```rust
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
    
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}
```

```typescript
// agent-service/src/middleware/errorHandler.ts
interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

app.use((err, req, res, next) => {
  const apiError: ApiError = {
    code: err.code || 'INTERNAL_ERROR',
    message: err.message,
    details: err.details
  };
  res.status(err.statusCode || 500).json(apiError);
});
```