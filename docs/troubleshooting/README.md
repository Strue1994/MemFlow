# MemFlow Troubleshooting Guide

## Common Issues and Solutions

---

## 1. MCP Connection Failed

### Error
```
Error: MCP connection failed: timeout
```

### Possible Causes
1. Node.js version incompatible
2. API Key not set
3. Network connectivity

### Solutions
```bash
# Check Node.js version
node --version  # Should be >= 18

# Verify API Key
echo $ANTHROPIC_API_KEY

# Test connection
curl http://localhost:3000/health
```

---

## 2. OpenAPI Service Timeout

### Error
```
Error: Request timeout after 30000ms
```

### Solutions
```bash
# Increase timeout in .env
OPENAI_TIMEOUT_MS=60000

# Or use faster model
export MODEL=gpt-3.5-turbo
```

### Fallback Configuration
```typescript
const router = new LLMRouter({
  gpt4o: { timeout: 30000 },
  gpt35: { timeout: 15000 },  // fallback
});
```

---

## 3. Workflow Validation Failed

### Error
```
Error: Validation failed: Missing required field 'url'
```

### Solutions
1. Check node parameters
2. Verify JSON structure
3. Use validator:

```typescript
import { validate } from './n8nValidator';
const result = await validate(workflowJson);
if (!result.valid) {
  console.log(result.errors);
}
```

---

## 4. Docker Container Failed to Start

### Error
```
Error: Container exited with code 1
```

### Solutions
```bash
# Check port conflicts
netstat -ano | findstr :3000

# Check logs
docker-compose logs

# Check volume permissions
ls -la ./data/
```

### Common Port Conflicts
| Port | Service | Fix |
|-----|--------|-----|
| 3000 | Agent | Change to 3001 |
| 8080 | Executor | Change to 8081 |
| 6379 | Redis | Usually OK |

---

## 5. ClickHouse Query Slow

### Error
```
Error: Query timeout
```

### Solutions
1. Check materialized views:
```sql
SELECT * FROM system.materialized_views;
```

2. Create indexes:
```sql
ALTER TABLE execution_logs ADD INDEX idx_workflow_id workflow_id TYPE bloom_filter;
```

3. Use date range filters:
```sql
SELECT * FROM execution_logs 
WHERE timestamp > now() - INTERVAL 7 DAY;
```

---

## 6. Memory Pool Exhausted

### Error
```
Error: No Environment available in pool
```

### Solutions
```bash
# Increase pool size
export ENV_POOL_SIZE=20

# Or disable pooling
export ENV_POOL_SIZE=0
```

---

## 7. Rate Limit Exceeded

### Error
```
Error: 429 Too Many Requests
```

### Solutions
```bash
# Wait for reset
# Or increase limit
export RATE_LIMIT_PER_MINUTE=120
```

---

## 8. Database Connection Failed

### Error
```
Error: Connection refused to postgres:5432
```

### Solutions
```bash
# Check PostgreSQL container
docker-compose ps

# Check connection
docker exec -it memflow-postgres-1 psql -U postgres -c "SELECT 1"

# Check .env
DATABASE_URL=postgres://user:pass@localhost:5432/memflow
```

---

## 9. Webhook Not Triggering

### Error
```
Error: Webhook not received
```

### Solutions
1. Verify Webhook URL:
```bash
curl -X POST https://your-webhook.com/test
```

2. Check firewall:
```bash
# Allow incoming ports
ufw allow 80/tcp
ufw allow 443/tcp
```

3. Use ngrok for testing:
```bash
ngrok http 3000
```

---

## 10. API Key Invalid

### Error
```
Error: 401 Unauthorized
```

### Solutions
```bash
# Verify key format
echo $MEMFLOW_API_KEY

# Should be: Bearer <your-key>

# Regenerate key
# POST /api/keys/regenerate
```

---

## Debug Commands

```bash
# Full system health
memflow-cli doctor

# View logs
docker-compose logs -f

# Check metrics
curl http://localhost:3000/metrics

# Test specific workflow
memflow-cli run wf_123 --json
```

---

## Getting Help

1. Check logs: `docker-compose logs -f`
2. Run doctor: `memflow-cli doctor`
3. Check status: `curl http://localhost:3000/health`
4. Review docs: `docs/best-practices/`

---

## Emergency Rollback

```bash
# Rollback to previous version
memflow-cli rollback wf_123

# Or use web UI: Workflow → Versions → Rollback
```