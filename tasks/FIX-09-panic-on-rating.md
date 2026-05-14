# FIX-09: Handle Rating Module Panic

## 问题描述

`executor/src/rating.rs` 中可能存在 panic 情况，需要改为优雅错误处理。

## 检查问题

查看 rating 模块中的错误处理:

```rust
// 可能的问题
let rating = rated_items.get(&task_id).unwrap();  // 可能 panic
```

## 修复方案

1. 替换 unwrap 为 proper error handling:
   ```rust
   // 修复前
   let rating = rated_items.get(&task_id).unwrap();
   
   // 修复后
   let rating = rated_items.get(&task_id)
       .ok_or_else(|| ExecError::NotFound(format!("Rating not found for task: {}", task_id)))?;
   ```

2. 添加默认值处理:
   ```rust
   let rating = rated_items.get(&task_id).unwrap_or(&Rating::default());
   ```

3. 添加日志记录

## 影响文件

- `executor/src/rating.rs`

## 验证方法

运行有问题的 workflow 确认不会 panic。

## 优先级

HIGH - 稳定性问题