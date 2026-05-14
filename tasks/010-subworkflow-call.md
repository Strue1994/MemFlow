# 任务 010：子工作流调用

## 目标
支持一个工作流调用另一个已编译的工作流，实现复用。

## 文件
- `compiler/src/ir.rs`（添加 `CallWorkflow` 指令）
- `executor/src/workflow_registry.rs`（新增）
- `executor/src/lib.rs`（实现调用逻辑）

## 具体要求

### 1. IR 扩展
```rust
CallWorkflow {
    workflow_id: String,
    params: Vec<(String, String)>,
    output_var: String,
}
```

### 2. 工作流注册表
- `WorkflowRegistry` 内部为 `HashMap<String, Workflow>`
- 提供 `register` 和 `get` 方法

### 3. 执行逻辑
- 从当前环境读取参数，构建子工作流环境
- 递归执行子工作流
- 最大递归深度 10

## 验收标准
- 单元测试通过