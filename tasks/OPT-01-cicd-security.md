# OPT-01: Add CI/CD Security Scanning

## 目标

在 CI/CD 流程中添加安全检查，提高代码安全性。

## 当前状态

没有安全扫描步骤。

## 实现方案

1. **添加 SAST 工具**
   ```yaml
   # .github/workflows/security.yml
   name: Security Scan
   
   on: [push, pull_request]
   
   jobs:
     security-scan:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         
         - name: Run Rust security audit
           run: |
             cargo install cargo-audit
             cargo audit
             
         - name: Run clippy
           run: cargo clippy -- -D warnings
           
         - name: Run safety checks
           run: |
             cargo install cargo-deny
             cargo deny check
   ```

2. **添加 dependency scanning**
   ```yaml
         - name: Check for vulnerabilities
           uses: rustsec/audit-check@v1.4
           with:
             token: ${{ secrets.GITHUB_TOKEN }}
   ```

3. **添加代码质量检查**
   ```yaml
         - name: Rustfmt
           run: cargo fmt -- --check
           
         - name: Miri (undefined behavior)
           run: cargo +nightly miri test
   ```

## 影响文件

- 添加 `.github/workflows/security.yml`
- 添加 `deny.toml`

## 验证方法

Pull request 时自动运行安全扫描。

## 优先级

MEDIUM - 质量提升