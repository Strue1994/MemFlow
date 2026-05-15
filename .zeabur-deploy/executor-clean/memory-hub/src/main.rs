use axum::{Router, routing::get, Json};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::main;

#[derive(serde::Serialize)]
struct Memory {
    id: String,
    content: String,
    memory_type: String,
    importance: f32,
    last_access: i64,
    created_at: i64,
}

async fn search_memories(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let query = params.get("q").cloned().unwrap_or_default();
    let k: usize = params.get("k").and_then(|v| v.parse().ok()).unwrap_or(5);
    
    println!("[MemoryHub] Search: '{}' (k={})", query, k);
    
    Json(json!({
        "memories": []
    }))
}

async fn add_memory(Json(payload): Json<Value>) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    
    Json(json!({
        "id": id,
        "created_at": now
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/memories/search", get(search_memories))
        .route("/memories", axum::routing::post(add_memory));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    println!("Memory Hub listening on http://{}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}