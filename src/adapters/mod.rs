use crate::account::Balance;
use crate::config::{BinanceExchangeConfig, BitgetExchangeConfig, OkxExchangeConfig};
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::market::Ticker;

#[cfg(feature = "binance")]
mod binance;
#[cfg(feature = "bitget")]
mod bitget;
#[cfg(feature = "okx")]
mod okx;

#[cfg(feature = "binance")]
pub(crate) use binance::BinanceAdapter;
#[cfg(feature = "bitget")]
pub(crate) use bitget::BitgetAdapter;
#[cfg(feature = "okx")]
pub(crate) use okx::OkxAdapter;

pub(crate) enum ExchangeClient {
    #[cfg(feature = "okx")]
    Okx(OkxAdapter),
    #[cfg(feature = "binance")]
    Binance(BinanceAdapter),
    #[cfg(feature = "bitget")]
    Bitget(BitgetAdapter),
}

impl ExchangeClient {
    #[cfg(feature = "okx")]
    pub(crate) fn okx(config: OkxExchangeConfig) -> Result<Self> {
        Ok(Self::Okx(OkxAdapter::new(config)?))
    }

    #[cfg(feature = "binance")]
    pub(crate) fn binance(config: BinanceExchangeConfig) -> Result<Self> {
        Ok(Self::Binance(BinanceAdapter::new(config)?))
    }

    #[cfg(feature = "bitget")]
    pub(crate) fn bitget(config: BitgetExchangeConfig) -> Result<Self> {
        Ok(Self::Bitget(BitgetAdapter::new(config)?))
    }

    pub(crate) fn exchange_id(&self) -> ExchangeId {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => ExchangeId::Okx,
            #[cfg(feature = "binance")]
            Self::Binance(_) => ExchangeId::Binance,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => ExchangeId::Bitget,
        }
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.ticker(instrument).await,
        }
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.balances().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.balances().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.balances().await,
        }
    }
}
