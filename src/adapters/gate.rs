use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, PositionModeSetting, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::GateExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::market::{Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice, MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume, Ticker};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use crate::trade::{CancelOrderRequest, OrderAck, OrderSide, OrderType, PlaceOrderRequest, TimeInForce};
use gate_rs::{
    CancelOrderRequest as GateCancelOrderRequest, Config as GateConfig,
    Credentials as GateCredentials, GateClient, OrderRequest as GateOrderRequest,
};
use serde_json::Value;

const DEFAULT_SETTLE: &str = "usdt";

macro_rules! unsupported_methods {
    ($($name:ident (&self $(, $arg:ident : $arg_ty:ty)*) -> $ret:ty, $capability:literal;)+) => {
        $(
            pub(crate) async fn $name(&self, $($arg: $arg_ty),*) -> Result<$ret> {
                Err(Error::Unsupported {
                    exchange: ExchangeId::Gate,
                    capability: $capability,
                })
            }
        )+
    };
}

pub(crate) struct GateAdapter {
    client: GateClient,
    settle: String,
}

impl GateAdapter {
    pub(crate) fn new(config: GateExchangeConfig) -> Result<Self> {
        let mut gate_config = GateConfig::from_env();
        if let Some(api_url) = config.api_url {
            gate_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            gate_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            gate_config.proxy_url = Some(proxy_url);
        }

        Ok(Self {
            client: GateClient::with_config(
                Some(GateCredentials::new(config.api_key, config.api_secret)),
                gate_config,
            )
            .map_err(Error::from_gate)?,
            settle: config.settle.unwrap_or_else(|| DEFAULT_SETTLE.to_string()),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Gate;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .ticker(&self.settle, &symbol)
            .await
            .map_err(Error::from_gate)?;
        let item = raw.as_array().and_then(|items| items.first()).ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("Gate ticker response is empty for {symbol}"),
        })?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: string_field(item, "last").unwrap_or_default(),
            bid_price: string_field(item, "highest_bid"),
            ask_price: string_field(item, "lowest_ask"),
            volume_24h: string_field(item, "volume_24h_quote").or_else(|| string_field(item, "volume_24h_base")),
            timestamp: None,
            raw,
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Gate;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .orderbook(&self.settle, &symbol, query.limit)
            .await
            .map_err(Error::from_gate)?;
        Ok(OrderBook {
            exchange,
            instrument,
            exchange_symbol: symbol,
            bids: gate_levels(raw.get("bids")),
            asks: gate_levels(raw.get("asks")),
            timestamp: raw.get("t").and_then(value_u64),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Gate;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .candlesticks(
                &self.settle,
                &symbol,
                &query.interval,
                query.limit,
                query.start_time.map(millis_to_seconds),
                query.end_time.map(millis_to_seconds),
            )
            .await
            .map_err(Error::from_gate)?;
        let rows = raw.as_array().cloned().unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|row| gate_candle(exchange, &instrument, &symbol, row))
            .collect())
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let Some(instrument) = instrument else {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "all positions",
            });
        };
        let exchange = ExchangeId::Gate;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .position(&self.settle, &symbol)
            .await
            .map_err(Error::from_gate)?;
        Ok(vec![Position {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            side: string_field(&raw, "mode").or_else(|| side_from_gate_size(&raw)),
            size: string_field(&raw, "size").unwrap_or_default(),
            entry_price: string_field(&raw, "entry_price"),
            mark_price: string_field(&raw, "mark_price"),
            unrealized_pnl: string_field(&raw, "unrealised_pnl"),
            leverage: string_field(&raw, "leverage"),
            margin_mode: string_field(&raw, "margin_mode"),
            liquidation_price: string_field(&raw, "liq_price"),
            raw,
        }])
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Gate;
        let symbol = request.instrument.symbol_for(exchange);
        let size = signed_gate_size(request.side, &request.size)?;
        let raw = self
            .client
            .place_order(
                &self.settle,
                &GateOrderRequest {
                    contract: symbol.clone(),
                    size,
                    price: gate_order_price(request.order_type, request.price),
                    tif: request.time_in_force.map(gate_tif).map(ToOwned::to_owned),
                    text: request.client_order_id.clone(),
                    reduce_only: request.reduce_only,
                },
            )
            .await
            .map_err(Error::from_gate)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "id"),
            client_order_id: string_field(&raw, "text").or(request.client_order_id),
            status: string_field(&raw, "status"),
            raw,
        })
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Gate;
        let symbol = request.instrument.symbol_for(exchange);
        let order_id = request.order_id.clone().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Gate cancel requires order_id".to_string(),
        })?;
        let raw = self
            .client
            .cancel_order(&GateCancelOrderRequest {
                settle: self.settle.clone(),
                order_id: order_id.clone(),
                contract: symbol.clone(),
            })
            .await
            .map_err(Error::from_gate)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "id").or(Some(order_id)),
            client_order_id: string_field(&raw, "text").or(request.client_order_id),
            status: Some("cancelled".to_string()),
            raw,
        })
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Gate;
        let symbol = query.instrument.symbol_for(exchange);
        let order_id = query.order_id.clone().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Gate order query requires order_id".to_string(),
        })?;
        let raw = self
            .client
            .order(&self.settle, &order_id, &symbol)
            .await
            .map_err(Error::from_gate)?;
        Ok(order_from_value(exchange, query.instrument, symbol, raw))
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

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn value_u64(value: &Value) -> Option<u64> {
    value.as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn millis_to_seconds(value: u64) -> u64 {
    value / 1_000
}

fn gate_levels(value: Option<&Value>) -> Vec<OrderBookLevel> {
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

fn gate_candle(exchange: ExchangeId, instrument: &Instrument, symbol: &str, raw: Value) -> Candle {
    Candle {
        exchange,
        instrument: instrument.clone(),
        exchange_symbol: symbol.to_string(),
        open_time: raw.get("t").and_then(value_u64),
        close_time: None,
        open: string_field(&raw, "o").unwrap_or_default(),
        high: string_field(&raw, "h").unwrap_or_default(),
        low: string_field(&raw, "l").unwrap_or_default(),
        close: string_field(&raw, "c").unwrap_or_default(),
        volume: string_field(&raw, "v").unwrap_or_default(),
        quote_volume: string_field(&raw, "sum"),
        closed: None,
        raw,
    }
}

fn order_from_value(exchange: ExchangeId, instrument: Instrument, symbol: String, raw: Value) -> Order {
    Order {
        exchange,
        instrument,
        exchange_symbol: symbol,
        order_id: string_field(&raw, "id"),
        client_order_id: string_field(&raw, "text"),
        side: side_from_gate_size(&raw),
        order_type: string_field(&raw, "tif"),
        price: string_field(&raw, "price"),
        size: string_field(&raw, "size"),
        filled_size: string_field(&raw, "left").and_then(|left| {
            let size = string_field(&raw, "size")?.parse::<f64>().ok()?.abs();
            let left = left.parse::<f64>().ok()?.abs();
            Some((size - left).to_string())
        }),
        average_price: string_field(&raw, "fill_price"),
        status: string_field(&raw, "status"),
        created_at: raw.get("create_time_ms").and_then(value_u64),
        updated_at: raw.get("finish_time_ms").and_then(value_u64),
        raw,
    }
}

fn signed_gate_size(side: OrderSide, size: &str) -> Result<i64> {
    let amount = size.parse::<i64>().map_err(|_| Error::Adapter {
        exchange: ExchangeId::Gate,
        message: format!("Gate futures size must be integer contracts: {size}"),
    })?;
    Ok(match side {
        OrderSide::Buy => amount.abs(),
        OrderSide::Sell => -amount.abs(),
    })
}

fn side_from_gate_size(raw: &Value) -> Option<String> {
    let size = string_field(raw, "size")?.parse::<f64>().ok()?;
    Some(if size >= 0.0 { "buy" } else { "sell" }.to_string())
}

fn gate_order_price(order_type: OrderType, price: Option<String>) -> Option<String> {
    match order_type {
        OrderType::Market => Some("0".to_string()),
        OrderType::Limit => price,
    }
}

fn gate_tif(tif: TimeInForce) -> &'static str {
    match tif {
        TimeInForce::Gtc => "gtc",
        TimeInForce::Ioc => "ioc",
        TimeInForce::Fok => "fok",
        TimeInForce::PostOnly => "poc",
    }
}
