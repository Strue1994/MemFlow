# MemFlow Best Practices Guide

## 1. Automate Daily Report

### Scenario
每天定时抓取数据，生成报表并发送邮件。

### Workflow JSON
```json
{
  "name": "Daily Report",
  "nodes": [
    {
      "id": "1",
      "name": "Timer",
      "type": "n8n-nodes-base.schedule",
      "parameters": {
        "rule": {"interval": [{"field": "hours", "hours": 1}]}
      }
    },
    {
      "id": "2",
      "name": "Fetch Data",
      "type": "n8n-nodes-base.httpRequest",
      "parameters": {
        "method": "GET",
        "url": "https://api.example.com/data"
      }
    },
    {
      "id": "3",
      "name": "Transform",
      "type": "n8n-nodes-base.code",
      "parameters": {
        "jsCode": "return { total: items.length, sum: items.reduce((a,b) => a + b.value, 0) }"
      }
    },
    {
      "id": "4",
      "name": "Send Email",
      "type": "n8n-nodes-base.emailSend",
      "parameters": {
        "to": "team@example.com",
        "subject": "Daily Report"
      }
    }
  ],
  "connections": {
    "1": {"main": [{"node": "2", "type": "main"}]},
    "2": {"main": [{"node": "3", "type": "main"}]},
    "3": {"main": [{"node": "4", "type": "main"}]}
  }
}
```

### Steps
1. 创建 Schedule 节点，设置为每小时或每天
2. 添加 HTTP Request 节点配置 API
3. 使用 Code 节点处理数据
4. 配置 Email Send 节点发送报告

---

## 2. Error Alerting Setup

### Scenario
监控系统异常并发送飞书通知。

### Steps
1. 创建监控工作流
2. 添加判断节点检查错误率
3. 配置飞书 Webhook

### Example
```json
{
  "name": "Error Alert",
  "nodes": [
    {"id": "1", "name": "Check", "type": "n8n-nodes-base.httpRequest"},
    {"id": "2", "name": "IF Error", "type": "n8n-nodes-base.if", "parameters": {"data1": ">{{json.error}}", "operator": ">", "data2": 0.1}},
    {"id": "3", "name": "Notify", "type": "n8n-nodes-base.httpRequest", "parameters": {"url": "https://webhook.feishu.cn", "method": "POST"}}
  ]
}
```

---

## 3. Optimize Token Cost

### Strategy
- 使用模式匹配减少重复调用
- 缓存常用结果
- 选择合适的模型

### Example Pattern
```typescript
// 在 Agent Service 中使用缓存
const cache = new Map();
async function generateWithCache(key, generator) {
  if (cache.has(key)) return cache.get(key);
  const result = await generator();
  cache.set(key, result);
  return result;
}
```

### Best Practices
1. 识别重复模式，使用 `patternMatcher`
2. 设置合理的 TTL
3. 监控缓存命中率

---

## 4. High Performance Workflows

### Tips
- 使用并行分支代替顺序执行
- 减少不必要的 HTTP 请求
- 使用数据流而非变量复制

### Example
```json
{
  "nodes": [
    {"id": "1", "name": "Fetch All", "type": "parallel-fork"},
    {"id": "2", "name": "API 1", "type": "httpRequest"},
    {"id": "3", "name": "API 2", "type": "httpRequest"},
    {"id": "4", "name": "Merge", "type": "merge", "parameters": {"mode": "combined"}}
  ]
}
```

---

## 5. Version Management

### Workflow
1. 每次修改创建新版本
2. 使用有意义的版本描述
3. 测试新版本后再promote

### Commands
```bash
memflow-cli versions wf_123
memflow-cli rollback wf_123 --version 3
```

---

## 6. Cost Optimization

### Strategies
1. **Batch Requests**: 合并多个小请求
2. **Timeout Settings**: 合理设置超时避免长期等待
3. **Retry Logic**: 只对可重试错误设置重试

### Cost Calculation
```typescript
const cost = tokensIn * costPer1kInput + tokensOut * costPer1kOutput;
```

---

## 7. Debugging Tips

### Enable Debug Logging
```bash
RUST_LOG=debug cargo run
```

### Common Issues
| Error | Solution |
|-------|----------|
| Timeout | 增加 timeout_ms |
| 401 | 检查 API_KEY |
| Connection | 检查网络/防火墙 |

---

## 8. Production Checklist

- [ ] Set up monitoring (Prometheus)
- [ ] Configure alerts (飞书/Slack)
- [ ] Set up backup (SQLite replication)
- [ ] Configure rate limiting
- [ ] Test rollback procedure
- [ ] Review security settings