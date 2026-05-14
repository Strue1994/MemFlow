use crate::platform::*;
use async_trait::async_trait;
use std::sync::Arc;

pub struct FeishuPlatform { app_id: String, app_secret: String }
impl FeishuPlatform {
    pub fn new(app_id: &str, app_secret: &str) -> Self { Self { app_id: app_id.to_string(), app_secret: app_secret.to_string() } }
    async fn get_token(&self) -> Result<String, GatewayError> {
        let c = reqwest::Client::new();
        let r = c.post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({"app_id": self.app_id, "app_secret": self.app_secret}))
            .send().await.map_err(|e| GatewayError::PlatformError(format!("FS token: {}", e)))?;
        let j: serde_json::Value = r.json().await.map_err(|e| GatewayError::PlatformError(format!("FS json: {}", e)))?;
        j["tenant_access_token"].as_str().map(|s| s.to_string()).ok_or(GatewayError::AuthFailed)
    }
}
#[async_trait]
impl MessagePlatform for FeishuPlatform {
    fn platform_name(&self) -> &'static str { "feishu" }
    async fn send_message(&self, chat_id: &str, text: &str) -> Result<(), GatewayError> {
        let token = self.get_token().await?;
        let c = reqwest::Client::new();
        c.post("https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({"receive_id": chat_id, "msg_type": "text", "content": serde_json::json!({"text": text}).to_string()}))
            .send().await.map_err(|e| GatewayError::PlatformError(format!("FS send: {}", e)))?;
        Ok(())
    }
    async fn send_rich_message(&self, ch: &str, t: &str, _a: Vec<Attachment>) -> Result<(), GatewayError> { self.send_message(ch, t).await }
    async fn handle_command(&self, cmd: &str, _a: Vec<&str>, _u: &str) -> Result<String, GatewayError> { Ok(format!("FS: {}", cmd)) }
    async fn start_listening(&self, _h: Arc<dyn MessageHandler>) -> Result<(), GatewayError> { Ok(()) }
    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
