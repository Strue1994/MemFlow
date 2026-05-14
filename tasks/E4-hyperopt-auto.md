# 任务 E4：动态超参数调优

## 目标
自动调整学习引擎本身的参数（如优化触发间隔、A/B 测试时长、决策阈值），通过贝叶斯优化找到最佳配置。

## 文件
- `learning-engine/src/hyperopt_auto.rs`（新建）
- `learning-engine/src/hyperopt_client.rs`（调用 Python 服务）
- `python/hyperopt_server.py`（使用 `hyperopt` 库）

## 具体要求

### 1. 可调参数
- `learning_interval_hours`（当前 6）
- `ab_test_duration_hours`（当前 24）
- `promote_latency_improvement`（当前 0.10）
- `emergency_error_rate_threshold`（当前 0.10）

### 2. 优化目标
- 最大化"综合效率分数" = 0.5 * (1 - 平均响应时间/基线) + 0.3 * 成功率 + 0.2 * (1 - token消耗/基线)

### 3. 执行流程
- 每隔 7 天运行一次超参数优化：
  - 收集过去 7 天的系统指标。
  - 调用贝叶斯优化器，探索新的参数组合。
  - 若新组合预期收益 > 5%，则自动应用并观察 7 天；否则保留原配置。

### 4. 安全限制
- 参数变化范围有限（如间隔不能小于 1 小时，不能大于 24 小时）。
- 自动回滚：若新配置导致系统性能下降，自动恢复上一组参数。

## 验收标准
- 系统能自动找到适合当前负载的学习频率（如高峰期缩短间隔）。
- 优化过程不引入性能震荡。

## 预估 token
2600