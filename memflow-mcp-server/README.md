# MemFlow MCP Server 配置

## 安装

```bash
cd memflow-mcp-server
npm install
npm run build
```

## 运行方式

### 方式 1: 通过环境变量运行

```bash
export MEMFLOW_API_URL=http://localhost:3000
export MEMFLOW_API_KEY=your-api-key
node dist/index.js
```

### 方式 2: Claude Code 集成

在 `~/.claude/settings.json` 中添加：

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

## 可用工具

| 工具名 | 描述 |
|--------|------|
| `memflow_create_workflow` | 根据自然语言描述创建工作流 |
| `memflow_execute_workflow` | 执行已存在的工作流 |
| `memflow_list_workflows` | 列出所有工作流 |
| `memflow_get_workflow` | 获取工作流详情 |
| `memflow_validate_workflow` | 验证工作流 JSON |
| `memflow_submit_feedback` | 提交用户反馈 |

## 使用示例

### Claude Code 中调用

```
用户: 创建一个每天早上9点抓取RSS并推送到飞书的工作流

Claude: (调用 memflow_create_workflow 工具)
```

### 直接命令行测试

```bash
echo '{"jsonrpc":"2.0","id":"1","method":"tools/list"}' | node dist/index.js
```