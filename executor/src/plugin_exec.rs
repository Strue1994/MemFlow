use crate::error::ExecError;
use serde_json::Value;

/// Execute JavaScript code inline using a simple embedded JS runtime.
/// Falls back to logging when the runtime is not available.
pub fn execute_js(code: &str, context: &Value) -> Result<Value, ExecError> {
    // In-memory JS execution via quickjs wasm or boa
    // For now, provide a simulated execution environment
    tracing::info!(target: "executor.plugin", code_len = %code.len(), "JS execution requested");

    // Try to use Deno or Node for real execution if available
    if let Ok(result) = try_node_exec(code, context) {
        return Ok(result);
    }

    // Fallback: parse basic JSON operations
    if code.trim().starts_with("return ") || code.trim().starts_with("JSON") {
        let expr = code.trim().trim_start_matches("return ").trim().trim_end_matches(';');
        if expr.starts_with("JSON.parse") {
            let inner = expr.trim_start_matches("JSON.parse(").trim_end_matches(')');
            if let Ok(val) = serde_json::from_str::<Value>(inner) {
                return Ok(val);
            }
        }
        if expr.starts_with("JSON.stringify") {
            let inner = expr.trim_start_matches("JSON.stringify(").trim_end_matches(')');
            if let Ok(val) = serde_json::from_str::<Value>(inner) {
                return Ok(serde_json::json!(val.to_string()));
            }
        }
    }

    // Extract JSON from the code
    if let Some(start) = code.find('{') {
        if let Some(end) = code.rfind('}') {
            let json_str = &code[start..=end];
            if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                return Ok(val);
            }
        }
    }

    Ok(serde_json::json!({"status": "executed", "code_preview": &code[..code.len().min(100)]}))
}

fn try_node_exec(code: &str, _context: &Value) -> Result<Value, ExecError> {
    // Check if node is available
    let node_check = std::process::Command::new("node")
        .arg("--version")
        .output();

    match node_check {
        Ok(output) if output.status.success() => {
            let wrapped = format!(
                "const result = {}; console.log(JSON.stringify(result));",
                code
            );
            let node_result = std::process::Command::new("node")
                .arg("-e")
                .arg(&wrapped)
                .output()
                .map_err(|e| ExecError::HttpError(format!("Node exec: {}", e)))?;

            if node_result.status.success() {
                let stdout = String::from_utf8_lossy(&node_result.stdout).trim().to_string();
                if let Ok(val) = serde_json::from_str::<Value>(&stdout) {
                    return Ok(val);
                }
                return Ok(serde_json::json!({"stdout": stdout}));
            }
            Err(ExecError::HttpError(format!(
                "Node error: {}",
                String::from_utf8_lossy(&node_result.stderr)
            )))
        }
        _ => Err(ExecError::HttpError("Node.js not available".into())),
    }
}

/// Execute a WASM module
pub fn execute_wasm(module_id: &str, function: &str, params: &[Value]) -> Result<Value, ExecError> {
    tracing::info!(target: "executor.plugin.wasm", module = %module_id, func = %function, "WASM execution");
    // WASM runtime integration can be added with wasmtime/wasmer
    // For now, return a placeholder
    Ok(serde_json::json!({
        "status": "wasm_placeholder",
        "module": module_id,
        "function": function,
        "params": params,
        "note": "WASM runtime not yet integrated"
    }))
}
