use super::value::{
    map_first_string_field as first_string_field, map_string_field as string_field,
    map_u64_field as u64_field,
};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::market::{Candle, OrderBookLevel, Ticker};
use serde_json::{Map, Value};

pub(crate) fn universe_and_contexts<'a>(
    raw: &'a Value,
    exchange: ExchangeId,
) -> Result<(&'a Vec<Value>, &'a Vec<Value>)> {
    universe_and_contexts_with_label(raw, exchange, "Hyperliquid metaAndAssetCtxs")
}

pub(crate) fn universe_and_contexts_with_label<'a>(
    raw: &'a Value,
    exchange: ExchangeId,
    label: &str,
) -> Result<(&'a Vec<Value>, &'a Vec<Value>)> {
    let items = raw.as_array().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} response is not an array"),
    })?;
    let meta = items
        .first()
        .and_then(Value::as_object)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} missing meta"),
        })?;
    let universe = meta
        .get("universe")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} missing universe"),
        })?;
    let contexts = items
        .get(1)
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} missing contexts"),
        })?;
    Ok((universe, contexts))
}

pub(crate) fn asset_context_from_response(
    raw: &Value,
    exchange: ExchangeId,
    symbol: &str,
    label: &str,
) -> Result<(Value, Value)> {
    let (universe, contexts) = universe_and_contexts_with_label(raw, exchange, label)?;
    let index = universe
        .iter()
        .position(|item| {
            item.as_object()
                .and_then(|object| string_field(object, "name"))
                .as_deref()
                == Some(symbol)
        })
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{label} asset not found: {symbol}"),
        })?;
    let meta = universe.get(index).cloned().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} universe missing asset: {symbol}"),
    })?;
    let ctx = contexts.get(index).cloned().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} context missing asset: {symbol}"),
    })?;
    Ok((meta, ctx))
}

pub(crate) fn ticker_from_context(
    exchange: ExchangeId,
    instrument: Instrument,
    instrument_type: &str,
    exchange_symbol: String,
    raw: Value,
) -> Result<Ticker> {
    let object = object_value(&raw, exchange, "Hyperliquid asset context")?;
    let last_price =
        first_string_field(object, &["midPx", "markPx", "oraclePx"]).unwrap_or_default();
    let open_24h = string_field(object, "prevDayPx");
    let volume_24h = string_field(object, "dayNtlVlm");

    Ok(Ticker {
        exchange,
        instrument,
        instrument_type: Some(instrument_type.to_string()),
        exchange_symbol,
        last_price,
        last_size: None,
        bid_price: None,
        bid_size: None,
        ask_price: None,
        ask_size: None,
        open_24h,
        high_24h: None,
        low_24h: None,
        volume_24h: volume_24h.clone(),
        base_volume_24h: None,
        quote_volume_24h: volume_24h,
        sod_utc0: None,
        sod_utc8: None,
        timestamp: None,
        raw,
    })
}

pub(crate) fn orderbook_levels(
    value: Option<&Value>,
    exchange: ExchangeId,
) -> Result<Vec<OrderBookLevel>> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    values
        .iter()
        .map(|item| {
            let object = object_value(item, exchange, "Hyperliquid orderbook level")?;
            Ok(OrderBookLevel {
                price: string_field(object, "px").unwrap_or_default(),
                size: string_field(object, "sz").unwrap_or_default(),
                raw: item.clone(),
            })
        })
        .collect()
}

pub(crate) fn hyperliquid_candle_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    raw: &Value,
) -> Result<Candle> {
    let object = object_value(raw, exchange, "Hyperliquid candle item")?;
    Ok(Candle {
        exchange,
        instrument,
        exchange_symbol: string_field(object, "s").unwrap_or_default(),
        open_time: u64_field(object, "t"),
        close_time: u64_field(object, "T"),
        open: string_field(object, "o").unwrap_or_default(),
        high: string_field(object, "h").unwrap_or_default(),
        low: string_field(object, "l").unwrap_or_default(),
        close: string_field(object, "c").unwrap_or_default(),
        volume: string_field(object, "v").unwrap_or_default(),
        quote_volume: None,
        closed: None,
        raw: raw.clone(),
    })
}

fn object_value<'a>(
    value: &'a Value,
    exchange: ExchangeId,
    message: &'static str,
) -> Result<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{message} is not an object"),
    })
}
