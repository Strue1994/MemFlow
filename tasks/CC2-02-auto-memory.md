# 任务 CC2-02：自动记忆提取

## 目标
从用户与 Agent 的对话历史中，自动识别并提取可长期记忆的信息（如偏好、常用工作流、错误修正模式），存入记忆中枢。

## 文件
- `learning-engine/src/memory_extractor.rs`（新建）
- `memory-hub/src/auto_extract.rs`（调用 LLM 进行提取）

## 具体要求

### 1. 提取时机
- 每次 `/chat` 交互结束后，异步分析对话。
- 定时任务（如每天）扫描历史对话。

### 2. 提取规则
- 使用 GPT-4o-mini 或本地小模型，提示词：

从以下对话中提取用户偏好、重复需求、纠正过的错误。输出 JSON 数组，每个元素包含 type, content, importance。

- 示例输出：`[{"type": "preference", "content": "用户喜欢使用 JSON 格式返回结果", "importance": 0.8}]`

### 3. 记忆存储
- 调用记忆中枢的 `store` 接口，`memory_type` 设为 `AutoExtracted`。
- 重要性由模型评估，可后续根据用户反馈调整。

### 4. 用户确认（可选）
- 在 Web UI 中显示"发现新记忆"，允许用户确认或拒绝。

## 验收标准
- 自动提取的记忆准确率 > 70%（人工评估）。
- 不重复提取相同信息。
- 提取过程对用户无感知，不影响主流程。

## 预估 token
2800