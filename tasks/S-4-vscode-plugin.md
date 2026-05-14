# 任务 S-4：工作流即代码编译器（VS Code 插件）

## 目标
开发 VS Code 插件，允许用户使用 TypeScript 编写工作流，自动编译为 n8n JSON 并上传到 MemFlow。

## 文件
- `extensions/vscode/package.json`
- `extensions/vscode/src/extension.ts`（插件入口）
- `extensions/vscode/src/compiler.ts`（调用 MemFlow 编译器 API）
- `extensions/vscode/src/language_features.ts`（语法高亮、补全）
- `sdk/typescript/src/index.ts`（提供给插件的 SDK）

## 具体要求

### 1. TypeScript SDK
- 定义工作流构建器 API：
  ```typescript
  import { workflow, http, set } from '@memflow/sdk';
  export default workflow()
    .step(http.get('https://api.github.com/zen'))
    .step(set('result', (ctx) => ctx.body))
    .export();
  ```
- 内部将 AST 转换为 n8n JSON。

### 2. VS Code 插件功能
- 识别 `.memflow.ts` 文件，提供语法高亮和代码补全。
- 提供命令 `MemFlow: Compile and Upload`：
  - 调用本地或远程 MemFlow 编译服务，将 TypeScript 转换为 JSON。
  - 调用 MemFlow API 创建工作流（或更新现有工作流）。
- 显示编译错误和上传结果。

### 3. 调试支持
- 提供 `MemFlow: Run Locally` 命令，在本地沙箱中执行工作流（需 Node.js 环境模拟）。

### 4. 与 MemFlow 实例集成
- 插件配置项：`memflow.apiUrl`, `memflow.apiKey`。
- 支持从工作流 ID 反向生成 TypeScript 代码（导出功能）。

## 验收标准
- 用户编写 TypeScript 工作流，保存后一键上传，在 MemFlow Web UI 中可见。
- 语法错误能在 VS Code 中红色下划线提示。
- 可导出已有工作流为 TypeScript 代码。

## 预估 token
4000