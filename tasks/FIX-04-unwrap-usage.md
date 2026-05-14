# FIX-04: Replace unwrap() with Proper Error Handling

## 问题描述

代码中多处使用 `unwrap()` 和 `expect()`，在生产环境中可能导致 panic。

## 需要检查的典型模式

1. `executor/src/code.rs` 第67行:
   ```rust
   Ok(Value::new_int(ctx.as_ref(), n.as_i64().unwrap_or(0)))
   ```

2. `executor/src/http.rs` 第65行:
   ```rust
   let value: Value = serde_json::from_str(&body).unwrap_or(Value::String(body));
   ```

3. `executor/src/concurrency.rs` - 环境变量解析

## 修复方案

1. 将 `unwrap()` 替换为 `?` 操作符和自定义错误
2. 使用 `ok_or()` 或 `ok_or_else()` 提供默认值
3. 添加明确的错误类型

```rust
// 修复前
n.as_i64().unwrap()

// 修复后
n.as_i64().ok_or_else(|| ExecError::CodeError("Cannot convert to i64".to_string()))?
```

## 影响文件

- `executor/src/code.rs`
- `executor/src/http.rs`
- `executor/src/concurrency.rs`
- 其他相关文件

## 验证方法

运行 `cargo clippy` 检查 `unwrap_used` 警告。

## 优先级

HIGH - 稳定性问题