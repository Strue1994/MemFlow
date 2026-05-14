use crate::platform::{GatewayError, Message, MessageHandler};
use async_trait::async_trait;
use std::sync::Arc;

pub struct MessageRouter {
    agent_handler: Arc<dyn AgentMessageHandler>,
}

#[async_trait]
pub trait AgentMessageHandler: Send + Sync {
    async fn process_message(&self, msg: Message) -> Result<String, GatewayError>;
}

impl MessageRouter {
    pub fn new(agent_handler: Arc<dyn AgentMessageHandler>) -> Self {
        Self { agent_handler }
    }

    pub async fn route(&self, msg: Message) -> Result<Option<String>, GatewayError> {
        let response = self.agent_handler.process_message(msg).await?;
        Ok(Some(response))
    }
}

pub struct GatewayMessageHandler {
    router: Arc<MessageRouter>,
}

impl GatewayMessageHandler {
    pub fn new(router: Arc<MessageRouter>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl MessageHandler for GatewayMessageHandler {
    async fn on_message(&self, msg: Message) -> Result<Option<String>, GatewayError> {
        self.router.route(msg).await
    }
}
