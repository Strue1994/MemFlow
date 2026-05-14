# P0-2: Learning Engine Automation Loop

## Priority

P0 - Immediate

## Key Files / Modules

- `learning-engine/src/main.rs`
- `learning-engine/src/param_optimizer.rs`
- `learning-engine/src/prompt_optimizer.rs`
- `learning-engine/src/safety.rs`
- `learning-engine/src/log_persistence.rs`

## Goals

让学习引擎真正运行起来，形成"分析 -> 优化 -> 部署"的自动化闭环。

## Specific Requirements

1.  **定时器实现**
   - 每小时周期性地执行优化逻辑
   - 可配置时间间隔 (`LEARNING_INTERVAL_HOURS`)

2.  **数据源接入**
   - 从 `executor` 收集执行日志
   - 从 `workflow_registry` 获取工作流执行结果
   - 统计成功率、平均耗时等指标

3.  **优化器集成**
   - 调用 `param_optimizer` 生成参数优化建议
   - 调用 `prompt_optimizer` 生成 Prompt 优化建议

4.  **自动审批**
   - 通过 `safety.rs` 的白名单机制
   - 自动审批低风险优化结果（阈值内的参数调整）
   - 高风险优化需人工确认

5.  **审计日志**
   - 记录每次优化的决策链
   - 包含: 时间、输入数据、优化建议、决策结果

## Acceptance Criteria

- [ ] 服务运行后能基于历史执行日志自动生成优化建议
- [ ] 通过审计日志可查每次优化的完整链路
- [ ] 低风险优化自动生效

## Implementation Hints

```rust
// main.rs 核心改动
use learning_engine::{ParamOptimizer, PromptOptimizer, SafetyChecker};

async fn run_learning_loop() {
    let logs = execution_log_collector.get_recent_logs(1000).await;
    let analysis = analyze_execution_patterns(&logs);
    
    // 参数优化
    let param_suggestions = param_optimizer.suggest(&analysis).await;
    for suggestion in param_suggestions {
        if safety.is_low_risk(&suggestion) {
            deploy_param_change(&suggestion).await;
            audit.log("auto_approved", &suggestion);
        }
    }
    
    // Prompt 优化
    let prompt_suggestions = prompt_optimizer.suggest(&analysis).await;
    for suggestion in prompt_suggestions {
        if safety.is_safe(&suggestion) {
            audit.log("pending_review", &suggestion);
        }
    }
}
```