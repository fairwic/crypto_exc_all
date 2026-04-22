use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub side: Option<String>,
    pub size: String,
    pub entry_price: Option<String>,
    pub mark_price: Option<String>,
    pub unrealized_pnl: Option<String>,
    pub leverage: Option<String>,
    pub margin_mode: Option<String>,
    pub liquidation_price: Option<String>,
    pub raw: Value,
}

pub struct PositionFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> PositionFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        self.client.positions(instrument).await
    }
}
