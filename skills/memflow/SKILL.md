# MemFlow Skill

通过自然语言与 MemFlow 交互，自动构建、执行和管理 n8n 工作流。

## 概述

MemFlow 是一个记忆驱动的自动化工作流平台，能够：
- 通过自然语言描述创建 n8n 工作流
- 根据用户确认的设计自动生成工作流 JSON
- 验证工作流配置并自动修复常见错误
- 部署和管理多个工作流版本

## 触发方式

此 Skill 会在检测到以下模式时自动激活：

- "用 MemFlow 创建一个工作流..."
- "MemFlow 创建一个..."
- "帮我创建一个...工作流"
- "创建一个定时...的工作流"
- "创建一个 webhook...的工作流"

## 工作流步骤

当用户请求创建工作流时，按以下步骤引导：

1. **需求收集** - 了解触发方式、数据源、目标动作
2. **模式匹配** - 从知识库推荐最合适的架构模式
3. **知识注入** - 加载相关验证规则和节点配置
4. **设计确认** - 展示生成的设计，等待用户确认
5. **工作流构建** - LLM 生成 n8n JSON
6. **验证修复** - 自动验证并修复常见错误
7. **凭据配置** - 提示用户配置必要的 API 密钥
8. **部署** - 保存并部署工作流
9. **测试** - 可选执行一次测试
10. **导出** - 提供 JSON 下载链接

## 使用示例

### 示例 1: 定时任务

```
用户: "用 MemFlow 创建一个每天早上 9 点抓取 Hacker News 并推送飞书的工作流"
助手: 好的，我来帮您创建这个工作流。
      
      触发器: 每天早上 9 点 (Schedule Trigger)
      数据源: Hacker News API
      目标: 飞书 Webhook
      
      请确认以上设计，或输入修改意见。
```

### 示例 2: Webhook 处理

```
用户: "创建一个接收支付回调后更新数据库的 webhook 工作流"
助手: 好的，我来创建这个 webhook 工作流。
      
      触发器: Webhook (POST)
      处理: 验证支付状态
      动作: 更新数据库记录
      
      请确认设计。
```

### 示例 3: 执行现有工作流

```
用户: "MemFlow 执行工作流 user-onboarding"
助手: 开始执行工作流 "user-onboarding"...
      
      执行完成，耗时 2.3 秒
      结果: 成功处理 5 条用户记录
```

## API 参考

所有操作通过 MemFlow Agent 服务 API 完成：

| 端点 | 方法 | 描述 |
|------|------|------|
| `/create_workflow` | POST | 从 n8n JSON 创建工作流 |
| `/create_workflow_v2` | POST | 多阶段流水线创建 |
| `/execute` | POST | 执行工作流 |
| `/workflows` | GET | 列出所有工作流 |
| `/feedback` | POST | 提交用户反馈 |

## 环境变量

运行 MemFlow 需要以下环境变量：

- `EXECUTOR_URL` - 执行引擎地址 (默认 http://127.0.0.1:8080)
- `EXECUTOR_API_KEY` - 执行引擎 API 密钥
- `OPENAI_API_KEY` - OpenAI API 密钥 (用于 LLM 生成)

## MCP Server 集成

MemFlow 提供了 MCP (Model Context Protocol) Server，可以与多种 AI 编码助手集成：

### 支持的 AI 助手

| AI 助手 | 集成方式 |
|---------|----------|
| Claude Code | MCP (原生支持) |
| Continue | MCP |
| OpenCode | MCP 或 CLI |

### MCP 配置

1. 安装 MCP Server:
   ```bash
   cd memflow-mcp-server
   npm install && npm run build
   ```

2. 在 Claude Code 中配置 (`~/.claude/settings.json`):
   ```json
   {
     "mcpServers": {
       "memflow": {
         "command": "node",
         "args": ["/path/to/memflow-mcp-server/dist/index.js"],
         "env": {
           "MEMFLOW_API_URL": "http://localhost:3000",
           "MEMFLOW_API_KEY": "your-api-key"
         }
       }
     }
   }
   ```

3. 可用 MCP 工具:
   - `memflow_create_workflow` - 创建工作流
   - `memflow_execute_workflow` - 执行工作流
   - `memflow_list_workflows` - 列出工作流
   - `memflow_validate_workflow` - 验证工作流
   - `memflow_submit_feedback` - 提交反馈

### CLI 工具

如果 AI 助手不支持 MCP，也可以使用 CLI 工具：

```bash
# 创建工作流
memflow create "每天抓取 RSS 推送到飞书"

# 执行工作流
memflow execute workflow-123

# 列出工作流
memflow list
```

## 更多信息

- 详细 API 文档: `reference/api.md`
- MCP Server 配置: `../memflow-mcp-server/README.md`
- 工作流示例: `workflow/`
- 凭据配置: `credentials/memflow.md`