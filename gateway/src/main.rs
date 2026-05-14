/// P2.4: Unified Gateway — single entry point for ALL external requests
/// REST API + WebSocket + gRPC + Static files

use axum::{
    Router, routing::{get, post, any}, 
    response::{Json, IntoResponse},
    extract::State,
    http::StatusCode,
};
use std::sync::Arc;
use tokio::sync::RwLock;

struct GatewayState {
    agent_service_url: String,
    executor_url: String,
    memory_hub_url: String,
    web_ui_dir: String,
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok", "service": "gateway"}))
}

async fn proxy_handler(
    State(state): State<Arc<RwLock<GatewayState>>>,
    axum::extract::Path(path): axum::extract::Path<String>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let state = state.read().await;
    let target = format!("{}/{}", state.agent_service_url, path);
    match reqwest::Client::new()
        .request(req.method().clone(), &target)
        .send().await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            (status, body)
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "Upstream unavailable".to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MemFlow Unified Gateway starting...");

    let state = Arc::new(RwLock::new(GatewayState {
        agent_service_url: std::env::var("AGENT_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3300".to_string()),
        executor_url: std::env::var("EXECUTOR_URL")
            .unwrap_or_else(|_| "http://localhost:8082".to_string()),
        memory_hub_url: std::env::var("MEMORY_HUB_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
        web_ui_dir: std::env::var("WEB_UI_DIR")
            .unwrap_or_else(|_| "./web-ui/dist".to_string()),
    }));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/*path", any(proxy_handler))
        .with_state(state);

    let port = std::env::var("GATEWAY_PORT").unwrap_or_else(|_| "8084".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("Gateway listening on {}\n  REST: :{}/api/* -> agent-service\n  Health: :{}/health", addr, port, port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
