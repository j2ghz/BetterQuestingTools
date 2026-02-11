use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unexpected format: {0}")]
    Unexpected(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;
