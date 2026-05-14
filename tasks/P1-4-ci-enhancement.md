# P1-4: CI Enhancement

## Priority

P1 - Near-term

## Key Files / Modules

- `.github/workflows/ci.yml`

## Goals

在 CI 中加入覆盖率报告和镜像构建，提高自动化水平。

## Specific Requirements

1.  **Coverage Report**
   - 添加 `cargo tarpaulin` 步骤
   - 上传到 Codecov

2.  **Security Audit**
   - 移除 `|| true`
   - 失败则 CI 失败

3.  **Docker Image Build**
   - 推送 tag `v*` 时触发构建
   - 构建并推送镜像到 registry

## Acceptance Criteria

- [ ] PR 创建后能看到覆盖率报告
- [ ] 推送 `v1.0.0` 标签后自动构建镜像

## Implementation

```yaml
name: CI

on:
  push:
  pull_request:
  release:
    types: [published]

jobs:
  rust-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --workspace
      
      - name: Coverage
        uses: action-tarpaulin@v1
        with:
          args: "--workspace"
          output-dir: ./coverage
      
      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/tarpaulin.xml
          flags: unittests

  security:
    runs-on: ubuntu-latest
    steps:
      - name: Rust audit
        run: |
          cargo install cargo-audit
          cargo audit
          # No longer || true - fail on vulnerability
      
      - name: NPM audit
        working-directory: agent-service
        run: npm audit --audit-level=high

  docker:
    needs: [rust-test, security]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Set tag
        run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_ENV
      
      - name: Build and push
        run: |
          docker build -t memflow/executor:${{ env.VERSION }} .
          docker push memflow/executor:${{ env.VERSION }}
```