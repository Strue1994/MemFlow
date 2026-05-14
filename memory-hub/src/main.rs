use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::main;

#[tokio::main]
async fn main() {
    println!("📚 MemFlow Memory Hub starting...");
    
    let port = std::env::var("MEMORY_HUB_PORT").unwrap_or_else(|_| "8081".to_string());
    let db_path = std::env::var("MEMORY_DB_PATH").ok();
    
    if let Err(e) = memory_hub::start_server(&port, db_path).await {
        eprintln!("Memory hub error: {}", e);
    }
}