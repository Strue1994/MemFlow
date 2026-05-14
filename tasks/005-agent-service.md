# 任务 005：Agent 服务（TypeScript）

## 目标
实现一个 TypeScript 编写的 Agent 服务，提供两个 HTTP 端点：
- `POST /execute`：直接执行工作流（传递工作流 ID 和参数）。
- `POST /chat`：接收自然语言，映射到预定义工作流并执行。

## 文件
- `agent-service/src/index.ts`
- `agent-service/src/workflowRegistry.ts`（新建）
- `agent-service/package.json`（已存在，需补充依赖）

## 具体要求

### 1. 依赖
- `express`
- `child_process`（调用 Rust 执行引擎）
- `openai`（用于 `/chat` 意图解析）

### 2. 工作流注册表
`workflowRegistry.ts` 导出：
- 一个 Map，键为工作流 ID（字符串），值为工作流 JSON（n8n 格式）。
- 预定义两个工作流：
  - `wf_zen`：只包含一个 HTTP 请求到 `https://api.github.com/zen`。
  - `wf_add`：包含两个 set 节点和一个 math 节点（a=2, b=3, add → c）。

### 3. `/execute` 端点
- 请求体：`{ workflowId: string, params: Record<string, any> }`
- 从注册表获取工作流 JSON，合并参数（简单实现：将 params 作为初始变量注入）。
- 调用 Rust 执行引擎：通过子进程执行 `executor/target/release/executor_cli`（你需要先生成一个 CLI 包装，见下）。
- 返回执行结果。

### 4. Rust CLI 包装（额外生成）
在 `executor/src/main.rs` 中实现一个命令行程序：
- 接收 `--workflow <json_file>` 和 `--params <json>`。
- 解析工作流，执行，输出结果 JSON。
这样 TypeScript 可以通过子进程调用。

### 5. `/chat` 端点
- 请求体：`{ text: string }`
- 调用 GPT-4o-mini（使用 OpenAI SDK），提示词：将用户输入映射到工作流 ID，并提取参数。
- 返回工作流执行结果。

## 验收标准
- `npm run dev` 启动服务。
- `curl -X POST /execute -d '{"workflowId":"wf_add"}'` 返回 5。
- `curl -X POST /chat -d '{"text":"add 2 and 3"}'` 返回 5。

## 预期输出格式
##FILE:agent-service/src/workflowRegistry.ts
```typescript
// 注册表
```
##FILE:agent-service/src/index.ts
```typescript
// 服务实现
```
##FILE:executor/src/main.rs
```rust
// CLI 入口
```