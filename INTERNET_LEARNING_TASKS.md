# MemFlow 互联网知识引擎升级任务表

版本：v1.0
日期：2026-04-19
目标：将自学习引擎从本地模式升级为互联网增强模式

## 一、升级架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                    互联网知识引擎架构                              │
├─────────────────────────────────────────────────────────────────────┤
│  数据源层                                                            │
│  ├── 本地执行日志 (已有)                                             │
│  ├── 互联网采集器 ─── 定时爬取/订阅外部知识源                      │
│  │     ├── 行业知识库 (如 GitHub Wiki, Stack Overflow)             │
│  │     ├── 开源项目 (API变更, Release Notes)                       │
│  │     ├── 权威文档 (官方API文档, RFC)                              │
│  │     └── 新闻/RSS 订阅                                           │
│  └── 多模态输入 ── 支持 Web/PDF/图片/音频解析                      │
├─────────────────────────────────────────────────────────────────────┤
│  处理层                                                            │
│  ├── 知识融合引擎                                                   │
│  │     ├── 去重检测 (SimHash / MinHash)                            │
│  │     ├── 冲突检测与可信度评估                                     │
│  │     └── 知识分级 (高/中/低可信)                                   │
│  ├── 陈旧检测器 ─── 定期自检知识库，发现过期内容                    │
│  └── 安全审查 ─── 自动分级、脱敏、合规审查                          │
├─────────────────────────────────────────────────────────────────────┤
│  升级层                                                            │
│  ├── 升级建议生成器 ── 发现新内容时生成升级方案                    │
│  ├── 自动合并流程 ── 支持一键/自动合并                             │
│  └── 回滚机制 ── 所有变更可追溯、可回滚                            │
├─────────────────────────────────────────────────────────────────────┤
│  接口层                                                            │
│  ├── 内部 API (现有 /learning/*)                                   │
│  ├── 开放 API ─── RESTful/GraphQL 对外暴露记忆库                    │
│  └── 多 Agent 协同 ── 支持团队知识共建                               │
└─────────────────────────────────────────────────────────────────────┘
```

## 二、P3 任务清单

### 模块一：互��网知识采集器 (Web Knowledge Collector)

| ID | 任务 | 交付物 | 估时 |
|---|---|---|---|
| INT-P3-01 | 采集器基础架构 | scraper 模块 + 任务调度器 | 3d |
| INT-P3-02 | HTTP/Web 抓取器 | 支持 GET/POST/登录态抓取 | 2d |
| INT-P3-03 | RSS 订阅器 | 支持定时拉取 RSS/Atom | 1d |
| INT-P3-04 | 多模态解析器 | PDF/图片/音频结构化 | 3d |
| INT-P3-05 | 源配置管理 | 可配置采集源列表 + 调度策略 | 1d |

### 模块二：知识融合引擎 (Knowledge Fusion Engine)

| ID | 任务 | 交付物 | 估时 |
|---|---|---|---|
| INT-P3-06 | 去重检测 | SimHash/MinHash 相似度检测 | 2d |
| INT-P3-07 | 冲突检测 | 多源内容冲突识别 + 可信度评分 | 2d |
| INT-P3-08 | 知识分级 | 自动分级为 高/中/低 可信度 | 1d |
| INT-P3-09 | 融合 API | 本地+互联网知识统一查询接口 | 2d |

### 模块三：自动升级流水线 (Auto Upgrade Pipeline)

| ID | 任务 | 交付物 | 估时 |
|---|---|---|---|
| INT-P3-10 | 陈旧检测器 | 定期检测过期知识，生成刷新建议 | 2d |
| INT-P3-11 | 升级建议生成 | 新知识对比，生成合并建议 | 2d |
| INT-P3-12 | 自动合并 | 一键/自动合并逻辑 + 管理员审核流 | 2d |
| INT-P3-13 | 回滚机制 | 变更日志 + 可回滚到任意版本 | 2d |

### 模块四：开放接口 (Open API)

| ID | 任务 | 交付物 | 估时 |
|---|---|---|---|
| INT-P3-14 | RESTful API 设计 | 对外读写记忆 API 设计 | 1d |
| INT-P3-15 | GraphQL 支持 (可选) | GraphQL 接口层 | 2d |
| INT-P3-16 | 多 Agent 协同 | 团队知识共建机制 | 3d |
| INT-P3-17 | 插件接口 | 第三方 Agent 接入规范 | 2d |

### 模块五：安全与合规 (Security & Compliance)

| ID | 任务 | 交付物 | 估时 |
|---|---|---|---|
| INT-P3-18 | 内容分级 | 自动敏感信息检测 | 1d |
| INT-P3-19 | 脱敏处理 | PII/敏感数据自动脱敏 | 1d |
| INT-P3-20 | 合规审查 | 采集源合规检查清单 | 1d |

## 三、推荐实施顺序

```
阶段 1: 采集器 (INT-P3-01 ~ 05)                    [5d]
阶段 2: 融合引擎 (INT-P3-06 ~ 09)                 [7d]
阶段 3: 升级流水线 (INT-P3-10 ~ 13)               [8d]
阶段 4: 开放接口 (INT-P3-14 ~ 17)                 [8d]
阶段 5: 安全合规 (INT-P3-18 ~ 20)                [3d]
────────────────────────────────────────────────
总计                                                ~31d (约 6-7 周)
```

## 四、首批采集源建议

| 源类型 | 示例 | 优先级 | 更新频率 |
|---|---|---|---|
| GitHub Releases | kubernetes/kubernetes/releases | P0 | 每小时 |
| API 文档 | OpenAI API Changelog | P0 | 每日 |
| Stack Overflow | [tag] 技术问答聚合 | P1 | 每日 |
| 技术博客 | Hacker News / Dev.to | P1 | 每小时 |
| RFC 文档 | IETF RFC 列表 | P2 | 每周 |

## 五、API 扩展设计

### 新增端点

```yaml
# 知识采集
POST /knowledge/sources          # 添加采集源
GET  /knowledge/sources           # 列出采集源
DELETE /knowledge/sources/:id       # 删除采集源
POST /knowledge/fetch              # 手动触发采集

# 知识融合
POST /knowledge/merge             # 合并知识建议
GET  /knowledge/conflicts        # 查看冲突知识
POST /knowledge/resolve         # 解决冲突

# 知识升级
GET  /knowledge/stale           # 过期知识列表
POST /knowledge/refresh         # 刷新过期知识

# 开放 API
GET  /memory/public             # 公开记忆查询
POST /memory/contribute        # 贡献新知识
GET  /memory/versions          # 记忆版本历史
POST /memory/rollback          # 回滚到某版本
```

### 数据模型扩展

```rust
// KnowledgeSource
struct KnowledgeSource {
    id: String,
    name: String,
    url: String,
    source_type: SourceType,  // http, rss, github, etc.
    auth: Option<AuthConfig>,
    schedule: Schedule,
    enabled: bool,
}

// KnowledgeUnit 扩展
struct KnowledgeUnit {
    // 现有字段...
    source: Option<String>,        // 来源
    confidence: f64,                // 0.0-1.0 可信度
    freshness: i64,                // 最后更新时间
    is_stale: bool,                // 是否过期
    tags: Vec<String>,             // 分类标签
    version: i32,                  // 版本号
}
```

## 六、验收门槛

1. ✅ 能配置并成功采集至少 3 个外部知识源
2. ✅ 自动去重 + 冲突检测准确率 > 90%
3. ✅ 知识库陈旧检测可识别 30 天未更新内容
4. ✅ 一键合并可保留完整变更历史
5. ✅ 可回滚到任意历史版本
6. ✅ 开放 API 可被外部 Agent 调用
7. ✅ 多 Agent 可同时读写同一知识库

## 七、风险与缓解

| 风险 | 缓解措施 |
|---|---|
| 采集频率过高被封禁 | 限速 + 代理池 + 指数回退 |
| 知识版权争议 | 白名单采集源 + 版权检测 |
| 内存爆炸 | 增量采集 + 淘汰策略 |
| 外部内容污染 | 多级可信度 + 人工复核 |