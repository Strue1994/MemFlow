# P3-2: Multi-tenant Isolation

## Priority

P3 - Long-term

## Key Files / Modules

- `executor/src/workflow_registry.rs`
- `executor/src/auth.rs`
- `agent-service/src/tenantMiddleware.ts`

## Goals

支持多个租户共享同一 MemFlow 集群，数据和资源相互隔离。

## Specific Requirements

1.  **Tenant ID Concept**
   - 为工作流引入 `tenant_id` 字段
   - 所有查询和写入带 `tenant_id` 过滤

2.  **Isolation**
   - 工作流列表按租户隔离
   - 限流和配额按租户粒度

3.  **Database Strategy**
   - 可选: 单库多表 或 多租户分离

4.  **API Key Management**
   - API Key 绑定租户
   - 所有请求带租户标识

## Acceptance Criteria

- [ ] 创建两个租户，工作流列表互不可见
- [ ] 一个租户资源耗尽不影响另一个

## Implementation

```rust
// workflow_registry.rs
pub fn list_workflows(&self, tenant_id: &str) -> Vec<WorkflowMeta> {
    self.db.query(
        "SELECT * FROM workflows WHERE tenant_id = ?",
        [tenant_id]
    )
}

pub fn register_workflow(
    &self,
    tenant_id: &str,
    id: &str,
    workflow: &Workflow,
) -> Result<u32, Error> {
    // Check quota
    let count = self.db.count("workflows WHERE tenant_id = ?", [tenant_id])?;
    if count >= MAX_WORKFLOWS_PER_TENANT {
        return Err(Error::QuotaExceeded);
    }
    
    self.db.insert("workflows", &[
        ("tenant_id", tenant_id),
        ("id", id),
    ])
}
```

```typescript
// agent-service/src/tenantMiddleware.ts
interface TenantContext {
  tenantId: string;
  quota: QuotaConfig;
}

function tenantMiddleware(req, res, next) {
  const apiKey = req.headers['x-api-key'];
  const tenant = tenantRegistry.resolve(apiKey);
  
  if (!tenant) {
    return res.status(401).json({ code: 'INVALID_API_KEY' });
  }
  
  req.tenant = tenant;
  next();
}

app.use(tenantMiddleware);
```