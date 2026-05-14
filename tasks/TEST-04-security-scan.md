# 任务 TEST-04：安全扫描

## 目标
集成依赖漏洞扫描。

## 文件
- `.github/workflows/security.yml`

## 具体要求
- Rust: `cargo audit`
- Node.js: `npm audit`
- 容器: `trivy`

## 验收标准
- CI 集成，阻止高危依赖

## 预估 token
2000