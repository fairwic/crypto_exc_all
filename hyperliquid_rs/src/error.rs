use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("hyperliquid api error status={status:?} code={code}: {message}")]
    HyperliquidApiError {
        status: Option<u16>,
        code: String,
        message: String,
    },
}
