# P2-3: Plugin System Security Sandbox

## Priority

P2 - Medium-term

## Key Files / Modules

- `executor/src/plugin.rs`
- `executor/src/js_plugin.rs`

## Goals

增强 JS 插件的资源限制和安全性。

## Specific Requirements

1.  **Memory Limit**
   - 使用 `rquickjs` 的 `set_memory_limit`
   - 默认 64MB
   - 可配置

2.  **Dangerous Operations**
   - 禁用 `__proto__`, `constructor`
   - 禁用全局 `eval`
   - 禁用 `Function` 构造函数

3.  **Plugin Signing**
   - 开发者模式下允许加载
   - 生产环境需签名校验
   - 签名密钥与 `EXECUTOR_API_KEY` 分离

4.  **Timeout**
   - 插件执行超时限制
   - 默认 30 秒

5.  **Network Restriction**
   - 插件默认不能发起网络请求
   - 需显式授权

## Acceptance Criteria

- [ ] 恶意插件尝试分配大量内存被强制终止
- [ ] 插件不能使用危险操作
- [ ] 未签名插件在生产环境被拒绝

## Implementation

```rust
pub struct JsPlugin {
    runtime: rquickjs::Runtime,
    ctx: rquickjs::Context,
}

impl JsPlugin {
    pub fn new(script: &str, config: &PluginConfig) -> Result<Self, PluginError> {
        let runtime = Runtime::new();
        
        // Memory limit
        runtime.set_memory_limit(config.max_memory_bytes);
        
        // Disable dangerous operations
        let ctx = Context::full(&runtime).map_err(PluginError::Init)?;
        
        // Check signature in production
        if !config.allow_dev && !verify_signature(script, &config.signature) {
            return Err(PluginError::SignatureInvalid);
        }
        
        Ok(Self { runtime, ctx })
    }
    
    pub fn execute(&self, params: Value) -> Result<Value, PluginError> {
        let start = Instant::now();
        
        self.ctx.with(|ctx| {
            // Set timeout
            let result = ctx.eval(script);
            
            // Check timeout
            if start.elapsed() > MAX_EXECUTION_TIME {
                return Err(PluginError::Timeout);
            }
            
            result
        })?;
        
        Ok(value)
    }
}

pub struct PluginConfig {
    pub max_memory_bytes: u64,
    pub allow_dev: bool,
    pub signature: Option<String>,
    pub network_allowed: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            allow_dev: false,
            signature: None,
            network_allowed: false,
        }
    }
}
```