# 任务 S-3：MemFlow 工作流作为 Hermes Agent 工具的高级集成

## 目标
将 MemFlow 的工作流执行能力封装为 Hermes Agent 可调用的工具，并允许 Hermes 在复杂任务中自动组合多个工作流。

## 文件
- `agent-service/src/hermes_tool.ts`（扩展）
- `agent-service/src/hermes_agent_loop.ts`（新建，轻量级 Hermes 决策循环）
- `agent-service/src/tool_registry.ts`（工作流 → 工具注册表）

## 具体要求

### 1. 自动生成工具定义
- 为每个工作流生成符合 Hermes 规范的 JSON Schema：
  - `name`: `memflow_workflow_<workflow_id>`
  - `description`: 工作流的名称和简短描述（从 n8n JSON 的 `nodes` 中提取）。
  - `parameters`: 工作流的输入参数 Schema（若工作流定义了 `input_schema` 元数据）。
- 提供 API `GET /hermes/tools`，返回所有工作流的工具定义列表。

### 2. Hermes 工具调用端点
- `POST /hermes/execute`：
  - 请求体：`{ tool: "memflow_workflow_xxx", arguments: {...} }`
  - 调用内部 `/execute` 执行对应工作流。
  - 返回标准化结果：`{ success: true, output: {...} }` 或 `{ success: false, error: "..." }`。

### 3. 集成轻量级 Hermes 决策循环（可选）
- 在 MemFlow Agent 中内置一个简化版 Hermes 循环：
  - 当用户请求不匹配任何已知工作流时，调用 LLM（如 GPT-4o-mini），传入工具列表，让 LLM 选择调用哪个工作流。
  - 执行工作流后，将结果再次提交给 LLM 生成最终回答。
- 该模式可通过环境变量 `AGENT_MODE=hermes` 启用。

### 4. 组合工作流调用
- 支持在 Hermes 的一个决策回合中调用多个工作流（并行或顺序）。
- 示例：用户说"获取今日汇率并发送邮件"，Hermes 可先调用 `get_exchange_rate` 工作流，再调用 `send_email` 工作流，将前者输出作为后者输入。

## 验收标准
- 在 Hermes 中配置 MemFlow 工具后，用户可通过自然语言触发 MemFlow 工作流。
- 工作流调用结果正确返回。
- （可选模式）MemFlow 自身能像 Hermes 一样进行工具选择。

## 预估 token
3500