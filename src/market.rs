use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ticker {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub last_price: String,
    pub bid_price: Option<String>,
    pub ask_price: Option<String>,
    pub volume_24h: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

pub struct MarketFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> MarketFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        self.client.ticker(instrument).await
    }
}
