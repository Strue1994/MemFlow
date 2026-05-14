# 任务 E7：工作流市场 API

## 目标
允许用户分享、评分、复用工作流，形成生态。

## 文件
- `agent-service/src/marketplace.ts`（新建）
- `agent-service/src/marketplace_db.rs`（数据库表）
- `web-ui/src/components/Marketplace.tsx`

## 具体要求

### 1. 发布工作流
- 端点：`POST /workflow/publish`
- 请求体：`{ workflow_id, description, tags, price (可选) }`
- 发布前需匿名化（移除敏感凭证、硬编码 URL）。

### 2. 搜索与导入
- 端点：`GET /workflow/marketplace/search?q=...&tags=...`
- 返回工作流列表（名称、描述、评分、下载次数）。
- 导入：`POST /workflow/import/{remote_id}`，自动下载并创建工作流副本。

### 3. 评分与评论
- 端点：`POST /workflow/rate/{workflow_id}`（1-5 星）
- 评论需审核或使用垃圾过滤。

### 4. 中央市场服务
- 可自建中央市场，或对接第三方平台（如 GitHub）。

## 验收标准
- 用户可发布工作流，其他用户可搜索、导入、评分。
- 导入的工作流能正确执行（需手动配置凭证）。

## 预估 token
2800