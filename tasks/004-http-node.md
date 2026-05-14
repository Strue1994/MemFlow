# 任务 004：HTTP 节点实现

## 目标
实现 `Instruction::HttpGet` 指令，发送 HTTP GET 请求并将响应存入变量。

## 文件
- `executor/src/http.rs`（新建）
- `executor/src/lib.rs`（集成）

## 具体要求

### 1. 添加依赖
在 `executor/Cargo.toml` 中添加 `reqwest`（使用 `blocking` feature）。

### 2. 实现 `execute_http_get` 函数
在 `http.rs` 中：
```rust
pub fn execute_http_get(
    url: &str,
    headers: &[(String, String)],
) -> Result<Value, ExecError>
```
使用 `reqwest::blocking::Client`。

设置默认超时 10 秒。

将响应体解析为 JSON（若失败则返回原始文本）。

返回 Value。

### 3. 在 Executor::execute 中处理 Instruction::HttpGet
调用 `execute_http_get`。

将结果存入 `output_var` 变量。

## 测试用例
使用公共 API：https://api.github.com/zen。预期返回字符串（非空）。

注意 GitHub API 需要 User-Agent header，请在请求中添加 `User-Agent: memflow-executor/1.0`。

## 验收标准
- 单元测试（需要网络，可标记 `#[ignore]`，但手动运行通过）。
- 错误处理：网络错误、非 JSON 响应、超时等应返回 ExecError。
- 代码中无 unwrap（除测试外）。

## 预期输出格式
##FILE:executor/src/http.rs
```rust
// 实现
```
##FILE:executor/src/lib.rs
```rust
// 添加 mod http; 并在 execute 匹配中添加 HttpGet 分支
```
