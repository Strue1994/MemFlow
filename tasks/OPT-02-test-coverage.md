# OPT-02: Increase Test Coverage

## 目标

提高测试覆盖率到 80%+。

## 当前状态

部分模块有测试，覆盖率不足。

## 实现方案

1. **添加 test coverage 工具**
   ```bash
   # 安装
   cargo install cargo-llvm-cov
   cargo install cargo-tarpaulin
   
   # 生成报告
   cargo llvm-cov --html --output target/coverage
   ```

2. **创建测试目标**
   - executor 模块: 85%+
   - compiler 模块: 90%+
   - learning-engine 模块: 70%+

3. **补充缺失测试**
   - HTTP node 测试
   - Database node 测试
   - Workflow execution tests
   - Concurrency tests

4. **添加集成测试**
   ```rust
   #[cfg(test)]
   mod integration_tests {
       #[test]
       fn test_full_workflow_execution() {
           // 测试完整 workflow 执行
       }
       
       #[test]
       fn test_error_handling() {
           // 测试错误处理
       }
   }
   ```

## 影响文件

- 添加 `tests/` 目录测试
- 配置 `.cargo/config.toml`

## 验证方法

运行 `cargo llvm-cov` 检查覆盖率。

## 优先级

MEDIUM - 质量提升