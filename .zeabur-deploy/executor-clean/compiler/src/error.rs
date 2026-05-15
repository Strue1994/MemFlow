use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Missing field: {0}")]
    MissingField(String),
    #[error("Unsupported node type: {0}")]
    UnsupportedNodeType(String),
}
