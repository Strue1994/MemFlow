# 任务 S-2：跨平台会话记忆同步

## 目标
将会话上下文同步到 MemFlow 的记忆中枢，实现跨平台、跨会话的用户偏好记忆。

## 文件
- `agent-service/src/messaging/session_memory_bridge.ts`
- `memory-hub/src/memory.rs`（增加 `ConversationContext` 类型）
- `agent-service/src/index.ts`（集成桥接逻辑）

## 具体要求

### 1. 会话摘要存储
- 每次用户消息处理完成后，提取：
  - 用户输入文本
  - Agent 回复内容
  - 关键实体（工作流 ID、参数值、用户偏好）
- 调用记忆中枢的 `/memories` API，存储为 `memory_type: "ConversationContext"`，重要性 0.5，TTL 7 天。

### 2. 记忆检索注入
- 在接收新消息前，先检索该用户的最近会话记忆（k=3）。
- 将检索到的记忆内容格式化为简短提示，附加到 `/chat` 请求的 `system_prompt` 中。
- 示例："用户之前提到过喜欢使用 JSON 格式输出，工作流 wf_123 执行成功。"

### 3. 跨平台用户映射
- 支持配置同一用户在不同平台的外部 ID 映射（例如企业微信的 `user_id` 与飞书的 `open_id` 对应同一内部 `user_uuid`）。
- 通过环境变量 `USER_MAPPING_TABLE` 或 Redis Hash 存储映射关系。

### 4. 记忆自动合并
- 当检测到多条相似记忆（如多次提及"使用 POST 方法"），自动合并为一条抽象偏好，重要性提升。

## 验收标准
- 用户在企业微信中说"我喜欢用 Markdown 格式回复"，后在飞书中提问，MemFlow 使用 Markdown 回复。
- 重启服务后，记忆不丢失。
- 跨平台同一用户能共享记忆。

## 预估 token
2800