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

    /// 私有流 transport 的脱敏生命周期错误，供 Account owner 决定恢复策略。
    #[error("私有流生命周期错误: {exchange} {phase}")]
    PrivateStreamLifecycle {
        /// 发生 transport 失败的交易所。
        exchange: ExchangeId,
        /// receive_connection_reset 等固定分类，不包含 URL、凭证或代理详情。
        phase: &'static str,
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
            okx_rs::Error::WebSocketError(category) => match okx_receive_phase(&category) {
                Some(phase) => Self::PrivateStreamLifecycle {
                    exchange: ExchangeId::Okx,
                    phase,
                },
                None => Self::Adapter {
                    exchange: ExchangeId::Okx,
                    message: okx_rs::Error::WebSocketError(category).to_string(),
                },
            },
            other => Self::Adapter {
                exchange: ExchangeId::Okx,
                message: other.to_string(),
            },
        }
    }

    /// 将 OKX mutation 的同步业务拒绝与传输/HTTP 不确定错误分开。
    #[cfg(feature = "okx")]
    pub(crate) fn from_okx_mutation(error: okx_rs::Error) -> Self {
        match error {
            okx_rs::Error::OkxApiError { code, message, smg }
                if !smg.is_empty()
                    || (code.len() == 5 && code.bytes().all(|byte| byte.is_ascii_digit())) =>
            {
                Self::Api {
                    exchange: ExchangeId::Okx,
                    status: Some(200),
                    code,
                    message: if smg.is_empty() {
                        message
                    } else {
                        format!("{message}: {smg}")
                    },
                }
            }
            other => Self::from_okx(other),
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
            binance_rs::Error::WebSocketReceiveError { category } => Self::PrivateStreamLifecycle {
                exchange: ExchangeId::Binance,
                phase: binance_receive_phase(category),
            },
            other => Self::Adapter {
                exchange: ExchangeId::Binance,
                message: other.to_string(),
            },
        }
    }

    #[cfg(feature = "bitget")]
    pub(crate) fn from_bitget(error: bitget_rs::Error) -> Self {
        match error {
            bitget_rs::Error::BitgetApiError {
                status,
                code,
                message,
            } => Self::Api {
                exchange: ExchangeId::Bitget,
                status,
                code,
                message,
            },
            bitget_rs::Error::ConfigError(message) => Self::Config(message),
            bitget_rs::Error::MissingCredentials => Self::MissingCredentials(ExchangeId::Bitget),
            other => Self::Adapter {
                exchange: ExchangeId::Bitget,
                message: other.to_string(),
            },
        }
    }

    #[cfg(feature = "bybit")]
    pub(crate) fn from_bybit(error: bybit_rs::Error) -> Self {
        match error {
            bybit_rs::Error::BybitApiError {
                status,
                code,
                message,
            } => Self::Api {
                exchange: ExchangeId::Bybit,
                status,
                code,
                message,
            },
            bybit_rs::Error::ConfigError(message) => Self::Config(message),
            bybit_rs::Error::MissingCredentials => Self::MissingCredentials(ExchangeId::Bybit),
            other => Self::Adapter {
                exchange: ExchangeId::Bybit,
                message: other.to_string(),
            },
        }
    }

    #[cfg(feature = "gate")]
    pub(crate) fn from_gate(error: gate_rs::Error) -> Self {
        match error {
            gate_rs::Error::GateApiError {
                status,
                code,
                message,
            } => Self::Api {
                exchange: ExchangeId::Gate,
                status,
                code,
                message,
            },
            gate_rs::Error::ConfigError(message) => Self::Config(message),
            gate_rs::Error::MissingCredentials => Self::MissingCredentials(ExchangeId::Gate),
            other => Self::Adapter {
                exchange: ExchangeId::Gate,
                message: other.to_string(),
            },
        }
    }

    #[cfg(feature = "hyperliquid")]
    pub(crate) fn from_hyperliquid(error: hyperliquid_rs::Error) -> Self {
        match error {
            hyperliquid_rs::Error::HyperliquidApiError {
                status,
                code,
                message,
            } => Self::Api {
                exchange: ExchangeId::Hyperliquid,
                status,
                code,
                message,
            },
            hyperliquid_rs::Error::ConfigError(message) => Self::Config(message),
            other => Self::Adapter {
                exchange: ExchangeId::Hyperliquid,
                message: other.to_string(),
            },
        }
    }
}

#[cfg(feature = "okx")]
fn okx_receive_phase(category: &str) -> Option<&'static str> {
    match category {
        "connection_closed" => Some("receive_connection_closed"),
        "already_closed" => Some("receive_already_closed"),
        "connection_reset" => Some("receive_connection_reset"),
        "connection_aborted" => Some("receive_connection_aborted"),
        "broken_pipe" => Some("receive_broken_pipe"),
        "timed_out" => Some("receive_timed_out"),
        "unexpected_eof" => Some("receive_unexpected_eof"),
        "io" => Some("receive_io"),
        "tls" => Some("receive_tls"),
        "capacity" => Some("receive_capacity"),
        "protocol" => Some("receive_protocol"),
        "utf8" => Some("receive_utf8"),
        "write_buffer_full" => Some("receive_write_buffer_full"),
        "attack_attempt" => Some("receive_attack_attempt"),
        "other" => Some("receive_other"),
        _ => None,
    }
}

/// 把 Binance SDK 的固定接收分类转换为统一 SDK 可公开的稳定 phase。
#[cfg(feature = "binance")]
fn binance_receive_phase(category: &'static str) -> &'static str {
    match category {
        "connection_closed" => "receive_connection_closed",
        "already_closed" => "receive_already_closed",
        "connection_reset" => "receive_connection_reset",
        "connection_aborted" => "receive_connection_aborted",
        "broken_pipe" => "receive_broken_pipe",
        "timed_out" => "receive_timed_out",
        "unexpected_eof" => "receive_unexpected_eof",
        "io" => "receive_io",
        "tls" => "receive_tls",
        "capacity" => "receive_capacity",
        "protocol" => "receive_protocol",
        "utf8" => "receive_utf8",
        "write_buffer_full" => "receive_write_buffer_full",
        "attack_attempt" => "receive_attack_attempt",
        _ => "receive_other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "okx")]
    #[test]
    fn maps_okx_receive_error_to_safe_private_stream_phase() {
        let error = Error::from_okx(okx_rs::Error::WebSocketError("connection_reset".to_owned()));

        assert!(matches!(
            error,
            Error::PrivateStreamLifecycle {
                exchange: ExchangeId::Okx,
                phase: "receive_connection_reset"
            }
        ));
    }

    #[test]
    fn maps_binance_receive_error_to_safe_private_stream_phase() {
        let error = Error::from_binance(binance_rs::Error::WebSocketReceiveError {
            category: "connection_reset",
        });

        assert!(matches!(
            error,
            Error::PrivateStreamLifecycle {
                exchange: ExchangeId::Binance,
                phase: "receive_connection_reset"
            }
        ));
    }
}
