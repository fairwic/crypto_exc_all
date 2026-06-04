use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("missing credentials")]
    MissingCredentials,
    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("signature error")]
    SignatureError,
    #[error("bybit api error status={status:?} code={code}: {message}")]
    BybitApiError {
        status: Option<u16>,
        code: String,
        message: String,
    },
}
