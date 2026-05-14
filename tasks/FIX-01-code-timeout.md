# FIX-01: Code Node Timeout Logic Bug

## 问题描述

在 `executor/src/code.rs` 中，超时检查逻辑有误。当前代码:

```rust
let start = Instant::now();
if start + DEFAULT_TIMEOUT < Instant::now() {
    return Err(ExecError::CodeError("Script execution timeout".to_string()));
}
```

这个检查是错误的:
- `start + DEFAULT_TIMEOUT` 得到的是截止时间
- 比较的是 `截止时间 < 现在`，永远为 false

## 预期行为

应该在执行 JS 代码后检查是否超时，而不是在执行前检查一个错误的时间。

## 修复方案

1. 将超时检查移到实际执行之后
2. 使用正确的超时判断逻辑: `if start.elapsed() > DEFAULT_TIMEOUT`
3. 确保超时能正确触发超时错误

## 影响文件

- `executor/src/code.rs`

## 验证方法

运行包含长时间执行的 Code 节点 Workflow，确认超时能正确触发。

## 优先级

HIGH - 功能性bug