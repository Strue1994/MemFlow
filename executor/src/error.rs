use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ExecError {
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Math error: {0}")]
    MathError(String),
    #[error("Invalid return: no value to return")]
    InvalidReturn,
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Code execution error: {0}")]
    CodeError(String),
    #[error("Database error: {0}")]
    DbError(String),
    #[error("File error: {0}")]
    FileError(String),
    #[error("Security error: {0}")]
    SecurityError(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl ExecError {
    pub fn code(&self) -> &str {
        match self {
            ExecError::VariableNotFound(_) => "VARIABLE_NOT_FOUND",
            ExecError::MathError(_) => "MATH_ERROR",
            ExecError::InvalidReturn => "INVALID_RETURN",
            ExecError::HttpError(_) => "HTTP_ERROR",
            ExecError::CodeError(_) => "CODE_ERROR",
            ExecError::DbError(_) => "DB_ERROR",
            ExecError::FileError(_) => "FILE_ERROR",
            ExecError::SecurityError(_) => "SECURITY_ERROR",
            ExecError::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            ExecError::WorkflowNotFound(_) => "WORKFLOW_NOT_FOUND",
            ExecError::ParseError(_) => "PARSE_ERROR",
            ExecError::Timeout(_) => "TIMEOUT",
            ExecError::ValidationError(_) => "VALIDATION_ERROR",
        }
    }
}

#[derive(Serialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ExecError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ExecError::VariableNotFound(_) => StatusCode::NOT_FOUND,
            ExecError::MathError(_) => StatusCode::BAD_REQUEST,
            ExecError::InvalidReturn => StatusCode::BAD_REQUEST,
            ExecError::HttpError(_) => StatusCode::BAD_GATEWAY,
            ExecError::CodeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ExecError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ExecError::FileError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ExecError::SecurityError(_) => StatusCode::FORBIDDEN,
            ExecError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ExecError::WorkflowNotFound(_) => StatusCode::NOT_FOUND,
            ExecError::ParseError(_) => StatusCode::BAD_REQUEST,
            ExecError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            ExecError::ValidationError(_) => StatusCode::BAD_REQUEST,
        };

        let response = ApiErrorResponse {
            code: self.code().to_string(),
            message: self.to_string(),
            details: None,
        };

        (status, Json(response)).into_response()
    }
}
