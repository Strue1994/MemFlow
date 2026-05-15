use crate::error::ExecError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub channel: String,
    pub text: String,
    pub username: Option<String>,
    pub icon_emoji: Option<String>,
    pub attachments: Option<Vec<SlackAttachment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackAttachment {
    pub color: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub fields: Option<Vec<SlackField>>,
    pub footer: Option<String>,
    pub ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackField {
    pub title: String,
    pub value: String,
    pub short: Option<bool>,
}

pub struct SlackClient {
    client: Client,
    token: String,
}

impl SlackClient {
    pub fn new(token: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            token: token.to_string(),
        }
    }

    pub fn send_message(&self, channel: &str, text: &str) -> Result<Value, ExecError> {
        let url = "https://slack.com/api/chat.postMessage";

        let body = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Slack request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!("Slack API error: {}", status)));
        }

        let body: Value = response
            .json()
            .map_err(|e| ExecError::HttpError(format!("Failed to parse Slack response: {}", e)))?;

        if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(body)
        } else {
            let error = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(ExecError::HttpError(format!("Slack API error: {}", error)))
        }
    }

    pub fn send_message_with_attachments(
        &self,
        channel: &str,
        text: &str,
        attachments: &[SlackAttachment],
    ) -> Result<Value, ExecError> {
        let url = "https://slack.com/api/chat.postMessage";

        let mut body_map = serde_json::Map::new();
        body_map.insert(
            "channel".to_string(),
            serde_json::Value::String(channel.to_string()),
        );
        body_map.insert(
            "text".to_string(),
            serde_json::Value::String(text.to_string()),
        );

        if !attachments.is_empty() {
            body_map.insert(
                "attachments".to_string(),
                serde_json::to_value(attachments).unwrap_or(serde_json::Value::Null),
            );
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body_map)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Slack request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!("Slack API error: {}", status)));
        }

        let body: Value = response
            .json()
            .map_err(|e| ExecError::HttpError(format!("Failed to parse Slack response: {}", e)))?;

        if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(body)
        } else {
            let error = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(ExecError::HttpError(format!("Slack API error: {}", error)))
        }
    }

    pub fn upload_file(
        &self,
        channel: &str,
        content: &str,
        filename: &str,
    ) -> Result<Value, ExecError> {
        let url = "https://slack.com/api/files.upload";

        let body = serde_json::json!({
            "channels": channel,
            "content": content,
            "filename": filename,
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Slack file upload failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!("Slack API error: {}", status)));
        }

        let body: Value = response
            .json()
            .map_err(|e| ExecError::HttpError(format!("Failed to parse Slack response: {}", e)))?;

        if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(body)
        } else {
            let error = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(ExecError::HttpError(format!("Slack API error: {}", error)))
        }
    }
}

pub fn execute_slack_send(channel: &str, text: &str, token: &str) -> Result<Value, ExecError> {
    let client = SlackClient::new(token);
    client.send_message(channel, text)
}

pub fn execute_slack_send_with_attachments(
    channel: &str,
    text: &str,
    attachments: &[SlackAttachment],
    token: &str,
) -> Result<Value, ExecError> {
    let client = SlackClient::new(token);
    client.send_message_with_attachments(channel, text, attachments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_client_creation() {
        let client = SlackClient::new("test-token");
        assert_eq!(client.token, "test-token");
    }

    #[test]
    fn test_slack_message_serialization() {
        let msg = SlackMessage {
            channel: "test-channel".to_string(),
            text: "Hello World".to_string(),
            username: Some("Bot".to_string()),
            icon_emoji: Some(":robot:".to_string()),
            attachments: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test-channel"));
        assert!(json.contains("Hello World"));
    }
}
