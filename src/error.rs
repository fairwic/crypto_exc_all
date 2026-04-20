use crate::exchange::ExchangeId;
use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("交易所未配置: {0}")]
    ExchangeNotConfigured(ExchangeId),

    #[error("缺少交易所凭证: {0}")]
    MissingCredentials(ExchangeId),

    #[error("交易所不支持该能力: {exchange} {capability}")]
    Unsupported {
        exchange: ExchangeId,
        capability: &'static str,
    },

    #[error("交易所 API 错误: {exchange} status={status:?} code={code}: {message}")]
    Api {
        exchange: ExchangeId,
        status: Option<u16>,
        code: String,
        message: String,
    },

    #[error("交易所适配器错误: {exchange}: {message}")]
    Adapter {
        exchange: ExchangeId,
        message: String,
    },

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    #[cfg(feature = "okx")]
    pub(crate) fn from_okx(error: okx_rs::Error) -> Self {
        match error {
            okx_rs::Error::OkxApiError { code, message, smg } => Self::Api {
                exchange: ExchangeId::Okx,
                status: None,
                code,
                message: if smg.is_empty() {
                    message
                } else {
                    format!("{message}: {smg}")
                },
            },
            okx_rs::Error::ConfigError(message) => Self::Config(message),
            other => Self::Adapter {
                exchange: ExchangeId::Okx,
                message: other.to_string(),
            },
        }
    }

    #[cfg(feature = "binance")]
    pub(crate) fn from_binance(error: binance_rs::Error) -> Self {
        match error {
            binance_rs::Error::BinanceApiError {
                status,
                code,
                message,
            } => Self::Api {
                exchange: ExchangeId::Binance,
                status,
                code: code.to_string(),
                message,
            },
            binance_rs::Error::ConfigError(message) => Self::Config(message),
            binance_rs::Error::MissingCredentials => Self::MissingCredentials(ExchangeId::Binance),
            other => Self::Adapter {
                exchange: ExchangeId::Binance,
                message: other.to_string(),
            },
        }
    }
}
