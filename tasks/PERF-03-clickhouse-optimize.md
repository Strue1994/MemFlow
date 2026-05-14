# 任务 PERF-03：ClickHouse 查询优化

## 目标
为学习引擎创建物化视图，加速聚合查询。

## 文件
- `migrations/materialized_views.sql`（新建）

## 具体要求
```sql
CREATE MATERIALIZED VIEW workflow_daily_stats
ENGINE = SummingMergeTree()
AS SELECT toDate(timestamp), workflow_id, count(), avg(duration_ms)
FROM execution_logs GROUP BY date, workflow_id;
```

## 验收标准
- 查询速度提升 5 倍

## 预估 token
1500