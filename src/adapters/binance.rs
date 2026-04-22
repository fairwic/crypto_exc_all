use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, PositionMode, PositionModeSetting, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::BinanceExchangeConfig;
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
use crate::trade::{CancelOrderRequest, OrderAck, OrderType, PlaceOrderRequest, TimeInForce};
use binance_rs::api::market::{
    FundingRateHistoryRequest as BinanceFundingRateHistoryRequest,
    FuturesDataRequest as BinanceFuturesDataRequest, KlineRequest as BinanceKlineRequest,
};
use binance_rs::api::trade::{
    ChangeLeverageRequest as BinanceChangeLeverageRequest,
    ChangeMarginTypeRequest as BinanceChangeMarginTypeRequest,
    ChangePositionModeRequest as BinanceChangePositionModeRequest,
    NewOrderRequest as BinanceNewOrderRequest, OrderIdRequest as BinanceOrderIdRequest,
    OrderListRequest as BinanceOrderListRequest,
};
use binance_rs::config::{Config as BinanceConfig, Credentials as BinanceCredentials};
use binance_rs::{BinanceAccount, BinanceClient, BinanceMarket, BinanceTrade};
use serde_json::Value;

pub(crate) struct BinanceAdapter {
    account: BinanceAccount,
    market: BinanceMarket,
    trade: BinanceTrade,
}

impl BinanceAdapter {
    pub(crate) fn new(config: BinanceExchangeConfig) -> Result<Self> {
        let mut binance_config = BinanceConfig::from_env();
        if let Some(api_url) = config.api_url {
            binance_config.api_url = api_url;
        }
        if let Some(sapi_api_url) = config.sapi_api_url {
            binance_config.sapi_api_url = sapi_api_url;
        }
        if let Some(web_api_url) = config.web_api_url {
            binance_config.web_api_url = web_api_url;
        }
        if let Some(ws_stream_url) = config.ws_stream_url {
            binance_config.ws_stream_url = ws_stream_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            binance_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(recv_window_ms) = config.recv_window_ms {
            binance_config.recv_window_ms = recv_window_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            binance_config.proxy_url = Some(proxy_url);
        }

        let credentials = BinanceCredentials::new(config.api_key, config.api_secret);
        let client = BinanceClient::with_config(Some(credentials), binance_config)
            .map_err(Error::from_binance)?;
        Ok(Self {
            account: BinanceAccount::new(client.clone()),
            market: BinanceMarket::new(client.clone()),
            trade: BinanceTrade::new(client),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_ticker_24hr(Some(&symbol))
            .await
            .map_err(Error::from_binance)?;
        let object = raw.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance ticker response is not an object".to_string(),
        })?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: string_field(object, "lastPrice").unwrap_or_default(),
            bid_price: string_field(object, "bidPrice"),
            ask_price: string_field(object, "askPrice"),
            volume_24h: string_field(object, "volume"),
            timestamp: u64_field(object, "closeTime"),
            raw,
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_depth(&symbol, query.limit)
            .await
            .map_err(Error::from_binance)?;

        binance_orderbook_from_value(exchange, instrument, symbol, raw)
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let mut request = BinanceKlineRequest::new(&symbol, &query.interval);
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(limit) = query.limit {
            request = request.with_limit(limit);
        }

        let raw = self
            .market
            .get_klines(request)
            .await
            .map_err(Error::from_binance)?;

        binance_candles_from_value(exchange, instrument, symbol, raw)
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_mark_price(Some(&symbol))
            .await
            .map_err(Error::from_binance)?;
        let item = first_owned_value(raw, exchange, "Binance funding rate response")?;

        binance_funding_rate_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let mut request = BinanceFundingRateHistoryRequest::new().with_symbol(&symbol);
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(limit) = query.limit {
            request = request.with_limit(limit);
        }

        let raw = self
            .market
            .get_funding_rate_history(request)
            .await
            .map_err(Error::from_binance)?;

        owned_value_items(raw, exchange, "Binance funding rate history response")?
            .into_iter()
            .map(|value| {
                binance_funding_rate_from_value(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    value,
                )
            })
            .collect()
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_mark_price(Some(&symbol))
            .await
            .map_err(Error::from_binance)?;
        let item = first_owned_value(raw, exchange, "Binance mark price response")?;

        binance_mark_price_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_open_interest(&symbol)
            .await
            .map_err(Error::from_binance)?;
        let item = first_owned_value(raw, exchange, "Binance open interest response")?;

        binance_open_interest_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Binance;
        let symbol = query.instrument.symbol_for(exchange);
        let request = binance_futures_data_request(&symbol, &query);
        let instrument = query.instrument;
        let period = query.period.clone();
        let raw = self
            .market
            .get_global_long_short_account_ratio(request)
            .await
            .map_err(Error::from_binance)?;

        owned_value_items(raw, exchange, "Binance long-short ratio response")?
            .into_iter()
            .map(|value| {
                binance_long_short_ratio_from_value(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    period.clone(),
                    value,
                )
            })
            .collect()
    }

    pub(crate) async fn taker_buy_sell_volume(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<TakerBuySellVolume>> {
        let exchange = ExchangeId::Binance;
        let symbol = query.instrument.symbol_for(exchange);
        let request = binance_futures_data_request(&symbol, &query);
        let instrument = query.instrument;
        let period = query.period.clone();
        let raw = self
            .market
            .get_taker_buy_sell_volume(request)
            .await
            .map_err(Error::from_binance)?;

        owned_value_items(raw, exchange, "Binance taker buy-sell volume response")?
            .into_iter()
            .map(|value| {
                binance_taker_volume_from_value(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    period.clone(),
                    value,
                )
            })
            .collect()
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let balances = self
            .account
            .get_balance()
            .await
            .map_err(Error::from_binance)?;

        balances
            .into_iter()
            .map(|balance| {
                let raw = serde_json::to_value(&balance)?;
                Ok(Balance {
                    exchange: ExchangeId::Binance,
                    asset: balance.asset,
                    total: balance.balance,
                    available: balance.available_balance,
                    frozen: non_empty(balance.cross_un_pnl),
                    raw,
                })
            })
            .collect()
    }

    pub(crate) async fn set_leverage(
        &self,
        request: SetLeverageRequest,
    ) -> Result<LeverageSetting> {
        let exchange = ExchangeId::Binance;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let leverage = request
            .leverage
            .parse::<u32>()
            .map_err(|_| Error::Adapter {
                exchange,
                message: format!(
                    "Binance leverage must be a positive integer: {}",
                    request.leverage
                ),
            })?;
        let raw = self
            .trade
            .change_leverage(BinanceChangeLeverageRequest::new(&symbol, leverage))
            .await
            .map_err(Error::from_binance)?;

        binance_leverage_setting_from_value(exchange, instrument, symbol, request, raw)
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: true,
            set_position_mode: true,
            set_symbol_margin_mode: true,
            order_level_margin_mode: false,
        }
    }

    pub(crate) async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        let exchange = ExchangeId::Binance;
        let dual_side_position = matches!(request.mode, PositionMode::Hedge);
        let raw = self
            .trade
            .change_position_mode(BinanceChangePositionModeRequest::new(dual_side_position))
            .await
            .map_err(Error::from_binance)?;

        Ok(PositionModeSetting {
            exchange,
            mode: request.mode,
            raw_mode: Some(dual_side_position.to_string()),
            product_type: request.product_type,
            raw,
        })
    }

    pub(crate) async fn set_symbol_margin_mode(
        &self,
        request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        let exchange = ExchangeId::Binance;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let raw_mode = request.mode.as_binance_margin_type();
        let raw = self
            .trade
            .change_margin_type(BinanceChangeMarginTypeRequest::new(
                &symbol,
                raw_mode.clone(),
            ))
            .await
            .map_err(Error::from_binance)?;

        Ok(SymbolMarginModeSetting {
            exchange,
            instrument,
            exchange_symbol: symbol,
            mode: request.mode,
            raw_mode: Some(raw_mode),
            product_type: request.product_type,
            margin_coin: request.margin_coin,
            raw,
        })
    }

    pub(crate) async fn ensure_order_margin_mode(
        &self,
        request: EnsureOrderMarginModeRequest,
    ) -> Result<EnsureOrderMarginModeResult> {
        let setting = self
            .set_symbol_margin_mode(request.into_set_symbol_request())
            .await?;

        Ok(EnsureOrderMarginModeResult::from_symbol_setting(setting))
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .account
            .get_positions(symbol.as_deref())
            .await
            .map_err(Error::from_binance)?;
        let values = value_items(&raw, exchange, "Binance positions response")?;

        let mut output = Vec::new();
        for value in values {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Binance position item is not an object".to_string(),
            })?;
            let exchange_symbol = string_field(object, "symbol").unwrap_or_default();
            if let Some(expected_symbol) = symbol.as_deref()
                && exchange_symbol != expected_symbol
            {
                continue;
            }

            let mapped_instrument = instrument
                .cloned()
                .unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));
            output.push(Position {
                exchange,
                instrument: mapped_instrument,
                exchange_symbol,
                side: string_field(object, "positionSide"),
                size: string_field(object, "positionAmt").unwrap_or_default(),
                entry_price: string_field(object, "entryPrice"),
                mark_price: string_field(object, "markPrice"),
                unrealized_pnl: string_field(object, "unRealizedProfit")
                    .or_else(|| string_field(object, "unrealizedProfit")),
                leverage: string_field(object, "leverage"),
                margin_mode: string_field(object, "marginType"),
                liquidation_price: string_field(object, "liquidationPrice"),
                raw: value.clone(),
            });
        }

        Ok(output)
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Binance;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let mut binance_request = match request.order_type {
            OrderType::Limit => BinanceNewOrderRequest::limit(
                &symbol,
                request.side.upper(),
                &request.size,
                required_price(exchange, &request)?,
                binance_time_in_force(request.time_in_force),
            ),
            OrderType::Market => {
                BinanceNewOrderRequest::market(&symbol, request.side.upper(), &request.size)
            }
        };

        if let Some(position_side) = request.position_side.as_deref() {
            binance_request =
                binance_request.with_position_side(position_side.to_ascii_uppercase());
        }
        if let Some(reduce_only) = request.reduce_only {
            binance_request = binance_request.with_reduce_only(reduce_only);
        }
        if let Some(client_order_id) = request.client_order_id.as_deref() {
            binance_request = binance_request.with_new_client_order_id(client_order_id);
        }

        let raw = self
            .trade
            .place_order(binance_request)
            .await
            .map_err(Error::from_binance)?;

        order_ack_from_value(exchange, instrument, symbol, raw, "Binance order response")
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Binance;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let mut binance_request = BinanceOrderIdRequest::new(&symbol);
        if let Some(order_id) = request.order_id.as_deref() {
            let parsed = order_id.parse::<u64>().map_err(|_| Error::Adapter {
                exchange,
                message: format!("Binance order_id must be numeric: {order_id}"),
            })?;
            binance_request = binance_request.with_order_id(parsed);
        } else if let Some(client_order_id) = request.client_order_id.as_deref() {
            binance_request = binance_request.with_orig_client_order_id(client_order_id);
        } else {
            return Err(missing_cancel_id(exchange));
        }

        let raw = self
            .trade
            .cancel_order(binance_request)
            .await
            .map_err(Error::from_binance)?;

        order_ack_from_value(exchange, instrument, symbol, raw, "Binance cancel response")
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let request =
            binance_order_id_request(exchange, &symbol, query.order_id, query.client_order_id)?;
        let raw = self
            .trade
            .get_order(request)
            .await
            .map_err(Error::from_binance)?;

        binance_order_from_value(
            exchange,
            Some(instrument),
            Some(symbol),
            raw,
            "Binance order response",
        )
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .trade
            .get_open_orders(symbol.as_deref())
            .await
            .map_err(Error::from_binance)?;

        binance_orders_from_value(
            exchange,
            instrument,
            symbol,
            raw,
            "Binance open orders response",
        )
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance order history requires an instrument".to_string(),
        })?;
        let symbol = instrument.symbol_for(exchange);
        let mut request = BinanceOrderListRequest::new(&symbol);
        if let Some(limit) = query.limit {
            request = request.with_limit(limit);
        }
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(after) = query.after.as_deref() {
            request = request.with_from_id(parse_u64_filter(exchange, "after", after)?);
        }

        let raw = self
            .trade
            .get_all_orders(request)
            .await
            .map_err(Error::from_binance)?;

        binance_orders_from_value(
            exchange,
            Some(instrument),
            Some(symbol),
            raw,
            "Binance order history response",
        )
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        let exchange = ExchangeId::Binance;
        let instrument = query.instrument.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance fills require an instrument".to_string(),
        })?;
        let symbol = instrument.symbol_for(exchange);
        let mut request = BinanceOrderListRequest::new(&symbol);
        if let Some(order_id) = query.order_id.as_deref() {
            request = request.with_order_id(parse_u64_filter(exchange, "order_id", order_id)?);
        }
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(after) = query.after.as_deref() {
            request = request.with_from_id(parse_u64_filter(exchange, "after", after)?);
        }
        if let Some(limit) = query.limit {
            request = request.with_limit(limit);
        }

        let raw = self
            .trade
            .get_user_trades(request)
            .await
            .map_err(Error::from_binance)?;

        binance_fills_from_value(exchange, instrument, symbol, raw, "Binance fills response")
    }
}

fn string_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(non_empty_value)
}

fn u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(|value| match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn non_empty_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_string_at(values: &[Value], index: usize) -> Option<String> {
    values.get(index).and_then(non_empty_value)
}

fn value_u64_at(values: &[Value], index: usize) -> Option<u64> {
    values.get(index).and_then(|value| match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn value_items<'a>(raw: &'a Value, exchange: ExchangeId, label: &str) -> Result<Vec<&'a Value>> {
    match raw {
        Value::Array(values) => Ok(values.iter().collect()),
        Value::Object(_) => Ok(vec![raw]),
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn owned_value_items(raw: Value, exchange: ExchangeId, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(_) => Ok(vec![raw]),
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn first_owned_value(raw: Value, exchange: ExchangeId, label: &str) -> Result<Value> {
    owned_value_items(raw, exchange, label)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} is empty"),
        })
}

fn instrument_from_linear_symbol(symbol: &str) -> Instrument {
    for quote in ["USDT", "USDC", "BUSD", "USD"] {
        if let Some(base) = symbol.strip_suffix(quote) {
            return Instrument::perp(base, quote);
        }
    }
    Instrument::perp(symbol, "USDT")
}

fn required_price(exchange: ExchangeId, request: &PlaceOrderRequest) -> Result<&str> {
    request.price.as_deref().ok_or_else(|| Error::Adapter {
        exchange,
        message: "limit orders require price".to_string(),
    })
}

fn binance_time_in_force(value: Option<TimeInForce>) -> &'static str {
    match value.unwrap_or(TimeInForce::Gtc) {
        TimeInForce::Gtc => "GTC",
        TimeInForce::Ioc => "IOC",
        TimeInForce::Fok => "FOK",
        TimeInForce::PostOnly => "GTX",
    }
}

fn order_ack_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
    label: &str,
) -> Result<OrderAck> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} is not an object"),
    })?;

    Ok(OrderAck {
        exchange,
        instrument,
        exchange_symbol,
        order_id: string_field(object, "orderId"),
        client_order_id: string_field(object, "clientOrderId"),
        status: string_field(object, "status"),
        raw,
    })
}

fn binance_order_id_request(
    exchange: ExchangeId,
    symbol: &str,
    order_id: Option<String>,
    client_order_id: Option<String>,
) -> Result<BinanceOrderIdRequest> {
    let request = BinanceOrderIdRequest::new(symbol);
    if let Some(order_id) = order_id {
        let parsed = order_id.parse::<u64>().map_err(|_| Error::Adapter {
            exchange,
            message: format!("Binance order_id must be numeric: {order_id}"),
        })?;
        Ok(request.with_order_id(parsed))
    } else if let Some(client_order_id) = client_order_id {
        Ok(request.with_orig_client_order_id(client_order_id))
    } else {
        Err(Error::Adapter {
            exchange,
            message: "order query requires order_id or client_order_id".to_string(),
        })
    }
}

fn parse_u64_filter(exchange: ExchangeId, field: &str, value: &str) -> Result<u64> {
    value.parse::<u64>().map_err(|_| Error::Adapter {
        exchange,
        message: format!("Binance {field} filter must be numeric: {value}"),
    })
}

fn first_string_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| string_field(object, field))
}

fn first_u64_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| u64_field(object, field))
}

fn binance_futures_data_request(
    symbol: &str,
    query: &MarketStatsQuery,
) -> BinanceFuturesDataRequest {
    let mut request = BinanceFuturesDataRequest::new(symbol, &query.period);
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    request
}

fn binance_funding_rate_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<FundingRate> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance funding rate item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(FundingRate {
        exchange,
        instrument,
        exchange_symbol,
        funding_rate: first_string_field(object, &["lastFundingRate", "fundingRate"])
            .unwrap_or_default(),
        funding_time: first_u64_field(object, &["fundingTime", "time"]),
        next_funding_rate: string_field(object, "nextFundingRate"),
        next_funding_time: u64_field(object, "nextFundingTime"),
        mark_price: string_field(object, "markPrice"),
        raw,
    })
}

fn binance_mark_price_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<MarkPrice> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance mark price item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(MarkPrice {
        exchange,
        instrument,
        exchange_symbol,
        mark_price: first_string_field(object, &["markPrice", "markPx"]).unwrap_or_default(),
        index_price: string_field(object, "indexPrice"),
        funding_rate: first_string_field(object, &["lastFundingRate", "fundingRate"]),
        next_funding_time: u64_field(object, "nextFundingTime"),
        timestamp: first_u64_field(object, &["time", "ts"]),
        raw,
    })
}

fn binance_open_interest_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<OpenInterest> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance open interest item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(OpenInterest {
        exchange,
        instrument,
        exchange_symbol,
        open_interest: first_string_field(object, &["openInterest", "oi"]).unwrap_or_default(),
        open_interest_value: first_string_field(
            object,
            &["openInterestValue", "sumOpenInterestValue", "oiCcy"],
        ),
        timestamp: first_u64_field(object, &["time", "ts"]),
        raw,
    })
}

fn binance_long_short_ratio_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    period: String,
    raw: Value,
) -> Result<LongShortRatio> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance long-short ratio item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(LongShortRatio {
        exchange,
        instrument,
        exchange_symbol,
        period,
        ratio: first_string_field(object, &["longShortRatio", "ratio"]).unwrap_or_default(),
        long_ratio: first_string_field(object, &["longAccount", "longPosition", "longRatio"]),
        short_ratio: first_string_field(object, &["shortAccount", "shortPosition", "shortRatio"]),
        timestamp: first_u64_field(object, &["timestamp", "time", "ts"]),
        raw,
    })
}

fn binance_taker_volume_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    period: String,
    raw: Value,
) -> Result<TakerBuySellVolume> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance taker buy-sell volume item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(TakerBuySellVolume {
        exchange,
        instrument,
        exchange_symbol,
        period,
        buy_volume: first_string_field(object, &["buyVol", "buyVolume"]).unwrap_or_default(),
        sell_volume: first_string_field(object, &["sellVol", "sellVolume"]).unwrap_or_default(),
        buy_sell_ratio: first_string_field(object, &["buySellRatio", "ratio"]),
        timestamp: first_u64_field(object, &["timestamp", "time", "ts"]),
        raw,
    })
}

fn binance_leverage_setting_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: String,
    request: SetLeverageRequest,
    raw: Value,
) -> Result<LeverageSetting> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance leverage response is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "symbol").unwrap_or(symbol_hint);

    Ok(LeverageSetting {
        exchange,
        instrument,
        exchange_symbol,
        leverage: string_field(object, "leverage").unwrap_or(request.leverage),
        margin_mode: None,
        margin_coin: request.margin_coin,
        position_side: request.position_side,
        raw,
    })
}

fn binance_orders_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Order>> {
    owned_value_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            binance_order_from_value(
                exchange,
                instrument.clone(),
                symbol_hint.clone(),
                value,
                "Binance order item",
            )
        })
        .collect()
}

fn binance_order_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Order> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} is not an object"),
    })?;
    let exchange_symbol = string_field(object, "symbol")
        .or(symbol_hint)
        .unwrap_or_default();
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: string_field(object, "orderId"),
        client_order_id: string_field(object, "clientOrderId"),
        side: string_field(object, "side"),
        order_type: string_field(object, "type"),
        price: string_field(object, "price"),
        size: first_string_field(object, &["origQty", "quantity"]),
        filled_size: first_string_field(object, &["executedQty", "cumQty", "filledQty"]),
        average_price: first_string_field(object, &["avgPrice", "averagePrice"]),
        status: string_field(object, "status"),
        created_at: u64_field(object, "time"),
        updated_at: u64_field(object, "updateTime"),
        raw,
    })
}

fn binance_orderbook_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<OrderBook> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance orderbook response is not an object".to_string(),
    })?;

    Ok(OrderBook {
        exchange,
        instrument,
        exchange_symbol,
        bids: binance_book_levels(object.get("bids"), exchange, "bids")?,
        asks: binance_book_levels(object.get("asks"), exchange, "asks")?,
        timestamp: u64_field(object, "E")
            .or_else(|| u64_field(object, "T"))
            .or_else(|| u64_field(object, "ts")),
        raw,
    })
}

fn binance_book_levels(
    value: Option<&Value>,
    exchange: ExchangeId,
    side: &str,
) -> Result<Vec<OrderBookLevel>> {
    let Some(Value::Array(levels)) = value else {
        return Err(Error::Adapter {
            exchange,
            message: format!("Binance orderbook {side} is not an array"),
        });
    };

    levels
        .iter()
        .map(|level| {
            let values = level.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("Binance orderbook {side} level is not an array"),
            })?;
            Ok(OrderBookLevel {
                price: value_string_at(values, 0).unwrap_or_default(),
                size: value_string_at(values, 1).unwrap_or_default(),
                raw: level.clone(),
            })
        })
        .collect()
}

fn binance_candles_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<Vec<Candle>> {
    let Value::Array(items) = raw else {
        return Err(Error::Adapter {
            exchange,
            message: "Binance candles response is not an array".to_string(),
        });
    };

    items
        .into_iter()
        .map(|item| {
            let values = item.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Binance candle item is not an array".to_string(),
            })?;
            Ok(Candle {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol: exchange_symbol.clone(),
                open_time: value_u64_at(values, 0),
                close_time: value_u64_at(values, 6),
                open: value_string_at(values, 1).unwrap_or_default(),
                high: value_string_at(values, 2).unwrap_or_default(),
                low: value_string_at(values, 3).unwrap_or_default(),
                close: value_string_at(values, 4).unwrap_or_default(),
                volume: value_string_at(values, 5).unwrap_or_default(),
                quote_volume: value_string_at(values, 7),
                closed: None,
                raw: item,
            })
        })
        .collect()
}

fn maker_role(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_bool)
        .map(|maker| if maker { "maker" } else { "taker" }.to_string())
}

fn binance_fills_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: String,
    raw: Value,
    label: &str,
) -> Result<Vec<Fill>> {
    owned_value_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Binance fill item is not an object".to_string(),
            })?;
            let exchange_symbol =
                string_field(object, "symbol").unwrap_or_else(|| symbol_hint.clone());
            Ok(Fill {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol,
                trade_id: string_field(object, "id"),
                order_id: string_field(object, "orderId"),
                side: string_field(object, "side"),
                price: string_field(object, "price"),
                size: first_string_field(object, &["qty", "quantity"]),
                fee: string_field(object, "commission"),
                fee_asset: string_field(object, "commissionAsset"),
                role: maker_role(object.get("maker")),
                timestamp: u64_field(object, "time"),
                raw: value,
            })
        })
        .collect()
}

fn missing_cancel_id(exchange: ExchangeId) -> Error {
    Error::Adapter {
        exchange,
        message: "cancel_order requires order_id or client_order_id".to_string(),
    }
}
