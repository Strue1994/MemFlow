use axum::{
    extract::{Path, Query},
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use axum::routing::delete;
use serde::{Deserialize, Serialize};
use std::sync::Arc as StdArc;
use std::sync::atomic::{AtomicBool, Ordering};
use compiler::parser::parse_n8n_workflow;
use crate::workflow_registry;
use crate::concurrency::CONCURRENCY_LIMITER;
use crate::metrics;
use crate::db::ExecutionLog;
use crate::auth::{self, ApiKey, Role};
use crate::cluster::{self, NodeInfo, LoadBalanceStrategy};
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
    // Prefer standard Authorization: Bearer <key> header (frontend default)
    if let Some(auth_value) = headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        if let Some(key) = auth_value.strip_prefix("Bearer ") {
            return Some(key.to_string());
        }
    }
    // Fallback: X-API-Key header (CLI / server-to-server)
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
    
    let params_for_exec = req.params.clone();
    let timeout_secs = req.timeout_seconds;

    // Run blocking executor on a thread-pool thread to avoid blocking async workers
    let exec_result = tokio::task::spawn_blocking(move || {
        let mut executor = crate::Executor::new();
        if let Some(params) = &params_for_exec {
            if let Some(obj) = params.as_object() {
                for (key, val) in obj {
                    executor.env.set(key, val.clone());
                }
            }
        }
        executor.execute_with_timeout(&workflow, timeout_secs)
    }).await;

    metrics::dec_active();

    let finished_at = Utc::now().timestamp();
    let duration_ms = (finished_at - started_at) * 1000;

    let result = match exec_result {
        Ok(r) => r,
        Err(join_err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Execution task panicked: {}", join_err)
            })).into_response();
        }
    };
    
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
    
    // Asynchronously report execution result to the learning engine
    if STRATEGY_INITIALIZED.load(Ordering::SeqCst) {
        let workflow_id_le = req.workflow_id.clone();
        let success = result.is_ok();
        let dur = duration_ms;
        let strategy_ref = STRATEGY_UPDATER.clone();
        tokio::spawn(async move {
            let guard = strategy_ref.read().await;
            if let Some(strategy) = guard.as_ref() {
                let ctx = Context::new()
                    .with_intent(&workflow_id_le)
                    .with_success_rate(if success { 0.9 } else { 0.1 });
                let reward = if success { 1.0 + if dur < 1000 { 0.5 } else { 0.0 } } else { -1.0 };
                strategy.update(&ctx, &workflow_id_le, reward).await;
            }
        });
    }

    match result {
        Ok(r) => {
            metrics::record_call(&req.workflow_id, duration_ms as f64 / 1000.0);
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

async fn health_handler() -> impl IntoResponse {
    let db_ok = workflow_registry::get_db().is_some();
    let status = if db_ok { "healthy" } else { "degraded" };
    let code = if db_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "db": if db_ok { "ok" } else { "unavailable" },
        "active_workflows": metrics::ACTIVE_WORKFLOWS.get() as i64,
    }))).into_response()
}

async fn stats_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_stats() {
            Ok(stats) => {
                let active = metrics::ACTIVE_WORKFLOWS.get() as i64;
                (StatusCode::OK, Json(serde_json::json!({
                    "total_workflows": stats.total_workflows,
                    "total_executions": stats.total_executions,
                    "successful_executions": stats.successful_executions,
                    "failed_executions": stats.failed_executions,
                    "success_rate": stats.success_rate,
                    "avg_duration_ms": stats.avg_duration_ms,
                    "executions_last_24h": stats.executions_last_24h,
                    "active_workflows": active,
                }))).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
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

// ─── Learning / AI 端点 ────────────────────────────────────────────────────

async fn learning_patterns_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_patterns() {
            Ok(patterns) => (StatusCode::OK, Json(serde_json::json!({ "patterns": patterns }))).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

// ─── Knowledge / Learning Engine 端点 ──────────────────────────────────────────

use crate::scraper::{KnowledgeSource, SourceType, KnowledgeEngine};
use crate::scraper::upgrade::UpgradePipeline;
use crate::scraper::fusion::KnowledgeUnit;
use crate::scraper::security::ContentClassifier;
use crate::db::LearningAction;

static KNOWLEDGE_ENGINE: Lazy<StdArc<RwLock<KnowledgeEngine>>> = 
    Lazy::new(|| StdArc::new(RwLock::new(KnowledgeEngine::new())));

static UPGRADE_PIPELINE: Lazy<StdArc<RwLock<UpgradePipeline>>> = 
    Lazy::new(|| StdArc::new(RwLock::new(UpgradePipeline::new(30))));

static CONTENT_CLASSIFIER: Lazy<ContentClassifier> = 
    Lazy::new(ContentClassifier::new);

#[derive(Debug, Deserialize)]
struct KnowledgeQuery {
    q: Option<String>,
    source: Option<String>,
    min_confidence: Option<f64>,
}

async fn knowledge_query_handler(Query(query): Query<KnowledgeQuery>) -> impl IntoResponse {
    let engine = KNOWLEDGE_ENGINE.read().await;
    let sources = engine.list_sources().await;
    
    let results: Vec<_> = sources.into_iter()
        .filter(|s| {
            if let Some(ref q) = query.q {
                s.name.contains(q) || s.url.contains(q)
            } else {
                true
            }
        })
        .collect();
    
    (StatusCode::OK, Json(serde_json::json!({ "sources": results, "count": results.len() }))).into_response()
}

async fn list_knowledge_sources_handler() -> impl IntoResponse {
    let engine = KNOWLEDGE_ENGINE.read().await;
    let sources = engine.list_sources().await;
    (StatusCode::OK, Json(serde_json::json!({ "sources": sources }))).into_response()
}

#[derive(Debug, Deserialize)]
struct AddKnowledgeSourceRequest {
    name: String,
    url: String,
    source_type: SourceType,
    auth_token: Option<String>,
}

async fn add_knowledge_source_handler(Json(req): Json<AddKnowledgeSourceRequest>) -> impl IntoResponse {
    let source = KnowledgeSource {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        url: req.url,
        source_type: req.source_type,
        enabled: true,
        schedule: crate::scraper::Schedule { interval_secs: 3600, enabled: true },
        auth: req.auth_token.map(|t| crate::scraper::AuthConfig {
            auth_type: "bearer".to_string(),
            token: Some(t),
            username: None,
            password: None,
        }),
        tags: vec![],
    };
    
    let engine = KNOWLEDGE_ENGINE.read().await;
    engine.add_source(source.clone()).await;
    
    (StatusCode::CREATED, Json(serde_json::json!({ "source": source }))).into_response()
}

async fn list_upgrade_suggestions_handler() -> impl IntoResponse {
    let pipeline = UPGRADE_PIPELINE.read().await;
    let suggestions = pipeline.list_suggestions();
    (StatusCode::OK, Json(serde_json::json!({ "suggestions": suggestions }))).into_response()
}

async fn approve_upgrade_handler(Path(id): Path<String>) -> impl IntoResponse {
    let mut pipeline = UPGRADE_PIPELINE.write().await;
    match pipeline.approve(&id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "approved" }))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn reject_upgrade_handler(Path(id): Path<String>) -> impl IntoResponse {
    let mut pipeline = UPGRADE_PIPELINE.write().await;
    match pipeline.reject(&id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "rejected" }))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn merge_upgrade_handler(Path(id): Path<String>) -> impl IntoResponse {
    let mut pipeline = UPGRADE_PIPELINE.write().await;
    match pipeline.merge(&id) {
        Ok(unit) => (StatusCode::OK, Json(serde_json::json!({ "unit": unit }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn upgrade_history_handler(Query(unit_id): Query<serde_json::Value>) -> impl IntoResponse {
    let pipeline = UPGRADE_PIPELINE.read().await;
    let history = pipeline.list_history(unit_id.get("unit_id").and_then(|v| v.as_str()));
    (StatusCode::OK, Json(serde_json::json!({ "history": history }))).into_response()
}

async fn detect_stale_handler() -> impl IntoResponse {
    let pipeline = UPGRADE_PIPELINE.read().await;
    let empty_units: Vec<KnowledgeUnit> = vec![];
    let stale_ids = pipeline.detect_stale_units(&empty_units);
    (StatusCode::OK, Json(serde_json::json!({ "stale_units": stale_ids }))).into_response()
}

#[derive(Debug, Deserialize)]
struct ComplianceCheckRequest {
    source: String,
    content: String,
}

async fn compliance_check_handler(Json(req): Json<ComplianceCheckRequest>) -> impl IntoResponse {
    let classifier = CONTENT_CLASSIFIER.clone();
    let result = classifier.check_compliance(&req.source, &req.content);
    (StatusCode::OK, Json(serde_json::json!({ "result": result }))).into_response()
}

// ─── Multi-Agent 协同端点 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub knowledge_shared: Vec<String>,
    pub registered_at: i64,
}

static AGENTS: Lazy<StdArc<RwLock<Vec<Agent>>>> = 
    Lazy::new(|| StdArc::new(RwLock::new(Vec::new())));

async fn list_agents_handler() -> impl IntoResponse {
    let agents = AGENTS.read().await;
    (StatusCode::OK, Json(serde_json::json!({ "agents": agents.clone() }))).into_response()
}

#[derive(Debug, Deserialize)]
struct RegisterAgentRequest {
    name: String,
    role: String,
}

async fn register_agent_handler(Json(req): Json<RegisterAgentRequest>) -> impl IntoResponse {
    let agent = Agent {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        role: req.role,
        knowledge_shared: vec![],
        registered_at: Utc::now().timestamp(),
    };
    
    let mut agents = AGENTS.write().await;
    agents.push(agent.clone());
    
    (StatusCode::CREATED, Json(serde_json::json!({ "agent": agent }))).into_response()
}

#[derive(Debug, Deserialize)]
struct ShareKnowledgeRequest {
    knowledge_id: String,
}

async fn share_knowledge_handler(Path(id): Path<String>, Json(req): Json<ShareKnowledgeRequest>) -> impl IntoResponse {
    let mut agents = AGENTS.write().await;
    
    if let Some(agent) = agents.iter_mut().find(|a| a.id == id) {
        agent.knowledge_shared.push(req.knowledge_id.clone());
        (StatusCode::OK, Json(serde_json::json!({ "status": "shared" }))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Agent not found".to_string() })).into_response()
    }
}

// ─── Plugin 接口端点 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub endpoint: String,
    pub registered_at: i64,
}

static KNOWLEDGE_PLUGINS: Lazy<StdArc<RwLock<Vec<KnowledgePlugin>>>> = 
    Lazy::new(|| StdArc::new(RwLock::new(Vec::new())));

async fn list_knowledge_plugins_handler() -> impl IntoResponse {
    let plugins = KNOWLEDGE_PLUGINS.read().await;
    (StatusCode::OK, Json(serde_json::json!({ "plugins": plugins.clone() }))).into_response()
}

#[derive(Debug, Deserialize)]
struct RegisterPluginRequest {
    name: String,
    version: String,
    endpoint: String,
}

async fn register_knowledge_plugin_handler(Json(req): Json<RegisterPluginRequest>) -> impl IntoResponse {
    let plugin = KnowledgePlugin {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        version: req.version,
        endpoint: req.endpoint,
        registered_at: Utc::now().timestamp(),
    };
    
    let mut plugins = KNOWLEDGE_PLUGINS.write().await;
    plugins.push(plugin.clone());
    
    (StatusCode::CREATED, Json(serde_json::json!({ "plugin": plugin }))).into_response()
}

async fn learning_insights_handler(headers: HeaderMap) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_insights() {
            Ok(insights) => (StatusCode::OK, Json(serde_json::json!({ "insights": insights }))).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

/// 为指定工作流生成 AI 优化建议（基于执行统计 + 工作流结构）
async fn ai_suggest_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let suggestions = if let Some(db) = workflow_registry::get_db() {
        let logs = db.get_execution_logs(&id, 50).unwrap_or_default();
        let total = logs.len() as f64;
        let errors: Vec<_> = logs.iter().filter(|l| l.error.is_some()).collect();
        let error_rate = if total > 0.0 { errors.len() as f64 / total * 100.0 } else { 0.0 };
        let avg_dur = if total > 0.0 { logs.iter().map(|l| l.duration_ms as f64).sum::<f64>() / total } else { 0.0 };

        let mut s: Vec<serde_json::Value> = Vec::new();
        if error_rate > 30.0 {
            s.push(serde_json::json!({ "type": "error_handling", "priority": "high", "title": "添加错误处理节点", "detail": format!("错误率 {:.1}%，建议在关键节点后添加 If 条件判断和错误捕获分支。", error_rate) }));
        }
        if avg_dur > 5000.0 {
            s.push(serde_json::json!({ "type": "performance", "priority": "medium", "title": "并行化 HTTP 请求", "detail": format!("平均耗时 {:.0}ms，将串行 HTTP 请求改为并行可减少约 40-60% 耗时。", avg_dur) }));
        }
        // 最常见错误模式
        if let Some(top_err) = errors.iter().filter_map(|l| l.error.as_deref()).next() {
            let short = &top_err[..top_err.len().min(80)];
            s.push(serde_json::json!({ "type": "reliability", "priority": "high", "title": "修复高频错误", "detail": format!("最常见错误: {}...", short) }));
        }
        if s.is_empty() {
            s.push(serde_json::json!({ "type": "optimization", "priority": "low", "title": "工作流运行良好", "detail": "当前执行统计未发现明显问题，可考虑添加监控节点以持续跟踪关键指标。" }));
        }
        s
    } else {
        vec![serde_json::json!({ "type": "info", "priority": "low", "title": "暂无数据", "detail": "执行工作流后将生成优化建议。" })]
    };
    (StatusCode::OK, Json(serde_json::json!({ "workflow_id": id, "suggestions": suggestions }))).into_response()
}

#[derive(Deserialize)]
struct NlCreateRequest {
    description: String,
}

/// 自然语言创建工作流：将描述转换为 n8n JSON 工作流
async fn nl_create_handler(headers: HeaderMap, Json(req): Json<NlCreateRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    // 基于描述文本启发式生成工作流框架
    // 生产环境可替换为 LLM API 调用
    let desc = req.description.to_lowercase();
    let mut nodes = Vec::new();
    let mut connections = Vec::new();
    let mut node_id = 1usize;

    let mk_id = |n: usize| format!("node_{n}");

    // 触发器节点（始终添加）
    nodes.push(serde_json::json!({
        "id": mk_id(node_id), "name": "触发器", "type": "trigger",
        "parameters": { "mode": "manual" }, "position": [100, 200]
    }));
    let trigger_id = mk_id(node_id); node_id += 1;

    let mut prev_id = trigger_id.clone();

    // 根据关键词推断节点
    if desc.contains("http") || desc.contains("api") || desc.contains("请求") || desc.contains("获取") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "HTTP 请求", "type": "http",
            "parameters": { "url": "", "method": "GET", "headers": {} }, "position": [300, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        prev_id = mk_id(node_id); node_id += 1;
    }
    if desc.contains("条件") || desc.contains("判断") || desc.contains("if") || desc.contains("分支") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "条件判断", "type": "if",
            "parameters": { "condition": "", "operator": "equals", "value": "" }, "position": [500, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        prev_id = mk_id(node_id); node_id += 1;
    }
    if desc.contains("循环") || desc.contains("遍历") || desc.contains("for") || desc.contains("列表") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "循环处理", "type": "for",
            "parameters": { "items": "", "batch_size": 10 }, "position": [700, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        prev_id = mk_id(node_id); node_id += 1;
    }
    if desc.contains("数据库") || desc.contains("sql") || desc.contains("查询") || desc.contains("存储") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "数据库操作", "type": "db",
            "parameters": { "operation": "query", "query": "" }, "position": [900, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        prev_id = mk_id(node_id); node_id += 1;
    }
    if desc.contains("邮件") || desc.contains("email") || desc.contains("通知") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "发送邮件", "type": "email",
            "parameters": { "to": "", "subject": "", "body": "" }, "position": [1100, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        prev_id = mk_id(node_id); node_id += 1;
    }
    if desc.contains("代码") || desc.contains("脚本") || desc.contains("处理") || desc.contains("转换") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "代码处理", "type": "code",
            "parameters": { "code": "// 在此编写处理逻辑\nreturn $input;" }, "position": [900, 400]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
        let _ = mk_id(node_id); node_id += 1;
    }

    // 确保至少有一个处理节点
    if nodes.len() == 1 {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "设置变量", "type": "set",
            "parameters": { "key": "result", "value": "" }, "position": [300, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));
    }

    let n8n_json = serde_json::json!({
        "name": req.description.chars().take(30).collect::<String>(),
        "description": req.description,
        "nodes": nodes,
        "connections": connections,
    });

    (StatusCode::OK, Json(serde_json::json!({
        "workflow": n8n_json,
        "node_count": nodes.len(),
        "message": format!("已根据描述生成 {} 个节点的工作流框架，请在编辑器中完善细节。", nodes.len()),
    }))).into_response()
}

// Duplicate handlers removed - see original definitions above

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

    // Generate a key if not supplied
    let key = if req.key.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        req.key.clone()
    };
    let role = req.role.unwrap_or(Role::Viewer);
    let rate_limit = req.rate_limit.unwrap_or(60);
    auth::add_api_key(key.clone(), role, rate_limit).await;
    (StatusCode::CREATED, Json(serde_json::json!({ "key": key, "role": format!("{:?}", role) }))).into_response()
}

async fn delete_api_key_handler(headers: HeaderMap, Path(key): Path<String>) -> impl IntoResponse {
    let _auth = match require_admin(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    auth::remove_api_key(&key).await;
    (StatusCode::OK, Json(serde_json::json!({ "deleted": true }))).into_response()
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
                 learning/patterns", get(learning_patterns_handler))
        .route("/learning/insights", get(learning_insights_handler))
        .route("/workflow/:id/ai-suggest", get(ai_suggest_handler))
        .route("/nl-create", post(nl_create_handler))
        .route("/   <textarea id="n8nJson" placeholder='{"nodes": []}'>{"nodes": []}</textarea>
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
    let default_rate_limit = if std::env::var("MEMFLOW_DISABLE_RATE_LIMIT").ok().as_deref() == Some("true")
        || admin_key == "memflow-local-dev-key"
    {
        0
    } else {
        1000
    };
    let key_source = if std::env::var("EXECUTOR_API_KEY").is_ok() {
        "EXECUTOR_API_KEY"
    } else {
        "generated at startup"
    };
    auth::init_api_keys(vec![
        (admin_key.clone(), Role::Admin, default_rate_limit),
    ]).await;
    // Load persisted API keys from database (avoids losing keys on restart)
    auth::load_keys_from_db().await;
    
    async fn add_security_headers(request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    let mut r = next.run(request).await;
    // Security headers
    r.headers_mut().insert(HeaderName::from_static("x-content-type-options"), HeaderValue::from_static("nosniff"));
    r.headers_mut().insert(HeaderName::from_static("x-frame-options"), HeaderValue::from_static("DENY"));
    r.headers_mut().insert(HeaderName::from_static("x-xss-protection"), HeaderValue::from_static("1; mode=block"));
    r.headers_mut().insert(HeaderName::from_static("strict-transport-security"), HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    r.headers_mut().insert(HeaderName::from_static("referrer-policy"), HeaderValue::from_static("no-referrer"));
    // CORS headers
    r.headers_mut().insert(HeaderName::from_static("access-control-allow-origin"), HeaderValue::from_static("*"));
    r.headers_mut().insert(HeaderName::from_static("access-control-allow-methods"), HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
    r.headers_mut().insert(HeaderName::from_static("access-control-allow-headers"), HeaderValue::from_static("Content-Type, Authorization, X-API-Key"));
    r
}

// ─── 自动优化 API ─────────────────────────────────────────
#[derive(Serialize)]
struct OptimizeResponse {
    params: Vec<serde_json::Value>,
    estimated_speedup: f64,
    estimated_accuracy: f64,
    estimated_cost_savings: f64,
}

async fn optimize_handler(headers: HeaderMap, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    let workflow_id = payload.get("workflow_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let params = if let Some(db) = workflow_registry::get_db() {
        let mut recommendations = Vec::new();
        
        if let Some(wf_id) = workflow_id.as_ref() {
            if let Ok(logs) = db.get_execution_logs(wf_id, 50) {
                let total = logs.len() as f64;
                if total > 0.0 {
                    let errors: Vec<_> = logs.iter().filter(|l| l.error.is_some()).collect();
                    let error_rate = errors.len() as f64 / total;
                    let avg_duration: f64 = logs.iter().map(|l| l.duration_ms as f64).sum::<f64>() / total;
                    
                    if error_rate > 0.2 {
                        recommendations.push(serde_json::json!({
                            "name": "max_retries",
                            "current": 3,
                            "recommended": 5,
                            "impact": "high",
                            "reason": &format!("错误率 {:.1}%，建议增加重试次数", error_rate * 100.0)
                        }));
                    }
                    
                    if avg_duration > 5000.0 {
                        recommendations.push(serde_json::json!({
                            "name": "timeout_ms",
                            "current": 5000,
                            "recommended": 8000,
                            "impact": "medium",
                            "reason": &format!("平均耗时 {:.0}ms，建议增加超时", avg_duration)
                        }));
                    }
                    
                    if let Some(err) = errors.first() {
                        if let Some(e) = &err.error {
                            recommendations.push(serde_json::json!({
                                "name": "error_handler",
                                "current": "none",
                                "recommended": "add_try_catch",
                                "impact": "high",
                                "reason": &format!("高频错误: {}", e.chars().take(50).collect::<String>())
                            }));
                        }
                    }
                }
            }
        }
        
        if recommendations.is_empty() {
            let stats = db.get_stats().ok();
            if let Some(s) = stats {
                recommendations.push(serde_json::json!({
                    "name": "concurrency",
                    "current": 10,
                    "recommended": if s.total_executions > 100 { 20 } else { 10 },
                    "impact": "medium",
                    "reason": &format!("总执行数 {}，建议调整并发", s.total_executions)
                }));
            }
        }
        
        recommendations
    } else {
        vec![serde_json::json!({
            "name": "max_retries",
            "current": 3,
            "recommended": 5,
            "impact": "high",
            "reason": "默认值建议"
        })]
    };

    let (speedup, accuracy, savings) = if let Some(db) = workflow_registry::get_db() {
        if let Ok(stats) = db.get_stats() {
            (1.0 + stats.success_rate * 0.1, stats.success_rate / 100.0, 0.0)
        } else {
            (1.0, 0.9, 0.0)
        }
    } else {
        (1.2, 0.95, 0.15)
    };

    let response = OptimizeResponse {
        params,
        estimated_speedup: speedup,
        estimated_accuracy: accuracy,
        estimated_cost_savings: savings,
    };
    (StatusCode::OK, Json(response)).into_response()
}

#[derive(Deserialize)]
struct ApplyTuningRequest {
    workflow_id: String,
    params: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct ApplyTuningResponse {
    success: bool,
    message: String,
    version: Option<u32>,
    applied_params: Vec<String>,
}

fn workflow_name_for_version(workflow_id: &str, next_version: u32) -> String {
    format!("{}_v{}", workflow_id, next_version)
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn ensure_json_object(value: &mut serde_json::Value) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
    value.as_object_mut()
}

fn apply_tuning_to_json(
    workflow_json: &mut serde_json::Value,
    params: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<String> {
    let mut applied = Vec::new();

    if let Some(nodes) = workflow_json.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) {
        for node in nodes.iter_mut() {
            let node_type = node
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_lowercase();
            let is_http = node_type.contains("httprequest") || node_type == "httprequest" || node_type == "http";

            if !is_http {
                continue;
            }

            let Some(parameters) = node.get_mut("parameters").and_then(ensure_json_object) else {
                continue;
            };

            if let Some(timeout_ms) = params.get("timeout_ms").and_then(|value| value.as_u64()) {
                parameters.insert("timeout".to_string(), serde_json::json!(timeout_ms));
                applied.push("timeout_ms".to_string());
            }

            if let Some(max_retries) = params.get("max_retries").and_then(|value| value.as_u64()) {
                parameters.insert("retries".to_string(), serde_json::json!(max_retries));
                applied.push("max_retries".to_string());
            }
        }
    }

    if let Some(concurrency) = params.get("concurrency").and_then(|value| value.as_u64()) {
        let meta = workflow_json
            .as_object_mut()
            .expect("workflow_json should remain an object")
            .entry("memflow".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(meta_obj) = ensure_json_object(meta) {
            meta_obj.insert("concurrency".to_string(), serde_json::json!(concurrency));
            applied.push("concurrency".to_string());
        }
    }

    applied.sort();
    applied.dedup();
    applied
}

async fn apply_tuning_handler(headers: HeaderMap, Json(req): Json<ApplyTuningRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    let Some(existing_json) = workflow_registry::get_n8n_json(&req.workflow_id, None) else {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Workflow not found".to_string() })).into_response();
    };

    let mut workflow_json: serde_json::Value = match serde_json::from_str(&existing_json) {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: error.to_string() })).into_response();
        }
    };

    let applied_params = apply_tuning_to_json(&mut workflow_json, &req.params);
    if applied_params.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "No supported tuning parameters were provided".to_string() })).into_response();
    }

    let workflow_json_str = workflow_json.to_string();
    let compiled = match parse_n8n_workflow(&workflow_json_str) {
        Ok(workflow) => workflow,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: error.to_string() })).into_response();
        }
    };

    let next_version_hint = workflow_registry::list_versions(&req.workflow_id)
        .first()
        .copied()
        .unwrap_or(0) + 1;
    let name = workflow_registry::get_workflow_metadata(&req.workflow_id, None)
        .map(|(existing_name, _)| existing_name)
        .unwrap_or_else(|| workflow_name_for_version(&req.workflow_id, next_version_hint));

    let version = match workflow_registry::register_workflow(&req.workflow_id, &name, &workflow_json, compiled) {
        Ok(version) => version,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: error.to_string() })).into_response();
        }
    };

    if let Some(db) = workflow_registry::get_db() {
        let action = LearningAction {
            id: Uuid::new_v4().to_string(),
            workflow_id: req.workflow_id.clone(),
            action_type: "apply_tuning".to_string(),
            params: serde_json::to_string(&req.params).unwrap_or_default(),
            applied: false,
            created_at: now_ts(),
            applied_at: None,
        };
        if db.save_learning_action(&action).is_ok() {
            let _ = db.apply_learning_action(&action.id);
        }
    }

    let response = ApplyTuningResponse {
        success: true,
        message: format!(
            "Applied {} tuning parameters to workflow {} and persisted version {}",
            applied_params.len(),
            req.workflow_id,
            version
        ),
        version: Some(version),
        applied_params,
    };
    (StatusCode::OK, Json(response)).into_response()
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
struct SummarizeResponse {
    insights: Vec<serde_json::Value>,
    total_executions: i64,
    success_rate: f64,
    avg_duration: f64,
    updated_at: String,
}

async fn summarize_handler(headers: HeaderMap, Json(_payload): Json<serde_json::Value>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    let insights = if let Some(db) = workflow_registry::get_db() {
        let mut result = Vec::new();
        
        if let Ok(stats) = db.get_stats() {
            if stats.total_executions > 0 {
                result.push(serde_json::json!({
                    "category": "accuracy",
                    "title": &format!("成功率 {:.1}%", stats.success_rate),
                    "description": &format!("总执行 {} 次，成功 {} 次", stats.total_executions, stats.successful_executions),
                    "impact": stats.success_rate / 100.0,
                    "suggestions": if stats.success_rate < 90.0 {
                        vec!["检查失败工作流日志".to_string(), "添加错误处理".to_string()]
                    } else {
                        vec!["当前配置运行良好".to_string()]
                    }
                }));
                
                if stats.avg_duration_ms > 0.0 {
                    result.push(serde_json::json!({
                        "category": "performance",
                        "title": &format!("平均耗时 {:.0}ms", stats.avg_duration_ms),
                        "description": &format!("过去 24h 执行 {} 次", stats.executions_last_24h),
                        "impact": (1.0 - stats.avg_duration_ms / 10000.0).clamp(0.0, 1.0),
                        "suggestions": if stats.avg_duration_ms > 5000.0 {
                            vec!["考虑优化 HTTP 并行化".to_string(), "减少串行节点".to_string()]
                        } else {
                            vec!["性能正常".to_string()]
                        }
                    }));
                }
            }
        }
        
        if let Ok(patterns) = db.get_patterns() {
            for p in patterns.iter().take(3) {
                result.push(serde_json::json!({
                    "category": "workflow",
                    "title": &p.workflow_name,
                    "description": &format!("执行 {} 次，成功率 {:.1}%", p.total_executions, p.success_rate),
                    "impact": p.success_rate / 100.0,
                    "suggestions": match p.trend.as_str() {
                        "degrading" => vec!["需要优化此工作流".to_string()],
                        "improving" => vec!["运行良好，可复制经验".to_string()],
                        _ => vec!["稳定运行".to_string()]
                    }
                }));
            }
        }
        
        result
    } else {
        vec![serde_json::json!({
            "category": "info",
            "title": "暂无数据",
            "description": "执行工作流后将生成学习报告",
            "impact": 0.0,
            "suggestions": vec!["创建并执行工作流".to_string()]
        })]
    };

    let (total, success_rate, avg_duration) = if let Some(db) = workflow_registry::get_db() {
        if let Ok(stats) = db.get_stats() {
            (stats.total_executions, stats.success_rate / 100.0, stats.avg_duration_ms)
        } else {
            (0, 0.0, 0.0)
        }
    } else {
        (0, 0.0, 0.0)
    };

    let response = SummarizeResponse {
        insights,
        total_executions: total,
        success_rate,
        avg_duration,
        updated_at: Utc::now().to_rfc3339(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

// ─── 自然语言创建增强 API ────────────────────────────────
#[derive(Deserialize)]
struct EnhanceNLRequest {
    description: String,
    workflow_id: Option<String>,
}

#[derive(Serialize)]
struct EnhanceNLResponse {
    improved_workflow: serde_json::Value,
    improvements: Vec<String>,
    learning_feedback: String,
}

async fn enhance_nl_workflow_handler(headers: HeaderMap, Json(req): Json<EnhanceNLRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    let desc = req.description.to_lowercase();
    let mut nodes = Vec::new();
    let mut connections = Vec::new();
    let mut improvements = Vec::new();
    let mut node_id = 1usize;
    let mut learning_feedback = String::new();

    let mk_id = |n: usize| format!("node_{n}");

    nodes.push(serde_json::json!({
        "id": mk_id(node_id), "name": "触发器", "type": "trigger",
        "parameters": { "mode": "manual" }, "position": [100, 200]
    }));
    let trigger_id = mk_id(node_id);
    node_id += 1;
    let mut prev_id = trigger_id.clone();

    // 基于历史数据增强
    let needs_error_handling = if let Some(wf_id) = req.workflow_id.as_ref() {
        if let Some(db) = workflow_registry::get_db() {
            if let Ok(logs) = db.get_execution_logs(wf_id, 20) {
                let error_count = logs.iter().filter(|l| l.error.is_some()).count();
                error_count > 3
            } else { false }
        } else { false }
    } else { true };

    if needs_error_handling {
        improvements.push("添加了错误处理节点".to_string());
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "错误处理", "type": "error",
            "parameters": { "on_error": "continue" }, "position": [300, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id.clone(), "to": mk_id(node_id) }));
        prev_id = mk_id(node_id);
        node_id += 1;
    }

    if desc.contains("http") || desc.contains("api") || desc.contains("请求") {
        improvements.push("优化了 HTTP 请求配置".to_string());
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "HTTP 请求", "type": "http",
            "parameters": { "url": "", "method": "GET", "timeout": 5000, "retries": 3 }, "position": [500, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id.clone(), "to": mk_id(node_id) }));
        prev_id = mk_id(node_id);
        node_id += 1;
    }

    if desc.contains("数据库") || desc.contains("sql") || desc.contains("存储") {
        improvements.push("添加了数据库连接池".to_string());
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "数据库操作", "type": "db",
            "parameters": { "operation": "query", "pool_size": 5 }, "position": [700, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id.clone(), "to": mk_id(node_id) }));
        prev_id = mk_id(node_id);
        node_id += 1;
    }

    if desc.contains("邮件") || desc.contains("通知") {
        nodes.push(serde_json::json!({
            "id": mk_id(node_id), "name": "发送通知", "type": "email",
            "parameters": { "to": "", "subject": "" }, "position": [900, 200]
        }));
        connections.push(serde_json::json!({ "from": prev_id.clone(), "to": mk_id(node_id) }));
        prev_id = mk_id(node_id);
        node_id += 1;
    }

    // 添加日志记录节点
    improvements.push("添加了执行日志".to_string());
    nodes.push(serde_json::json!({
        "id": mk_id(node_id), "name": "日志记录", "type": "log",
        "parameters": { "level": "info" }, "position": [1100, 200]
    }));
    connections.push(serde_json::json!({ "from": prev_id, "to": mk_id(node_id) }));

    if let Some(db) = workflow_registry::get_db() {
        if let Ok(stats) = db.get_stats() {
            learning_feedback = format!(
                "基于历史数据(执行{}次，成功率{:.1}%)优化，预期性能提升 {:.0}%",
                stats.total_executions,
                stats.success_rate,
                10.0_f64.min(stats.success_rate * 0.2)
            );
        } else {
            learning_feedback = "工作流已增强，添加了错误处理和日志记录".to_string();
        }
    } else {
        learning_feedback = "工作流已增强".to_string();
    }

    if improvements.is_empty() {
        improvements.push("工作流已按标准模板增强".to_string());
    }

    let response = EnhanceNLResponse {
        improved_workflow: serde_json::json!({
            "name": req.description.chars().take(30).collect::<String>(),
            "description": req.description,
            "nodes": nodes,
            "connections": connections,
        }),
        improvements,
        learning_feedback,
    };
    (StatusCode::OK, Json(response)).into_response()
}

// ─── Task API ─────────────────────────────────────────────────────
use crate::db::{validate_state_transition, get_valid_transitions};

#[derive(Deserialize)]
struct CreateTaskRequest {
    workflow_id: String,
    owner: Option<String>,
    checkpoint: Option<String>,
}

#[derive(Deserialize)]
struct UpdateTaskRequest {
    status: String,
    checkpoint: Option<String>,
}

async fn create_task_handler(headers: HeaderMap, Json(req): Json<CreateTaskRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let task_id = Uuid::new_v4().to_string();
    if let Some(db) = workflow_registry::get_db() {
        match db.create_task(&task_id, &req.workflow_id, req.owner.as_deref(), req.checkpoint.as_deref()) {
            Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({ "task_id": task_id, "status": "created" }))).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

async fn get_task_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_task(&id) {
            Ok(Some(task)) => (StatusCode::OK, Json(task)).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Task not found".to_string() })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

async fn update_task_handler(headers: HeaderMap, Path(id): Path<String>, Json(req): Json<UpdateTaskRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_task(&id) {
            Ok(Some(task)) => {
                if !validate_state_transition(&task.status, &req.status) {
                    let valid = get_valid_transitions(&task.status);
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "error": format!("Invalid transition from {} to {}", task.status, req.status),
                        "valid_transitions": valid
                    }))).into_response();
                }
                match db.update_task_status(&id, &req.status, req.checkpoint.as_deref()) {
                    Ok(_) => {
                        db.add_task_event(&Uuid::new_v4().to_string(), &id, &format!("status_change:{}", req.status), None).ok();
                        (StatusCode::OK, Json(serde_json::json!({ "task_id": id, "status": req.status }))).into_response()
                    }
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
                }
            }
            Ok(None) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Task not found".to_string() })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

#[derive(Deserialize)]
struct TaskListQuery {
    workflow_id: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

async fn list_tasks_handler(headers: HeaderMap, Query(query): Query<TaskListQuery>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let limit = query.limit.unwrap_or(50);
    if let Some(db) = workflow_registry::get_db() {
        match db.list_tasks(query.workflow_id.as_deref(), query.status.as_deref(), limit) {
            Ok(tasks) => (StatusCode::OK, Json(tasks)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

async fn task_events_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    if let Some(db) = workflow_registry::get_db() {
        match db.get_task_events(&id) {
            Ok(events) => (StatusCode::OK, Json(events)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

// ─── Task Evidence Chain API ───────────────────────────────────────
#[derive(Deserialize)]
struct EvidenceRequest {
    checkpoint: String,
    owner: Option<String>,
    evidence: serde_json::Value,
}

async fn add_evidence_handler(headers: HeaderMap, Path(id): Path<String>, Json(req): Json<EvidenceRequest>) -> impl IntoResponse {
    let _auth = match require_auth(&headers).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let event_id = Uuid::new_v4().to_string();
    let evidence_json = serde_json::to_string(&req.evidence).unwrap_or_default();
    
    if let Some(db) = workflow_registry::get_db() {
        match db.update_task_status(&id, "running", Some(&req.checkpoint)) {
            Ok(_) => {
                db.add_task_event(&event_id, &id, "evidence", Some(&evidence_json)).ok();
                (StatusCode::OK, Json(serde_json::json!({ "event_id": event_id, "checkpoint": req.checkpoint }))).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "DB not initialized".to_string() })).into_response()
    }
}

let app = Router::new()
        .layer(axum::middleware::from_fn(add_security_headers))
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
        .route("/health", get(health_handler))
        .route("/stats", get(stats_handler))
        .route("/learning/patterns", get(learning_patterns_handler))
        .route("/learning/insights", get(learning_insights_handler))
        .route("/knowledge/query", get(knowledge_query_handler))
        .route("/knowledge/sources", get(list_knowledge_sources_handler))
        .route("/knowledge/sources", post(add_knowledge_source_handler))
        .route("/knowledge/upgrade/suggestions", get(list_upgrade_suggestions_handler))
        .route("/knowledge/upgrade/suggestions/:id/approve", post(approve_upgrade_handler))
        .route("/knowledge/upgrade/suggestions/:id/reject", post(reject_upgrade_handler))
        .route("/knowledge/upgrade/:id/merge", post(merge_upgrade_handler))
        .route("/knowledge/upgrade/history", get(upgrade_history_handler))
        .route("/knowledge/stale", get(detect_stale_handler))
        .route("/knowledge/compliance/check", post(compliance_check_handler))
        .route("/agents", get(list_agents_handler).post(register_agent_handler))
        .route("/agents/:id/knowledge", post(share_knowledge_handler))
        .route("/plugins/knowledge", get(list_knowledge_plugins_handler))
        .route("/plugins/knowledge", post(register_knowledge_plugin_handler))
        .route("/workflow/:id/ai-suggest", get(ai_suggest_handler))
        .route("/nl-create", post(nl_create_handler))
        .route("/optimize", post(optimize_handler))
        .route("/apply-tuning", post(apply_tuning_handler))
        .route("/summarize", post(summarize_handler))
        .route("/enhance-nl-workflow", post(enhance_nl_workflow_handler))
        .route("/tasks", post(create_task_handler))
        .route("/tasks", get(list_tasks_handler))
        .route("/tasks/:id", get(get_task_handler))
        .route("/tasks/:id", put(update_task_handler))
        .route("/tasks/:id/events", get(task_events_handler))
        .route("/tasks/:id/evidence", post(add_evidence_handler))
        .route("/admin/keys", post(create_api_key_handler))
        .route("/admin/keys", get(list_api_keys_handler))
        .route("/admin/keys/:key", delete(delete_api_key_handler))
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
