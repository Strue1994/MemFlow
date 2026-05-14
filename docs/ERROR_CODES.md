# MemFlow Error Codes

All API errors return a structured format:
```json
{
  "code": "ERROR_CODE",
  "message": "Human readable message"
}
```

## Error Codes Reference

### Workflow Errors

| Code | HTTP Status | Description | Example |
|------|-------------|-------------|----------|
| `WORKFLOW_NOT_FOUND` | 404 | Workflow doesn't exist | `{ "workflow_id": "unknown" }` in message |
| `WORKFLOW_ALREADY_EXISTS` | 409 | Duplicate workflow ID | |
| `WORKFLOW_INVALID` | 400 | Invalid workflow JSON | Missing required node |
| `CYCLE_DETECTED` | 400 | Circular dependency | Nodes form a cycle |

### Node Execution Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `NODE_EXECUTION_FAILED` | 500 | Node failed to execute | 
| `NODE_NOT_FOUND` | 404 | Unknown node type |
| `NODE_TIMEOUT` | 408 | Node exceeded timeout |
| `NODE_INVALID_PARAMS` | 400 | Invalid node parameters |

### HTTP Node Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `HTTP_ERROR` | 502 | HTTP request failed |
| `HTTP_TIMEOUT` | 504 | Request timeout |
| `SSRF_BLOCKED` | 403 | Private IP blocked |

### Authentication & Security

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Missing API key |
| `FORBIDDEN` | 403 | Invalid API key |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `SECURITY_ERROR` | 403 | Security violation |

### System Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INTERNAL_ERROR` | 500 | Unexpected error |
| `DB_ERROR` | 500 | Database operation failed |
| `PARSE_ERROR` | 400 | JSON parse failed |
| `VALIDATION_ERROR` | 400 | Input validation failed |
| `TIMEOUT` | 408 | Operation timeout |

## Response Examples

```json
// 404 Not Found
{
  "code": "WORKFLOW_NOT_FOUND",
  "message": "Workflow 'my-workflow' not found"
}

// 429 Too Many Requests  
{
  "code": "RATE_LIMIT_EXCEEDED",
  "message": "Rate limit exceeded. Try again later."
}

// 403 Forbidden
{
  "code": "SSRF_BLOCKED",
  "message": "URL 'http://192.168.1.1:8080' is not allowed (SSRF protection)"
}
```