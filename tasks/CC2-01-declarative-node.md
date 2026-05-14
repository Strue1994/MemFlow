# 任务 CC2-01：声明式节点注册与 Schema 验证

## 目标
允许通过 JSON/YAML 定义新节点，包括输入/输出 Schema（Zod 或 JSON Schema），并支持动态加载（无需重新编译）。借鉴 Claude Code 的工具定义方式。

## 文件
- `executor/src/dynamic_node.rs`（已有 P1 部分，需增强）
- `compiler/src/schema_validator.rs`（新建，使用 `jsonschema` 库）
- `agent-service/src/node_registry_api.ts`（提供节点上传接口）
- `web-ui/src/components/NodeCreator.tsx`（UI 表单生成器）

## 具体要求

### 1. 节点定义格式
```json
{
  "name": "my_api",
  "description": "调用我的自定义 API",
  "input_schema": {
    "type": "object",
    "properties": {
      "endpoint": { "type": "string" },
      "payload": { "type": "object" }
    },
    "required": ["endpoint"]
  },
  "output_schema": { "type": "object" },
  "executor": {
    "type": "http",
    "url": "https://myapi.com/{{endpoint}}",
    "method": "POST"
  }
}
```

### 2. 执行器类型
- `wasm`: 调用 WASM 模块
- `http`: 发送 HTTP 请求
- `command`: 执行本地命令（沙箱）
- `inline_js`: 安全执行 JavaScript（使用 vm2 或 isolated-vm）

### 3. 编译时集成
- 启动时扫描 `./nodes/` 目录，加载所有 `.node.json` 文件。
- 将定义转换为内部 `Instruction::CallDynamicNode`。
- 执行时根据 executor 类型分发。

### 4. UI 支持
- 根据 input_schema 自动生成表单。
- 提供"测试节点"功能。

## 验收标准
- 添加一个新节点只需创建一个 JSON 文件，无需重启执行引擎（热加载）。
- 输入验证错误时返回清晰提示。
- 性能不低于原生 Rust 节点的 80%。

## 预估 token
3500