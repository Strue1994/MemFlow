# MemFlow API Reference

Base URL: `http://localhost:3000` (或配置的 `MEMFLOW_URL`)

## 认证

通过 `X-API-Key` header 传递 API 密钥：

```
X-API-Key: your-executor-api-key
```

## 端点

### 创建工作流 (简单模式)

```http
POST /create_workflow
Content-Type: application/json

{
  "name": "my-workflow",
  "n8n_json": { /* n8n workflow JSON */ }
}
```

### 创建工作流 (多阶段流水线)

```http
POST /create_workflow_v2
Content-Type: application/json

{
  "user_request": "创建一个每天抓取 RSS 推送到飞书的工作流",
  "step": 1
}
```

响应:
```json
{
  "step": 1,
  "session_id": "ws_1234567890",
  "message": "需求已收到...",
  "suggested_patterns": [...]
}
```

### 执行工作流

```http
POST /execute
Content-Type: application/json

{
  "workflowId": "workflow-123",
  "params": { "key": "value" }
}
```

### 列出工作流

```http
GET /workflows
```

### 获取工作流详情

```http
GET /workflows/{id}
```

### 提交反馈

```http
POST /feedback
Content-Type: application/json

{
  "pattern_id": "P001",
  "user_request": "创建定时任务",
  "accepted": true,
  "modifications": null
}
```

## 错误响应

```json
{
  "error": "Error message description"
}
```

状态码:
- `400` - 请求参数错误
- `500` - 服务器内部错误
- `502` - 执行引擎不可用