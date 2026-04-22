use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fill {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub trade_id: Option<String>,
    pub order_id: Option<String>,
    pub side: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    pub fee: Option<String>,
    pub fee_asset: Option<String>,
    pub role: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FillListQuery {
    pub instrument: Option<Instrument>,
    pub order_id: Option<String>,
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

impl FillListQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_instrument(instrument: Instrument) -> Self {
        Self::new().with_instrument(instrument)
    }

    pub fn with_instrument(mut self, value: Instrument) -> Self {
        self.instrument = Some(value);
        self
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    pub fn with_after(mut self, value: impl Into<String>) -> Self {
        self.after = Some(value.into());
        self
    }

    pub fn with_before(mut self, value: impl Into<String>) -> Self {
        self.before = Some(value.into());
        self
    }

    pub fn with_start_time(mut self, value: u64) -> Self {
        self.start_time = Some(value);
        self
    }

    pub fn with_end_time(mut self, value: u64) -> Self {
        self.end_time = Some(value);
        self
    }
}

pub struct FillFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> FillFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        self.client.fills(query).await
    }
}
