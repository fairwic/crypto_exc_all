//! Binance 与 OKX 公共 REST capability 的 provider-specific 传输配置。
//!
//! 本层只转交 SDK 配置类型，不把不同 provider 压平成会丢失差异的通用配置。

#[cfg(any(
    feature = "binance-public-instrument",
    feature = "binance-public-kline"
))]
pub use binance_rs::BinancePublicTransportConfig;
#[cfg(feature = "okx-public-market")]
pub use okx_rs::OkxPublicTransportConfig;
