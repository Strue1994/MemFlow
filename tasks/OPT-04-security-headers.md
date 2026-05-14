# OPT-04: Add Security Headers Middleware

## 目标

为 HTTP 服务添加安全 headers，防止常见 Web 攻击。

## 当前状态

没有安全 headers。

## 实现方案

1. **Rust (Executor HTTP Server)**
   ```rust
   use hyper::header::*;
   
   pub fn add_security_headers(mut rsp: Response) -> Response {
       rsp.headers_mut().insert(
           HeaderName::from_static("x-content-type-options"),
           HeaderValue::from_static("nosniff"),
       );
       rsp.headers_mut().insert(
           HeaderName::from_static("x-frame-options"),
           HeaderValue::from_static("DENY"),
       );
       rsp.headers_mut().insert(
           HeaderName::from_static("x-xss-protection"),
           HeaderValue::from_static("1; mode=block"),
       );
       rsp.headers_mut().insert(
           HeaderName::from_static("strict-transport-security"),
           HeaderValue::from_static("max-age=31536000; includeSubDomains"),
       );
       rsp.headers_mut().insert(
           HeaderName::from_static("content-security-policy"),
           HeaderValue::from_static("default-src 'self'"),
       );
       rsp
   }
   ```

2. **TypeScript (Agent Service)**
   ```typescript
   import helmet from 'helmet';
   
   const helmetConfig = {
     contentSecurityPolicy: {
       directives: {
         defaultSrc: ["'self'"],
         scriptSrc: ["'self'"],
         styleSrc: ["'self'", "'unsafe-inline'"],
         imgSrc: ["'self'", 'data:', 'https:'],
       },
     },
     crossOriginEmbedderPolicy: false,
     contentSecurityPolicy: {
       'upgrade-insecure-requests': [],
     },
   };
   
   app.use(helmet(helmetConfig));
   ```

## 影响文件

- `executor/src/http_server.rs`
- `agent-service/src/index.ts`

## 验证方法

检查响应 headers 包含安全 headers。

## 优先级

MEDIUM - 安全加固