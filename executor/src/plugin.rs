#![allow(unused_imports)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin load error: {0}")]
    LoadError(String),
    #[error("Plugin execution error: {0}")]
    ExecutionError(String),
    #[error("Invalid plugin format: {0}")]
    InvalidFormat(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub plugin_type: PluginType,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PluginType {
    Wasm,
    JavaScript,
}

#[derive(Clone)]
pub struct WasmPlugin {
    metadata: PluginMetadata,
    #[allow(dead_code)]
    code: Vec<u8>,
}

impl WasmPlugin {
    pub fn new(metadata: PluginMetadata, code: Vec<u8>) -> Self {
        Self { metadata, code }
    }

    pub fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, PluginError> {
        Ok(serde_json::json!({
            "status": "success",
            "plugin": self.metadata.name,
            "result": params
        }))
    }
}

pub struct JsPlugin {
    metadata: PluginMetadata,
    #[allow(dead_code)]
    code: String,
}

impl JsPlugin {
    pub fn new(metadata: PluginMetadata, code: String) -> Self {
        Self { metadata, code }
    }

    pub fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, PluginError> {
        Ok(serde_json::json!({
            "status": "success",
            "plugin": self.metadata.name,
            "result": params
        }))
    }
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, PluginEntry>>>,
    plugins_dir: PathBuf,
}

enum PluginEntry {
    Wasm(WasmPlugin),
    Js(JsPlugin),
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&plugins_dir).ok();
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugins_dir,
        }
    }

    pub fn register_wasm(
        &self,
        name: String,
        metadata: PluginMetadata,
        code: Vec<u8>,
    ) -> Result<(), PluginError> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;
        plugins.insert(name, PluginEntry::Wasm(WasmPlugin::new(metadata, code)));
        Ok(())
    }

    pub fn register_js(
        &self,
        name: String,
        metadata: PluginMetadata,
        code: String,
    ) -> Result<(), PluginError> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;
        plugins.insert(name, PluginEntry::Js(JsPlugin::new(metadata, code)));
        Ok(())
    }

    pub fn unregister_plugin(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;
        plugins.remove(name);
        Ok(())
    }

    pub fn get_metadata(&self, name: &str) -> Result<PluginMetadata, PluginError> {
        let plugins = self
            .plugins
            .read()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;

        match plugins.get(name) {
            Some(PluginEntry::Wasm(p)) => Ok(p.metadata.clone()),
            Some(PluginEntry::Js(p)) => Ok(p.metadata.clone()),
            None => Err(PluginError::NotFound(name.to_string())),
        }
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginMetadata>, PluginError> {
        let plugins = self
            .plugins
            .read()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;

        let mut list = Vec::new();
        for entry in plugins.values() {
            let metadata = match entry {
                PluginEntry::Wasm(p) => p.metadata.clone(),
                PluginEntry::Js(p) => p.metadata.clone(),
            };
            list.push(metadata);
        }
        Ok(list)
    }

    pub fn call_plugin(
        &self,
        name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let plugins = self
            .plugins
            .read()
            .map_err(|e| PluginError::LoadError(e.to_string()))?;

        match plugins.get(name) {
            Some(PluginEntry::Wasm(p)) => p.execute(params),
            Some(PluginEntry::Js(p)) => p.execute(params),
            None => Err(PluginError::NotFound(name.to_string())),
        }
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }
}

impl Clone for PluginManager {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            plugins_dir: self.plugins_dir.clone(),
        }
    }
}

pub static PLUGIN_MANAGER: once_cell::sync::Lazy<PluginManager> =
    once_cell::sync::Lazy::new(|| PluginManager::new(PathBuf::from("./plugins")));

pub fn init_plugin_manager(plugins_dir: &Path) -> PluginManager {
    PluginManager::new(plugins_dir.to_path_buf())
}
