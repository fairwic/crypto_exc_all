//! Binance USDⓈ-M 与 OKX SWAP 的公共 instrument 协议门面。
//!
//! 本模块只组合 provider SDK 已经解码的 wire DTO 和响应证据。币种池选择、
//! canonical identity、Decimal 量化、完整性判断、重试与恢复仍由 Market owner 负责。

#[cfg(feature = "binance")]
mod binance;
#[cfg(feature = "okx-public-market")]
mod okx;

#[cfg(feature = "binance")]
pub use binance::{
    BINANCE_USDM_EXCHANGE_INFO_IP_WEIGHT, BinanceUsdmPublicInstrumentClient,
    BinanceUsdmPublicInstrumentConfig,
};
#[cfg(feature = "okx-public-market")]
pub use okx::{
    OKX_SWAP_INSTRUMENT_RATE_LIMIT, OKX_SWAP_INSTRUMENT_RATE_WINDOW_MS,
    OkxSwapPublicInstrumentClient, OkxSwapPublicInstrumentConfig,
};

#[cfg(feature = "binance")]
pub use binance_rs::client::{
    BinanceHttpEvidence, BinancePublicFailureKind, BinancePublicRequestFailure,
    BinancePublicResponse,
};
#[cfg(feature = "binance")]
pub use binance_rs::dto::market::{
    BinanceExchangeAsset, BinanceExchangeFilter, BinanceExchangeInfo, BinanceExchangeSymbol,
    BinanceRateLimit, BinanceSymbolFilter, BinanceWireDecimal,
};
#[cfg(feature = "okx-public-market")]
pub use okx_rs::{
    OkxPublicFailureEvidence, OkxPublicFailureKind, OkxPublicInstrument, OkxPublicResponse,
    OkxPublicResponseEvidence,
};

/// 公共 instrument 请求在对应 provider SDK 中产生的错误。
///
/// 枚举保留 provider 原始错误类型，调用方可以继续读取 HTTP、业务码与 quota
/// 证据；这里不把不同交易所的错误压平成丢失信息的通用字符串。
#[derive(Debug, thiserror::Error)]
pub enum PublicInstrumentError {
    /// Binance USDⓈ-M 公共 instrument 请求失败。
    #[cfg(feature = "binance")]
    #[error(transparent)]
    Binance(#[from] binance_rs::Error),
    /// OKX SWAP 公共 instrument 请求失败。
    #[cfg(feature = "okx-public-market")]
    #[error(transparent)]
    Okx(#[from] okx_rs::Error),
}

/// 公共 instrument 门面的返回类型。
pub type PublicInstrumentResult<T> = std::result::Result<T, PublicInstrumentError>;
