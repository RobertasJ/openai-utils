use serde_derive::{Deserialize, Serialize};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorWrapper {
    pub error: ApiError,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl ApiError {
    fn new(message: String, error_type: String) -> ApiError {
        ApiError {
            message,
            error_type,
            param: None,
            code: None,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}