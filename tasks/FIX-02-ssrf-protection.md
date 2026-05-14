# FIX-02: HTTP Node SSRF Protection

## 问题描述

HTTP 节点没有对 URL 进行验证，可能允许 SSRF (Server-Side Request Forgery) 攻击。攻击者可能利用内部服务 URL 进行端口扫描或访问内部系统。

## 当前代码

`executor/src/http.rs` 中直接使用用户提供的 URL，没有验证:

```rust
pub fn execute_http_request(
    method: HttpMethod,
    url: &str,
    headers: &[(String, String)],
    body: &Option<Value>,
) -> Result<Value, ExecError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    // 直接使用 url，没有任何验证
    let mut request = match method {
        HttpMethod::Get => client.get(url),
        ...
    };
}
```

## 修复方案

1. 添加 URL 黑名单检查:
   - localhost, 127.0.0.1, ::1
   - 10.x.x.x, 172.16.x.x, 192.168.x.x (私有地址)
   - 169.254.x.x (链路本地地址)
   - 0.0.0.0, metadata 服务

2. 添加 URL 白名单机制 (可选)

3. 添加警告日志记录内部请求

## 影响文件

- `executor/src/http.rs`

## 验证方法

尝试使用内部 URL 执行 HTTP 请求，确认被阻止或警告。

## 优先级

HIGH - 安全漏洞