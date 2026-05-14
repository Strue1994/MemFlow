# P0-3: Core Unit Tests

## Priority

P0 - Immediate

## Key Files / Modules

- `compiler/src/parser.rs`
- `executor/src/http.rs`
- `executor/src/code.rs`
- `executor/src/workflow_registry.rs`
- `compiler/src/ir.rs`

## Goals

为关键模块添加单元测试，确保基础逻辑的健壮性。

## Specific Requirements

1.  **Parser Tests**
   - 测试合法 n8n JSON 解析成功
   - 测试非法 JSON 解析失败
   - 测试边界条件 (空数组、null 节点等)

2.  **Node Tests**
   - `http.rs`: 模拟 HTTP 响应，测试成功/失败分支
   - `code.rs`: 测试 JS 执行和超时处理
   - `db.rs`: 测试 SQL 注入防护

3.  **Registry Tests**
   - 并发读写测试
   - 错误处理测试 (未初始化、版本不存在)

4.  **Coverage Target**
   - 使用 `cargo tarpaulin` 测量
   - 目标覆盖率 > 60%

## Acceptance Criteria

- [ ] `cargo test` 通过所有测试
- [ ] 覆盖率报告生成成功
- [ ] 关键路径有测试覆盖

## Test Examples

```rust
#[test]
fn test_parser_valid_workflow() {
    let json = r#"{"nodes": [{"id": "1", "type": "HttpRequest"}]}"#;
    assert!(parse_n8n_workflow(json).is_ok());
}

#[test]
fn test_parser_invalid_json() {
    let json = r#"{"nodes": "invalid"}"#;
    assert!(parse_n8n_workflow(json).is_err());
}

#[test]
fn test_http_node_success() {
    let mock = MockServer::new().returning(200, json!({"ok": true}));
    let result = execute_http_request(HttpMethod::Get, mock.url(), &[], &None);
    assert!(result.is_ok());
}
```