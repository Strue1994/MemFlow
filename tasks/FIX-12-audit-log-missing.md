# FIX-12: Add Complete Audit Logging

## 问题描述

系统缺少完整的 audit logs，无法满足合规要求。需要记录所有敏感操作。

## 需要记录的操作

1. **认证相关**
   - 登录/登出
   - 密码更改
   - 权限变更

2. **工作流操作**
   - 创建/修改/删除 workflow
   - 执行开始/结束
   - 执行结果

3. **数据操作**
   - 敏感数据访问
   - 数据导出
   - 数据删除

4. **系统操作**
   - 配置变更
   - 用户管理

## 修复方案

1. 创建 audit logger:
   ```rust
   use serde::{Deserialize, Serialize};
   use chrono::{DateTime, Utc};
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AuditLog {
       pub id: String,
       pub timestamp: DateTime<Utc>,
       pub user_id: Option<String>,
       pub action: String,
       pub resource: String,
       pub resource_id: String,
       pub result: AuditResult,
       pub details: Option<Value>,
       pub ip_address: Option<String>,
   }
   
   pub fn log_audit(log: &AuditLog) -> Result<()> {
       let conn = Connection::open("audit.db")?;
       conn.execute(
           "INSERT INTO audit_logs (id, timestamp, user_id, action, resource, 
            resource_id, result, details, ip_address) VALUES (?1, ?2, ?3, ?4, 
            ?5, ?6, ?7, ?8, ?9)",
           params![
               log.id,
               log.timestamp.to_rfc3339(),
               log.user_id,
               log.action,
               log.resource,
               log.resource_id,
               serde_json::to_string(&log.result).unwrap(),
               log.details.as_ref().map(|v| serde_json::to_string(v).unwrap()),
               log.ip_address,
           ],
       )?;
       Ok(())
   }
   ```

2. 集成到关键模块:
   ```rust
   // 在执行前
   log_audit(&AuditLog {
       id: Uuid::new_v4().to_string(),
       timestamp: Utc::now(),
       user_id: Some(user.id.clone()),
       action: "workflow.execute".to_string(),
       resource: "workflow".to_string(),
       resource_id: workflow.id.clone(),
       result: AuditResult::Success,
       details: None,
       ip_address: req.ip(),
   });
   ```

## 影响文件

- 新建 `executor/src/audit.rs`
- `executor/src/lib.rs`
- `agent-service/src/audit.ts`

## 验证方法

执行敏感操作，检查 audit logs 记录。

## 优先级

MEDIUM - 合规要求