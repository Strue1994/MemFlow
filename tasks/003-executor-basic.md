# 任务 003：执行引擎基础框架

## 目标
实现执行引擎的基础结构，支持 `SetVariable` 和 `MathOp` 指令，以及变量环境管理。

## 文件
- `executor/src/lib.rs`
- `executor/src/environment.rs`（新建）
- `executor/src/error.rs`（新建）

## 具体要求

### 1. 定义 `ExecError`
使用 `thiserror`，包含：
- `VariableNotFound(String)`
- `MathError(String)`
- `InvalidReturn`

### 2. 实现 `Environment`
结构体，内部为 `HashMap<String, Value>`，提供：
- `fn set(&mut self, name: &str, value: Value)`
- `fn get(&self, name: &str) -> Result<&Value, ExecError>`

### 3. 实现 `Executor`
结构体包含 `env: Environment`。
方法：
- `fn new() -> Self`
- `fn execute(&mut self, workflow: &Workflow) -> Result<Value, ExecError>`

### 4. 指令执行逻辑
- `SetVariable`：将值存入环境。
- `MathOp`：从环境读取 `lhs` 和 `rhs`（支持数字或字符串数字），执行运算，结果存入 `output` 变量。
- `Return`：从环境读取指定变量并返回。

## 测试用例
工作流：
```rust
let wf = Workflow {
    instructions: vec![
        Instruction::SetVariable { name: "a".to_string(), value: Value::Number(2.into()) },
        Instruction::SetVariable { name: "b".to_string(), value: Value::Number(3.into()) },
        Instruction::MathOp { op: MathOp::Add, lhs: "a".to_string(), rhs: "b".to_string(), output: "c".to_string() },
        Instruction::Return { value: "c".to_string() },
    ]
};
```
执行应返回 `Value::Number(5)`.

## 验收标准
- 单元测试覆盖上述用例。
- 支持整数和浮点数运算。
- 错误处理完善（变量缺失、类型错误）。

## 预期输出格式
##FILE:executor/src/error.rs
```rust
// 错误定义
```
##FILE:executor/src/environment.rs
```rust
// Environment 实现
```
##FILE:executor/src/lib.rs
```rust
// Executor 实现
```