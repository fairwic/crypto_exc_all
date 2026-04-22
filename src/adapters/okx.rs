use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, MarginModeApplyMethod, PositionMode, PositionModeSetting, SetLeverageRequest,
    SetPositionModeRequest, SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::OkxExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::{Instrument, MarketType};
use crate::margin::MarginMode;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use crate::trade::{CancelOrderRequest, OrderAck, OrderType, PlaceOrderRequest, TimeInForce};
use okx_rs::api::api_trait::OkxApiTrait;
use okx_rs::config::Credentials as OkxCredentials;
use okx_rs::dto::account_dto::{
    SetLeverageRequest as OkxSetLeverageRequest,
    SetPositionModeRequest as OkxSetPositionModeRequest,
};
use okx_rs::dto::public_data_dto::{FundingRateHistoryOkxRespDto, FundingRateOkxRespDto};
use okx_rs::dto::trade_dto::{OrdListReqDto, OrderDetailRespDto, OrderPendingRespDto, OrderReqDto};
use okx_rs::dto::{
    CandleOkxRespDto, EnumToStrTrait, MarginMode as OkxMarginMode, OrderType as OkxRawOrderType,
};
use okx_rs::{OkxAccount, OkxBigData, OkxClient, OkxMarket, OkxPublicData, OkxTrade};
use serde_json::Value;

pub(crate) struct OkxAdapter {
    account: OkxAccount,
    big_data: OkxBigData,
    market: OkxMarket,
    public_data: OkxPublicData,
    trade: OkxTrade,
}

impl OkxAdapter {
    pub(crate) fn new(config: OkxExchangeConfig) -> Result<Self> {
        let credentials = OkxCredentials::new(
            config.api_key,
            config.api_secret,
            config.passphrase,
            if config.simulated { "1" } else { "0" },
        );
        let mut client = OkxClient::new(credentials).map_err(Error::from_okx)?;
        client.set_simulated_trading(if config.simulated { "1" } else { "0" }.to_string());
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }
        if let Some(request_expiration_ms) = config.request_expiration_ms {
            client.set_request_expiration(request_expiration_ms);
        }

        Ok(Self {
            account: <OkxAccount as OkxApiTrait>::new(client.clone()),
            big_data: <OkxBigData as OkxApiTrait>::new(client.clone()),
            market: <OkxMarket as OkxApiTrait>::new(client.clone()),
            public_data: <OkxPublicData as OkxApiTrait>::new(client.clone()),
            trade: <OkxTrade as OkxApiTrait>::new(client),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let mut tickers = self
            .market
            .get_ticker(&symbol)
            .await
            .map_err(Error::from_okx)?;
        let ticker = tickers.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX ticker response is empty for {symbol}"),
        })?;
        let raw = serde_json::to_value(&ticker)?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: ticker.last,
            bid_price: non_empty(ticker.bid_px),
            ask_price: non_empty(ticker.ask_px),
            volume_24h: non_empty(ticker.vol24h),
            timestamp: ticker.ts.parse::<u64>().ok(),
            raw,
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let depth = self
            .market
            .get_books(&symbol, query.limit)
            .await
            .map_err(Error::from_okx)?;
        let raw = serde_json::to_value(&depth)?;

        Ok(OrderBook {
            exchange,
            instrument,
            exchange_symbol: if depth.inst_id.is_empty() {
                symbol
            } else {
                depth.inst_id
            },
            bids: okx_book_levels(depth.bids),
            asks: okx_book_levels(depth.asks),
            timestamp: parse_u64_string(&depth.ts),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let candles = self
            .market
            .get_candles(
                &symbol,
                &query.interval,
                query.after.as_deref(),
                query.before.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        candles
            .into_iter()
            .map(|candle| okx_candle_from_dto(exchange, &instrument, &symbol, candle))
            .collect()
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let mut response = self
            .public_data
            .get_funding_rate(&symbol)
            .await
            .map_err(Error::from_okx)?;
        let item = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX funding rate response is empty for {symbol}"),
        })?;

        okx_funding_rate_from_dto(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let before = query
            .before
            .as_deref()
            .map(|value| parse_i64_filter(exchange, "before", value))
            .transpose()?;
        let after = query
            .after
            .as_deref()
            .map(|value| parse_i64_filter(exchange, "after", value))
            .transpose()?;
        let limit = query.limit.map(i64::from);
        let response = self
            .public_data
            .get_funding_rate_history(&symbol, before, after, limit)
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|item| {
                okx_funding_rate_from_history_dto(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    item,
                )
            })
            .collect()
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .public_data
            .get_mark_price("SWAP", Some(&symbol), None, None)
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX mark price response")?;

        okx_mark_price_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .public_data
            .get_open_interest("SWAP", Some(&symbol), None, None)
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX open interest response")?;

        okx_open_interest_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = self
            .big_data
            .get_long_short_account_ratio_contract_top_trader(
                &symbol,
                Some(&query.period),
                begin.as_deref(),
                end.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        raw.into_iter()
            .map(|values| {
                okx_long_short_ratio_from_values(
                    exchange,
                    instrument.clone(),
                    symbol.clone(),
                    query.period.clone(),
                    values,
                )
            })
            .collect()
    }

    pub(crate) async fn taker_buy_sell_volume(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<TakerBuySellVolume>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = self
            .big_data
            .get_taker_volume_contract(
                &symbol,
                Some(&query.period),
                None,
                begin.as_deref(),
                end.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        raw.into_iter()
            .map(|values| {
                okx_taker_volume_from_values(
                    exchange,
                    instrument.clone(),
                    symbol.clone(),
                    query.period.clone(),
                    values,
                )
            })
            .collect()
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let accounts = self
            .account
            .get_balance(None)
            .await
            .map_err(Error::from_okx)?;
        let mut output = Vec::new();

        for account in accounts {
            for detail in account.details {
                let raw = serde_json::to_value(&detail)?;
                output.push(Balance {
                    exchange: ExchangeId::Okx,
                    asset: detail.ccy,
                    total: detail.eq,
                    available: if detail.avail_bal.is_empty() {
                        detail.avail_eq
                    } else {
                        detail.avail_bal
                    },
                    frozen: non_empty(detail.frozen_bal),
                    raw,
                });
            }
        }

        Ok(output)
    }

    pub(crate) async fn set_leverage(
        &self,
        request: SetLeverageRequest,
    ) -> Result<LeverageSetting> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let ccy = if matches!(
            &instrument.market_type,
            MarketType::Spot | MarketType::Margin
        ) {
            request.margin_coin.clone()
        } else {
            None
        };
        let raw = self
            .account
            .set_leverage(OkxSetLeverageRequest {
                inst_id: Some(symbol.clone()),
                ccy,
                lever: request.leverage.clone(),
                mgn_mode: okx_margin_mode(request.margin_mode.as_ref()),
                pos_side: request
                    .position_side
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase()),
            })
            .await
            .map_err(Error::from_okx)?;

        okx_leverage_setting_from_value(exchange, instrument, symbol, request, raw)
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: true,
            set_position_mode: true,
            set_symbol_margin_mode: false,
            order_level_margin_mode: true,
        }
    }

    pub(crate) async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        let exchange = ExchangeId::Okx;
        let raw_mode = okx_position_mode(request.mode);
        let raw = self
            .account
            .set_position_mode(OkxSetPositionModeRequest {
                pos_mode: raw_mode.to_string(),
            })
            .await
            .map_err(Error::from_okx)?;

        okx_position_mode_setting_from_value(exchange, request, raw)
    }

    pub(crate) async fn set_symbol_margin_mode(
        &self,
        _request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        Err(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "set_symbol_margin_mode",
        })
    }

    pub(crate) async fn ensure_order_margin_mode(
        &self,
        request: EnsureOrderMarginModeRequest,
    ) -> Result<EnsureOrderMarginModeResult> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument;
        let exchange_symbol = instrument.symbol_for(exchange);
        let raw_mode = request.mode.as_okx_td_mode();

        Ok(EnsureOrderMarginModeResult {
            exchange,
            instrument,
            exchange_symbol,
            mode: request.mode,
            apply_method: MarginModeApplyMethod::OrderLevel,
            raw_mode: Some(raw_mode),
            product_type: request.product_type,
            margin_coin: request.margin_coin,
            raw: Value::Null,
        })
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let positions = self
            .account
            .get_positions(Some("SWAP"), symbol.as_deref(), None)
            .await
            .map_err(Error::from_okx)?;

        positions
            .into_iter()
            .map(|position| {
                let raw = serde_json::to_value(&position)?;
                let mapped_instrument = instrument
                    .cloned()
                    .unwrap_or_else(|| instrument_from_okx_symbol(&position.inst_id));
                Ok(Position {
                    exchange,
                    instrument: mapped_instrument,
                    exchange_symbol: position.inst_id,
                    side: Some(position.position_side.as_str().to_string()),
                    size: position.pos,
                    entry_price: non_empty(position.average_price),
                    mark_price: None,
                    unrealized_pnl: non_empty(position.upl),
                    leverage: non_empty(position.leverage),
                    margin_mode: Some(okx_margin_mode_from_enum(position.margin_mode).to_string()),
                    liquidation_price: position.liquidation_price.and_then(non_empty),
                    raw,
                })
            })
            .collect()
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let ord_type = okx_order_type(&request);
        let px = if ord_type == "market" {
            request.price.clone()
        } else {
            Some(required_price(exchange, &request)?.to_string())
        };
        let order = OrderReqDto {
            inst_id: symbol.clone(),
            td_mode: okx_margin_mode(request.margin_mode.as_ref()),
            ccy: request.margin_coin.clone(),
            cl_ord_id: request.client_order_id.clone(),
            tag: None,
            side: request.side.lower().to_string(),
            pos_side: request
                .position_side
                .as_deref()
                .map(|value| value.to_ascii_lowercase()),
            ord_type: ord_type.to_string(),
            sz: request.size.clone(),
            px,
            px_usd: None,
            px_vol: None,
            reduce_only: request.reduce_only,
            tgt_ccy: None,
            ban_amend: None,
            quick_mgn_type: None,
            stp_id: None,
            stp_mode: None,
            attach_algo_ords: None,
        };
        let mut response = self
            .trade
            .place_order(order)
            .await
            .map_err(Error::from_okx)?;
        let order = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX order response is empty".to_string(),
        })?;
        let raw = serde_json::to_value(&order)?;

        Ok(OrderAck {
            exchange,
            instrument,
            exchange_symbol: symbol,
            order_id: non_empty(order.ord_id),
            client_order_id: order.cl_ord_id.and_then(non_empty),
            status: non_empty(order.s_code),
            raw,
        })
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        if request.order_id.is_none() && request.client_order_id.is_none() {
            return Err(missing_cancel_id(exchange));
        }

        let raw = self
            .trade
            .cancel_order(
                &symbol,
                request.order_id.as_deref(),
                request.client_order_id.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX cancel response")?;
        let object = item.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX cancel response item is not an object".to_string(),
        })?;

        Ok(OrderAck {
            exchange,
            instrument,
            exchange_symbol: symbol,
            order_id: string_field(object, "ordId"),
            client_order_id: string_field(object, "clOrdId"),
            status: string_field(object, "sCode"),
            raw: item,
        })
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        if query.order_id.is_none() && query.client_order_id.is_none() {
            return Err(missing_order_query_id(exchange));
        }

        let mut response = self
            .trade
            .get_order_details(
                &symbol,
                query.order_id.as_deref(),
                query.client_order_id.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;
        let order = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX order response is empty for {symbol}"),
        })?;

        okx_order_from_detail(exchange, Some(instrument), Some(symbol), order)
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let response = self
            .trade
            .get_pending_orders(
                Some("SWAP"),
                symbol.as_deref(),
                None,
                query.status.as_deref(),
                query.after.as_deref(),
                query.before.as_deref(),
                query.limit,
            )
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|order| {
                okx_order_from_pending(exchange, instrument.clone(), symbol.clone(), order)
            })
            .collect()
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let response = self
            .trade
            .get_order_history(OrdListReqDto {
                inst_type: "SWAP".to_string(),
                inst_id: symbol.clone(),
                ord_type: None,
                state: query.status,
                after: query.after,
                before: query.before,
                limit: query.limit,
            })
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|order| okx_order_from_detail(exchange, instrument.clone(), symbol.clone(), order))
            .collect()
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .trade
            .get_fills(
                Some("SWAP"),
                symbol.as_deref(),
                query.order_id.as_deref(),
                query.after.as_deref(),
                query.before.as_deref(),
                query.limit,
            )
            .await
            .map_err(Error::from_okx)?;

        okx_fills_from_value(exchange, instrument, symbol, raw, "OKX fills response")
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn instrument_from_okx_symbol(symbol: &str) -> Instrument {
    let mut parts = symbol.split('-');
    let base = parts.next().unwrap_or(symbol);
    let quote = parts.next().unwrap_or("USDT");
    Instrument::perp(base, quote)
}

fn required_price(exchange: ExchangeId, request: &PlaceOrderRequest) -> Result<&str> {
    request.price.as_deref().ok_or_else(|| Error::Adapter {
        exchange,
        message: "limit orders require price".to_string(),
    })
}

fn okx_margin_mode(value: Option<&MarginMode>) -> String {
    value
        .map(MarginMode::as_okx_td_mode)
        .unwrap_or_else(|| "cross".to_string())
}

fn okx_margin_mode_from_enum(value: OkxMarginMode) -> &'static str {
    match value {
        OkxMarginMode::Cross => "cross",
        OkxMarginMode::Isolated => "isolated",
    }
}

fn okx_position_mode(value: PositionMode) -> &'static str {
    match value {
        PositionMode::OneWay => "net_mode",
        PositionMode::Hedge => "long_short_mode",
    }
}

fn okx_order_type(request: &PlaceOrderRequest) -> &'static str {
    match (request.order_type, request.time_in_force) {
        (_, Some(TimeInForce::PostOnly)) => "post_only",
        (_, Some(TimeInForce::Ioc)) => "ioc",
        (_, Some(TimeInForce::Fok)) => "fok",
        (OrderType::Limit, _) => "limit",
        (OrderType::Market, _) => "market",
    }
}

fn first_object_value(raw: Value, exchange: ExchangeId, label: &str) -> Result<Value> {
    match raw {
        Value::Array(values) => values.into_iter().next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} is empty"),
        }),
        Value::Object(_) => Ok(raw),
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
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

fn parse_u64_string(value: &str) -> Option<u64> {
    if value.is_empty() {
        None
    } else {
        value.parse::<u64>().ok()
    }
}

fn parse_i64_filter(exchange: ExchangeId, field: &str, value: &str) -> Result<i64> {
    value.parse::<i64>().map_err(|_| Error::Adapter {
        exchange,
        message: format!("OKX {field} filter must be numeric: {value}"),
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

fn okx_order_type_from_enum(value: OkxRawOrderType) -> &'static str {
    match value {
        OkxRawOrderType::Market => "market",
        OkxRawOrderType::Limit => "limit",
        OkxRawOrderType::PostOnly => "post_only",
        OkxRawOrderType::FillOrKill => "fok",
        OkxRawOrderType::ImmediateOrCancel => "ioc",
        OkxRawOrderType::OptimalLimitIoc => "optimal_limit_ioc",
    }
}

fn okx_order_from_detail(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    order: OrderDetailRespDto,
) -> Result<Order> {
    let raw = serde_json::to_value(&order)?;
    let exchange_symbol = if order.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        order.inst_id.clone()
    };
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: non_empty(order.ord_id),
        client_order_id: non_empty(order.cl_ord_id),
        side: non_empty(order.side),
        order_type: non_empty(order.ord_type),
        price: non_empty(order.px),
        size: non_empty(order.sz),
        filled_size: non_empty(order.acc_fill_sz),
        average_price: non_empty(order.avg_px),
        status: non_empty(order.state),
        created_at: parse_u64_string(&order.c_time),
        updated_at: parse_u64_string(&order.u_time),
        raw,
    })
}

fn okx_order_from_pending(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    order: OrderPendingRespDto,
) -> Result<Order> {
    let raw = serde_json::to_value(&order)?;
    let exchange_symbol = if order.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        order.inst_id.clone()
    };
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: non_empty(order.order_id),
        client_order_id: order.client_order_id.and_then(non_empty),
        side: Some(order.side.as_str().to_string()),
        order_type: Some(okx_order_type_from_enum(order.order_type).to_string()),
        price: non_empty(order.px),
        size: non_empty(order.sz),
        filled_size: order.filled_size.and_then(non_empty),
        average_price: order.filled_price.and_then(non_empty),
        status: non_empty(order.state),
        created_at: parse_u64_string(&order.creation_time),
        updated_at: order.update_time.as_deref().and_then(parse_u64_string),
        raw,
    })
}

fn okx_book_levels(levels: Vec<Vec<String>>) -> Vec<OrderBookLevel> {
    levels
        .into_iter()
        .map(|level| OrderBookLevel {
            price: level.first().cloned().unwrap_or_default(),
            size: level.get(1).cloned().unwrap_or_default(),
            raw: Value::Array(level.into_iter().map(Value::String).collect()),
        })
        .collect()
}

fn okx_candle_from_dto(
    exchange: ExchangeId,
    instrument: &Instrument,
    exchange_symbol: &str,
    candle: CandleOkxRespDto,
) -> Result<Candle> {
    let raw = serde_json::to_value(&candle)?;
    Ok(Candle {
        exchange,
        instrument: instrument.clone(),
        exchange_symbol: exchange_symbol.to_string(),
        open_time: parse_u64_string(&candle.ts),
        close_time: None,
        open: candle.o,
        high: candle.h,
        low: candle.l,
        close: candle.c,
        volume: candle.v,
        quote_volume: non_empty(candle.vol_ccy_quote),
        closed: match candle.confirm.as_str() {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        },
        raw,
    })
}

fn okx_funding_rate_from_dto(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    item: FundingRateOkxRespDto,
) -> Result<FundingRate> {
    let raw = serde_json::to_value(&item)?;
    let exchange_symbol = if item.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        item.inst_id
    };

    Ok(FundingRate {
        exchange,
        instrument,
        exchange_symbol,
        funding_rate: item.funding_rate,
        funding_time: parse_u64_string(&item.funding_time),
        next_funding_rate: non_empty(item.next_funding_rate),
        next_funding_time: parse_u64_string(&item.next_funding_time),
        mark_price: None,
        raw,
    })
}

fn okx_funding_rate_from_history_dto(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    item: FundingRateHistoryOkxRespDto,
) -> Result<FundingRate> {
    let raw = serde_json::to_value(&item)?;
    let exchange_symbol = if item.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        item.inst_id
    };

    Ok(FundingRate {
        exchange,
        instrument,
        exchange_symbol,
        funding_rate: item.funding_rate,
        funding_time: parse_u64_string(&item.funding_time),
        next_funding_rate: None,
        next_funding_time: None,
        mark_price: None,
        raw,
    })
}

fn okx_mark_price_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<MarkPrice> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX mark price item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["instId", "inst_id"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(MarkPrice {
        exchange,
        instrument,
        exchange_symbol,
        mark_price: first_string_field(object, &["markPx", "markPrice"]).unwrap_or_default(),
        index_price: first_string_field(object, &["idxPx", "indexPrice", "indexPx"]),
        funding_rate: first_string_field(object, &["fundingRate", "lastFundingRate"]),
        next_funding_time: u64_field(object, "nextFundingTime"),
        timestamp: u64_field(object, "ts"),
        raw,
    })
}

fn okx_open_interest_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<OpenInterest> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX open interest item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["instId", "inst_id"])
        .or(symbol_hint)
        .unwrap_or_default();

    Ok(OpenInterest {
        exchange,
        instrument,
        exchange_symbol,
        open_interest: first_string_field(object, &["oi", "openInterest"]).unwrap_or_default(),
        open_interest_value: first_string_field(object, &["oiCcy", "oiUsd", "openInterestValue"]),
        timestamp: u64_field(object, "ts"),
        raw,
    })
}

fn okx_string_at(values: &[String], index: usize) -> Option<String> {
    values.get(index).and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.clone())
        }
    })
}

fn okx_u64_at(values: &[String], index: usize) -> Option<u64> {
    values
        .get(index)
        .and_then(|value| value.parse::<u64>().ok())
}

fn okx_long_short_ratio_from_values(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    period: String,
    values: Vec<String>,
) -> Result<LongShortRatio> {
    let raw = Value::Array(values.iter().cloned().map(Value::String).collect());

    Ok(LongShortRatio {
        exchange,
        instrument,
        exchange_symbol,
        period,
        ratio: okx_string_at(&values, 1).unwrap_or_default(),
        long_ratio: None,
        short_ratio: None,
        timestamp: okx_u64_at(&values, 0),
        raw,
    })
}

fn okx_taker_volume_from_values(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    period: String,
    values: Vec<String>,
) -> Result<TakerBuySellVolume> {
    let raw = Value::Array(values.iter().cloned().map(Value::String).collect());

    Ok(TakerBuySellVolume {
        exchange,
        instrument,
        exchange_symbol,
        period,
        buy_volume: okx_string_at(&values, 2).unwrap_or_default(),
        sell_volume: okx_string_at(&values, 1).unwrap_or_default(),
        buy_sell_ratio: None,
        timestamp: okx_u64_at(&values, 0),
        raw,
    })
}

fn okx_leverage_setting_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: String,
    request: SetLeverageRequest,
    raw: Value,
) -> Result<LeverageSetting> {
    let item = first_object_value(raw, exchange, "OKX leverage response")?;
    let object = item.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX leverage response item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "instId").unwrap_or(symbol_hint);

    Ok(LeverageSetting {
        exchange,
        instrument,
        exchange_symbol,
        leverage: string_field(object, "lever").unwrap_or(request.leverage),
        margin_mode: string_field(object, "mgnMode")
            .or_else(|| request.margin_mode.map(|mode| mode.as_str().to_string())),
        margin_coin: string_field(object, "ccy").or(request.margin_coin),
        position_side: string_field(object, "posSide").or(request.position_side),
        raw: item,
    })
}

fn okx_position_mode_setting_from_value(
    exchange: ExchangeId,
    request: SetPositionModeRequest,
    raw: Value,
) -> Result<PositionModeSetting> {
    let item = first_object_value(raw, exchange, "OKX position mode response")?;
    let object = item.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX position mode response item is not an object".to_string(),
    })?;

    Ok(PositionModeSetting {
        exchange,
        mode: request.mode,
        raw_mode: string_field(object, "posMode")
            .or_else(|| Some(okx_position_mode(request.mode).to_string())),
        product_type: request.product_type,
        raw: item,
    })
}

fn okx_owned_items(raw: Value, exchange: ExchangeId, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(_) => Ok(vec![raw]),
        _ => Err(Error::Adapter {
            exchange,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn okx_fill_role(value: Option<String>) -> Option<String> {
    value.map(|value| match value.to_ascii_uppercase().as_str() {
        "M" | "MAKER" => "maker".to_string(),
        "T" | "TAKER" => "taker".to_string(),
        other => other.to_ascii_lowercase(),
    })
}

fn okx_fills_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Fill>> {
    okx_owned_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX fill item is not an object".to_string(),
            })?;
            let exchange_symbol = first_string_field(object, &["instId", "inst_id"])
                .or_else(|| symbol_hint.clone())
                .unwrap_or_default();
            let mapped_instrument = instrument
                .clone()
                .unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));
            Ok(Fill {
                exchange,
                instrument: mapped_instrument,
                exchange_symbol,
                trade_id: first_string_field(object, &["tradeId", "trade_id"]),
                order_id: first_string_field(object, &["ordId", "ord_id"]),
                side: string_field(object, "side"),
                price: first_string_field(object, &["fillPx", "fill_px"]),
                size: first_string_field(object, &["fillSz", "fill_sz"]),
                fee: string_field(object, "fee"),
                fee_asset: first_string_field(object, &["feeCcy", "fee_ccy"]),
                role: okx_fill_role(first_string_field(object, &["execType", "exec_type"])),
                timestamp: u64_field(object, "ts").or_else(|| u64_field(object, "fillTime")),
                raw: value,
            })
        })
        .collect()
}
