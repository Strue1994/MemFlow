# 任务 007：实现 code 节点（JavaScript 引擎）

## 目标
在执行引擎中增加 `Instruction::Code`，能够执行一段 JavaScript 代码，访问环境变量，并返回结果。

## 文件
- `executor/src/code.rs`（新建）
- `executor/src/lib.rs`（修改，添加指令处理）
- `compiler/src/ir.rs`（添加 `Code` 指令变体）
- `compiler/src/parser.rs`（支持解析 n8n 的 `code` 节点）

## 具体要求

### 1. 依赖添加
在 `executor/Cargo.toml` 中添加：
```toml
rquickjs = { version = "0.5", features = ["alloc", "futures"] }
```

### 2. IR 扩展
在 compiler/src/ir.rs 的 Instruction 枚举中添加：
```rust
Code { script: String, output_var: String }
```

### 3. 解析器支持
在 parser.rs 中，对于 type 为 n8n-nodes-base.code 的节点：
- 从 parameters.jsCode 获取脚本字符串
- 从 parameters.output 获取输出变量名（默认 "output"）
- 生成 Instruction::Code

### 4. 执行引擎实现
在 executor/src/code.rs 中实现 JavaScript 执行，支持将环境变量注入为全局变量。

### 5. 错误处理
定义 ExecError::CodeError(String)。

## 测试用例
输入工作流：
- 先 SetVariable 设置 x = 5
- 然后 Code 节点脚本：x * 2
- 返回结果应为 10

## 验收标准
- 单元测试通过。
- 支持访问已定义的变量。
- 支持返回数字、字符串、对象、数组。
- 脚本执行超时（默认 1 秒）保护。

## 预期输出格式
##FILE:compiler/src/ir.rs
（添加 Code 变体）
##FILE:compiler/src/parser.rs
（添加解析逻辑）
##FILE:executor/src/code.rs
（完整实现）
##FILE:executor/src/lib.rs
（添加 mod code，并在 execute 中处理 Code 指令）