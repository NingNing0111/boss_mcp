use std::path::PathBuf;

use thiserror::Error;

/// All errors produced by the agent module.
#[derive(Debug, Clone, Error)]
pub enum AgentError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("profile not found for candidate: {0}")]
    ProfileNotFound(String),

    #[error("session not found for candidate: {0}")]
    SessionNotFound(String),

    #[error("profile persistence failed: {0}")]
    ProfilePersistence(String),

    #[error("session persistence failed: {0}")]
    SessionPersistence(String),

    #[error("LLM extraction failed: {0}")]
    Extraction(String),

    #[error("LLM response generation failed: {0}")]
    ResponseGeneration(String),

    #[error("external service error: {0}")]
    ExternalService(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::Json(err.to_string())
    }
}

/// Result alias for agent operations.
pub type Result<T> = std::result::Result<T, AgentError>;

/// Internal error type for file-based persistence that converts to AgentError.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl PersistenceError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into(),
            source,
        }
    }
}

impl From<PersistenceError> for AgentError {
    fn from(err: PersistenceError) -> Self {
        match err {
            PersistenceError::Io { ref path, .. } => {
                AgentError::ProfilePersistence(format!("IO error at {}: {}", path.display(), err))
            }
            PersistenceError::Json { ref path, .. } => {
                AgentError::ProfilePersistence(format!("JSON error at {}: {}", path.display(), err))
            }
        }
    }
}
