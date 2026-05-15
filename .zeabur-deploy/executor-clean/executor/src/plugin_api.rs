#![allow(unused_imports)]

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::plugin::{PluginManager, PluginMetadata, PluginError};

#[derive(Serialize)]
struct PluginResponse {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct UploadRequest {
    name: String,
    version: Option<String>,
    description: Option<String>,
}

pub async fn upload_plugin(
    State(manager): State<Arc<RwLock<PluginManager>>>,
    Json(req): Json<UploadRequest>,
) -> impl IntoResponse {
    let mut manager = manager.write().await;
    
    let metadata = PluginMetadata {
        name: req.name.clone(),
        version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
        description: req.description.unwrap_or_default(),
        schema: serde_json::json!({}),
        output_schema: serde_json::json!({}),
        plugin_type: crate::plugin::PluginType::Wasm,
    };
    
    match manager.register_wasm(req.name.clone(), metadata, vec![]) {
        Ok(_) => (
            StatusCode::OK,
            Json(PluginResponse {
                success: true,
                message: "Plugin registered successfully".to_string(),
                data: None,
            })
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(PluginResponse {
                success: false,
                message: e.to_string(),
                data: None,
            })
        ).into_response(),
    }
}

pub async fn list_plugins(
    State(manager): State<Arc<RwLock<PluginManager>>>,
) -> impl IntoResponse {
    let manager = manager.read().await;
    match manager.list_plugins() {
        Ok(plugins) => (
            StatusCode::OK,
            Json(PluginResponse {
                success: true,
                message: "Plugins retrieved".to_string(),
                data: Some(serde_json::to_value(plugins).unwrap_or_default()),
            })
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(PluginResponse {
                success: false,
                message: e.to_string(),
                data: None,
            })
        ).into_response(),
    }
}

pub async fn delete_plugin(
    State(manager): State<Arc<RwLock<PluginManager>>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut manager = manager.write().await;
    match manager.unregister_plugin(&name) {
        Ok(_) => (
            StatusCode::OK,
            Json(PluginResponse {
                success: true,
                message: "Plugin deleted".to_string(),
                data: None,
            })
        ).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(PluginResponse {
                success: false,
                message: e.to_string(),
                data: None,
            })
        ).into_response(),
    }
}

pub async fn call_plugin(
    State(manager): State<Arc<RwLock<PluginManager>>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(params): Json<serde_json::Value>,
) -> impl IntoResponse {
    let manager = manager.read().await;
    match manager.call_plugin(&name, params) {
        Ok(result) => (
            StatusCode::OK,
            Json(PluginResponse {
                success: true,
                message: "Plugin executed".to_string(),
                data: Some(result),
            })
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(PluginResponse {
                success: false,
                message: e.to_string(),
                data: None,
            })
        ).into_response(),
    }
}

pub fn create_plugin_router(manager: Arc<RwLock<PluginManager>>) -> Router {
    Router::new()
        .route("/plugins", post(upload_plugin))
        .route("/plugins", get(list_plugins))
        .route("/plugins/:name", delete(delete_plugin))
        .route("/plugins/:name/execute", post(call_plugin))
        .with_state(manager)
}