# 任务 E2：全局工作流仓库

## 目标
连接中央工作流市场（如 GitHub 仓库），自动拉取社区验证的高效工作流模板，供 Agent 推荐使用。

## 文件
- `executor/src/global_registry.rs`（新建）
- `agent-service/src/global_pattern_sync.ts`

## 具体要求

### 1. 仓库源配置
- 支持配置多个 Git 仓库 URL（如 GitHub、GitLab）。
- 定期（每天）同步仓库中的工作流 JSON 文件。

### 2. 模板索引
- 解析工作流 JSON，提取元数据（名称、描述、标签、所需节点类型）。
- 存入本地 `global_patterns` 表。

### 3. Agent 集成
- 在模式匹配时，同时检索本地模式和全局模式。
- 使用向量检索（已有）返回最相关模板。

### 4. 版本更新
- 当仓库更新时，自动重新拉取并更新本地索引。

## 验收标准
- 能同步 GitHub 上指定仓库的工作流模板。
- Agent 可推荐并使用全局模板。

## 预估 token
2000