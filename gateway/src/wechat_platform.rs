use crate::platform::*;
use async_trait::async_trait;
use std::sync::Arc;

pub struct WeChatPlatform { app_id: String, app_secret: String }
impl WeChatPlatform {
    pub fn new(app_id: &str, app_secret: &str) -> Self { Self { app_id: app_id.to_string(), app_secret: app_secret.to_string() } }
    async fn get_token(&self) -> Result<String, GatewayError> {
        let c = reqwest::Client::new();
        let r = c.get(&format!("https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}", self.app_id, self.app_secret))
            .send().await.map_err(|e| GatewayError::PlatformError(format!("WX token: {}", e)))?;
        let j: serde_json::Value = r.json().await.map_err(|e| GatewayError::PlatformError(format!("WX json: {}", e)))?;
        j["access_token"].as_str().map(|s| s.to_string()).ok_or(GatewayError::AuthFailed)
    }
}
#[async_trait]
impl MessagePlatform for WeChatPlatform {
    fn platform_name(&self) -> &'static str { "wechat" }
    async fn send_message(&self, open_id: &str, text: &str) -> Result<(), GatewayError> {
        let token = self.get_token().await?;
        let c = reqwest::Client::new();
        c.post(&format!("https://api.weixin.qq.com/cgi-bin/message/custom/send?access_token={}", token))
            .json(&serde_json::json!({"touser": open_id, "msgtype": "text", "text": {"content": text}}))
            .send().await.map_err(|e| GatewayError::PlatformError(format!("WX send: {}", e)))?;
        Ok(())
    }
    async fn send_rich_message(&self, ch: &str, t: &str, _a: Vec<Attachment>) -> Result<(), GatewayError> { self.send_message(ch, t).await }
    async fn handle_command(&self, cmd: &str, _a: Vec<&str>, _u: &str) -> Result<String, GatewayError> { Ok(format!("WX: {}", cmd)) }
    async fn start_listening(&self, _h: Arc<dyn MessageHandler>) -> Result<(), GatewayError> { Ok(()) }
    async fn disconnect(&self) -> Result<(), GatewayError> { Ok(()) }
}
