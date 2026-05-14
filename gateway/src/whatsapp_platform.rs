use crate::platform::*;
use async_trait::async_trait;
use std::sync::Arc;

pub struct WhatsAppPlatform { phone_id: String, token: String }
impl WhatsAppPlatform {
    pub fn new(phone_id: &str, token: &str) -> Self { Self { phone_id: phone_id.to_string(), token: token.to_string() } }
}
#[async_trait]
impl MessagePlatform for WhatsAppPlatform {
    fn platform_name(&self) -> &'static str { "whatsapp" }
    async fn send_message(&self, to: &str, text: &str) -> Result<(), GatewayError> {
        let c = reqwest::Client::new();
        c.post(&format!("https://graph.facebook.com/v21.0/{}/messages", self.phone_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({"messaging_product": "whatsapp", "to": to, "type": "text", "text": {"body": text}}))
            .send().await.map_err(|e| GatewayError::PlatformError(format!("WA: {}", e)))?;
        Ok(())
    }
    async fn send_rich_message(&self, channel: &str, text: &str, _a: Vec<Attachment>) -> Result<(), GatewayError> { self.send_message(channel, text).await }
    async fn handle_command(&self, cmd: &str, _a: Vec<&str>, _u: &str) -> Result<String, GatewayError> { Ok(format!("WA cmd: {}", cmd)) }
    async fn start_listening(&self, _h: Arc<dyn MessageHandler>) -> Result<(), GatewayError> { Ok(()) }
    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
