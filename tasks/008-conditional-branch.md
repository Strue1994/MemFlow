# 任务 008：实现条件分支（IF 指令）

## 目标
支持条件执行：`IF` 指令根据条件变量跳转到不同指令块。

## 文件
- `compiler/src/ir.rs`（添加 `If` 指令）
- `executor/src/lib.rs`（实现跳转逻辑）
- `compiler/src/parser.rs`（解析）

## 具体要求

### 1. IR 扩展
```rust
If {
    condition_var: String,  // 变量名
    then_label: usize,      // 条件为真跳转的目标指令索引
    else_label: usize,      // 条件为假跳转的目标指令索引
}
```

### 2. 执行引擎修改
- 维护当前指令指针 `pc`
- 遇到 If 指令时读取条件变量，为真则跳转 then_label，否则跳转 else_label
- 设置最大执行步数（例如 10000）防止无限循环

### 3. 解析器支持
解析 n8n IF 节点，转换为 If 指令。

## 测试用例
- 设置 condition = true
- If condition, then 跳转到第3条指令，else 跳转到第4条
- 应执行 then 分支

## 验收标准
- 单元测试通过
- 支持嵌套 IF
- 最大步数保护生效