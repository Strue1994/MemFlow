use crate::environment::Environment;
use crate::error::ExecError;
use rquickjs::{Array, Context, Function, Object, Runtime, Value};
use serde_json::Value as JsonValue;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

pub fn execute_js(
    script: &str,
    env: &Environment,
    output_var: &str,
    result_env: &mut Environment,
) -> Result<(), ExecError> {
    let start = Instant::now();
    if start + DEFAULT_TIMEOUT < Instant::now() {
        return Err(ExecError::CodeError("Script execution timeout".to_string()));
    }

    let (tx, rx) = mpsc::channel();
    let script = script.to_string();
    let env_vars: Vec<(String, JsonValue)> =
        env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    let _ = thread::spawn(move || {
        let rt = Runtime::new().map_err(|e| ExecError::CodeError(e.to_string()))?;
        let ctx = Context::full(&rt).map_err(|e| ExecError::CodeError(e.to_string()))?;

        let result = ctx.with(|ctx| {
            for (key, value) in env_vars {
                let js_value = json_to_js(ctx, value)?;
                ctx.globals()
                    .set(key.as_str(), js_value)
                    .map_err(|e| ExecError::CodeError(e.to_string()))?;
            }

            let result: Value = ctx
                .eval(&script)
                .map_err(|e| ExecError::CodeError(e.to_string()))?;
            let output = js_to_json(result)?;
            Ok(output)
        });

        let _ = tx.send(result);
    });

    match rx.recv_timeout(DEFAULT_TIMEOUT) {
        Ok(Ok(value)) => {
            result_env.set(output_var, value);
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ExecError::CodeError("Script execution timeout".to_string())),
    }
}

fn json_to_js(ctx: &rquickjs::Context, value: JsonValue) -> Result<Value<'static>, ExecError> {
    match value {
        JsonValue::Null => Ok(Value::new_null(ctx.as_ref())),
        JsonValue::Bool(b) => Ok(Value::new_bool(ctx.as_ref(), b)),
        JsonValue::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(Value::new_number(ctx.as_ref(), f))
            } else {
                Ok(Value::new_int(ctx.as_ref(), n.as_i64().unwrap_or(0)))
            }
        }
        JsonValue::String(s) => Ok(Value::new_string(ctx.as_ref(), &s)
            .map_err(|e| ExecError::CodeError(e.to_string()))?
            .into()),
        JsonValue::Array(arr) => {
            let js_arr =
                Array::new(ctx.as_ref()).map_err(|e| ExecError::CodeError(e.to_string()))?;
            for (i, v) in arr.into_iter().enumerate() {
                let js_v = json_to_js(ctx, v)?;
                js_arr
                    .set(i, js_v)
                    .map_err(|e| ExecError::CodeError(e.to_string()))?;
            }
            Ok(js_arr.into())
        }
        JsonValue::Object(obj) => {
            let js_obj =
                Object::new(ctx.as_ref()).map_err(|e| ExecError::CodeError(e.to_string()))?;
            for (k, v) in obj.into_iter() {
                let js_v = json_to_js(ctx, v)?;
                js_obj
                    .set(k.as_str(), js_v)
                    .map_err(|e| ExecError::CodeError(e.to_string()))?;
            }
            Ok(js_obj.into())
        }
    }
}

fn js_to_json(value: Value) -> Result<JsonValue, ExecError> {
    if value.is_null() || value.is_undefined() {
        return Ok(JsonValue::Null);
    }
    if let Some(b) = value.as_bool() {
        return Ok(JsonValue::Bool(b));
    }
    if let Some(n) = value.as_number() {
        return Ok(JsonValue::Number(
            serde_json::Number::from_f64(n).unwrap_or(serde_json::Number::from(0)),
        ));
    }
    if let Some(s) = value.as_string() {
        return Ok(JsonValue::String(s.to_string()));
    }
    if let Some(arr) = value.as_array() {
        let mut vec = Vec::new();
        for i in 0..arr.len() {
            if let Ok(v) = arr.get::<rquickjs::Value>(i) {
                vec.push(js_to_json(v)?);
            }
        }
        return Ok(JsonValue::Array(vec));
    }
    if let Some(obj) = value.as_object() {
        let map = serde_json::Map::new();
        let keys: Vec<String> = obj.keys().map(|k| k.to_string()).collect();
        let mut result_map = map;
        for key in keys {
            if let Ok(v) = obj.get::<rquickjs::Value>(&key) {
                result_map.insert(key, js_to_json(v)?);
            }
        }
        return Ok(JsonValue::Object(result_map));
    }
    Ok(JsonValue::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;
    use serde_json::Number;

    #[test]
    fn test_execute_js_simple() {
        let mut env = Environment::new();
        env.set("x", JsonValue::Number(Number::from(5)));

        let mut result_env = Environment::new();
        execute_js("x * 2", &env, "result", &mut result_env).unwrap();

        let result = result_env.get("result").unwrap();
        assert_eq!(to_number(result).unwrap(), 10.0);
    }

    fn to_number(v: &JsonValue) -> Result<f64, ExecError> {
        match v {
            JsonValue::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
            JsonValue::String(s) => s
                .parse::<f64>()
                .map_err(|_| ExecError::CodeError("Cannot parse as number".to_string())),
            _ => Err(ExecError::CodeError("Expected number".to_string())),
        }
    }
}
