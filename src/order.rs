use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub side: Option<String>,
    pub order_type: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    pub filled_size: Option<String>,
    pub average_price: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderQuery {
    pub instrument: Instrument,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub margin_coin: Option<String>,
}

impl OrderQuery {
    pub fn new(instrument: Instrument) -> Self {
        Self {
            instrument,
            order_id: None,
            client_order_id: None,
            margin_coin: None,
        }
    }

    pub fn by_order_id(instrument: Instrument, order_id: impl Into<String>) -> Self {
        Self::new(instrument).with_order_id(order_id)
    }

    pub fn by_client_order_id(instrument: Instrument, client_order_id: impl Into<String>) -> Self {
        Self::new(instrument).with_client_order_id(client_order_id)
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.client_order_id = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderListQuery {
    pub instrument: Option<Instrument>,
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub status: Option<String>,
}

impl OrderListQuery {
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

    pub fn with_status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
        self
    }
}

pub struct OrderFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> OrderFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn get(&self, query: OrderQuery) -> Result<Order> {
        self.client.order(query).await
    }

    pub async fn open(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        self.client.open_orders(query).await
    }

    pub async fn history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        self.client.order_history(query).await
    }
}
