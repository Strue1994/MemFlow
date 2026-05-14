# 任务 E3：RL 决策器

## 目标
用强化学习替代规则引擎，使系统能动态调整优化策略（何时 promote、rollback、修改参数）。

## 文件
- `learning-engine/src/rl_decision.rs`（新建）
- `learning-engine/src/rl_env.rs`（环境模拟）
- `python/train_rl.py`（Python 训练脚本，使用 Stable-Baselines3）

## 具体要求

### 1. 环境建模
- 状态：工作流最近 10 次执行的指标（成功率、延迟、token 消耗）、历史优化次数、当前版本年龄。
- 动作：{promote, rollback, adjust_timeout, adjust_retry, do_nothing}。
- 奖励：成功率变化 * 1.0 + 延迟降低 * 0.5 - token 增加 * 0.2。

### 2. 训练与部署
- 使用 PPO 算法在模拟环境中训练策略网络（Python）。
- 导出模型为 ONNX 或通过 HTTP 服务调用。
- Rust 端通过 gRPC 或 REST 请求决策。

### 3. 在线学习
- 支持在线微调：实际执行后根据真实奖励更新策略网络（可选）。

### 4. 回退机制
- 若 RL 服务不可用，自动切换回规则引擎（R8b）。

## 验收标准
- 在模拟环境中，RL 决策器累计奖励高于规则引擎 20%。
- 实际部署后，工作流优化成功率提升 10%。

## 预估 token
3500