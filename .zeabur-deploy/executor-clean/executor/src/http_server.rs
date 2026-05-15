use axum::{
    extract::{Path, Query},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::{get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc as StdArc;
use std::sync::atomic::{AtomicBool, Ordering};
use compiler::parser::parse_n8n_workflow;
use crate::workflow_registry;
use crate::concurrency::CONCURRENCY_LIMITER;
use crate::metrics;
use crate::db::ExecutionLog;
use crate::auth::{self, ApiKey, Role};
use crate::cluster::{self, NodeInfo, ClusterStatus, LoadBalanceStrategy};
use uuid::Uuid;
use prometheus::{Encoder, TextEncoder};
use chrono::{Utc, Timelike, Datelike};

use learning_engine::strategy_updater::{StrategyUpdater, Context, Action};
use learning_engine::prompt_optimizer::PromptOptimizer;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

static STRATEGY_UPDATER: Lazy<StdArc<RwLock<Option<StrategyUpdater>>>> = 
    Lazy::new(|| StdArc::new(RwLock::new(None)));

static PROMPT_OPTIMIZER: Lazy<StdArc<PromptOptimizer>> = 
    Lazy::new(|| StdArc::new(PromptOptimizer::new()));

static STRATEGY_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize)]
struct CompileRequest {
    n8n_json: serde_json::Value,
    name: Option<String>,
}

#[derive(Serialize)]
struct CompileResponse {
    workflow_id: String,
    version: u32,
}

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<u32>,
}

#[derive(Deserialize)]
struct WorkflowDiffRequest {
    workflow_id: String,
    modified_n8n_json: serde_json::Value,
    diff_patch: Vec<DiffPatch>,
    user_id: Option<String>,
}

#[derive(Serialize)]
struct WorkflowDiffResponse {
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct DiffPatch {
    op: String,
    path: String,
    value: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct WorkflowInfo {
    id: String,
    name: String,
    version: u32,
}

#[derive(Serialize)]
struct WorkflowDetail {
    id: String,
    name: String,
    version: u32,
    n8n_json: Option<String>,
}

#[derive(Serialize)]
struct VersionList {
    versions: Vec<u32>,
}

#[derive(Serialize)]
struct RollbackResponse {
    new_version: Option<u32>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct ExecuteResponse {
    result: serde_json::Value,
}

fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn require_auth(headers: &HeaderMap) -> Result<ApiKey, (StatusCode, Json<ErrorResponse>)> {
    let key = extract_api_key(headers).ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Missing API key".to_string() }))
    })?;

    if !auth::check_rate_limit(&key).await {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(ErrorResponse { error: "Rate limit exceeded".to_string() })));
    }

    auth::validate_api_key(&key).await.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Invalid API key".to_string() }))
    })
}

async fn require_edit(headers: &HeaderMap) -> Result<ApiKey, (StatusCode, Json<ErrorResponse>)> {
    let key = require_auth(headers).await?;
    if !key.can_edit() {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Insufficient permissions".to_string() })));
    }
    Ok(key)
}

async fn require_admin(headers: &HeaderMap) -> Result<ApiKey, (StatusCode, Json<ErrorResponse>)> {
    let key = require_auth(headers).await?;
    if !key.can_admin() {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Admin access required".to_string() })));
    }
    Ok(key)
}

async fn compile_handler(headers: HeaderMap, Json(req): Json<CompileRequest>) -> impl IntoResponse {
    let _auth = match require_edit(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    metrics::record_compile();
    let n8n_json_str = req.n8n_json.to_string();
    let workflow = match parse_n8n_workflow(&n8n_json_str) {
        Ok(wf) => wf,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })).into_response(),
    };
    let id = Uuid::new_v4().to_string();
    let name = req.name.unwrap_or_else(|| format!("workflow_{}", id));
    
    match workflow_registry::register_workflow(&id, &name, &req.n8n_json, workflow) {
        Ok(version) => (StatusCode::OK, Json(CompileResponse { workflow_id: id, version })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
    }
}

async fn get_workflow_handler(headers: HeaderMap, Path(id): Path<String>, Query(query): Query<VersionQuery>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    match workflow_registry::get_workflow(&id, query.version) {
        Some(_) => {
            let n8n_json = workflow_registry::get_n8n_json(&id, query.version);
            let (name, version) = workflow_registry::get_workflow_metadata(&id, query.version)
                .unwrap_or_else(|| {
                    let versions = workflow_registry::list_versions(&id);
                    (
                        format!("workflow_{}", &id[..8]),
                        query.version.or_else(|| versions.first().copied()).unwrap_or(1),
                    )
                });
            let detail = WorkflowDetail {
                id: id.clone(),
                name,
                version,
                n8n_json,
            };
            (StatusCode::OK, Json(detail)).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Workflow not found".to_string() })).into_response(),
    }
}

async fn list_workflows_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let list = workflow_registry::list_workflows();
    let infos: Vec<WorkflowInfo> = list.into_iter().map(|(id, name, version)| WorkflowInfo { id, name, version }).collect();
    (StatusCode::OK, Json(infos)).into_response()
}

async fn list_versions_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let versions = workflow_registry::list_versions(&id);
    if versions.is_empty() {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Workflow not found".to_string() })).into_response();
    }
    (StatusCode::OK, Json(VersionList { versions })).into_response()
}

async fn rollback_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let _auth = match require_edit(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    match workflow_registry::rollback(&id) {
        Ok(Some(new_version)) => (StatusCode::OK, Json(RollbackResponse { new_version: Some(new_version) })).into_response(),
        Ok(None) => (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "No previous version to rollback to".to_string() })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
    }
}

async fn save_diff_handler(headers: HeaderMap, Json(req): Json<WorkflowDiffRequest>) -> impl IntoResponse {
    let _auth = match require_edit(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    let user_id = req.user_id.unwrap_or_else(|| "anonymous".to_string());
    let diff_id = Uuid::new_v4().to_string();
    
    let versions = workflow_registry::list_versions(&req.workflow_id);
    let from_version = versions.first().copied().unwrap_or(1);
    
    if let Some(db) = workflow_registry::get_db() {
        let diff_patch_json = serde_json::to_string(&req.diff_patch).unwrap_or_default();
        if let Err(e) = db.save_workflow_diff(&diff_id, &req.workflow_id, from_version, from_version + 1, &diff_patch_json, &user_id) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response();
        }
    }

    let n8n_json_str = req.modified_n8n_json.to_string();
    let workflow = match compiler::parser::parse_n8n_workflow(&n8n_json_str) {
        Ok(wf) => wf,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })).into_response(),
    };

    let name = format!("{}_v{}", req.workflow_id, from_version + 1);
    match workflow_registry::register_workflow(&req.workflow_id, &name, &req.modified_n8n_json, workflow) {
        Ok(version) => (StatusCode::OK, Json(WorkflowDiffResponse { version })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
    }
}

async fn list_prompt_versions_handler() -> impl IntoResponse {
    let versions = PROMPT_OPTIMIZER.list_versions().await;
    let result: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "id": v.id,
                "version": v.version,
                "created_at": v.created_at,
                "is_active": v.is_active,
                "ab_test_percentage": v.ab_test_percentage,
            })
        })
        .collect();
    (StatusCode::OK, Json(result)).into_response()
}

async fn create_prompt_version_handler(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let content = req.get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let version = PROMPT_OPTIMIZER.create_prompt_version(content).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": version.id,
            "version": version.version,
            "created_at": version.created_at,
        }))
    ).into_response()
}

async fn get_prompt_handler() -> impl IntoResponse {
    let prompt = PROMPT_OPTIMIZER.get_current_prompt().await;
    (StatusCode::OK, Json(serde_json::json!({ "prompt": prompt }))).into_response()
}

#[derive(Deserialize)]
struct ExecuteRequest {
    workflow_id: String,
    params: Option<serde_json::Value>,
    version: Option<u32>,
    timeout_seconds: Option<u64>,
}

async fn execute_handler(headers: HeaderMap, Json(req): Json<ExecuteRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    metrics::inc_active();
    let _permit = match CONCURRENCY_LIMITER.acquire().await {
        Ok(p) => p,
        Err(e) => {
            metrics::dec_active();
            return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: e.to_string() })).into_response();
        }
    };

    let workflow = match workflow_registry::get_workflow(&req.workflow_id, req.version) {
        Some(wf) => wf,
        None => {
            metrics::dec_active();
            return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Workflow not found".to_string() })).into_response();
        }
    };

    let versions = workflow_registry::list_versions(&req.workflow_id);
    let version = req.version.or_else(|| versions.first().copied()).unwrap_or(1);
    let started_at = Utc::now().timestamp();
    let params_json = req.params.as_ref().map(|p| p.to_string()).unwrap_or_default();
    
    let mut executor = crate::Executor::new();
    
    if let Some(params) = &req.params {
        if let Some(obj) = params.as_object() {
            for (key, val) in obj {
                executor.env.set(key, val.clone());
            }
        }
    }

    let result = executor.execute_with_timeout(&workflow, req.timeout_seconds);
    metrics::dec_active();
    
    let finished_at = Utc::now().timestamp();
    let duration_ms = (finished_at - started_at) * 1000;
    
    if let Some(db) = workflow_registry::get_db() {
        let result_json = result.as_ref().ok().map(|r| r.to_string()).unwrap_or_default();
        let error_str = result.as_ref().err().map(|e| e.to_string());
        let log = ExecutionLog {
            id: Uuid::new_v4().to_string(),
            workflow_id: req.workflow_id.clone(),
            version,
            params: params_json,
            result: result_json,
            error: error_str,
            started_at,
            finished_at,
            duration_ms,
        };
        let _ = db.save_execution_log(&log);
    }
    
    match result {
        Ok(r) => {
            metrics::record_call(&req.workflow_id, 0.0);
            (StatusCode::OK, Json(ExecuteResponse { result: r })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
    }
}

async fn prometheus_metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    (StatusCode::OK, output).into_response()
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<usize>,
}

async fn logs_handler(headers: HeaderMap, Path(id): Path<String>, Query(query): Query<LogsQuery>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let limit = query.limit.unwrap_or(50);
    if let Some(db) = workflow_registry::get_db() {
        match db.get_execution_logs(&id, limit) {
            Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

async fn recent_logs_handler(headers: HeaderMap, Query(query): Query<LogsQuery>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let limit = query.limit.unwrap_or(50);
    if let Some(db) = workflow_registry::get_db() {
        match db.get_recent_logs(limit) {
            Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

#[derive(Deserialize)]
struct ApiKeyRequest {
    key: String,
    role: Option<Role>,
    rate_limit: Option<u32>,
}

#[derive(Serialize)]
struct ApiKeyResponse {
    keys: Vec<String>,
}

async fn create_api_key_handler(headers: HeaderMap, Json(req): Json<ApiKeyRequest>) -> impl IntoResponse {
    let _auth = match require_admin(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let role = req.role.unwrap_or(Role::Viewer);
    let rate_limit = req.rate_limit.unwrap_or(60);
    auth::add_api_key(req.key, role, rate_limit).await;
    (StatusCode::OK, Json(ApiKeyResponse { keys: vec!["Key created".to_string()] })).into_response()
}

async fn list_api_keys_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_admin(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let keys = auth::list_api_keys().await;
    (StatusCode::OK, Json(keys)).into_response()
}

#[derive(Deserialize)]
struct RegisterNodeRequest {
    id: String,
    address: String,
    port: u16,
}

async fn register_node_handler(headers: HeaderMap, Json(req): Json<RegisterNodeRequest>) -> impl IntoResponse {
    let _auth = match require_admin(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let node = NodeInfo::new(req.id, req.address, req.port);
    cluster::register_node(node).await;
    (StatusCode::OK, Json(ErrorResponse { error: "Node registered".to_string() })).into_response()
}

#[derive(Serialize)]
struct NodeListResponse {
    nodes: Vec<NodeInfo>,
}

async fn list_nodes_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let nodes = cluster::get_nodes().await;
    (StatusCode::OK, Json(NodeListResponse { nodes })).into_response()
}

async fn cluster_status_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    let status = cluster::get_cluster_status().await;
    (StatusCode::OK, Json(status)).into_response()
}

#[derive(Deserialize)]
struct StrategyRequest {
    strategy: LoadBalanceStrategy,
}

async fn set_strategy_handler(headers: HeaderMap, Json(req): Json<StrategyRequest>) -> impl IntoResponse {
    let _auth = match require_admin(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    
    cluster::set_strategy(req.strategy).await;
    (StatusCode::OK, Json(ErrorResponse { error: "Strategy updated".to_string() })).into_response()
}

#[derive(Deserialize)]
struct StrategySelectRequest {
    intent: String,
    actions: Vec<Action>,
}

async fn select_strategy_handler(Json(req): Json<StrategySelectRequest>) -> impl IntoResponse {
    let mut strategy_guard: tokio::sync::RwLockWriteGuard<'_, Option<StrategyUpdater>> = STRATEGY_UPDATER.write().await;
    
    if !STRATEGY_INITIALIZED.load(Ordering::SeqCst) {
        *strategy_guard = Some(StrategyUpdater::new(0.1));
        STRATEGY_INITIALIZED.store(true, Ordering::SeqCst);
    }
    
    let strategy = strategy_guard.as_ref().unwrap();
    let context = Context::new()
        .with_intent(&req.intent)
        .with_time(
            chrono::Utc::now().hour(),
            chrono::Utc::now().weekday().num_days_from_monday(),
        );
    
    let selected = strategy.select_action(&context, &req.actions).await;
    
    (StatusCode::OK, Json(serde_json::json!({ "selected_workflow_id": selected }))).into_response()
}

#[derive(Deserialize)]
struct StrategyUpdateRequest {
    intent: String,
    action: String,
    success: bool,
    duration_ms: i64,
}

async fn update_strategy_handler(Json(req): Json<StrategyUpdateRequest>) -> impl IntoResponse {
    let strategy_guard: tokio::sync::RwLockReadGuard<'_, Option<StrategyUpdater>> = STRATEGY_UPDATER.read().await;
    
    if let Some(strategy) = strategy_guard.as_ref() {
        let context = Context::new()
            .with_intent(&req.intent)
            .with_success_rate(if req.success { 0.9 } else { 0.3 })
            .with_time(
                chrono::Utc::now().hour(),
                chrono::Utc::now().weekday().num_days_from_monday(),
            );
        
        let base_reward = if req.success { 1.0 } else { -1.0 };
        let bonus = if req.duration_ms < 1000 { 0.5 } else { 0.0 };
        let total_reward = base_reward + bonus;
        
        strategy.update(&context, &req.action, total_reward).await;
        
        (StatusCode::OK, Json(serde_json::json!({ "updated": true, "reward": total_reward }))).into_response()
    } else {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Strategy not initialized".to_string() })).into_response()
    }
}

async fn strategy_stats_handler() -> impl IntoResponse {
    let strategy_guard: tokio::sync::RwLockReadGuard<'_, Option<StrategyUpdater>> = STRATEGY_UPDATER.read().await;
    
    if let Some(strategy) = strategy_guard.as_ref() {
        let stats = strategy.get_statistics().await;
        let policy = strategy.get_policy().await;
        
        (StatusCode::OK, Json(serde_json::json!({
            "stats": stats,
            "policy": policy,
        }))).into_response()
    } else {
        (StatusCode::OK, Json(serde_json::json!({
            "stats": { "total_decisions": 0, "successful_decisions": 0, "success_rate": 0.0 },
            "policy": {}
        }))).into_response()
    }
}

async fn ui_handler() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MemFlow: 记忆驱动的自动化流</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a2e; color: #eee; min-height: 100vh; }
        .container { max-width: 1400px; margin: 0 auto; padding: 20px; }
        header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 30px; padding-bottom: 20px; border-bottom: 1px solid #333; }
        h1 { font-size: 24px; color: #00d9ff; }
        .api-key-input { display: flex; gap: 10px; align-items: center; }
        .api-key-input input { padding: 8px 12px; border-radius: 4px; border: 1px solid #444; background: #252540; color: #fff; width: 200px; }
        .main { display: grid; grid-template-columns: 300px 1fr; gap: 20px; }
        .sidebar { background: #252540; border-radius: 8px; padding: 20px; }
        .sidebar h2 { font-size: 16px; margin-bottom: 15px; color: #aaa; }
        .workflow-list { list-style: none; }
        .workflow-item { padding: 12px; margin-bottom: 8px; background: #1a1a2e; border-radius: 4px; cursor: pointer; transition: background 0.2s; }
        .workflow-item:hover { background: #2a2a4a; }
        .workflow-item.active { background: #00d9ff22; border: 1px solid #00d9ff; }
        .editor { background: #252540; border-radius: 8px; padding: 20px; }
        .editor-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
        .editor-header h2 { font-size: 18px; }
        .btn { padding: 10px 20px; border-radius: 4px; border: none; cursor: pointer; font-weight: 500; transition: all 0.2s; }
        .btn-primary { background: #00d9ff; color: #000; }
        .btn-primary:hover { background: #00b8d9; }
        .btn-secondary { background: #444; color: #fff; }
        .btn-secondary:hover { background: #555; }
        .btn-danger { background: #ff4757; color: #fff; }
        .n8n-editor { background: #1a1a2e; border-radius: 8px; padding: 15px; min-height: 400px; font-family: monospace; font-size: 13px; }
        .n8n-editor textarea { width: 100%; height: 400px; background: transparent; border: none; color: #aaa; resize: vertical; font-family: inherit; font-size: inherit; }
        .n8n-editor textarea:focus { outline: none; }
        .output { margin-top: 20px; padding: 15px; background: #1a1a2e; border-radius: 8px; max-height: 300px; overflow-y: auto; }
        .output pre { white-space: pre-wrap; word-wrap: break-word; color: #7bed9f; font-size: 13px; }
        .output.error pre { color: #ff6b6b; }
        .tabs { display: flex; gap: 10px; margin-bottom: 15px; }
        .tab { padding: 8px 16px; border-radius: 4px; background: #333; cursor: pointer; }
        .tab.active { background: #00d9ff; color: #000; }
        .empty-state { text-align: center; padding: 60px 20px; color: #666; }
        .empty-state h3 { margin-bottom: 10px; }
        .toast { position: fixed; bottom: 20px; right: 20px; padding: 15px 25px; background: #00d9ff; color: #000; border-radius: 8px; animation: slideIn 0.3s ease; }
        @keyframes slideIn { from { transform: translateX(100%); } to { transform: translateX(0); } }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🎯 MemFlow: 记忆驱动的自动化流</h1>
            <div class="api-key-input">
                <input type="password" id="apiKey" placeholder="Enter X-API-Key">
            </div>
        </header>
        
        <div class="main">
            <aside class="sidebar">
                <h2>Workflows</h2>
                <button class="btn btn-primary" style="width: 100%; margin-bottom: 15px;" onclick="createNew()">+ New Workflow</button>
                <ul class="workflow-list" id="workflowList">
                    <li class="empty-state"><p>No workflows yet</p></li>
                </ul>
            </aside>
            
            <main class="editor">
                <div class="editor-header">
                    <h2 id="editorTitle">New Workflow</h2>
                    <div>
                        <button class="btn btn-secondary" onclick="compileWorkflow()">Compile</button>
                        <button class="btn btn-primary" onclick="executeWorkflow()">Execute</button>
                    </div>
                </div>
                
                <div class="tabs">
                    <div class="tab active" onclick="switchTab('n8n')">n8n JSON</div>
                    <div class="tab" onclick="switchTab('info')">Info</div>
                </div>
                
                <div id="n8nTab" class="n8n-editor">
                    <textarea id="n8nJson" placeholder='{"nodes": []}'>{"nodes": []}</textarea>
                </div>
                
                <div id="infoTab" style="display: none;">
                    <div id="workflowInfo"></div>
                </div>
                
                <div class="output" id="output" style="display: none;">
                    <pre id="outputContent"></pre>
                </div>
            </main>
        </div>
    </div>
    
    <script>
        const API_BASE = window.location.origin;
        let currentWorkflowId = null;
        
        function getApiKey() { return document.getElementById('apiKey').value; }
        
        async function apiCall(endpoint, options = {}) {
            const headers = { 'Content-Type': 'application/json', 'X-API-Key': getApiKey() };
            const response = await fetch(`${API_BASE}${endpoint}`, { ...options, headers });
            return response.json();
        }
        
        async function loadWorkflows() {
            try {
                const workflows = await apiCall('/workflows');
                const list = document.getElementById('workflowList');
                if (workflows.length === 0) {
                    list.innerHTML = '<li class="empty-state"><p>No workflows yet</p></li>';
                    return;
                }
                list.innerHTML = workflows.map(w => `
                    <li class="workflow-item ${w.id === currentWorkflowId ? 'active' : ''}" 
                        onclick="selectWorkflow('${w.id}')">
                        <strong>${w.name}</strong><br>
                        <small>ID: ${w.id.slice(0, 8)}... | v${w.version}</small>
                    </li>
                `).join('');
            } catch (e) { console.error(e); }
        }
        
        async function createNew() {
            const n8nJson = prompt('Enter n8n JSON:', '{"nodes": []}');
            if (!n8nJson) return;
            try {
                const result = await apiCall('/compile', {
                    method: 'POST',
                    body: JSON.stringify({ n8n_json: JSON.parse(n8nJson) })
                });
                currentWorkflowId = result.workflow_id;
                document.getElementById('editorTitle').textContent = 'Workflow: ' + result.workflow_id.slice(0, 8);
                await loadWorkflows();
                showToast('Workflow compiled! ID: ' + result.workflow_id);
            } catch (e) { alert('Error: ' + e.message); }
        }
        
        async function selectWorkflow(id) {
            currentWorkflowId = id;
            try {
                const wf = await apiCall('/workflow/' + id);
                document.getElementById('editorTitle').textContent = 'Workflow: ' + wf.name;
                document.getElementById('n8nJson').value = wf.n8n_json || '{"nodes": []}';
                document.getElementById('infoTab').innerHTML = '<p><strong>ID:</strong> ' + wf.id + '</p><p><strong>Version:</strong> ' + wf.version + '</p>';
                await loadWorkflows();
            } catch (e) { alert('Error: ' + e.message); }
        }
        
        async function compileWorkflow() {
            const n8nJson = document.getElementById('n8nJson').value;
            try {
                const result = await apiCall('/compile', {
                    method: 'POST',
                    body: JSON.stringify({ n8n_json: JSON.parse(n8nJson) })
                });
                currentWorkflowId = result.workflow_id;
                showToast('Compiled! Version: ' + result.version);
                await loadWorkflows();
            } catch (e) { alert('Error: ' + e.message); }
        }
        
        async function executeWorkflow() {
            if (!currentWorkflowId) { alert('Please select a workflow first'); return; }
            const output = document.getElementById('output');
            output.style.display = 'block';
            output.classList.remove('error');
            document.getElementById('outputContent').textContent = 'Executing...';
            try {
                const result = await apiCall('/execute', {
                    method: 'POST',
                    body: JSON.stringify({ workflow_id: currentWorkflowId })
                });
                document.getElementById('outputContent').textContent = JSON.stringify(result, null, 2);
            } catch (e) { output.classList.add('error'); document.getElementById('outputContent').textContent = 'Error: ' + e.message; }
        }
        
        function switchTab(tab) {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            event.target.classList.add('active');
            document.getElementById('n8nTab').style.display = tab === 'n8n' ? 'block' : 'none';
            document.getElementById('infoTab').style.display = tab === 'info' ? 'block' : 'none';
        }
        
        function showToast(message) {
            const toast = document.createElement('div');
            toast.className = 'toast';
            toast.textContent = message;
            document.body.appendChild(toast);
            setTimeout(() => toast.remove(), 3000);
        }
        
        loadWorkflows();
    </script>
</body>
</html>"#;
    
    (StatusCode::OK, html).into_response()
}

pub async fn start_server(addr: &str) -> anyhow::Result<()> {
    let admin_key = std::env::var("EXECUTOR_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let key_source = if std::env::var("EXECUTOR_API_KEY").is_ok() {
        "EXECUTOR_API_KEY"
    } else {
        "generated at startup"
    };
    auth::init_api_keys(vec![
        (admin_key.clone(), Role::Admin, 1000),
    ]).await;
    
    let app = Router::new()
        .route("/", get(ui_handler))
        .route("/compile", post(compile_handler))
        .route("/execute", post(execute_handler))
        .route("/workflow/:id", get(get_workflow_handler))
        .route("/workflow/:id/versions", get(list_versions_handler))
        .route("/workflow/:id/rollback", post(rollback_handler))
        .route("/workflow/:id/logs", get(logs_handler))
        .route("/workflow/diff", post(save_diff_handler))
        .route("/workflows", get(list_workflows_handler))
        .route("/logs", get(recent_logs_handler))
        .route("/prompts/versions", get(list_prompt_versions_handler))
        .route("/prompts/versions", post(create_prompt_version_handler))
        .route("/prompts/current", get(get_prompt_handler))
        .route("/metrics", get(prometheus_metrics_handler))
        .route("/admin/keys", post(create_api_key_handler))
        .route("/admin/keys", get(list_api_keys_handler))
        .route("/cluster/nodes", post(register_node_handler))
        .route("/cluster/nodes", get(list_nodes_handler))
        .route("/cluster/status", get(cluster_status_handler))
        .route("/cluster/strategy", post(set_strategy_handler));
        // Strategy endpoints temporarily disabled due to async trait issues
        // .route("/strategy/select", post(select_strategy_handler))
        // .route("/strategy/update", post(update_strategy_handler))
        // .route("/strategy/stats", get(strategy_stats_handler));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("HTTP server listening on {}", addr);
    println!("Max concurrent workflows: {}", CONCURRENCY_LIMITER.max_concurrent());
    println!("Admin API key source: {}", key_source);
    println!("Admin API key: {}", admin_key);
    axum::serve(listener, app).await?;
    Ok(())
}
