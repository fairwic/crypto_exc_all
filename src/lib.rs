pub mod account;
pub mod adapters;
pub mod config;
pub mod error;
pub mod exchange;
pub mod instrument;
pub mod market;
pub mod sdk;

pub mod raw {
    #[cfg(feature = "binance")]
    pub use binance_rs as binance;

    #[cfg(feature = "bitget")]
    pub use bitget_rs as bitget;

    #[cfg(feature = "okx")]
    pub use okx_rs as okx;
}

pub use account::{AccountFacade, Balance};
pub use config::{BinanceExchangeConfig, BitgetExchangeConfig, OkxExchangeConfig, SdkConfig};
pub use error::{Error, Result};
pub use exchange::ExchangeId;
pub use instrument::{Instrument, MarketType};
pub use market::{MarketFacade, Ticker};
pub use sdk::CryptoSdk;
