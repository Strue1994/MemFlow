use crate::platform::{Attachment, GatewayError, MessagePlatform, MessageHandler};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SlackPlatform {
    bot_token: String,
    signing_secret: String,
}

impl SlackPlatform {
    pub fn new(bot_token: &str, signing_secret: &str) -> Self {
        Self { bot_token: bot_token.to_string(), signing_secret: signing_secret.to_string() }
    }
}

#[async_trait]
impl MessagePlatform for SlackPlatform {
    fn platform_name(&self) -> &'static str { "slack" }

    async fn send_message(&self, channel_id: &str, text: &str) -> Result<(), GatewayError> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({"channel": channel_id, "text": text, "mrkdwn": true});
        client.post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body).send().await
            .map_err(|e| GatewayError::PlatformError(format!("Slack: {}", e)))?;
        Ok(())
    }

    async fn send_rich_message(&self, channel_id: &str, text: &str, _attachments: Vec<Attachment>) -> Result<(), GatewayError> {
        self.send_message(channel_id, text).await
    }

    async fn handle_command(&self, command: &str, _args: Vec<&str>, _user_id: &str) -> Result<String, GatewayError> {
        match command { "hello" => Ok("Hey! I am MemFlow on Slack.".to_string()), _ => Ok(format!("Slash: {}", command)) }
    }

    async fn start_listening(&self, _handler: Arc<dyn MessageHandler>) -> Result<(), GatewayError> { Ok(()) }
    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
