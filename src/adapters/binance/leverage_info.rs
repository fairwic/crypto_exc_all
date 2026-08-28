use crate::account::LeverageSetting;
use crate::adapters::value::map_string_field;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde_json::Value;

pub(super) fn from_symbol_config(
    instrument: Instrument,
    raw: Value,
) -> Result<Vec<LeverageSetting>> {
    let exchange = ExchangeId::Binance;
    let symbol = instrument.symbol_for(exchange);
    let rows = raw.as_array().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance symbolConfig response is not an array".to_owned(),
    })?;
    if rows.len() != 1 {
        return Err(Error::Adapter {
            exchange,
            message: "Binance symbolConfig response must contain exactly one symbol".to_owned(),
        });
    }

    let row = &rows[0];
    let object = row.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance symbolConfig item is not an object".to_owned(),
    })?;
    let exchange_symbol = map_string_field(object, "symbol").ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance symbolConfig response missing symbol".to_owned(),
    })?;
    if exchange_symbol != symbol {
        return Err(Error::Adapter {
            exchange,
            message: "Binance symbolConfig identity mismatch".to_owned(),
        });
    }

    let leverage = map_string_field(object, "leverage")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance symbolConfig response has invalid leverage".to_owned(),
        })?
        .to_string();
    let raw_margin_mode = map_string_field(object, "marginType").ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance symbolConfig response has invalid marginType".to_owned(),
    })?;
    let margin_mode = match raw_margin_mode.to_ascii_uppercase().as_str() {
        "ISOLATED" => "isolated",
        "CROSS" | "CROSSED" => "cross",
        _ => {
            return Err(Error::Adapter {
                exchange,
                message: "Binance symbolConfig response has invalid marginType".to_owned(),
            });
        }
    };

    Ok(["long", "short"]
        .into_iter()
        .map(|position_side| LeverageSetting {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: exchange_symbol.clone(),
            leverage: leverage.clone(),
            margin_mode: Some(margin_mode.to_owned()),
            margin_coin: None,
            position_side: Some(position_side.to_owned()),
            raw: row.clone(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flat_symbol_config_maps_to_hedge_long_and_short_settings() {
        let instrument = Instrument::perp("ETH", "USDT");
        let settings = from_symbol_config(
            instrument.clone(),
            json!([{
                "symbol": "ETHUSDT",
                "marginType": "ISOLATED",
                "leverage": 5
            }]),
        )
        .expect("valid Binance symbol configuration");

        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].position_side.as_deref(), Some("long"));
        assert_eq!(settings[1].position_side.as_deref(), Some("short"));
        assert!(settings.iter().all(|setting| {
            setting.instrument == instrument
                && setting.exchange_symbol == "ETHUSDT"
                && setting.leverage == "5"
                && setting.margin_mode.as_deref() == Some("isolated")
        }));
    }

    #[test]
    fn crossed_symbol_config_preserves_actual_mode() {
        let settings = from_symbol_config(
            Instrument::perp("ETH", "USDT"),
            json!([{
                "symbol": "ETHUSDT",
                "marginType": "CROSSED",
                "leverage": "3"
            }]),
        )
        .expect("valid Binance cross configuration");

        assert!(
            settings
                .iter()
                .all(|setting| setting.margin_mode.as_deref() == Some("cross"))
        );
    }

    #[test]
    fn missing_duplicate_or_mismatched_symbol_config_fails_closed() {
        let instrument = Instrument::perp("ETH", "USDT");
        let valid = json!({
            "symbol": "ETHUSDT",
            "marginType": "ISOLATED",
            "leverage": 5
        });

        assert!(from_symbol_config(instrument.clone(), json!([])).is_err());
        assert!(
            from_symbol_config(instrument.clone(), json!([valid.clone(), valid.clone()])).is_err()
        );
        assert!(
            from_symbol_config(
                instrument,
                json!([{
                    "symbol": "BTCUSDT",
                    "marginType": "ISOLATED",
                    "leverage": 5
                }])
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_leverage_or_margin_mode_fails_closed() {
        let instrument = Instrument::perp("ETH", "USDT");
        for raw in [
            json!([{"symbol": "ETHUSDT", "marginType": "ISOLATED", "leverage": 0}]),
            json!([{"symbol": "ETHUSDT", "marginType": "ISOLATED", "leverage": "2.5"}]),
            json!([{"symbol": "ETHUSDT", "marginType": "UNKNOWN", "leverage": 5}]),
            json!([{"symbol": "ETHUSDT", "leverage": 5}]),
        ] {
            assert!(from_symbol_config(instrument.clone(), raw).is_err());
        }
    }
}
