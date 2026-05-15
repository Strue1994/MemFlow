use thiserror::Error;

#[derive(Debug, Error)]
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
}
