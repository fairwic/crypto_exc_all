use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::margin::MarginMode;
use crate::order::OrderQuery;
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
    pub attached_stop_loss_price: Option<String>,
    pub attached_take_profit_price: Option<String>,
    pub attached_stop_loss_client_order_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProtectiveOrderWorkingType {
    MarkPrice,
    ContractPrice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeCapabilities {
    pub attached_stop_loss_on_place_order: bool,
    pub attached_take_profit_on_place_order: bool,
    pub protective_order: bool,
    pub protective_stop_amendment: bool,
}

impl ProtectiveOrderWorkingType {
    pub(crate) fn binance_value(self) -> &'static str {
        match self {
            Self::MarkPrice => "MARK_PRICE",
            Self::ContractPrice => "CONTRACT_PRICE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectiveOrderRequest {
    pub instrument: Instrument,
    pub side: OrderSide,
    pub stop_price: String,
    pub quantity: Option<String>,
    pub position_side: Option<String>,
    pub reduce_only: Option<bool>,
    pub close_position: Option<bool>,
    pub working_type: Option<ProtectiveOrderWorkingType>,
    pub price_protect: Option<bool>,
    pub client_order_id: Option<String>,
}

impl ProtectiveOrderRequest {
    pub fn stop_market(
        instrument: Instrument,
        side: OrderSide,
        stop_price: impl Into<String>,
    ) -> Self {
        Self {
            instrument,
            side,
            stop_price: stop_price.into(),
            quantity: None,
            position_side: None,
            reduce_only: None,
            close_position: None,
            working_type: None,
            price_protect: None,
            client_order_id: None,
        }
    }

    pub fn with_position_side(mut self, value: impl Into<String>) -> Self {
        self.position_side = Some(value.into());
        self
    }

    pub fn with_quantity(mut self, value: impl Into<String>) -> Self {
        self.quantity = Some(value.into());
        self
    }

    pub fn with_reduce_only(mut self, value: bool) -> Self {
        self.reduce_only = Some(value);
        self
    }

    pub fn with_close_position(mut self, value: bool) -> Self {
        self.close_position = Some(value);
        self
    }

    pub fn with_working_type(mut self, value: ProtectiveOrderWorkingType) -> Self {
        self.working_type = Some(value);
        self
    }

    pub fn with_price_protect(mut self, value: bool) -> Self {
        self.price_protect = Some(value);
        self
    }

    pub fn with_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.client_order_id = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectiveOrderQuery {
    pub instrument: Instrument,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
}

impl ProtectiveOrderQuery {
    pub fn new(instrument: Instrument) -> Self {
        Self {
            instrument,
            order_id: None,
            client_order_id: None,
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

    pub fn into_order_query(self) -> OrderQuery {
        let mut query = OrderQuery::new(self.instrument);
        if let Some(order_id) = self.order_id {
            query = query.with_order_id(order_id);
        }
        if let Some(client_order_id) = self.client_order_id {
            query = query.with_client_order_id(client_order_id);
        }
        query
    }
}

/// 原位修改既有保护止损策略单的 provider-neutral 请求。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AmendProtectiveStopRequest {
    pub instrument: Instrument,
    pub order_id: String,
    pub request_id: String,
    pub quantity: String,
    pub stop_price: String,
    pub take_profit_price: Option<String>,
    pub working_type: ProtectiveOrderWorkingType,
}

impl AmendProtectiveStopRequest {
    pub fn new(
        instrument: Instrument,
        order_id: impl Into<String>,
        request_id: impl Into<String>,
        quantity: impl Into<String>,
        stop_price: impl Into<String>,
        working_type: ProtectiveOrderWorkingType,
    ) -> Self {
        Self {
            instrument,
            order_id: order_id.into(),
            request_id: request_id.into(),
            quantity: quantity.into(),
            stop_price: stop_price.into(),
            take_profit_price: None,
            working_type,
        }
    }

    pub fn with_take_profit_price(mut self, value: impl Into<String>) -> Self {
        self.take_profit_price = Some(value.into());
        self
    }
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
            attached_stop_loss_price: None,
            attached_take_profit_price: None,
            attached_stop_loss_client_order_id: None,
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

    pub fn with_attached_stop_loss_price(mut self, value: impl Into<String>) -> Self {
        self.attached_stop_loss_price = Some(value.into());
        self
    }

    pub fn with_attached_take_profit_price(mut self, value: impl Into<String>) -> Self {
        self.attached_take_profit_price = Some(value.into());
        self
    }

    /// 为随主单提交的保护止损保留调用方稳定 identity。
    pub fn with_attached_stop_loss_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.attached_stop_loss_client_order_id = Some(value.into());
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

    pub fn capabilities(&self) -> TradeCapabilities {
        self.client.trade_capabilities()
    }

    pub async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        if request.attached_take_profit_price.is_some()
            && !self.capabilities().attached_take_profit_on_place_order
        {
            return Err(crate::error::Error::Unsupported {
                exchange: self.client.exchange_id(),
                capability: "attached take profit on place_order",
            });
        }
        self.client.place_order(request).await
    }

    pub async fn place_protective_order(
        &self,
        request: ProtectiveOrderRequest,
    ) -> Result<OrderAck> {
        self.client.place_protective_order(request).await
    }

    pub async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        self.client.cancel_order(request).await
    }

    pub async fn cancel_protective_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        self.client.cancel_protective_order(request).await
    }

    pub async fn amend_protective_stop(
        &self,
        request: AmendProtectiveStopRequest,
    ) -> Result<OrderAck> {
        self.client.amend_protective_stop(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::OrderQuery;

    #[test]
    fn protective_order_request_captures_stop_market_contract() {
        let request = ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("LONG")
        .with_reduce_only(true)
        .with_close_position(false)
        .with_working_type(ProtectiveOrderWorkingType::MarkPrice)
        .with_price_protect(true)
        .with_client_order_id("sl-rqethopen3");

        assert_eq!(request.instrument, Instrument::perp("ETH", "USDT"));
        assert_eq!(request.side, OrderSide::Sell);
        assert_eq!(request.stop_price, "2200");
        assert_eq!(request.position_side.as_deref(), Some("LONG"));
        assert_eq!(request.reduce_only, Some(true));
        assert_eq!(request.close_position, Some(false));
        assert_eq!(
            request.working_type,
            Some(ProtectiveOrderWorkingType::MarkPrice)
        );
        assert_eq!(request.price_protect, Some(true));
        assert_eq!(request.client_order_id.as_deref(), Some("sl-rqethopen3"));
    }

    #[test]
    fn place_order_request_carries_attached_exit_prices() {
        let request =
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                .with_attached_stop_loss_price("2200.5")
                .with_attached_take_profit_price("2800")
                .with_attached_stop_loss_client_order_id("rqstop00000000000000000000000000");

        assert_eq!(request.attached_stop_loss_price.as_deref(), Some("2200.5"));
        assert_eq!(request.attached_take_profit_price.as_deref(), Some("2800"));
        assert_eq!(
            request.attached_stop_loss_client_order_id.as_deref(),
            Some("rqstop00000000000000000000000000")
        );
    }

    #[test]
    fn protective_order_query_reuses_standard_order_identity_contract() {
        let query = ProtectiveOrderQuery::by_client_order_id(
            Instrument::perp("ETH", "USDT"),
            "sl-rqethopen3",
        );
        let order_query: OrderQuery = query.into_order_query();

        assert_eq!(order_query.instrument, Instrument::perp("ETH", "USDT"));
        assert_eq!(order_query.order_id, None);
        assert_eq!(
            order_query.client_order_id.as_deref(),
            Some("sl-rqethopen3")
        );
    }

    #[test]
    fn protective_stop_amendment_keeps_provider_and_request_identity() {
        let request = AmendProtectiveStopRequest::new(
            Instrument::perp("ETH", "USDT"),
            "2510789768709120",
            "rq0123456789abcdef0123456789abcd",
            "0.02",
            "2400.5",
            ProtectiveOrderWorkingType::MarkPrice,
        )
        .with_take_profit_price("2800");

        assert_eq!(request.order_id, "2510789768709120");
        assert_eq!(request.request_id, "rq0123456789abcdef0123456789abcd");
        assert_eq!(request.quantity, "0.02");
        assert_eq!(request.stop_price, "2400.5");
        assert_eq!(request.take_profit_price.as_deref(), Some("2800"));
        assert_eq!(request.working_type, ProtectiveOrderWorkingType::MarkPrice);
    }
}
