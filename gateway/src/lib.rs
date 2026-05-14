pub mod platform;
pub mod router;
pub mod telegram_platform;
pub mod discord_platform;
pub mod slack_platform;
pub mod whatsapp_platform;
pub mod wechat_platform;
pub mod feishu_platform;

pub use platform::{Message, Attachment, MessagePlatform, MessageHandler, GatewayError, GatewayRouter};
pub use router::MessageRouter;

