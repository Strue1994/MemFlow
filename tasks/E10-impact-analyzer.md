# 任务 E10：优化影响分析

## 目标
量化每次优化对业务指标的贡献，生成"优化价值报告"。

## 文件
- `learning-engine/src/impact_analyzer.rs`（新建）
- `agent-service/src/report_api.ts`

## 具体要求

### 1. 影响计算
- 对每次优化，对比前 7 天和后 7 天指标：
  - 总 token 消耗变化
  - 平均延迟变化
  - 成功率变化
- 计算节省成本：(token减少量 * 每token成本) + (人工干预减少小时数 * 每小时成本)

### 2. 报告生成
- 每周自动生成汇总报告（JSON 或 Markdown）
- 包括：本周优化次数、总节省金额、最佳案例
- API `GET /reports/weekly`

### 3. 可视化
- Web UI 增加"优化价值"仪表盘

## 验收标准
- 报告数据准确，误差 <5%
- 用户能看到实际节省

## 预估 token
2000