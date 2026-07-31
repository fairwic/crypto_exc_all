use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 调用方提供的请求无法由目标 Binance endpoint 合法表达。
    #[error("请求参数错误: {0}")]
    InvalidRequest(String),

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

    /// Typed public endpoint 在 transport、decode 或 provider 阶段返回的证据化错误。
    #[error("{failure}")]
    BinancePublicRequestFailed {
        /// Typed public endpoint 的结构化失败证据。
        failure: Box<crate::client::BinancePublicRequestFailure>,
    },

    #[error("WebSocket错误: {0}")]
    WebSocketError(String),

    #[error("缺少 API 凭证")]
    MissingCredentials,
}
