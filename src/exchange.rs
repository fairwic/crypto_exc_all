use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeId {
    Okx,
    Binance,
}

impl ExchangeId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Okx => "okx",
            Self::Binance => "binance",
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
            other => Err(format!("unsupported exchange: {other}")),
        }
    }
}
