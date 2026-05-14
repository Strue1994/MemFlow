# 任务 P1：插件化节点注册

## 目标
允许用户通过配置文件或 Web UI 上传 WASM / JavaScript 代码作为新节点类型，运行时动态加载，无需重新编译 MemFlow。

## 文件
- `executor/src/plugin.rs`（新建，插件管理器）
- `executor/src/wasm_plugin.rs`（使用 wasmtime 加载 WASM 插件）
- `executor/src/js_plugin.rs`（使用 rquickjs 加载 JS 插件）
- `compiler/src/dynamic_node.rs`（动态节点类型注册）
- `executor/src/plugin_api.rs`（提供插件上传/管理 API）
- `web-ui/src/components/PluginManager.tsx`（UI 管理界面）

## 具体要求

### 1. 插件定义规范
每个插件是一个独立的 WASM 或 JS 模块，需导出以下元数据：
- `name`: 节点类型名称（唯一）
- `schema`: 输入参数的 JSON Schema
- `output_schema`: 输出数据的 JSON Schema
- `execute`: 执行函数（接收参数，返回结果）

示例 WASM 插件（Rust 编写）：
```rust
#[no_mangle]
pub fn execute(input_ptr: *const u8, input_len: usize) -> *mut u8 {
    // 解析输入 JSON，执行业务逻辑，返回输出 JSON
}
```

### 2. 插件管理器
- 维护 `HashMap<String, Plugin>`，支持动态注册/卸载。
- 插件存储目录：`./plugins/`，支持热加载（文件变化自动重新加载）。
- 提供 HTTP API：`POST /plugins/upload`（上传 .wasm 或 .js），`GET /plugins`（列出已安装插件），`DELETE /plugins/{name}`（卸载）。

### 3. 执行引擎集成
- 添加新的指令 `Instruction::CallPlugin { name, params, output_var }`。
- 在 `execute` 中调用插件管理器的 `call_plugin(name, params)`。

### 4. 解析器支持
- 允许 n8n JSON 中使用 `"type": "plugin:my_custom_api"`，解析时映射到 `CallPlugin`。

### 5. 安全限制
- WASM 插件运行在沙箱中（内存限制 16MB，CPU 时间限制 1 秒）。
- JS 插件使用 `rquickjs` 隔离，禁止访问文件系统和网络。

## 验收标准
- 上传一个简单的 WASM 插件（例如"乘以2"），可在工作流中调用并返回正确结果。
- 插件热加载生效（修改文件后无需重启）。
- 插件调用性能接近原生节点（损耗 < 20%）。

## 预估 token
3500