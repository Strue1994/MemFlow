/// P4.1: Real WASM plugin runtime using wasmtime

use crate::error::ExecError;
use serde_json::Value;

#[cfg(feature = "wasm")]
pub fn execute_wasm_plugin(module_bytes: &[u8], function: &str, params: &Value) -> Result<Value, ExecError> {
    use wasmtime::{Engine, Module, Store, Func, TypedFunc};
    use wasmtime::Val;

    let engine = Engine::default();
    let module = Module::new(&engine, module_bytes)
        .map_err(|e| ExecError::HttpError(format!("WASM module: {}", e)))?;
    let mut store = Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])
        .map_err(|e| ExecError::HttpError(format!("WASM instance: {}", e)))?;

    let func = instance.get_typed_func::<(i32,), i32>(&mut store, function)
        .map_err(|e| ExecError::HttpError(format!("WASM func {}: {}", function, e)))?;

    let input = params.to_string();
    let result = func.call(&mut store, input.len() as i32)
        .map_err(|e| ExecError::HttpError(format!("WASM call: {}", e)))?;

    Ok(serde_json::json!({"status": "executed", "result": result, "module": "wasm"}))
}

#[cfg(not(feature = "wasm"))]
pub fn execute_wasm_plugin(_module: &[u8], function: &str, _params: &Value) -> Result<Value, ExecError> {
    tracing::info!(target: "executor.wasm", func = %function, "WASM execution (enable 'wasm' feature)");
    Ok(serde_json::json!({"status": "simulated", "function": function}))
}
