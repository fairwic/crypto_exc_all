use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeId {
    Okx,
    Binance,
    Bitget,
    Bybit,
    Gate,
    Hyperliquid,
}

impl ExchangeId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Okx => "okx",
            Self::Binance => "binance",
            Self::Bitget => "bitget",
            Self::Bybit => "bybit",
            Self::Gate => "gate",
            Self::Hyperliquid => "hyperliquid",
        }
    }
}

impl fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExchangeId {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "okx" => Ok(Self::Okx),
            "binance" => Ok(Self::Binance),
            "bitget" => Ok(Self::Bitget),
            "bybit" => Ok(Self::Bybit),
            "gate" | "gateio" | "gate.io" => Ok(Self::Gate),
            "hyperliquid" | "hl" => Ok(Self::Hyperliquid),
            other => Err(format!("unsupported exchange: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bitget_exchange_id() {
        assert_eq!("bitget".parse::<ExchangeId>().unwrap(), ExchangeId::Bitget);
        assert_eq!(ExchangeId::Bitget.as_str(), "bitget");
    }

    #[test]
    fn parses_secondary_listing_exchange_ids() {
        assert_eq!("bybit".parse::<ExchangeId>().unwrap(), ExchangeId::Bybit);
        assert_eq!("gate".parse::<ExchangeId>().unwrap(), ExchangeId::Gate);
        assert_eq!(
            "hyperliquid".parse::<ExchangeId>().unwrap(),
            ExchangeId::Hyperliquid
        );
        assert_eq!(ExchangeId::Bybit.as_str(), "bybit");
        assert_eq!(ExchangeId::Gate.as_str(), "gate");
        assert_eq!(ExchangeId::Hyperliquid.as_str(), "hyperliquid");
    }
}
