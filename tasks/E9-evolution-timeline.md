# 任务 E9：进化时间线 UI

## 目标
提供可视化时间线，展示工作流各版本的成功率、延迟、token 消耗变化，支持回溯。

## 文件
- `web-ui/src/components/EvolutionTimeline.tsx`
- `web-ui/src/components/VersionComparison.tsx`
- `agent-service/src/version_history.ts`（API）

## 具体要求

### 1. 数据聚合
- 从 ClickHouse 查询每个版本的每日聚合指标（成功率、P95 延迟、平均 token）。
- 缓存结果，加速加载。

### 2. 时间线展示
- 使用 ECharts 或 Recharts 绘制折线图，横轴为时间，纵轴为指标。
- 不同版本用不同颜色标记，鼠标悬停显示版本号和具体数值。

### 3. 版本对比
- 选择两个版本，并排显示指标对比表。
- 突出显示差异（如"延迟降低 15%"）。

### 4. 回溯操作
- 支持将某个历史版本重新设为最新版本（需权限确认）。

## 验收标准
- 加载时间线 < 2 秒。
- 版本对比清晰直观。
- 回溯操作成功生效。

## 预估 token
2200