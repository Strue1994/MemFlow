use crate::error::ExecError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
    pub disable_web_page_preview: Option<bool>,
    pub disable_notification: Option<bool>,
    pub reply_to_message_id: Option<i64>,
    pub reply_markup: Option<TelegramReplyMarkup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelegramReplyMarkup {
    InlineKeyboard(Vec<Vec<TelegramInlineButton>>),
    ReplyKeyboard(Vec<Vec<String>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramInlineButton {
    pub text: String,
    pub url: Option<String>,
    pub callback_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramResponse {
    pub ok: bool,
    pub result: Option<TelegramMessageResult>,
    pub error_code: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessageResult {
    pub message_id: i64,
    pub chat: TelegramChat,
    pub date: i64,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    pub title: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

pub struct TelegramClient {
    client: Client,
    bot_token: String,
}

impl TelegramClient {
    pub fn new(bot_token: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            bot_token: bot_token.to_string(),
        }
    }

    pub fn send_message(&self, chat_id: &str, text: &str) -> Result<TelegramResponse, ExecError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Telegram request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                status
            )));
        }

        let telegram_resp: TelegramResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Telegram response: {}", e))
        })?;

        if telegram_resp.ok {
            Ok(telegram_resp)
        } else {
            let error = telegram_resp
                .description
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                error
            )))
        }
    }

    pub fn send_message_with_options(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
        disable_web_page_preview: Option<bool>,
    ) -> Result<TelegramResponse, ExecError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let mut body = serde_json::Map::new();
        body.insert(
            "chat_id".to_string(),
            serde_json::Value::String(chat_id.to_string()),
        );
        body.insert(
            "text".to_string(),
            serde_json::Value::String(text.to_string()),
        );

        if let Some(mode) = parse_mode {
            body.insert(
                "parse_mode".to_string(),
                serde_json::Value::String(mode.to_string()),
            );
        }

        if let Some(disable) = disable_web_page_preview {
            body.insert(
                "disable_web_page_preview".to_string(),
                serde_json::Value::Bool(disable),
            );
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Telegram request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                status
            )));
        }

        let telegram_resp: TelegramResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Telegram response: {}", e))
        })?;

        if telegram_resp.ok {
            Ok(telegram_resp)
        } else {
            let error = telegram_resp
                .description
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                error
            )))
        }
    }

    pub fn send_photo(
        &self,
        chat_id: &str,
        photo_url: &str,
        caption: Option<&str>,
    ) -> Result<TelegramResponse, ExecError> {
        let url = format!("https://api.telegram.org/bot{}/sendPhoto", self.bot_token);

        let mut body = serde_json::Map::new();
        body.insert(
            "chat_id".to_string(),
            serde_json::Value::String(chat_id.to_string()),
        );
        body.insert(
            "photo".to_string(),
            serde_json::Value::String(photo_url.to_string()),
        );

        if let Some(cap) = caption {
            body.insert(
                "caption".to_string(),
                serde_json::Value::String(cap.to_string()),
            );
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Telegram request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                status
            )));
        }

        let telegram_resp: TelegramResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Telegram response: {}", e))
        })?;

        if telegram_resp.ok {
            Ok(telegram_resp)
        } else {
            let error = telegram_resp
                .description
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(ExecError::HttpError(format!(
                "Telegram API error: {}",
                error
            )))
        }
    }

    pub fn get_updates(
        &self,
        offset: Option<i64>,
        limit: Option<i64>,
        timeout: Option<i64>,
    ) -> Result<Value, ExecError> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);

        let mut body = serde_json::Map::new();

        if let Some(off) = offset {
            body.insert("offset".to_string(), serde_json::Value::Number(off.into()));
        }
        if let Some(lim) = limit {
            body.insert("limit".to_string(), serde_json::Value::Number(lim.into()));
        }
        if let Some(tout) = timeout {
            body.insert(
                "timeout".to_string(),
                serde_json::Value::Number(tout.into()),
            );
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Telegram request failed: {}", e)))?;

        let value: Value = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Telegram response: {}", e))
        })?;

        Ok(value)
    }
}

pub fn execute_telegram_send(
    chat_id: &str,
    text: &str,
    bot_token: &str,
) -> Result<Value, ExecError> {
    let client = TelegramClient::new(bot_token);
    let response = client.send_message(chat_id, text)?;
    Ok(serde_json::to_value(response).unwrap_or(serde_json::Value::Null))
}

pub fn execute_telegram_send_html(
    chat_id: &str,
    text: &str,
    bot_token: &str,
) -> Result<Value, ExecError> {
    let client = TelegramClient::new(bot_token);
    let response = client.send_message_with_options(chat_id, text, Some("HTML"), None)?;
    Ok(serde_json::to_value(response).unwrap_or(serde_json::Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_client_creation() {
        let client = TelegramClient::new("test-token");
        assert_eq!(client.bot_token, "test-token");
    }

    #[test]
    fn test_telegram_message_serialization() {
        let msg = TelegramMessage {
            chat_id: "123456".to_string(),
            text: "Hello World".to_string(),
            parse_mode: Some("HTML".to_string()),
            disable_web_page_preview: Some(true),
            disable_notification: None,
            reply_to_message_id: None,
            reply_markup: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("123456"));
        assert!(json.contains("Hello World"));
    }
}
