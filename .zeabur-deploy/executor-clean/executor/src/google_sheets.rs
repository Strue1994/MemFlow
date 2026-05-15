use crate::error::ExecError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSheetsConfig {
    pub spreadsheet_id: String,
    pub sheet_name: Option<String>,
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetRow {
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetResponse {
    pub range: String,
    pub major_dimension: String,
    pub values: Vec<Vec<Value>>,
}

pub struct GoogleSheetsClient {
    client: Client,
    access_token: String,
}

impl GoogleSheetsClient {
    pub fn new(access_token: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            access_token: access_token.to_string(),
        }
    }

    pub fn get_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> Result<SheetResponse, ExecError> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet_id, range
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .map_err(|e| ExecError::HttpError(format!("Google Sheets request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Google Sheets API error: {}",
                status
            )));
        }

        let sheet_resp: SheetResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Google Sheets response: {}", e))
        })?;

        Ok(sheet_resp)
    }

    pub fn update_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: &[Vec<Value>],
    ) -> Result<SheetResponse, ExecError> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=USER_ENTERED",
            spreadsheet_id,
            range
        );

        let body = serde_json::json!({
            "values": values
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Google Sheets update failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Google Sheets API error: {}",
                status
            )));
        }

        let sheet_resp: SheetResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Google Sheets response: {}", e))
        })?;

        Ok(sheet_resp)
    }

    pub fn append_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: &[Vec<Value>],
    ) -> Result<SheetResponse, ExecError> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:append?valueInputOption=USER_ENTERED",
            spreadsheet_id,
            range
        );

        let body = serde_json::json!({
            "values": values
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ExecError::HttpError(format!("Google Sheets append failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Google Sheets API error: {}",
                status
            )));
        }

        let sheet_resp: SheetResponse = response.json().map_err(|e| {
            ExecError::HttpError(format!("Failed to parse Google Sheets response: {}", e))
        })?;

        Ok(sheet_resp)
    }

    pub fn clear_values(&self, spreadsheet_id: &str, range: &str) -> Result<Value, ExecError> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet_id, range
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .map_err(|e| ExecError::HttpError(format!("Google Sheets clear failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ExecError::HttpError(format!(
                "Google Sheets API error: {}",
                status
            )));
        }

        let value: Value = response
            .json()
            .map_err(|e| ExecError::HttpError(format!("Failed to parse response: {}", e)))?;

        Ok(value)
    }
}

pub fn execute_google_sheets_read(
    spreadsheet_id: &str,
    range: &str,
    access_token: &str,
) -> Result<Value, ExecError> {
    let client = GoogleSheetsClient::new(access_token);
    let response = client.get_values(spreadsheet_id, range)?;
    Ok(serde_json::to_value(response).unwrap_or(serde_json::Value::Null))
}

pub fn execute_google_sheets_write(
    spreadsheet_id: &str,
    range: &str,
    values: &[Vec<Value>],
    access_token: &str,
) -> Result<Value, ExecError> {
    let client = GoogleSheetsClient::new(access_token);
    let response = client.update_values(spreadsheet_id, range, values)?;
    Ok(serde_json::to_value(response).unwrap_or(serde_json::Value::Null))
}

pub fn execute_google_sheets_append(
    spreadsheet_id: &str,
    range: &str,
    values: &[Vec<Value>],
    access_token: &str,
) -> Result<Value, ExecError> {
    let client = GoogleSheetsClient::new(access_token);
    let response = client.append_values(spreadsheet_id, range, values)?;
    Ok(serde_json::to_value(response).unwrap_or(serde_json::Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheets_client_creation() {
        let client = GoogleSheetsClient::new("test-token");
        assert_eq!(client.access_token, "test-token");
    }

    #[test]
    fn test_sheet_response_serialization() {
        let resp = SheetResponse {
            range: "Sheet1!A1:C3".to_string(),
            major_dimension: "ROWS".to_string(),
            values: vec![
                vec![
                    Value::String("A1".to_string()),
                    Value::String("B1".to_string()),
                    Value::String("C1".to_string()),
                ],
                vec![
                    Value::String("A2".to_string()),
                    Value::String("B2".to_string()),
                    Value::String("C2".to_string()),
                ],
            ],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Sheet1!A1:C3"));
    }
}
