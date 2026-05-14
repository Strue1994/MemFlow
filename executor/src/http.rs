use crate::error::ExecError;
use compiler::ir::HttpMethod;
use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

const BLOCKED_IP_PREFIXES: &[&str] = &[
    "10.", "172.16.", "172.17.", "172.18.", "172.19.", "172.2", "172.30.", "172.31.", "192.168.",
    "127.", "169.254.", "0.", "::1", "fc00:", "fe80:",
];

const BLOCKED_HOSTNAMES: &[&str] = &[
    "localhost",
    "metadata.google.internal",
    "metadata.google",
    "instancemetadata.google.internal",
];

fn is_blocked_url(url: &str) -> bool {
    if let Ok(parsed) = url::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("");
        let host_lower = host.to_lowercase();

        for blocked in BLOCKED_HOSTNAMES {
            if host_lower == *blocked {
                return true;
            }
        }

        for prefix in BLOCKED_IP_PREFIXES {
            if host.starts_with(prefix) {
                return true;
            }
        }

        if host.parse::<std::net::Ipv6Addr>().is_ok() {
            if host.starts_with("::") {
                return true;
            }
        }
    }
    false
}

#[allow(dead_code)]
pub fn check_url_allowed(url: &str) -> Result<(), ExecError> {
    if is_blocked_url(url) {
        return Err(ExecError::SecurityError(format!(
            "URL '{}' is not allowed (SSRF protection)",
            url
        )));
    }
    Ok(())
}

pub fn execute_http_request(
    method: HttpMethod,
    url: &str,
    headers: &[(String, String)],
    body: &Option<Value>,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
) -> Result<Value, ExecError> {
    check_url_allowed(url)?;

    let client = Client::builder()
        .timeout(Duration::from_millis(timeout_ms.unwrap_or(30_000)))
        .build()
        .map_err(|e| ExecError::HttpError(e.to_string()))?;

    let retry_config = RetryConfig {
        max_retries: max_retries.unwrap_or(3),
        ..RetryConfig::default()
    };

    let response = with_retry(retry_config, || {
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

        request
            .send()
            .map_err(|e| ExecError::HttpError(e.to_string()))
    })?;

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

    let value: Value = serde_json::from_str(&body).unwrap_or_else(|_| Value::String(body));

    Ok(value)
}

pub fn execute_http_get(url: &str, headers: &[(String, String)]) -> Result<Value, ExecError> {
    execute_http_request(HttpMethod::Get, url, headers, &None, None, None)
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<AtomicU8>,
    failures: Arc<AtomicU32>,
    last_failure: Arc<AtomicU32>,
    threshold: u32,
    recovery_timeout_secs: u64,
}

impl CircuitBreaker {
    pub fn new(threshold: u32) -> Self {
        Self {
            state: Arc::new(AtomicU8::new(CircuitState::Closed as u8)),
            failures: Arc::new(AtomicU32::new(0)),
            last_failure: Arc::new(AtomicU32::new(0)),
            threshold,
            recovery_timeout_secs: 30,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::SeqCst) == CircuitState::Open as u8
    }

    pub fn record_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        self.state
            .store(CircuitState::Closed as u8, Ordering::SeqCst);
    }

    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= self.threshold {
            self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
        }
    }
}

pub static HTTP_CIRCUIT_BREAKER: once_cell::sync::Lazy<CircuitBreaker> = once_cell::sync::Lazy::new(|| CircuitBreaker::new(5));

pub fn with_retry<R>(
    config: RetryConfig,
    mut f: impl FnMut() -> Result<R, ExecError>,
) -> Result<R, ExecError> {
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match f() {
            Ok(result) => {
                if attempt > 0 {
                    println!("[HTTP] Retry succeeded on attempt {}", attempt + 1);
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries {
                    let delay = config.backoff_ms * 2_u64.pow(attempt as u32);
                    println!("[HTTP] Retry {} after {}ms", attempt + 1, delay);
                }
            }
        }
    }

    Err(last_error.unwrap())
}
