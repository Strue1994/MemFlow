# FIX-03: Remove Hardcoded API Keys

## 问题描述

代码库中可能存在硬编码的 API keys、secrets 或 tokens。这是严重的安全问题。

## 需要检查的文件

1. `agent-service/src/llm_router.ts` - 检查 API key 配置
2. `agent-service/src/marketplace.ts` - 检查第三方集成
3. `executor/src/slack.rs` - 检查 Webhook tokens
4. `executor/src/telegram.rs` - 检查 Bot tokens
5. `executor/src/google_sheets.rs` - 检查 credentials

## 修复方案

1. 搜索所有硬编码的密钥模式:
   - `sk-xxx` (OpenAI)
   - `xoxb-xxx` (Slack)
   - `[\w]{32,}` (可能的 token)

2. 替换为环境变量:
   ```rust
   // 错误
   let api_key = "sk-1234567890";
   
   // 正确
   let api_key = std::env::var("OPENAI_API_KEY")
       .map_err(|_| ExecError::ConfigError("OPENAI_API_KEY not set"))?;
   ```

3. 添加配置验证启动检查

## 影响文件

- 多个文件需要检查

## 验证方法

1. 运行静态分析工具检查密钥
2. 确认所有 API keys 来自环境变量

## 优先级

CRITICAL - 严重安全漏洞