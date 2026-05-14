# 任务 S-1：多平台消息适配器 + 自然语言创建工作流

## 目标
实现企业微信、飞书、Telegram 等通讯工具的消息适配器，并允许用户通过自然语言直接在聊天中创建新的 MemFlow 工作流。

## 文件
- `agent-service/src/messaging/adapter.ts`（定义通用接口）
- `agent-service/src/messaging/wecom.ts`（企业微信适配器）
- `agent-service/src/messaging/feishu.ts`（飞书适配器）
- `agent-service/src/messaging/telegram.ts`（Telegram 适配器）
- `agent-service/src/messaging/nlp_workflow_creator.ts`（自然语言创建工作流）
- `agent-service/src/index.ts`（挂载路由）

## 具体要求

### 1. 通用消息适配器接口
- 定义 `MessageAdapter` 接口：
  - `parse(raw: any): { userId: string; text: string; raw: any }`
  - `send(userId: string, reply: string): Promise<void>`
  - `verify(req: any): boolean`（可选，验证签名）
- 提供注册函数 `registerAdapter(platform, adapter)`。

### 2. 平台适配器实现
- **企业微信**：解析 XML 消息，处理加密（使用 `wecom-crypto`），支持被动回复。
- **飞书**：处理事件回调 JSON，支持飞书机器人。
- **Telegram**：使用 `telegraf` 库简化 Webhook 处理。
- 每个适配器通过环境变量配置（`WECOM_TOKEN`, `FEISHU_APP_ID`, `TELEGRAM_BOT_TOKEN` 等）。

### 3. 自然语言创建工作流集成
- 当消息文本不匹配 `/command` 时，调用 `NlpWorkflowCreator`。
- 流程：
  - 调用 `/create_workflow_v2` 接口（现有流水线），传入用户文本。
  - 获取工作流 ID。
  - 返回确认消息："已为您创建工作流 [ID: xxx]，您可以使用 '运行 xxx' 来执行。"
- 若创建失败，返回错误提示并建议用户更具体地描述。

### 4. 会话上下文支持
- 使用 Redis 存储每个用户的最近 5 条消息（userId 格式：`platform:userId`）。
- 在调用创建接口前，将最近消息作为上下文附加到描述中，提高生成准确率。

## 验收标准
- 在企业微信中发送"创建一个每天早上 9 点发送天气预报的工作流"，MemFlow 成功创建并返回 ID。
- 在飞书和 Telegram 中同样生效。
- 连续对话时，能记住之前提到的参数（如"明天改为 8 点"）。

## 预估 token
3000