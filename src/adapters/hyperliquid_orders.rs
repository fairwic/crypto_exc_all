use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::Fill;
use crate::instrument::Instrument;
use crate::order::Order;
use serde_json::{Map, Value};

pub(crate) fn hyperliquid_order_from_status_response(
    exchange: ExchangeId,
    instrument: Instrument,
    raw: Value,
) -> Result<Order> {
    let object = object_value(&raw, exchange, "Hyperliquid orderStatus response")?;
    let status = string_field(object, "status");
    let order = object.get("order").ok_or_else(|| Error::Adapter {
        exchange,
        message: "Hyperliquid orderStatus response missing order".to_string(),
    })?;
    hyperliquid_order_from_value(exchange, Some(instrument), order.clone(), status)
}

pub(crate) fn hyperliquid_order_from_history_item(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    raw: &Value,
) -> Result<Order> {
    let object = object_value(raw, exchange, "Hyperliquid historicalOrders item")?;
    let status = string_field(object, "status");
    let status_timestamp = u64_field(object, "statusTimestamp");
    let order = object.get("order").ok_or_else(|| Error::Adapter {
        exchange,
        message: "Hyperliquid historicalOrders item missing order".to_string(),
    })?;
    let mut mapped = hyperliquid_order_from_value(exchange, instrument, order.clone(), status)?;
    mapped.updated_at = status_timestamp.or(mapped.updated_at);
    mapped.raw = raw.clone();
    Ok(mapped)
}

pub(crate) fn hyperliquid_order_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    raw: Value,
    status: Option<String>,
) -> Result<Order> {
    let object = object_value(&raw, exchange, "Hyperliquid order item")?;
    let coin = string_field(object, "coin").unwrap_or_default();
    let size = first_string_field(object, &["origSz", "sz"]);
    let resting_size = string_field(object, "sz");

    Ok(Order {
        exchange,
        instrument: instrument.unwrap_or_else(|| Instrument::perp(&coin, "USDC")),
        exchange_symbol: coin,
        order_id: string_field(object, "oid"),
        client_order_id: first_string_field(object, &["cloid", "clientOrderId"]),
        side: string_field(object, "side").and_then(|value| hyperliquid_side(&value)),
        order_type: first_string_field(object, &["orderType", "tif"]),
        price: string_field(object, "limitPx"),
        size: size.clone(),
        filled_size: size
            .as_deref()
            .zip(resting_size.as_deref())
            .and_then(|(original, resting)| decimal_difference(original, resting)),
        average_price: None,
        status,
        created_at: u64_field(object, "timestamp"),
        updated_at: u64_field(object, "statusTimestamp"),
        raw,
    })
}

pub(crate) fn hyperliquid_fill_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    raw: &Value,
) -> Result<Fill> {
    let object = object_value(raw, exchange, "Hyperliquid fill item")?;
    let coin = string_field(object, "coin").unwrap_or_default();

    Ok(Fill {
        exchange,
        instrument: instrument.unwrap_or_else(|| Instrument::perp(&coin, "USDC")),
        exchange_symbol: coin,
        trade_id: string_field(object, "tid"),
        order_id: string_field(object, "oid"),
        side: string_field(object, "side").and_then(|value| hyperliquid_side(&value)),
        price: string_field(object, "px"),
        size: string_field(object, "sz"),
        fee: string_field(object, "fee"),
        fee_asset: string_field(object, "feeToken"),
        role: string_field(object, "dir"),
        timestamp: u64_field(object, "time"),
        raw: raw.clone(),
    })
}

fn hyperliquid_side(value: &str) -> Option<String> {
    match value {
        "B" | "buy" | "Buy" => Some("buy".to_string()),
        "A" | "S" | "sell" | "Sell" => Some("sell".to_string()),
        _ => None,
    }
}

fn decimal_difference(left: &str, right: &str) -> Option<String> {
    let left = left.parse::<f64>().ok()?;
    let right = right.parse::<f64>().ok()?;
    Some(trim_decimal(left - right))
}

fn trim_decimal(value: f64) -> String {
    let value = if value.abs() < f64::EPSILON {
        0.0
    } else {
        value
    };
    let mut text = format!("{value:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
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

fn first_string_field(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| string_field(object, key))
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(value_to_string)
}

fn u64_field(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(|value| match value {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
