use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub platform: String,
    pub channel_id: String,
    pub user_id: String,
    pub text: String,
    pub attachments: Vec<Attachment>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String,
    pub url: String,
    pub name: Option<String>,
    pub size: Option<u64>,
}

#[async_trait]
pub trait MessagePlatform: Send + Sync {
    fn platform_name(&self) -> &'static str;
    async fn send_message(&self, channel_id: &str, text: &str) -> Result<(), GatewayError>;
    async fn send_rich_message(&self, channel_id: &str, text: &str, attachments: Vec<Attachment>) -> Result<(), GatewayError>;
    async fn handle_command(&self, command: &str, args: Vec<&str>, user_id: &str) -> Result<String, GatewayError>;
    async fn start_listening(&self, handler: Arc<dyn MessageHandler>) -> Result<(), GatewayError>;
    async fn disconnect(&self) -> Result<(), GatewayError>;
}

#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn on_message(&self, msg: Message) -> Result<Option<String>, GatewayError>;
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Platform error: {0}")]
    PlatformError(String),
    #[error("Not connected: {0}")]
    NotConnected(String),
    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Message too long ({0} chars, max {1})")]
    MessageTooLong(usize, usize),
}

pub struct GatewayRouter {
    platforms: RwLock<HashMap<String, Box<dyn MessagePlatform>>>,
    handler: Arc<dyn MessageHandler>,
}

impl GatewayRouter {
    pub fn new(handler: Arc<dyn MessageHandler>) -> Self {
        Self { platforms: RwLock::new(HashMap::new()), handler }
    }

    pub async fn register_platform(&self, platform: Box<dyn MessagePlatform>) {
        let name = platform.platform_name().to_string();
        self.platforms.write().await.insert(name, platform);
    }

    pub async fn start_all(&self) -> Vec<Result<(), GatewayError>> {
        let platforms = self.platforms.read().await;
        let mut results = Vec::new();
        for (_name, platform) in platforms.iter() {
            results.push(platform.start_listening(self.handler.clone()).await);
        }
        results
    }

    pub async fn broadcast(&self, channel_id: &str, text: &str) -> Vec<(String, Result<(), GatewayError>)> {
        let platforms = self.platforms.read().await;
        let mut results = Vec::new();
        for (name, platform) in platforms.iter() {
            results.push((name.clone(), platform.send_message(channel_id, text).await));
        }
        results
    }

    pub async fn platform_count(&self) -> usize {
        self.platforms.read().await.len()
    }

    pub async fn unregister_platform(&self, name: &str) {
        self.platforms.write().await.remove(name);
    }
}
