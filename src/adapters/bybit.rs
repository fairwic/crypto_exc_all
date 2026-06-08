use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, PositionModeSetting, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::BybitExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use crate::trade::{
    CancelOrderRequest, OrderAck, OrderSide, OrderType, PlaceOrderRequest, TimeInForce,
};
use bybit_rs::{
    BybitClient, CancelOrderRequest as BybitCancelOrderRequest, Config as BybitConfig,
    Credentials as BybitCredentials, OrderRequest as BybitOrderRequest,
    OrderStatusRequest as BybitOrderStatusRequest, PositionListRequest as BybitPositionListRequest,
};
use serde_json::Value;

const DEFAULT_CATEGORY: &str = "linear";

macro_rules! unsupported_methods {
    ($($name:ident (&self $(, $arg:ident : $arg_ty:ty)*) -> $ret:ty, $capability:literal;)+) => {
        $(
            pub(crate) async fn $name(&self, $($arg: $arg_ty),*) -> Result<$ret> {
                Err(Error::Unsupported {
                    exchange: ExchangeId::Bybit,
                    capability: $capability,
                })
            }
        )+
    };
}

pub(crate) struct BybitAdapter {
    client: BybitClient,
    category: String,
}

impl BybitAdapter {
    pub(crate) fn new(config: BybitExchangeConfig) -> Result<Self> {
        let mut bybit_config = BybitConfig::from_env();
        if let Some(api_url) = config.api_url {
            bybit_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            bybit_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(recv_window_ms) = config.recv_window_ms {
            bybit_config.recv_window_ms = recv_window_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            bybit_config.proxy_url = Some(proxy_url);
        }

        Ok(Self {
            client: BybitClient::with_config(
                Some(BybitCredentials::new(config.api_key, config.api_secret)),
                bybit_config,
            )
            .map_err(Error::from_bybit)?,
            category: config
                .category
                .unwrap_or_else(|| DEFAULT_CATEGORY.to_string()),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .ticker(&self.category, &symbol)
            .await
            .map_err(Error::from_bybit)?;
        let item = first_list_item(&raw, exchange, "Bybit ticker response")?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: string_field(item, "lastPrice").unwrap_or_default(),
            bid_price: string_field(item, "bid1Price"),
            ask_price: string_field(item, "ask1Price"),
            volume_24h: string_field(item, "turnover24h")
                .or_else(|| string_field(item, "volume24h")),
            timestamp: None,
            raw,
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .orderbook(&self.category, &symbol, query.limit)
            .await
            .map_err(Error::from_bybit)?;

        Ok(OrderBook {
            exchange,
            instrument,
            exchange_symbol: symbol,
            bids: bybit_levels(raw.get("b")),
            asks: bybit_levels(raw.get("a")),
            timestamp: raw.get("ts").and_then(value_u64),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .kline(
                &self.category,
                &symbol,
                &query.interval,
                query.limit,
                query.start_time,
                query.end_time,
            )
            .await
            .map_err(Error::from_bybit)?;
        let rows = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|row| bybit_candle(exchange, &instrument, &symbol, row))
            .collect())
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .client
            .positions(&BybitPositionListRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        let list = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(list
            .into_iter()
            .map(|item| {
                let exchange_symbol = string_field(&item, "symbol").unwrap_or_default();
                let instrument = instrument
                    .cloned()
                    .unwrap_or_else(|| instrument_from_symbol(&exchange_symbol));
                Position {
                    exchange,
                    instrument,
                    exchange_symbol,
                    side: string_field(&item, "side"),
                    size: string_field(&item, "size").unwrap_or_default(),
                    entry_price: string_field(&item, "avgPrice"),
                    mark_price: string_field(&item, "markPrice"),
                    unrealized_pnl: string_field(&item, "unrealisedPnl"),
                    leverage: string_field(&item, "leverage"),
                    margin_mode: string_field(&item, "tradeMode"),
                    liquidation_price: string_field(&item, "liqPrice"),
                    raw: item,
                }
            })
            .collect())
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bybit;
        let symbol = request.instrument.symbol_for(exchange);
        let raw = self
            .client
            .place_order(&BybitOrderRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                side: bybit_side(request.side).to_string(),
                order_type: bybit_order_type(request.order_type).to_string(),
                qty: request.size,
                price: request.price,
                time_in_force: request.time_in_force.map(bybit_tif).map(ToOwned::to_owned),
                order_link_id: request.client_order_id.clone(),
                reduce_only: request.reduce_only,
            })
            .await
            .map_err(Error::from_bybit)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "orderId"),
            client_order_id: string_field(&raw, "orderLinkId").or(request.client_order_id),
            status: None,
            raw,
        })
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bybit;
        let symbol = request.instrument.symbol_for(exchange);
        let raw = self
            .client
            .cancel_order(&BybitCancelOrderRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                order_id: request.order_id.clone(),
                order_link_id: request.client_order_id.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "orderId").or(request.order_id),
            client_order_id: string_field(&raw, "orderLinkId").or(request.client_order_id),
            status: Some("cancelled".to_string()),
            raw,
        })
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Bybit;
        let symbol = query.instrument.symbol_for(exchange);
        let raw = self
            .client
            .order_status(&BybitOrderStatusRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                order_id: query.order_id.clone(),
                order_link_id: query.client_order_id.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        let item = first_list_item(&raw, exchange, "Bybit order response")?;
        Ok(order_from_value(
            exchange,
            query.instrument,
            symbol,
            item.clone(),
        ))
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: false,
            set_position_mode: false,
            set_symbol_margin_mode: false,
            order_level_margin_mode: false,
        }
    }

    unsupported_methods! {
        funding_rate(&self, _instrument: &Instrument) -> FundingRate, "funding rate";
        funding_rate_history(&self, _query: FundingRateQuery) -> Vec<FundingRate>, "funding rate history";
        mark_price(&self, _instrument: &Instrument) -> MarkPrice, "mark price";
        open_interest(&self, _instrument: &Instrument) -> OpenInterest, "open interest";
        long_short_ratio(&self, _query: MarketStatsQuery) -> Vec<LongShortRatio>, "long short ratio";
        taker_buy_sell_volume(&self, _query: MarketStatsQuery) -> Vec<TakerBuySellVolume>, "taker buy sell volume";
        balances(&self) -> Vec<Balance>, "balances";
        set_leverage(&self, _request: SetLeverageRequest) -> LeverageSetting, "set leverage";
        set_position_mode(&self, _request: SetPositionModeRequest) -> PositionModeSetting, "set position mode";
        set_symbol_margin_mode(&self, _request: SetSymbolMarginModeRequest) -> SymbolMarginModeSetting, "set symbol margin mode";
        ensure_order_margin_mode(&self, _request: EnsureOrderMarginModeRequest) -> EnsureOrderMarginModeResult, "ensure order margin mode";
        open_orders(&self, _query: OrderListQuery) -> Vec<Order>, "open orders";
        order_history(&self, _query: OrderListQuery) -> Vec<Order>, "order history";
        fills(&self, _query: FillListQuery) -> Vec<Fill>, "fills";
    }
}

fn first_list_item<'a>(raw: &'a Value, exchange: ExchangeId, context: &str) -> Result<&'a Value> {
    raw.get("list")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{context} is empty"),
        })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn value_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn bybit_levels(value: Option<&Value>) -> Vec<OrderBookLevel> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let values = row.as_array()?;
                    Some(OrderBookLevel {
                        price: values.first().and_then(Value::as_str)?.to_string(),
                        size: values.get(1).and_then(Value::as_str)?.to_string(),
                        raw: row.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn bybit_candle(
    exchange: ExchangeId,
    instrument: &Instrument,
    symbol: &str,
    row: Value,
) -> Option<Candle> {
    let values = row.as_array()?;
    Some(Candle {
        exchange,
        instrument: instrument.clone(),
        exchange_symbol: symbol.to_string(),
        open_time: values.first().and_then(value_u64),
        close_time: None,
        open: values.get(1)?.as_str()?.to_string(),
        high: values.get(2)?.as_str()?.to_string(),
        low: values.get(3)?.as_str()?.to_string(),
        close: values.get(4)?.as_str()?.to_string(),
        volume: values.get(5)?.as_str()?.to_string(),
        quote_volume: values.get(6).and_then(Value::as_str).map(ToOwned::to_owned),
        closed: None,
        raw: row,
    })
}

fn order_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol: String,
    raw: Value,
) -> Order {
    Order {
        exchange,
        instrument,
        exchange_symbol: symbol,
        order_id: string_field(&raw, "orderId"),
        client_order_id: string_field(&raw, "orderLinkId"),
        side: string_field(&raw, "side"),
        order_type: string_field(&raw, "orderType"),
        price: string_field(&raw, "price"),
        size: string_field(&raw, "qty"),
        filled_size: string_field(&raw, "cumExecQty"),
        average_price: string_field(&raw, "avgPrice"),
        status: string_field(&raw, "orderStatus"),
        created_at: raw.get("createdTime").and_then(value_u64),
        updated_at: raw.get("updatedTime").and_then(value_u64),
        raw,
    }
}

fn bybit_side(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "Buy",
        OrderSide::Sell => "Sell",
    }
}

fn bybit_order_type(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Market => "Market",
        OrderType::Limit => "Limit",
    }
}

fn bybit_tif(tif: TimeInForce) -> &'static str {
    match tif {
        TimeInForce::Gtc => "GTC",
        TimeInForce::Ioc => "IOC",
        TimeInForce::Fok => "FOK",
        TimeInForce::PostOnly => "PostOnly",
    }
}

fn instrument_from_symbol(symbol: &str) -> Instrument {
    let base = symbol.strip_suffix("USDT").unwrap_or(symbol);
    Instrument::perp(base, "USDT")
}
