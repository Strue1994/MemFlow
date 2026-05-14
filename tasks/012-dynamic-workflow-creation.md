# 任务 012：动态创建工作流

## 目标
Agent 服务提供 `/create_workflow` 端点，接收自然语言描述，调用 GPT 生成 n8n JSON，自动编译入库。

## 文件
- `executor/src/http_server.rs`（新增 HTTP 服务）
- `agent-service/src/workflowGenerator.ts`（新建）
- `agent-service/src/index.ts`（添加端点）

## 具体要求

### 1. 执行引擎 HTTP 服务
- `POST /compile`：接收 n8n JSON，返回 workflow_id
- `GET /workflows`：返回所有工作流

### 2. Agent 端点
- `POST /create_workflow`：调用 GPT 生成 JSON，调用编译 API

## 验收标准
- 端到端测试通过