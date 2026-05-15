use crate::error::ExecError;
use compiler::ir::HttpMethod;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

pub fn execute_http_request(
    method: HttpMethod,
    url: &str,
    headers: &[(String, String)],
    body: &Option<Value>,
) -> Result<Value, ExecError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| ExecError::HttpError(e.to_string()))?;

    let mut request = match method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
        HttpMethod::Put => client.put(url),
        HttpMethod::Delete => client.delete(url),
        HttpMethod::Patch => client.patch(url),
    };

    let mut has_user_agent = false;
    let mut has_content_type = false;
    for (key, val) in headers {
        if key.to_lowercase() == "user-agent" {
            has_user_agent = true;
        }
        if key.to_lowercase() == "content-type" {
            has_content_type = true;
        }
        request = request.header(key, val);
    }

    if !has_user_agent {
        request = request.header("User-Agent", "memflow-executor/1.0");
    }

    if let Some(b) = body {
        if !has_content_type {
            request = request.header("Content-Type", "application/json");
        }
        request = request.json(b);
    }

    let response = request
        .send()
        .map_err(|e| ExecError::HttpError(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ExecError::HttpError(format!(
            "HTTP error: {} {}",
            status, url
        )));
    }

    let body = response
        .text()
        .map_err(|e| ExecError::HttpError(e.to_string()))?;

    let value: Value = serde_json::from_str(&body).unwrap_or(Value::String(body));

    Ok(value)
}

pub fn execute_http_get(url: &str, headers: &[(String, String)]) -> Result<Value, ExecError> {
    execute_http_request(HttpMethod::Get, url, headers, &None)
}
