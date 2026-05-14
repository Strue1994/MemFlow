# 任务 009：实现 FOR 循环

## 目标
支持 `For` 指令，用于循环执行一段指令块固定次数。

## 文件
- `compiler/src/ir.rs`（添加 `For` 指令）
- `executor/src/lib.rs`（实现循环执行）

## 具体要求

### 1. IR 扩展
```rust
For {
    iterator_var: String,
    start: i64,
    end: i64,
    step: i64,
    body_start: usize,
    body_end: usize,
}
```

### 2. 执行逻辑
- 进入循环前，将 iterator_var 设为 start
- 执行 body_start 到 body_end-1 的指令
- 每次迭代结束 iterator_var += step，直到超过 end

## 测试用例
For i from 1 to 3, body: sum = sum + i（初始 sum=0），结果应为 6。

## 验收标准
- 单元测试通过
- 支持步长不为1