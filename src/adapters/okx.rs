use super::value::{
    map_first_string_field as first_string_field, map_string_field as string_field,
    map_u64_field as u64_field,
};
use crate::account::{
    AccountBill, AccountBillQuery, AccountCapabilities, AccountIdentity, AccountMarginSummary,
    AccountOrderPermission, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageInfoQuery, LeverageSetting, MarginModeApplyMethod, MaxOrderSize, MaxOrderSizeRequest,
    PositionMode, PositionModeSetting, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, SourcedBalance, SymbolMarginModeSetting,
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
use crate::position::{Position, PositionHistory, PositionHistoryQuery, SourcedPosition};
use crate::private_account_stream::PrivateAccountStreamSession;
use crate::trade::{
    AmendProtectiveStopRequest, CancelOrderRequest, OrderAck, PlaceOrderRequest,
    ProtectiveOrderQuery, ProtectiveOrderWorkingType,
};
use okx_rs::api::announcements::announcements_api::OkxAnnouncements;
use okx_rs::api::api_trait::OkxApiTrait;
use okx_rs::config::Credentials as OkxCredentials;
use okx_rs::dto::account_dto::{
    SetLeverageRequest as OkxSetLeverageRequest,
    SetPositionModeRequest as OkxSetPositionModeRequest, TradingSwapNumResponseData,
};
use okx_rs::dto::public_data_dto::{FundingRateHistoryOkxRespDto, FundingRateOkxRespDto};
use okx_rs::dto::trade_dto::{
    AmendAlgoOrderReqDto, OrdListReqDto, OrderDetailRespDto, OrderPendingRespDto, OrderReqDto,
    OrderResDto,
};
use okx_rs::dto::{
    CandleOkxRespDto, EnumToStrTrait, MarginMode as OkxMarginMode, OrderType as OkxRawOrderType,
    TickerOkxResDto,
};
use okx_rs::websocket::OkxPrivateAccountStreamClient;
use okx_rs::{OkxAccount, OkxBigData, OkxClient, OkxMarket, OkxPublicData, OkxTrade};
use serde_json::{Value, json};

#[path = "okx/account.rs"]
mod account;
#[path = "okx/account_permission.rs"]
mod account_permission;
#[path = "okx/margin_summary.rs"]
mod margin_summary;
#[path = "okx/market.rs"]
mod market;
#[path = "okx/order_mapping.rs"]
mod order_mapping;
#[path = "okx/platform.rs"]
mod platform;
#[path = "okx/shared.rs"]
mod shared;
#[cfg(test)]
#[path = "okx/tests.rs"]
mod tests;
#[path = "okx/trade.rs"]
mod trade;
#[path = "okx/trade_request.rs"]
mod trade_request;
use self::shared::{non_empty, parse_u64_string};
use self::trade_request::{attached_exit_orders, order_type};

pub(crate) struct OkxAdapter {
    account: OkxAccount,
    announcements: OkxAnnouncements,
    big_data: OkxBigData,
    market: OkxMarket,
    public_data: OkxPublicData,
    private_stream: Option<OkxPrivateAccountStreamClient>,
    trade: OkxTrade,
}

impl OkxAdapter {
    pub(crate) fn new(config: OkxExchangeConfig) -> Result<Self> {
        let simulated = config.simulated;
        let credentials = OkxCredentials::new(
            config.api_key,
            config.api_secret,
            config.passphrase,
            if simulated { "1" } else { "0" },
        );
        // W3 只承诺生产私有流；demo credential 不能静默发送到生产 WebSocket。
        let private_stream =
            (!simulated).then(|| OkxPrivateAccountStreamClient::new(credentials.clone()));
        let mut client = OkxClient::new(credentials).map_err(Error::from_okx)?;
        client.set_simulated_trading(if simulated { "1" } else { "0" }.to_string());
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }
        if let Some(request_expiration_ms) = config.request_expiration_ms {
            client.set_request_expiration(request_expiration_ms);
        }

        Ok(Self {
            account: <OkxAccount as OkxApiTrait>::new(client.clone()),
            announcements: <OkxAnnouncements as OkxApiTrait>::new(client.clone()),
            big_data: <OkxBigData as OkxApiTrait>::new(client.clone()),
            market: <OkxMarket as OkxApiTrait>::new(client.clone()),
            public_data: <OkxPublicData as OkxApiTrait>::new(client.clone()),
            private_stream,
            trade: <OkxTrade as OkxApiTrait>::new(client),
        })
    }

    pub(crate) async fn open_private_account_stream(&self) -> Result<PrivateAccountStreamSession> {
        let private_stream = self.private_stream.as_ref().ok_or(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "simulated private account stream",
        })?;
        let session = private_stream.connect().await.map_err(Error::from_okx)?;
        Ok(PrivateAccountStreamSession::from_okx(session))
    }
}

fn okx_inst_type_for_instrument(instrument: &Instrument) -> String {
    match instrument.market_type {
        MarketType::Spot => "SPOT",
        MarketType::Margin => "MARGIN",
        MarketType::Perpetual => "SWAP",
        MarketType::Futures => "FUTURES",
        MarketType::Option => "OPTION",
    }
    .to_string()
}

fn okx_ticker_from_dto(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol: Option<String>,
    ticker: TickerOkxResDto,
) -> Result<Ticker> {
    let exchange_symbol = if ticker.inst_id.is_empty() {
        symbol.unwrap_or_default()
    } else {
        ticker.inst_id.clone()
    };
    let instrument = instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));
    let instrument_type = non_empty(ticker.inst_type.clone());
    let is_spot = instrument_type
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("SPOT"))
        .unwrap_or(false);
    let raw = serde_json::to_value(&ticker)?;

    Ok(Ticker {
        exchange,
        instrument,
        instrument_type,
        exchange_symbol,
        last_price: ticker.last,
        last_size: non_empty(ticker.last_sz),
        bid_price: non_empty(ticker.bid_px),
        bid_size: non_empty(ticker.bid_sz),
        ask_price: non_empty(ticker.ask_px),
        ask_size: non_empty(ticker.ask_sz),
        open_24h: non_empty(ticker.open24h),
        high_24h: non_empty(ticker.high24h),
        low_24h: non_empty(ticker.low24h),
        volume_24h: non_empty(ticker.vol24h.clone()),
        base_volume_24h: if is_spot {
            non_empty(ticker.vol24h)
        } else {
            non_empty(ticker.vol_ccy24h.clone())
        },
        quote_volume_24h: if is_spot {
            non_empty(ticker.vol_ccy24h)
        } else {
            None
        },
        sod_utc0: non_empty(ticker.sod_utc0),
        sod_utc8: non_empty(ticker.sod_utc8),
        timestamp: ticker.ts.parse::<u64>().ok(),
        raw,
    })
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

/// 把 OKX `acctLv` 映射为当前官方账户模式；未知等级不做猜测。
fn okx_account_mode(account_level: &str) -> Option<&'static str> {
    match account_level {
        "1" => Some("spot"),
        "2" => Some("futures"),
        "3" => Some("multi_currency_margin"),
        "4" => Some("portfolio_margin"),
        _ => None,
    }
}

/// OKX 主账户的 `mainUid` 等于 `uid`；只有子账户才保留真实父账户标识。
fn okx_parent_account_id(
    provider_account_id: &str,
    parent_account_id: Option<String>,
) -> Option<String> {
    parent_account_id.filter(|value| value != provider_account_id)
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

fn parse_i64_filter(exchange: ExchangeId, field: &str, value: &str) -> Result<i64> {
    value.parse::<i64>().map_err(|_| Error::Adapter {
        exchange,
        message: format!("OKX {field} filter must be numeric: {value}"),
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

fn okx_candle_interval(interval: &str) -> String {
    match interval.trim() {
        "1Dutc" | "1DUTC" => "1Dutc".to_string(),
        value if value.ends_with('h') || value.ends_with('H') => {
            format!("{}H", &value[..value.len() - 1])
        }
        value if value.ends_with('d') || value.ends_with('D') => {
            format!("{}D", &value[..value.len() - 1])
        }
        value => value.to_string(),
    }
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

fn okx_leverage_info_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol: String,
    expected_margin_mode: &str,
    raw: Value,
) -> Result<Vec<LeverageSetting>> {
    let values = raw.as_array().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX leverage info response is not an array".to_owned(),
    })?;
    if values.is_empty() {
        return Err(Error::Adapter {
            exchange,
            message: "OKX leverage info response is empty".to_owned(),
        });
    }
    values
        .iter()
        .map(|item| {
            let object = item.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX leverage info item is not an object".to_owned(),
            })?;
            let exchange_symbol = string_field(object, "instId").ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX leverage info item misses instId".to_owned(),
            })?;
            let margin_mode = string_field(object, "mgnMode").ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX leverage info item misses mgnMode".to_owned(),
            })?;
            let leverage = string_field(object, "lever").ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX leverage info item misses lever".to_owned(),
            })?;
            if exchange_symbol != symbol || margin_mode != expected_margin_mode {
                return Err(Error::Adapter {
                    exchange,
                    message: "OKX leverage info scope mismatch".to_owned(),
                });
            }
            Ok(LeverageSetting {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol,
                leverage,
                margin_mode: Some(margin_mode),
                margin_coin: string_field(object, "ccy"),
                position_side: string_field(object, "posSide"),
                raw: item.clone(),
            })
        })
        .collect()
}

/// 将 OKX max-size 响应转成统一账户模型，保留 maxBuy/maxSell 的原始单位。
fn okx_max_order_size_from_dto(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol_hint: String,
    request: MaxOrderSizeRequest,
    item: TradingSwapNumResponseData,
) -> Result<MaxOrderSize> {
    let raw = serde_json::to_value(&item)?;
    Ok(MaxOrderSize {
        exchange,
        instrument,
        exchange_symbol: if item.inst_id.is_empty() {
            symbol_hint
        } else {
            item.inst_id
        },
        margin_mode: request.margin_mode,
        margin_coin: non_empty(item.ccy).or(request.margin_coin),
        max_buy: item.max_buy,
        max_sell: item.max_sell,
        raw,
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

fn okx_position_mode_from_config(raw: Value, exchange: ExchangeId) -> Result<Option<String>> {
    let values = okx_owned_items(raw, exchange, "OKX account config response")?;
    Ok(values.into_iter().find_map(|value| {
        value
            .as_object()
            .and_then(|object| string_field(object, "posMode"))
            .filter(|mode| !mode.trim().is_empty())
    }))
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

fn okx_position_history_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<PositionHistory> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX position history item is not an object".to_string(),
    })?;
    let exchange_symbol = first_string_field(object, &["instId", "inst_id"])
        .or(symbol_hint)
        .unwrap_or_default();
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));

    Ok(PositionHistory {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        position_id: first_string_field(object, &["posId", "pos_id"]),
        side: first_string_field(object, &["posSide", "pos_side"]),
        direction: string_field(object, "direction"),
        leverage: string_field(object, "lever"),
        margin_mode: first_string_field(object, &["mgnMode", "mgn_mode"]),
        open_avg_price: first_string_field(object, &["openAvgPx", "open_avg_px"]),
        close_avg_price: first_string_field(object, &["closeAvgPx", "close_avg_px"]),
        open_max_position: first_string_field(object, &["openMaxPos", "open_max_pos"]),
        close_total_position: first_string_field(object, &["closeTotalPos", "close_total_pos"]),
        realized_pnl: first_string_field(object, &["realizedPnl", "realized_pnl"]),
        pnl: string_field(object, "pnl"),
        pnl_ratio: first_string_field(object, &["pnlRatio", "pnl_ratio"]),
        fee: string_field(object, "fee"),
        funding_fee: first_string_field(object, &["fundingFee", "funding_fee"]),
        liquidation_penalty: first_string_field(object, &["liqPenalty", "liq_penalty"]),
        close_type: string_field(object, "type"),
        open_time: u64_field(object, "cTime").or_else(|| u64_field(object, "c_time")),
        close_time: u64_field(object, "uTime").or_else(|| u64_field(object, "u_time")),
        raw,
    })
}

fn okx_account_bills_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<Vec<AccountBill>> {
    okx_owned_items(raw, exchange, "OKX account bills response")?
        .into_iter()
        .filter_map(|value| {
            let object = match value.as_object() {
                Some(object) => object,
                None => {
                    return Some(Err(Error::Adapter {
                        exchange,
                        message: "OKX account bill item is not an object".to_string(),
                    }));
                }
            };
            let exchange_symbol = first_string_field(object, &["instId", "inst_id"]);
            if let Some(expected) = symbol_hint.as_deref() {
                match exchange_symbol.as_deref() {
                    Some(actual) if actual.eq_ignore_ascii_case(expected) => {}
                    _ => {
                        return None;
                    }
                }
            }
            let mapped_instrument = exchange_symbol
                .as_deref()
                .map(instrument_from_okx_symbol)
                .or_else(|| instrument.clone());
            Some(Ok(AccountBill {
                exchange,
                instrument: mapped_instrument,
                exchange_symbol,
                bill_id: first_string_field(object, &["billId", "bill_id"]),
                asset: first_string_field(object, &["ccy", "currency", "asset"]),
                balance_change: first_string_field(
                    object,
                    &["balChg", "bal_chg", "balance_change"],
                ),
                balance_after: first_string_field(object, &["bal", "balance", "balance_after"]),
                fee: string_field(object, "fee"),
                pnl: string_field(object, "pnl"),
                bill_type: string_field(object, "type"),
                bill_sub_type: first_string_field(object, &["subType", "sub_type"]),
                order_id: first_string_field(object, &["ordId", "ord_id"]),
                trade_id: first_string_field(object, &["tradeId", "trade_id"]),
                timestamp: u64_field(object, "ts"),
                raw: value,
            }))
        })
        .collect()
}
