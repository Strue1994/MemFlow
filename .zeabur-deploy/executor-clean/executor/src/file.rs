use crate::error::ExecError;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

const SANDBOX_BASE: &str = "./workflow_files";

fn sandbox_base() -> Result<PathBuf, ExecError> {
    let base = PathBuf::from(SANDBOX_BASE);
    if !base.exists() {
        std::fs::create_dir_all(&base)
            .map_err(|e| ExecError::FileError(format!("Failed to create sandbox: {}", e)))?;
    }
    base
        .canonicalize()
        .map_err(|e| ExecError::FileError(format!("Failed to resolve base: {}", e)))
}

fn sanitize_relative_path(user_path: &str) -> Result<PathBuf, ExecError> {
    let user_path = Path::new(user_path);
    let mut sanitized = PathBuf::new();

    for component in user_path.components() {
        match component {
            Component::Normal(part) => sanitized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ExecError::FileError("Path traversal detected".to_string()));
            }
        }
    }

    if sanitized.as_os_str().is_empty() {
        return Err(ExecError::FileError("Path must not be empty".to_string()));
    }

    Ok(sanitized)
}

fn safe_existing_path(user_path: &str) -> Result<PathBuf, ExecError> {
    let base_canonical = sandbox_base()?;
    let relative = sanitize_relative_path(user_path)?;
    let full = base_canonical.join(relative);
    let full_canonical = full
        .canonicalize()
        .map_err(|e| ExecError::FileError(format!("Failed to resolve path: {}", e)))?;

    if !full_canonical.starts_with(&base_canonical) {
        return Err(ExecError::FileError("Path traversal detected".to_string()));
    }

    Ok(full_canonical)
}

fn safe_write_path(user_path: &str) -> Result<PathBuf, ExecError> {
    let base_canonical = sandbox_base()?;
    let relative = sanitize_relative_path(user_path)?;
    Ok(base_canonical.join(relative))
}

pub fn read_file(path: &str) -> Result<Value, ExecError> {
    let full = safe_existing_path(path)?;
    let content = std::fs::read_to_string(&full)
        .map_err(|e| ExecError::FileError(format!("Failed to read file: {}", e)))?;
    Ok(Value::String(content))
}

pub fn write_file(path: &str, content: &Value, append: bool) -> Result<(), ExecError> {
    let full = safe_write_path(path)?;

    if let Some(parent) = full.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ExecError::FileError(format!("Failed to create directories: {}", e))
            })?;
        }
    }

    let data = match content {
        Value::String(s) => s.clone(),
        _ => content.to_string(),
    };

    if append {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&full)
            .map_err(|e| ExecError::FileError(format!("Failed to open file: {}", e)))?;
        file.write_all(data.as_bytes())
            .map_err(|e| ExecError::FileError(format!("Failed to write file: {}", e)))?;
    } else {
        std::fs::write(&full, data)
            .map_err(|e| ExecError::FileError(format!("Failed to write file: {}", e)))?;
    }

    Ok(())
}

pub fn delete_file(path: &str) -> Result<(), ExecError> {
    let full = safe_existing_path(path)?;
    std::fs::remove_file(&full)
        .map_err(|e| ExecError::FileError(format!("Failed to delete file: {}", e)))?;
    Ok(())
}

pub fn file_exists(path: &str) -> bool {
    safe_write_path(path).map(|p| p.exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_rel_path(name: &str) -> String {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("tests/{stamp}/{name}")
    }

    #[test]
    fn write_file_creates_new_nested_file() {
        let relative = unique_rel_path("created.txt");
        write_file(&relative, &Value::String("hello".to_string()), false).unwrap();
        let content = read_file(&relative).unwrap();
        assert_eq!(content, Value::String("hello".to_string()));
        delete_file(&relative).unwrap();
    }

    #[test]
    fn traversal_is_rejected() {
        let error = write_file("../escape.txt", &Value::String("x".to_string()), false).unwrap_err();
        assert!(error.to_string().contains("Path traversal"));
    }
}
