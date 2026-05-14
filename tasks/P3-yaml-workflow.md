# 任务 P3：声明式 YAML 工作流定义

## 目标
支持使用简洁的 YAML 格式定义工作流，替代冗长的 n8n JSON，并可通过编译器转换为内部 IR。

## 文件
- `compiler/src/yaml_parser.rs`（新建，YAML 到 IR 转换器）
- `compiler/src/yaml_schema.rs`（YAML 格式定义）
- `executor/src/main.rs`（支持 `--yaml` 参数加载 YAML 文件）
- `agent-service/src/workflowGenerator.ts`（可选，支持输出 YAML 格式）

## 具体要求

### 1. YAML 格式示例
```yaml
name: Fetch GitHub Zen
steps:
  - id: request
    http:
      method: GET
      url: https://api.github.com/zen
      headers:
        User-Agent: MemFlow
  - id: extract
    set:
      result: "{{steps.request.response}}"
  - return: result
```

支持的控制流：
- `if` 条件分支
- `for` 循环
- `call` 调用子工作流

### 2. 编译器转换
- 将 YAML 解析为 AST，再转换为 n8n 兼容的 JSON（或直接生成 IR）。
- 提供错误检查（变量引用、类型匹配）。

### 3. CLI 支持
- 增加 `compile --yaml input.yaml --output workflow.json` 命令。
- 执行引擎可直接接受 YAML 文件：`run --yaml workflow.yaml`。

### 4. 与 n8n 格式双向转换
- 提供 `yaml2n8n` 和 `n8n2yaml` 工具，便于与 n8n 生态互通。

## 测试用例
- 将上述 YAML 示例转换为 n8n JSON，再反向转换，应保持一致。
- 执行 YAML 定义的工作流，结果与 n8n JSON 相同。

## 验收标准
- YAML 语法直观，学习成本低。
- 转换工具正确性 100%。
- 执行性能不低于 n8n JSON 格式。

## 预估 token
2800