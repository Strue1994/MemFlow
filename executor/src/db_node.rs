use crate::error::ExecError;
use once_cell::sync::Lazy;
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

static DB_POOLS: Lazy<Mutex<HashMap<String, Connection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_db_connection(name: &str, path: &str) -> Result<(), ExecError> {
    let conn = Connection::open(path)
        .map_err(|e| ExecError::DbError(format!("Failed to open database: {}", e)))?;

    let mut pools = DB_POOLS
        .lock()
        .map_err(|e| ExecError::DbError(format!("Lock error: {}", e)))?;
    pools.insert(name.to_string(), conn);
    Ok(())
}

pub fn execute_db_query(
    conn_name: &str,
    query: &str,
    params: &[Value],
) -> Result<Value, ExecError> {
    let pools = DB_POOLS
        .lock()
        .map_err(|e| ExecError::DbError(format!("Lock error: {}", e)))?;

    let conn = pools
        .get(conn_name)
        .ok_or_else(|| ExecError::DbError(format!("Connection '{}' not found", conn_name)))?;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| ExecError::DbError(format!("Query prepare error: {}", e)))?;

    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let mut results = Vec::new();
    let converted_params: Vec<SqlValue> = params
        .iter()
        .map(to_sql_value)
        .collect::<Result<_, _>>()?;

    let mut rows = stmt
        .query(params_from_iter(converted_params.iter()))
        .map_err(|e| ExecError::DbError(e.to_string()))?;

    while let Some(row) = rows.next().map_err(|e| ExecError::DbError(e.to_string()))? {
        let mut map = serde_json::Map::new();
        for i in 0..column_count {
            let val: rusqlite::types::Value =
                row.get(i).map_err(|e| ExecError::DbError(e.to_string()))?;
            let json_val = match val {
                rusqlite::types::Value::Null => Value::Null,
                rusqlite::types::Value::Integer(i) => Value::Number(i.into()),
                rusqlite::types::Value::Real(f) => serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                rusqlite::types::Value::Text(s) => {
                    serde_json::from_str(&s).unwrap_or(Value::String(s))
                }
                rusqlite::types::Value::Blob(b) => {
                    Value::String(format!("[blob {} bytes]", b.len()))
                }
            };
            map.insert(column_names[i].clone(), json_val);
        }
        results.push(Value::Object(map));
    }

    Ok(Value::Array(results))
}

fn to_sql_value(value: &Value) -> Result<SqlValue, ExecError> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::Bool(boolean) => Ok(SqlValue::Integer(i64::from(*boolean))),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(SqlValue::Integer(integer))
            } else if let Some(unsigned) = number.as_u64() {
                let integer = i64::try_from(unsigned)
                    .map_err(|_| ExecError::DbError(format!("Integer out of range: {unsigned}")))?;
                Ok(SqlValue::Integer(integer))
            } else if let Some(real) = number.as_f64() {
                Ok(SqlValue::Real(real))
            } else {
                Err(ExecError::DbError("Unsupported number parameter".to_string()))
            }
        }
        Value::String(text) => Ok(SqlValue::Text(text.clone())),
        Value::Array(_) | Value::Object(_) => Ok(SqlValue::Text(value.to_string())),
    }
}

pub fn list_connections() -> Vec<String> {
    DB_POOLS
        .lock()
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_db_path() -> String {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("memflow-db-node-{stamp}.sqlite"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn execute_db_query_binds_parameters() {
        let db_path = unique_db_path();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE items (id INTEGER, name TEXT)", []).unwrap();
        conn.execute("INSERT INTO items (id, name) VALUES (1, 'alpha'), (2, 'beta')", [])
            .unwrap();
        drop(conn);

        let connection_name = format!(
            "test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        register_db_connection(&connection_name, &db_path).unwrap();

        let result = execute_db_query(
            &connection_name,
            "SELECT name FROM items WHERE id = ?1",
            &[Value::from(2)],
        )
        .unwrap();

        assert_eq!(
            result,
            Value::Array(vec![Value::Object(
                [("name".to_string(), Value::String("beta".to_string()))]
                    .into_iter()
                    .collect()
            )])
        );

        let _ = std::fs::remove_file(&db_path);
    }
}
