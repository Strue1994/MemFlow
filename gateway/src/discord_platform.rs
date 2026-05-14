use crate::platform::{Attachment, GatewayError, MessagePlatform, MessageHandler};
use async_trait::async_trait;
use std::sync::Arc;

pub struct DiscordPlatform {
    bot_token: String,
    api_url: String,
}

impl DiscordPlatform {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            api_url: "https://discord.com/api/v10".to_string(),
        }
    }

    fn auth_header(&self) -> String { format!("Bot {}", self.bot_token) }
}

#[async_trait]
impl MessagePlatform for DiscordPlatform {
    fn platform_name(&self) -> &'static str { "discord" }

    async fn send_message(&self, channel_id: &str, text: &str) -> Result<(), GatewayError> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({"content": text});
        client.post(format!("{}/channels/{}/messages", self.api_url, channel_id))
            .header("Authorization", self.auth_header())
            .json(&body).send().await
            .map_err(|e| GatewayError::PlatformError(format!("Discord: {}", e)))?;
        Ok(())
    }

    async fn send_rich_message(&self, channel_id: &str, text: &str, _attachments: Vec<Attachment>) -> Result<(), GatewayError> {
        self.send_message(channel_id, text).await
    }

    async fn handle_command(&self, command: &str, _args: Vec<&str>, _user_id: &str) -> Result<String, GatewayError> {
        match command {
            "ping" => Ok("Pong! MemFlow is online.".to_string()),
            _ => Ok(format!("Discord command: {}", command)),
        }
    }

    async fn start_listening(&self, _handler: Arc<dyn MessageHandler>) -> Result<(), GatewayError> {
        tracing::info!("Discord gateway active (requires gateway process)");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
