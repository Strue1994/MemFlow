# 任务 E8：工作流评分与评论

## 目标
为工作流市场增加评分和评论功能，用户可打分、留言，系统根据评分优化推荐排序。

## 文件
- `agent-service/src/rating.rs`（新建）
- `web-ui/src/components/RatingStars.tsx`
- `web-ui/src/components/CommentSection.tsx`

## 具体要求

### 1. 评分接口
- `POST /workflow/{id}/rate`：1-5 星，可附文字评论。
- `GET /workflow/{id}/ratings`：返回平均分、评论列表。

### 2. 防滥用
- 同一用户对同一工作流只能评分一次（可修改）。
- 需要登录（API Key 绑定用户 ID）。

### 3. 推荐排序
- 在工作流市场搜索时，默认按 `(平均分 * log(下载数+1))` 排序，兼顾质量和热度。

### 4. 评论审核
- 可选：敏感词过滤或人工审核。

## 验收标准
- 用户可评分评论，界面友好。
- 推荐排序明显偏好高分工作流。

## 预估 token
1800