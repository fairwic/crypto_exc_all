use crate::exchange::ExchangeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    Spot,
    Margin,
    Perpetual,
    Futures,
    Option,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instrument {
    pub base: String,
    pub quote: String,
    pub market_type: MarketType,
    pub settlement: Option<String>,
}

impl Instrument {
    pub fn new(base: impl Into<String>, quote: impl Into<String>, market_type: MarketType) -> Self {
        Self {
            base: base.into().to_ascii_uppercase(),
            quote: quote.into().to_ascii_uppercase(),
            market_type,
            settlement: None,
        }
    }

    pub fn spot(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self::new(base, quote, MarketType::Spot)
    }

    pub fn perp(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self::new(base, quote, MarketType::Perpetual)
    }

    pub fn with_settlement(mut self, settlement: impl Into<String>) -> Self {
        self.settlement = Some(settlement.into().to_ascii_uppercase());
        self
    }

    pub fn symbol_for(&self, exchange: ExchangeId) -> String {
        match exchange {
            ExchangeId::Okx => self.okx_symbol(),
            ExchangeId::Binance => self.binance_symbol(),
            ExchangeId::Bitget => self.bitget_symbol(),
        }
    }

    fn okx_symbol(&self) -> String {
        match self.market_type {
            MarketType::Spot | MarketType::Margin => format!("{}-{}", self.base, self.quote),
            MarketType::Perpetual => {
                let settlement = self.settlement.as_deref().unwrap_or(&self.quote);
                format!("{}-{settlement}-SWAP", self.base)
            }
            MarketType::Futures | MarketType::Option => {
                let settlement = self.settlement.as_deref().unwrap_or(&self.quote);
                format!("{}-{settlement}", self.base)
            }
        }
    }

    fn binance_symbol(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }

    fn bitget_symbol(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }
}
