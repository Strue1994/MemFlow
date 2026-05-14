use crate::platform::{Attachment, GatewayError, Message, MessagePlatform, MessageHandler};
use async_trait::async_trait;
use std::sync::Arc;

pub struct TelegramPlatform {
    bot_token: String,
    api_url: String,
}

impl TelegramPlatform {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            api_url: format!("https://api.telegram.org/bot{}", bot_token),
        }
    }

    async fn api_call(&self, method: &str, body: &serde_json::Value) -> Result<serde_json::Value, GatewayError> {
        let client = reqwest::Client::new();
        let resp = client.post(format!("{}/{}", self.api_url, method))
            .json(body)
            .send()
            .await
            .map_err(|e| GatewayError::PlatformError(format!("Telegram API: {}", e)))?;
        resp.json().await.map_err(|e| GatewayError::PlatformError(format!("Telegram JSON: {}", e)))
    }
}

#[async_trait]
impl MessagePlatform for TelegramPlatform {
    fn platform_name(&self) -> &'static str { "telegram" }

    async fn send_message(&self, channel_id: &str, text: &str) -> Result<(), GatewayError> {
        let body = serde_json::json!({
            "chat_id": channel_id,
            "text": text,
            "parse_mode": "Markdown",
        });
        self.api_call("sendMessage", &body).await?;
        Ok(())
    }

    async fn send_rich_message(&self, channel_id: &str, text: &str, _attachments: Vec<Attachment>) -> Result<(), GatewayError> {
        self.send_message(channel_id, text).await
    }

    async fn handle_command(&self, command: &str, _args: Vec<&str>, _user_id: &str) -> Result<String, GatewayError> {
        match command {
            "start" => Ok("MemFlow Agent ready! Send me a task.".to_string()),
            "help" => Ok("Commands: /task <text> - run a task, /skills - list skills, /status - system status".to_string()),
            _ => Ok(format!("Unknown command: {}", command)),
        }
    }

    async fn start_listening(&self, _handler: Arc<dyn MessageHandler>) -> Result<(), GatewayError> {
        tracing::info!("Telegram listening via webhook at: {}/webhook", self.api_url);
        // Webhook registration would go here via setWebhook API
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
