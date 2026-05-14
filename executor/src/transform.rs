use crate::error::ExecError;
use serde_json::Value;

pub fn execute_transform_json(input: &Value, transformation: &Value) -> Result<Value, ExecError> {
    match transformation {
        Value::String(path) => jsonpath_query(input, path),
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, val) in map {
                if let Some(path) = val.as_str() {
                    let resolved = jsonpath_query(input, path)?;
                    result.insert(key.clone(), resolved);
                } else { result.insert(key.clone(), val.clone()); }
            }
            Ok(Value::Object(result))
        }
        _ => Err(ExecError::HttpError("Transform must be string (JSONPath) or object".into())),
    }
}

fn jsonpath_query(val: &Value, path: &str) -> Result<Value, ExecError> {
    let p = path.trim_start_matches("$.").trim_start_matches('$');
    if p.is_empty() { return Ok(val.clone()); }
    let mut cur = val.clone();
    for seg in p.split('.') {
        if seg.is_empty() { continue; }
        if let Some(bracket) = seg.find('[') {
            let field = &seg[..bracket];
            if !field.is_empty() { cur = cur.get(field).cloned().unwrap_or(Value::Null); }
            let idx_str = seg[bracket..].trim_start_matches('[').trim_end_matches(']');
            if idx_str == "*" { break; }
            if let Ok(idx) = idx_str.parse::<usize>() { cur = cur.get(idx).cloned().unwrap_or(Value::Null); }
            if let Some(dot) = seg.rfind(']') {
                let after = &seg[dot+1..];
                if after.starts_with('.') { return jsonpath_query(&cur, &format!("${}", after)); }
            }
        } else { cur = cur.get(seg).cloned().unwrap_or(Value::Null); }
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_simple() { assert_eq!(jsonpath_query(&serde_json::json!({"a":1}), "$.a").unwrap(), serde_json::json!(1)); }
    #[test] fn test_nested() { assert_eq!(jsonpath_query(&serde_json::json!({"x":{"y":[{"z":3}]}}), "$.x.y[0].z").unwrap(), serde_json::json!(3)); }
}
