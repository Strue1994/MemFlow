# MemFlow 凭据配置

## 概述

MemFlow 工作流可能需要以下外部服务的凭据：

## 必需凭据

### OpenAI API Key (用于 LLM 生成)
- 环境变量: `OPENAI_API_KEY`
- 获取地址: https://platform.openai.com/api-keys
- 需要: GPT-4 访问权限

### Executor API Key (用于执行引擎)
- 环境变量: `EXECUTOR_API_KEY`
- 配置方式: 在 executor 配置中生成

## 可选凭据

### 飞书 Webhook
- 获取方式: 飞书群机器人设置 → Webhook 地址
- 格式: `https://open.feishu.cn/open-apis/bot/v2/hook/xxx`

### Slack
- 配置方式: Slack App → Incoming Webhooks
- 格式: `https://hooks.slack.com/services/xxx`

### Telegram Bot
- 获取方式: @BotFather 创建机器人，获取 Bot Token
- 配置: Bot API Token

### Google Sheets
- 配置方式: Google Cloud Console → 启用 Sheets API
- 需要: Service Account 或 OAuth2

### 数据库连接
- 支持: PostgreSQL, MySQL, SQLite
- 配置: 连接字符串

## 配置示例

### Docker Compose

```yaml
services:
  agent-service:
    environment:
      - OPENAI_API_KEY=sk-xxx
      - EXECUTOR_API_KEY=memflow-key-xxx
      - EXECUTOR_URL=http://executor:8080
```

### 本地开发 (.env)

```
OPENAI_API_KEY=sk-xxx
EXECUTOR_API_KEY=memflow-key-xxx
EXECUTOR_URL=http://localhost:8080
MEMORY_HUB_URL=http://localhost:8081
```

## 安全建议

1. **不要**将凭据提交到代码仓库
2. 使用环境变量或 secret 管理工具
3. 定期轮换 API Key
4. 最小权限原则 - 只授予必要的访问权限