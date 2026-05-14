# 任务 002：n8n JSON → IR 解析器

## 目标
实现将 n8n 工作流 JSON 转换为 `Workflow` IR 的解析器。先支持线性顺序（忽略连线），支持三种节点类型：`httpRequest`、`set`、`code`。

## 文件
- `compiler/src/parser.rs`（新建）
- `compiler/src/error.rs`（新建，定义 `ParseError`）
- `compiler/src/lib.rs`（添加模块）

## 具体要求

### 1. 定义 `ParseError`
使用 `thiserror`，至少包含：
- `InvalidJson`
- `MissingField(String)`
- `UnsupportedNodeType(String)`

### 2. 实现 `parse_n8n_workflow`
函数签名：
```rust
pub fn parse_n8n_workflow(json_str: &str) -> Result<Workflow, ParseError>
```

### 3. 节点转换规则
- `httpRequest` 节点 → `Instruction::HttpGet`
  - 从 `parameters.url` 获取 URL
  - 从 `parameters.headers` 获取 headers（可选，数组格式）
  - 输出变量名：`parameters.responseFormat` 或默认 "response"
- `set` 节点 → `Instruction::SetVariable`
  - 从 `parameters.values` 获取键值对
- `code` 节点 → 暂时忽略（返回空指令占位）

### 4. 处理顺序
忽略 connections，按 nodes 数组的顺序生成指令。

### 测试用例
提供以下 JSON 输入：

```json
{
  "nodes": [
    {
      "id": "1",
      "type": "n8n-nodes-base.httpRequest",
      "parameters": { "url": "https://api.github.com/zen" }
    },
    {
      "id": "2",
      "type": "n8n-nodes-base.set",
      "parameters": { "values": { "myVar": "hello" } }
    }
  ]
}
```
解析后应得到 Workflow 包含两条指令：先 HttpGet，后 SetVariable。

## 验收标准
- 单元测试通过（编写 #[cfg(test)] 模块，测试上述用例）。
- 错误处理完善，无崩溃。

## 预期输出格式
##FILE:compiler/src/error.rs
```rust
// 错误定义
```
##FILE:compiler/src/parser.rs
```rust
// 解析器实现
```
##FILE:compiler/src/lib.rs
```rust
// 添加 pub mod error; pub mod parser;
```