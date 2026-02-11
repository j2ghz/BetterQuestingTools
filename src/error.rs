use crate::model::QuestId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid format: {0}")]
    InvalidFormat(String),

    #[error("duplicate quest id from file: {0}")]
    DuplicateQuestId(String),

    #[error(
        "missing quest reference: questline {questline} references missing quest id {quest_id:?}"
    )]
    MissingQuestReference { questline: u64, quest_id: QuestId },

    #[error("cycle detected in prerequisites: {0:?}")]
    CycleDetected(Vec<QuestId>),

    #[error("alpha out of range: {0}")]
    AlphaOutOfRange(f64),

    #[error("other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;
