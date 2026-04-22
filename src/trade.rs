use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::margin::MarginMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub(crate) fn lower(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }

    pub(crate) fn upper(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
    PostOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceOrderRequest {
    pub instrument: Instrument,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub size: String,
    pub price: Option<String>,
    pub margin_mode: Option<MarginMode>,
    pub margin_coin: Option<String>,
    pub position_side: Option<String>,
    pub trade_side: Option<String>,
    pub client_order_id: Option<String>,
    pub reduce_only: Option<bool>,
    pub time_in_force: Option<TimeInForce>,
}

impl PlaceOrderRequest {
    pub fn new(
        instrument: Instrument,
        side: OrderSide,
        order_type: OrderType,
        size: impl Into<String>,
    ) -> Self {
        Self {
            instrument,
            side,
            order_type,
            size: size.into(),
            price: None,
            margin_mode: None,
            margin_coin: None,
            position_side: None,
            trade_side: None,
            client_order_id: None,
            reduce_only: None,
            time_in_force: None,
        }
    }

    pub fn limit(
        instrument: Instrument,
        side: OrderSide,
        size: impl Into<String>,
        price: impl Into<String>,
    ) -> Self {
        Self::new(instrument, side, OrderType::Limit, size).with_price(price)
    }

    pub fn market(instrument: Instrument, side: OrderSide, size: impl Into<String>) -> Self {
        Self::new(instrument, side, OrderType::Market, size)
    }

    pub fn with_price(mut self, value: impl Into<String>) -> Self {
        self.price = Some(value.into());
        self
    }

    pub fn with_margin_mode(mut self, value: impl Into<MarginMode>) -> Self {
        self.margin_mode = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_position_side(mut self, value: impl Into<String>) -> Self {
        self.position_side = Some(value.into());
        self
    }

    pub fn with_trade_side(mut self, value: impl Into<String>) -> Self {
        self.trade_side = Some(value.into());
        self
    }

    pub fn with_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.client_order_id = Some(value.into());
        self
    }

    pub fn with_reduce_only(mut self, value: bool) -> Self {
        self.reduce_only = Some(value);
        self
    }

    pub fn with_time_in_force(mut self, value: TimeInForce) -> Self {
        self.time_in_force = Some(value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CancelOrderRequest {
    pub instrument: Instrument,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub margin_coin: Option<String>,
}

impl CancelOrderRequest {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderAck {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub status: Option<String>,
    pub raw: Value,
}

pub struct TradeFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> TradeFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        self.client.place_order(request).await
    }

    pub async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        self.client.cancel_order(request).await
    }
}
