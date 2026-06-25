use super::value::{
    map_first_string_field as first_string_field, map_string_field as string_field,
    map_u64_field as u64_field,
};
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
