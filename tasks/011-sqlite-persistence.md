# 任务 011：工作流持久化（SQLite）

## 目标
将工作流定义存储到 SQLite 数据库。

## 文件
- `executor/src/db.rs`（新建）
- `executor/src/workflow_registry.rs`（从 DB 加载）

## 具体要求

### 1. 数据库 Schema
使用 rusqlite 创建表：
```sql
CREATE TABLE workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    n8n_json TEXT NOT NULL,
    ir_blob BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
```

### 2. WorkflowRegistry 改造
从数据库按需加载 IR

## 验收标准
- 重启后工作流仍可用