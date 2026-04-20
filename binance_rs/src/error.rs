use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("HTTP错误: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("签名错误: {0}")]
    SignatureError(String),

    #[error("Binance API错误 (HTTP: {status:?}, 代码: {code}): {message}")]
    BinanceApiError {
        status: Option<u16>,
        code: i64,
        message: String,
    },

    #[error("WebSocket错误: {0}")]
    WebSocketError(String),

    #[error("缺少 API 凭证")]
    MissingCredentials,
}
