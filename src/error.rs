use thiserror::Error;
use serde::{Deserialize, Serialize};

// Define an enum for internal errors.
#[derive(Debug, Error)]
pub enum InternalError {
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Request build error: {0}")]
    RequestBuildError(#[from] reqwest::Error),

    #[error("Event source error: {0}")]
    EventSourceError(#[from] reqwest_eventsource::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("no deltas were received, cannot construct chat")]
    NoDeltasReceived,
}

// Define an enum for OpenAI API errors.
#[derive(Debug, Error, Clone, Deserialize, Serialize)]
pub enum OpenAIError {
    #[error("OpenAI API error: {message}")]
    ApiError {
        message: String,
        #[serde(rename = "type")]
        error_type: String,
        param: Option<String>,
        code: Option<String>,
    },
}

// Define a wrapper enum for all types of errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),

    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] OpenAIError),
}

// Convenience type alias for `Result` with our custom error type.
pub type UtilsResult<T> = Result<T, Error>;
