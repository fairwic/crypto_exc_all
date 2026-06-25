use super::value::{
    map_string_field as string_field, map_u64_field as u64_field, value_as_u64 as value_to_u64,
};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde_json::{Map, Value};

pub(crate) fn spot_instrument_from_symbol(symbol: &str) -> Instrument {
    if let Some((base, quote)) = symbol.split_once('/') {
        Instrument::spot(base, quote)
    } else {
        Instrument::spot(symbol, "USDC")
    }
}

pub(crate) fn spot_instrument_from_universe_item(
    tokens: &[Value],
    item: &Value,
) -> Option<Instrument> {
    let object = item.as_object()?;
    let pair = object.get("tokens").and_then(Value::as_array)?;
    let base_index = pair.first().and_then(value_to_u64)?;
    let quote_index = pair.get(1).and_then(value_to_u64)?;
    let base = spot_token_name(tokens, base_index)?;
    let quote = spot_token_name(tokens, quote_index)?;
    Some(Instrument::spot(base, quote))
}

pub(crate) fn spot_exchange_symbol_from_universe_item(item: &Value) -> Option<String> {
    let object = item.as_object()?;
    if let Some(name) = string_field(object, "name")
        && (name == "PURR/USDC" || name.starts_with('@'))
    {
        return Some(name);
    }
    if let Some(index) = u64_field(object, "index") {
        return Some(format!("@{index}"));
    }
    string_field(object, "name")
}

pub(crate) fn spot_instrument_from_exchange_symbol(
    raw: &Value,
    symbol: &str,
) -> Result<Instrument> {
    let exchange = ExchangeId::Hyperliquid;
    let meta = object_value(raw, exchange, "Hyperliquid spotMeta response")?;
    let tokens = meta
        .get("tokens")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid spotMeta missing tokens".to_string(),
        })?;
    let universe = meta
        .get("universe")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid spotMeta missing universe".to_string(),
        })?;

    for item in universe {
        if spot_exchange_symbol_from_universe_item(item).as_deref() != Some(symbol) {
            continue;
        }
        return spot_instrument_from_universe_item(tokens, item).ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("Hyperliquid spot asset missing tokens for {symbol}"),
        });
    }

    Err(Error::Adapter {
        exchange,
        message: format!("Hyperliquid spot asset not found: {symbol}"),
    })
}

pub(crate) fn spot_market_data_coin_from_meta(
    raw: &Value,
    instrument: &Instrument,
    fallback: &str,
) -> Result<String> {
    let exchange = ExchangeId::Hyperliquid;
    let meta = object_value(raw, exchange, "Hyperliquid spotMeta response")?;
    let tokens = meta
        .get("tokens")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid spotMeta missing tokens".to_string(),
        })?;
    let universe = meta
        .get("universe")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid spotMeta missing universe".to_string(),
        })?;

    for item in universe {
        let object = object_value(item, exchange, "Hyperliquid spot universe item")?;
        let Some(pair) = object.get("tokens").and_then(Value::as_array) else {
            continue;
        };
        let (Some(base_index), Some(quote_index)) = (
            pair.first().and_then(Value::as_u64),
            pair.get(1).and_then(Value::as_u64),
        ) else {
            continue;
        };
        if spot_token_name(tokens, base_index).as_deref() == Some(instrument.base.as_str())
            && spot_token_name(tokens, quote_index).as_deref() == Some(instrument.quote.as_str())
        {
            return spot_exchange_symbol_from_universe_item(item).ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("Hyperliquid spot asset missing exchange symbol: {fallback}"),
            });
        }
    }

    Err(Error::Adapter {
        exchange,
        message: format!("Hyperliquid spot asset not found: {fallback}"),
    })
}

fn spot_token_name(tokens: &[Value], index: u64) -> Option<String> {
    tokens.iter().find_map(|item| {
        let object = item.as_object()?;
        (u64_field(object, "index") == Some(index)).then(|| string_field(object, "name"))?
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
