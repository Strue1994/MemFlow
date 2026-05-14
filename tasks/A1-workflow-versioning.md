# 任务 A1：工作流版本管理

## 目标
为工作流增加版本控制，支持更新时保留旧版本，Agent 可指定版本号调用，支持回滚。

## 文件
- `executor/src/db.rs`（修改 schema 和操作函数）
- `executor/src/workflow_registry.rs`（修改注册和获取逻辑，支持版本）
- `executor/src/http_server.rs`（修改 API 支持版本参数）
- `executor/src/main.rs`（CLI 添加 version, rollback 子命令）
- `agent-service/src/workflowRegistry.ts`（调用时传递版本）

## 具体要求

### 1. 数据库 Schema 升级
将 `workflows` 表改为复合主键，增加 `is_latest` 标志：
```sql
CREATE TABLE workflows (
    id TEXT NOT NULL,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    n8n_json TEXT NOT NULL,
    ir_blob BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    is_latest BOOLEAN DEFAULT 1,
    PRIMARY KEY (id, version)
);
CREATE INDEX idx_latest ON workflows(id, is_latest) WHERE is_latest=1;
```

### 2. 注册工作流时自动版本递增
- 查询同一 `id` 的最大版本号，新版本 = max_version + 1。
- 将旧版本的 `is_latest` 设为 0。
- 新版本的 `is_latest` 设为 1。
- 保存到数据库。

### 3. 获取工作流接口
- `get_workflow(id, version: Option<u32>)`：若 version 为 None，返回最新版本；否则返回指定版本。
- 缓存 key 改为 `(id, version)`。

### 4. HTTP API 修改
- `GET /workflow/:id?version=2` 支持版本参数。
- `POST /compile` 可选返回 `version` 字段。

### 5. Agent 调用支持版本
- `/execute` 请求体增加可选字段 `version`。
- 执行引擎根据版本加载工作流。

### 6. CLI 命令
- `executor_cli import --id xxx --file wf.json --name "name"`（自动分配版本）
- `executor_cli list --id xxx` 显示所有版本
- `executor_cli rollback --id xxx` 将最新版本回滚到上一版本（修改 is_latest）

## 验收标准
- 同一 id 可保存多个版本。
- 不指定版本时执行最新版本。
- 指定旧版本能正确执行。
- 回滚后最新版本改变。