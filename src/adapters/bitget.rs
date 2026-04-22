use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, PositionMode, PositionModeSetting, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::BitgetExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::margin::MarginMode;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use crate::trade::{CancelOrderRequest, OrderAck, OrderType, PlaceOrderRequest, TimeInForce};
use bitget_rs::api::market::TickerRequest;
use bitget_rs::api::trade::{
    CancelOrderRequest as BitgetCancelOrderRequest, NewOrderRequest as BitgetNewOrderRequest,
    OrderQueryRequest as BitgetOrderQueryRequest,
};
use bitget_rs::config::{Config as BitgetConfig, Credentials as BitgetCredentials};
use bitget_rs::{BitgetAccount, BitgetClient, BitgetMarket, BitgetTrade};
use serde_json::Value;

const DEFAULT_PRODUCT_TYPE: &str = "USDT-FUTURES";

pub(crate) struct BitgetAdapter {
    account: BitgetAccount,
    market: BitgetMarket,
    trade: BitgetTrade,
    product_type: String,
}

impl BitgetAdapter {
    pub(crate) fn new(config: BitgetExchangeConfig) -> Result<Self> {
        let mut bitget_config = BitgetConfig::from_env();
        if let Some(api_url) = config.api_url {
            bitget_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            bitget_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            bitget_config.proxy_url = Some(proxy_url);
        }

        let product_type = config
            .product_type
            .unwrap_or_else(|| DEFAULT_PRODUCT_TYPE.to_string());
        let credentials =
            BitgetCredentials::new(config.api_key, config.api_secret, config.passphrase);
        let client = BitgetClient::with_config(Some(credentials), bitget_config)
            .map_err(Error::from_bitget)?;

        Ok(Self {
            account: BitgetAccount::new(client.clone()),
            market: BitgetMarket::new(client.clone()),
            trade: BitgetTrade::new(client),
            product_type,
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.symbol_for(exchange);
        let mut tickers = self
            .market
            .get_ticker(TickerRequest::new(&symbol, &self.product_type))
            .await
            .map_err(Error::from_bitget)?;
        let ticker = tickers.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("Bitget ticker response is empty for {symbol}"),
        })?;
        let raw = serde_json::to_value(&ticker)?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: ticker.last_price,
            bid_price: non_empty(ticker.bid_price),
            ask_price: non_empty(ticker.ask_price),
            volume_24h: non_empty(ticker.quote_volume).or_else(|| non_empty(ticker.base_volume)),
            timestamp: ticker.ts.parse::<u64>().ok(),
            raw,
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let raw = self
            .market
            .get_orderbook(&symbol, &self.product_type, limit.as_deref())
            .await
            .map_err(Error::from_bitget)?;

        bitget_orderbook_from_value(exchange, instrument, symbol, raw)
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_candles(&symbol, &self.product_type, &query.interval, query.limit)
            .await
            .map_err(Error::from_bitget)?;

        bitget_candles_from_value(exchange, instrument, symbol, raw)
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_current_funding_rate(&self.product_type, Some(&symbol))
            .await
            .map_err(Error::from_bitget)?;
        let item = first_metric_value(raw, exchange, "Bitget funding rate response")?;

        bitget_funding_rate_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_funding_rate_history(&symbol, &self.product_type)
            .await
            .map_err(Error::from_bitget)?;

        owned_metric_items(raw, exchange, "Bitget funding rate history response")?
            .into_iter()
            .map(|value| {
                bitget_funding_rate_from_value(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    value,
                )
            })
            .collect()
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_symbol_price(&symbol, &self.product_type)
            .await
            .map_err(Error::from_bitget)?;
        let item = first_metric_value(raw, exchange, "Bitget mark price response")?;

        bitget_mark_price_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_open_interest(&symbol, &self.product_type)
            .await
            .map_err(Error::from_bitget)?;
        let item = first_metric_value(raw, exchange, "Bitget open interest response")?;

        bitget_open_interest_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let period = query.period.clone();
        let raw = self
            .market
            .get_account_long_short_ratio(&symbol, Some(&query.period))
            .await
            .map_err(Error::from_bitget)?;

        owned_metric_items(raw, exchange, "Bitget long-short ratio response")?
            .into_iter()
            .map(|value| {
                bitget_long_short_ratio_from_value(
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
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let period = query.period.clone();
        let raw = self
            .market
            .get_taker_buy_sell_volume(&symbol, Some(&query.period))
            .await
            .map_err(Error::from_bitget)?;

        owned_metric_items(raw, exchange, "Bitget taker buy-sell volume response")?
            .into_iter()
            .map(|value| {
                bitget_taker_volume_from_value(
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
        let accounts = self
            .account
            .get_accounts(&self.product_type)
            .await
            .map_err(Error::from_bitget)?;

        accounts
            .into_iter()
            .map(|account| {
                let raw = serde_json::to_value(&account)?;
                Ok(Balance {
                    exchange: ExchangeId::Bitget,
                    asset: account.margin_coin,
                    total: first_non_empty(&account.account_equity, &account.usdt_equity)
                        .unwrap_or_default(),
                    available: account.available,
                    frozen: non_empty(account.locked),
                    raw,
                })
            })
            .collect()
    }

    pub(crate) async fn set_leverage(
        &self,
        request: SetLeverageRequest,
    ) -> Result<LeverageSetting> {
        let exchange = ExchangeId::Bitget;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let margin_coin = request
            .margin_coin
            .clone()
            .or_else(|| instrument.settlement.clone())
            .unwrap_or_else(|| instrument.quote.clone());
        let raw = self
            .account
            .set_leverage(&symbol, &self.product_type, &margin_coin, &request.leverage)
            .await
            .map_err(Error::from_bitget)?;

        bitget_leverage_setting_from_value(exchange, instrument, symbol, request, raw)
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: true,
            set_position_mode: true,
            set_symbol_margin_mode: true,
            order_level_margin_mode: true,
        }
    }

    pub(crate) async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        let exchange = ExchangeId::Bitget;
        let product_type = request
            .product_type
            .clone()
            .unwrap_or_else(|| self.product_type.clone());
        let raw_mode = bitget_position_mode(request.mode);
        let raw = self
            .account
            .set_position_mode(&product_type, raw_mode)
            .await
            .map_err(Error::from_bitget)?;

        bitget_position_mode_setting_from_value(exchange, request, product_type, raw)
    }

    pub(crate) async fn set_symbol_margin_mode(
        &self,
        request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        let exchange = ExchangeId::Bitget;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let product_type = request
            .product_type
            .clone()
            .unwrap_or_else(|| self.product_type.clone());
        let margin_coin = request
            .margin_coin
            .clone()
            .or_else(|| instrument.settlement.clone())
            .unwrap_or_else(|| instrument.quote.clone());
        let raw_mode = request.mode.as_bitget_margin_mode();
        let raw = self
            .account
            .set_margin_mode(&symbol, &product_type, &margin_coin, &raw_mode)
            .await
            .map_err(Error::from_bitget)?;

        let object = raw.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Bitget symbol margin mode response is not an object".to_string(),
        })?;
        let exchange_symbol = first_string_field(object, &["symbol", "instId"]).unwrap_or(symbol);
        let response_margin_mode =
            first_string_field(object, &["marginMode", "mgnMode"]).or(Some(raw_mode));
        let response_margin_coin =
            first_string_field(object, &["marginCoin", "ccy"]).or(Some(margin_coin));

        Ok(SymbolMarginModeSetting {
            exchange,
            instrument,
            exchange_symbol,
            mode: request.mode,
            raw_mode: response_margin_mode,
            product_type: Some(product_type),
            margin_coin: response_margin_coin,
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
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .account
            .get_all_positions(&self.product_type, Some("USDT"))
            .await
            .map_err(Error::from_bitget)?;
        let values = value_items(&raw, exchange, "Bitget positions response")?;

        let mut output = Vec::new();
        for value in values {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Bitget position item is not an object".to_string(),
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
                side: string_field(object, "holdSide").or_else(|| string_field(object, "posSide")),
                size: first_string_field(object, &["total", "available", "size", "pos"])
                    .unwrap_or_default(),
                entry_price: first_string_field(
                    object,
                    &["openPriceAvg", "avgPrice", "entryPrice"],
                ),
                mark_price: string_field(object, "markPrice"),
                unrealized_pnl: first_string_field(
                    object,
                    &["unrealizedPL", "unrealizedPnl", "unrealizedProfit"],
                ),
                leverage: string_field(object, "leverage"),
                margin_mode: string_field(object, "marginMode"),
                liquidation_price: first_string_field(object, &["liquidationPrice", "liqPx"]),
                raw: value.clone(),
            });
        }

        Ok(output)
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bitget;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let margin_coin = request
            .margin_coin
            .clone()
            .or_else(|| instrument.settlement.clone())
            .unwrap_or_else(|| instrument.quote.clone());
        let margin_mode = bitget_margin_mode(request.margin_mode.as_ref());
        let mut bitget_request = match request.order_type {
            OrderType::Limit => BitgetNewOrderRequest::limit(
                &symbol,
                &self.product_type,
                margin_mode,
                &margin_coin,
                &request.size,
                request.side.lower(),
                required_price(exchange, &request)?,
            ),
            OrderType::Market => BitgetNewOrderRequest::market(
                &symbol,
                &self.product_type,
                margin_mode,
                &margin_coin,
                &request.size,
                request.side.lower(),
            ),
        };

        if let Some(trade_side) = request.trade_side.as_deref() {
            bitget_request = bitget_request.with_trade_side(trade_side);
        }
        if let Some(force) = bitget_force(request.time_in_force) {
            bitget_request = bitget_request.with_force(force);
        }
        if let Some(client_order_id) = request.client_order_id.as_deref() {
            bitget_request = bitget_request.with_client_oid(client_order_id);
        }
        if let Some(reduce_only) = request.reduce_only {
            bitget_request =
                bitget_request.with_reduce_only(if reduce_only { "YES" } else { "NO" });
        }

        let raw = self
            .trade
            .place_order(bitget_request)
            .await
            .map_err(Error::from_bitget)?;

        order_ack_from_value(exchange, instrument, symbol, raw, "Bitget order response")
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bitget;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let margin_coin = request
            .margin_coin
            .clone()
            .or_else(|| instrument.settlement.clone())
            .unwrap_or_else(|| instrument.quote.clone());
        let mut bitget_request = BitgetCancelOrderRequest::new(&symbol, &self.product_type)
            .with_margin_coin(margin_coin);
        if let Some(order_id) = request.order_id.as_deref() {
            bitget_request = bitget_request.with_order_id(order_id);
        } else if let Some(client_order_id) = request.client_order_id.as_deref() {
            bitget_request = bitget_request.with_client_oid(client_order_id);
        } else {
            return Err(missing_cancel_id(exchange));
        }

        let raw = self
            .trade
            .cancel_order(bitget_request)
            .await
            .map_err(Error::from_bitget)?;

        order_ack_from_value(exchange, instrument, symbol, raw, "Bitget cancel response")
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        if query.order_id.is_none() && query.client_order_id.is_none() {
            return Err(missing_order_query_id(exchange));
        }

        let raw = self
            .trade
            .get_order_detail(
                &symbol,
                &self.product_type,
                query.order_id.as_deref(),
                query.client_order_id.as_deref(),
            )
            .await
            .map_err(Error::from_bitget)?;

        bitget_order_from_value(
            exchange,
            Some(instrument),
            Some(symbol),
            raw,
            "Bitget order response",
        )
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument.clone();
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let request = bitget_order_query_request(&self.product_type, &query, symbol.as_deref());
        let raw = self
            .trade
            .get_pending_orders_with(request)
            .await
            .map_err(Error::from_bitget)?;

        bitget_orders_from_value(
            exchange,
            instrument,
            symbol,
            raw,
            "Bitget open orders response",
        )
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument.clone();
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let request = bitget_order_query_request(&self.product_type, &query, symbol.as_deref());
        let raw = self
            .trade
            .get_order_history_with(request)
            .await
            .map_err(Error::from_bitget)?;

        bitget_orders_from_value(
            exchange,
            instrument,
            symbol,
            raw,
            "Bitget order history response",
        )
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        let exchange = ExchangeId::Bitget;
        let instrument = query.instrument.clone();
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let request = bitget_fill_query_request(&self.product_type, &query, symbol.as_deref());
        let raw = self
            .trade
            .get_fills_with(request)
            .await
            .map_err(Error::from_bitget)?;

        bitget_fills_from_value(exchange, instrument, symbol, raw, "Bitget fills response")
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn first_non_empty(primary: &str, fallback: &str) -> Option<String> {
    if !primary.is_empty() {
        Some(primary.to_string())
    } else if !fallback.is_empty() {
        Some(fallback.to_string())
    } else {
        None
    }
}

fn bitget_position_mode(value: PositionMode) -> &'static str {
    match value {
        PositionMode::OneWay => "one_way_mode",
        PositionMode::Hedge => "hedge_mode",
    }
}

fn string_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(non_empty_value)
}

fn first_string_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| string_field(object, field))
}

fn u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(|value| match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn first_u64_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| u64_field(object, field))
}

fn bitget_funding_rate_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<FundingRate> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget funding rate item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(FundingRate {
        exchange,
        instrument,
        exchange_symbol,
        funding_rate: first_string_field(object, &["fundingRate", "rate"]).unwrap_or_default(),
        funding_time: first_u64_field(object, &["fundingTime", "setTime", "time", "ts"]),
        next_funding_rate: first_string_field(object, &["nextFundingRate", "nextRate"]),
        next_funding_time: first_u64_field(object, &["nextFundingTime", "nextUpdate"]),
        mark_price: first_string_field(object, &["markPrice", "markPx"]),
        raw,
    })
}

fn bitget_mark_price_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<MarkPrice> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget mark price item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(MarkPrice {
        exchange,
        instrument,
        exchange_symbol,
        mark_price: first_string_field(object, &["markPrice", "markPx", "price"])
            .unwrap_or_default(),
        index_price: first_string_field(object, &["indexPrice", "indexPx"]),
        funding_rate: first_string_field(object, &["fundingRate", "lastFundingRate"]),
        next_funding_time: first_u64_field(object, &["nextFundingTime", "nextUpdate"]),
        timestamp: first_u64_field(object, &["ts", "time"]),
        raw,
    })
}

fn bitget_open_interest_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<OpenInterest> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget open interest item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(OpenInterest {
        exchange,
        instrument,
        exchange_symbol,
        open_interest: first_string_field(object, &["openInterest", "oi", "size"])
            .unwrap_or_default(),
        open_interest_value: first_string_field(
            object,
            &["openInterestValue", "oiCcy", "amount", "value"],
        ),
        timestamp: first_u64_field(object, &["ts", "time"]),
        raw,
    })
}

fn bitget_long_short_ratio_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    period: String,
    raw: Value,
) -> Result<LongShortRatio> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget long-short ratio item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(LongShortRatio {
        exchange,
        instrument,
        exchange_symbol,
        period,
        ratio: first_string_field(
            object,
            &["longShortRatio", "longShortAccountRatio", "ratio"],
        )
        .unwrap_or_default(),
        long_ratio: first_string_field(object, &["longAccountRatio", "longAccount", "longRatio"]),
        short_ratio: first_string_field(
            object,
            &["shortAccountRatio", "shortAccount", "shortRatio"],
        ),
        timestamp: first_u64_field(object, &["ts", "time", "timestamp"]),
        raw,
    })
}

fn bitget_taker_volume_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    period: String,
    raw: Value,
) -> Result<TakerBuySellVolume> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget taker buy-sell volume item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(TakerBuySellVolume {
        exchange,
        instrument,
        exchange_symbol,
        period,
        buy_volume: first_string_field(object, &["buyVolume", "buyVol"]).unwrap_or_default(),
        sell_volume: first_string_field(object, &["sellVolume", "sellVol"]).unwrap_or_default(),
        buy_sell_ratio: first_string_field(object, &["buySellRatio", "ratio"]),
        timestamp: first_u64_field(object, &["ts", "time", "timestamp"]),
        raw,
    })
}

fn bitget_leverage_setting_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: String,
    request: SetLeverageRequest,
    raw: Value,
) -> Result<LeverageSetting> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget leverage response is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"]).unwrap_or(symbol_hint);

    Ok(LeverageSetting {
        exchange,
        instrument,
        exchange_symbol,
        leverage: first_string_field(object, &["leverage", "lever"]).unwrap_or(request.leverage),
        margin_mode: first_string_field(object, &["marginMode", "mgnMode"])
            .or_else(|| request.margin_mode.map(|mode| mode.as_str().to_string())),
        margin_coin: first_string_field(object, &["marginCoin", "ccy"]).or(request.margin_coin),
        position_side: first_string_field(object, &["holdSide", "posSide", "positionSide"])
            .or(request.position_side),
        raw,
    })
}

fn bitget_position_mode_setting_from_value(
    exchange: ExchangeId,
    request: SetPositionModeRequest,
    product_type: String,
    raw: Value,
) -> Result<PositionModeSetting> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget position mode response is not an object".to_string(),
    })?;

    Ok(PositionModeSetting {
        exchange,
        mode: request.mode,
        raw_mode: first_string_field(object, &["posMode", "positionMode"])
            .or_else(|| Some(bitget_position_mode(request.mode).to_string())),
        product_type: Some(product_type),
        raw,
    })
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

fn owned_metric_items(raw: Value, exchange: ExchangeId, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(object) => {
            for field in [
                "openInterestList",
                "fundingRateList",
                "rateList",
                "priceList",
                "list",
                "items",
            ] {
                if let Some(values) = object.get(field).and_then(Value::as_array) {
                    return Ok(values.clone());
                }
            }
            Ok(vec![Value::Object(object)])
        }
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn first_metric_value(raw: Value, exchange: ExchangeId, label: &str) -> Result<Value> {
    owned_metric_items(raw, exchange, label)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} is empty"),
        })
}

fn owned_order_items(raw: Value, exchange: ExchangeId, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(object) => {
            for field in ["entrustedList", "orderList", "list", "orders"] {
                if let Some(values) = object.get(field).and_then(Value::as_array) {
                    return Ok(values.clone());
                }
            }
            Ok(vec![Value::Object(object)])
        }
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn owned_fill_items(raw: Value, exchange: ExchangeId, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(object) => {
            for field in ["fillList", "fills", "list", "orderList"] {
                if let Some(values) = object.get(field).and_then(Value::as_array) {
                    return Ok(values.clone());
                }
            }
            Ok(vec![Value::Object(object)])
        }
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
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

fn bitget_margin_mode(value: Option<&MarginMode>) -> String {
    value
        .map(MarginMode::as_bitget_margin_mode)
        .unwrap_or_else(|| "crossed".to_string())
}

fn bitget_force(value: Option<TimeInForce>) -> Option<&'static str> {
    match value {
        Some(TimeInForce::Gtc) => Some("gtc"),
        Some(TimeInForce::Ioc) => Some("ioc"),
        Some(TimeInForce::Fok) => Some("fok"),
        Some(TimeInForce::PostOnly) => Some("post_only"),
        None => None,
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
        client_order_id: string_field(object, "clientOid"),
        status: first_string_field(object, &["status", "state"]),
        raw,
    })
}

fn missing_cancel_id(exchange: ExchangeId) -> Error {
    Error::Adapter {
        exchange,
        message: "cancel_order requires order_id or client_order_id".to_string(),
    }
}

fn missing_order_query_id(exchange: ExchangeId) -> Error {
    Error::Adapter {
        exchange,
        message: "order query requires order_id or client_order_id".to_string(),
    }
}

fn bitget_order_query_request(
    product_type: &str,
    query: &OrderListQuery,
    symbol: Option<&str>,
) -> BitgetOrderQueryRequest {
    let mut request = BitgetOrderQueryRequest::new(product_type);
    if let Some(symbol) = symbol {
        request = request.with_symbol(symbol);
    }
    if let Some(status) = query.status.as_deref() {
        request = request.with_status(status);
    }
    if let Some(before) = query.before.as_deref() {
        request = request.with_id_less_than(before);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

fn bitget_fill_query_request(
    product_type: &str,
    query: &FillListQuery,
    symbol: Option<&str>,
) -> BitgetOrderQueryRequest {
    let mut request = BitgetOrderQueryRequest::new(product_type);
    if let Some(symbol) = symbol {
        request = request.with_symbol(symbol);
    }
    if let Some(order_id) = query.order_id.as_deref() {
        request = request.with_order_id(order_id);
    }
    if let Some(before) = query.before.as_deref() {
        request = request.with_id_less_than(before);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

fn bitget_orders_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Order>> {
    owned_order_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            bitget_order_from_value(
                exchange,
                instrument.clone(),
                symbol_hint.clone(),
                value,
                "Bitget order item",
            )
        })
        .collect()
}

fn bitget_order_from_value(
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
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: first_string_field(object, &["orderId", "ordId"]),
        client_order_id: first_string_field(object, &["clientOid", "clientOrderId", "clOrdId"]),
        side: string_field(object, "side"),
        order_type: first_string_field(object, &["orderType", "ordType"]),
        price: string_field(object, "price"),
        size: first_string_field(object, &["size", "sz", "origQty"]),
        filled_size: first_string_field(
            object,
            &["baseVolume", "filledQty", "filledSize", "fillSz"],
        ),
        average_price: first_string_field(object, &["priceAvg", "avgPrice", "averagePrice"]),
        status: first_string_field(object, &["status", "state"]),
        created_at: u64_field(object, "cTime").or_else(|| u64_field(object, "time")),
        updated_at: u64_field(object, "uTime").or_else(|| u64_field(object, "updateTime")),
        raw,
    })
}

fn bitget_orderbook_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<OrderBook> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget orderbook response is not an object".to_string(),
    })?;

    Ok(OrderBook {
        exchange,
        instrument,
        exchange_symbol,
        bids: bitget_book_levels(object.get("bids"), exchange, "bids")?,
        asks: bitget_book_levels(object.get("asks"), exchange, "asks")?,
        timestamp: u64_field(object, "ts").or_else(|| u64_field(object, "time")),
        raw,
    })
}

fn bitget_book_levels(
    value: Option<&Value>,
    exchange: ExchangeId,
    side: &str,
) -> Result<Vec<OrderBookLevel>> {
    let Some(Value::Array(levels)) = value else {
        return Err(Error::Adapter {
            exchange,
            message: format!("Bitget orderbook {side} is not an array"),
        });
    };

    levels
        .iter()
        .map(|level| {
            let values = level.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("Bitget orderbook {side} level is not an array"),
            })?;
            Ok(OrderBookLevel {
                price: value_string_at(values, 0).unwrap_or_default(),
                size: value_string_at(values, 1).unwrap_or_default(),
                raw: level.clone(),
            })
        })
        .collect()
}

fn bitget_candles_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<Vec<Candle>> {
    let Value::Array(items) = raw else {
        return Err(Error::Adapter {
            exchange,
            message: "Bitget candles response is not an array".to_string(),
        });
    };

    items
        .into_iter()
        .map(|item| {
            let values = item.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Bitget candle item is not an array".to_string(),
            })?;
            Ok(Candle {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol: exchange_symbol.clone(),
                open_time: value_u64_at(values, 0),
                close_time: None,
                open: value_string_at(values, 1).unwrap_or_default(),
                high: value_string_at(values, 2).unwrap_or_default(),
                low: value_string_at(values, 3).unwrap_or_default(),
                close: value_string_at(values, 4).unwrap_or_default(),
                volume: value_string_at(values, 5).unwrap_or_default(),
                quote_volume: value_string_at(values, 6),
                closed: None,
                raw: item,
            })
        })
        .collect()
}

fn bitget_fills_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Fill>> {
    owned_fill_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Bitget fill item is not an object".to_string(),
            })?;
            let exchange_symbol = first_string_field(object, &["symbol", "instId"])
                .or_else(|| symbol_hint.clone())
                .unwrap_or_default();
            let mapped_instrument = instrument
                .clone()
                .unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));

            Ok(Fill {
                exchange,
                instrument: mapped_instrument,
                exchange_symbol,
                trade_id: first_string_field(object, &["tradeId", "fillId", "id"]),
                order_id: first_string_field(object, &["orderId", "ordId"]),
                side: string_field(object, "side"),
                price: string_field(object, "price"),
                size: first_string_field(object, &["baseVolume", "fillSz", "size", "qty"]),
                fee: string_field(object, "fee"),
                fee_asset: first_string_field(object, &["feeCcy", "feeCoin", "feeAsset"]),
                role: first_string_field(object, &["role", "execType"]),
                timestamp: u64_field(object, "cTime")
                    .or_else(|| u64_field(object, "ts"))
                    .or_else(|| u64_field(object, "time")),
                raw: value,
            })
        })
        .collect()
}
