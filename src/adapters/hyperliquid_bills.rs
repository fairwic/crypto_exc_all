use crate::account::AccountBill;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde_json::{Map, Value};

pub(crate) fn hyperliquid_bill_from_value(
    exchange: ExchangeId,
    raw: &Value,
) -> Result<AccountBill> {
    let object = object_value(raw, exchange, "Hyperliquid ledger item")?;
    let delta = object.get("delta").and_then(Value::as_object);
    let coin = delta.and_then(|object| string_field(object, "coin"));

    Ok(AccountBill {
        exchange,
        instrument: coin.as_deref().map(|coin| Instrument::perp(coin, "USDC")),
        exchange_symbol: coin,
        bill_id: string_field(object, "hash"),
        asset: Some("USDC".to_string()),
        balance_change: delta.and_then(|object| {
            first_string_field(object, &["usdc", "amount", "usdcDelta", "delta"])
        }),
        balance_after: None,
        fee: None,
        pnl: None,
        bill_type: delta.and_then(|object| string_field(object, "type")),
        bill_sub_type: None,
        order_id: None,
        trade_id: None,
        timestamp: u64_field(object, "time"),
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
