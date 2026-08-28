use super::BinanceAdapter;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::order::Order;
use crate::trade::{OrderSide, ProtectiveOrderRequest};
use binance_rs::api::trade::AlgoOrderRequest as BinanceAlgoOrderRequest;
use serde_json::{Map, Value};

impl BinanceAdapter {
    pub(crate) async fn open_protective_orders(&self) -> Result<Vec<Order>> {
        let raw = self
            .trade
            .get_open_algo_orders(None)
            .await
            .map_err(Error::from_binance)?;
        binance_open_algo_orders_from_value(raw)
    }
}

pub(super) fn binance_protective_order_request(
    request: &ProtectiveOrderRequest,
) -> Result<BinanceAlgoOrderRequest> {
    let fixed_size = request.quantity.is_some();
    let close_all = request.close_position == Some(true);
    let position_side = request.position_side.as_deref();
    let hedge_side = match position_side {
        Some(value) if value.eq_ignore_ascii_case("LONG") => Some("LONG"),
        Some(value) if value.eq_ignore_ascii_case("SHORT") => Some("SHORT"),
        Some(value) if value.eq_ignore_ascii_case("BOTH") => None,
        Some(_) => {
            return Err(Error::Adapter {
                exchange: ExchangeId::Binance,
                message:
                    "Binance STOP_MARKET protective order positionSide must be BOTH, LONG, or SHORT"
                        .to_string(),
            });
        }
        None => None,
    };
    if fixed_size && close_all {
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: "Binance STOP_MARKET protective order quantity and closePosition=true are mutually exclusive"
                .to_string(),
        });
    }
    if !fixed_size && !close_all {
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: "Binance STOP_MARKET protective order requires exactly one of quantity or closePosition=true"
                .to_string(),
        });
    }
    if close_all && request.reduce_only.is_some() {
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: "Binance close-all STOP_MARKET protective order rejects the reduceOnly field"
                .to_string(),
        });
    }
    if hedge_side.is_some() && request.reduce_only.is_some() {
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: "Binance Hedge Mode STOP_MARKET protective order rejects the reduceOnly field"
                .to_string(),
        });
    }
    if matches!(
        (request.side, hedge_side),
        (OrderSide::Buy, Some("LONG")) | (OrderSide::Sell, Some("SHORT"))
    ) {
        let message = if close_all {
            "Binance Hedge Mode close-all STOP_MARKET side cannot close the selected positionSide"
        } else {
            "Binance Hedge Mode fixed-size STOP_MARKET side cannot close the selected positionSide"
        };
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: message.to_string(),
        });
    }
    if fixed_size && hedge_side.is_none() && request.reduce_only != Some(true) {
        return Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: "Binance One-way Mode fixed-size STOP_MARKET protective order requires reduceOnly=true"
                .to_string(),
        });
    }

    let symbol = request.instrument.symbol_for(ExchangeId::Binance);
    let mut binance_request =
        BinanceAlgoOrderRequest::stop_market(symbol, request.side.upper(), &request.stop_price);

    if let Some(position_side) = request.position_side.as_deref() {
        binance_request = binance_request.with_position_side(position_side.to_ascii_uppercase());
    }
    if let Some(quantity) = request.quantity.as_deref() {
        binance_request = binance_request.with_quantity(quantity);
    }
    if let Some(reduce_only) = request.reduce_only {
        binance_request = binance_request.with_reduce_only(reduce_only);
    }
    if let Some(close_position) = request.close_position {
        binance_request = binance_request.with_close_position(close_position);
    }
    if let Some(working_type) = request.working_type {
        binance_request = binance_request.with_working_type(working_type.binance_value());
    }
    if let Some(price_protect) = request.price_protect {
        binance_request = binance_request.with_price_protect(price_protect);
    }
    if let Some(client_order_id) = request.client_order_id.as_deref() {
        binance_request = binance_request.with_client_algo_id(client_order_id);
    }

    Ok(binance_request)
}

pub(super) fn binance_open_algo_orders_from_value(raw: Value) -> Result<Vec<Order>> {
    let values = match raw {
        Value::Array(values) => values,
        _ => {
            return Err(adapter_error(
                "Binance open algo orders response is not an array",
            ));
        }
    };
    values
        .into_iter()
        .map(binance_open_algo_order_from_value)
        .collect()
}

fn binance_open_algo_order_from_value(raw: Value) -> Result<Order> {
    let object = raw
        .as_object()
        .ok_or_else(|| adapter_error("Binance open algo order item is not an object"))?;
    let algo_id = required_u64(object, "algoId")?;
    let client_algo_id = required_string(object, "clientAlgoId")?;
    let status = required_string(object, "algoStatus")?;
    let order_type = required_string(object, "orderType")?;
    let symbol = required_string(object, "symbol")?;
    let side = required_string(object, "side")?;
    required_string(object, "positionSide")?;
    let trigger_price = required_string(object, "triggerPrice")?;
    let quantity = required_string(object, "quantity")?;
    required_bool(object, "closePosition")?;
    let created_at = required_u64(object, "createTime")?;
    let updated_at = required_u64(object, "updateTime")?;

    Ok(Order {
        exchange: ExchangeId::Binance,
        instrument: instrument_from_linear_symbol(&symbol),
        exchange_symbol: symbol,
        order_id: Some(algo_id.to_string()),
        client_order_id: Some(client_algo_id),
        side: Some(side),
        order_type: Some(order_type),
        price: Some(trigger_price),
        size: Some(quantity),
        filled_size: None,
        average_price: None,
        status: Some(status),
        created_at: Some(created_at),
        updated_at: Some(updated_at),
        raw,
    })
}

fn required_string(object: &Map<String, Value>, field: &'static str) -> Result<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| malformed_field(field))
}

fn required_u64(object: &Map<String, Value>, field: &'static str) -> Result<u64> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| malformed_field(field))
}

fn required_bool(object: &Map<String, Value>, field: &'static str) -> Result<bool> {
    object
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| malformed_field(field))
}

fn malformed_field(field: &'static str) -> Error {
    adapter_error(&format!(
        "Binance open algo order field `{field}` is missing or malformed"
    ))
}

fn adapter_error(message: &str) -> Error {
    Error::Adapter {
        exchange: ExchangeId::Binance,
        message: message.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrument::Instrument;
    use crate::trade::ProtectiveOrderWorkingType;

    #[test]
    fn maps_protective_stop_market_request_to_binance_algo_order_params() {
        let request = ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("LONG")
        .with_close_position(true)
        .with_working_type(ProtectiveOrderWorkingType::MarkPrice)
        .with_price_protect(true)
        .with_client_order_id("sl-rqethopen3");

        let mapped = binance_protective_order_request(&request).expect("valid close-all request");
        let params = mapped.to_params();

        assert_eq!(mapped.order_type, "STOP_MARKET");
        assert!(params.contains(&("algoType", "CONDITIONAL".to_string())));
        assert!(params.contains(&("symbol", "ETHUSDT".to_string())));
        assert!(params.contains(&("side", "SELL".to_string())));
        assert!(params.contains(&("type", "STOP_MARKET".to_string())));
        assert!(params.contains(&("triggerPrice", "2200".to_string())));
        assert!(params.contains(&("positionSide", "LONG".to_string())));
        assert!(!params.iter().any(|(key, _)| *key == "reduceOnly"));
        assert!(params.contains(&("closePosition", "true".to_string())));
        assert!(params.contains(&("workingType", "MARK_PRICE".to_string())));
        assert!(params.contains(&("priceProtect", "true".to_string())));
        assert!(params.contains(&("clientAlgoId", "sl-rqethopen3".to_string())));
    }

    #[test]
    fn maps_fixed_size_protective_stop_market_request_to_binance_algo_order_params() {
        let request = ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("LONG")
        .with_quantity("0.012")
        .with_working_type(ProtectiveOrderWorkingType::MarkPrice)
        .with_price_protect(true)
        .with_client_order_id("sl-rqethopen3");

        let params = binance_protective_order_request(&request)
            .expect("valid fixed-size request")
            .to_params();

        assert!(params.contains(&("quantity", "0.012".to_string())));
        assert!(!params.iter().any(|(key, _)| *key == "reduceOnly"));
        assert!(!params.iter().any(|(key, _)| *key == "closePosition"));
    }

    #[test]
    fn maps_one_way_fixed_size_protection_with_reduce_only() {
        let request = ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("BOTH")
        .with_quantity("0.012")
        .with_reduce_only(true)
        .with_working_type(ProtectiveOrderWorkingType::MarkPrice)
        .with_price_protect(true)
        .with_client_order_id("sl-rqethopen4");

        let params = binance_protective_order_request(&request)
            .expect("valid one-way fixed-size request")
            .to_params();

        assert!(params.contains(&("positionSide", "BOTH".to_string())));
        assert!(params.contains(&("quantity", "0.012".to_string())));
        assert!(params.contains(&("reduceOnly", "true".to_string())));
        assert!(!params.iter().any(|(key, _)| *key == "closePosition"));
    }

    #[test]
    fn maps_every_open_algo_order_without_filtering_unknown_activity() {
        let orders = binance_open_algo_orders_from_value(serde_json::json!([
            open_algo_order("STOP_MARKET"),
            open_algo_order("PROVIDER_FUTURE_ALGO_TYPE")
        ]))
        .expect("all well-formed open algo orders");

        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order_id.as_deref(), Some("2000000953242572"));
        assert_eq!(orders[0].client_order_id.as_deref(), Some("sl-rqethopen3"));
        assert_eq!(orders[0].status.as_deref(), Some("NEW"));
        assert_eq!(orders[0].price.as_deref(), Some("2200"));
        assert_eq!(orders[0].size.as_deref(), Some("0"));
        assert_eq!(orders[0].created_at, Some(1_779_023_785_699));
        assert_eq!(
            orders[1].order_type.as_deref(),
            Some("PROVIDER_FUTURE_ALGO_TYPE")
        );
    }

    #[test]
    fn rejects_any_missing_required_open_algo_order_field() {
        for field in [
            "algoId",
            "clientAlgoId",
            "algoStatus",
            "orderType",
            "symbol",
            "side",
            "positionSide",
            "triggerPrice",
            "quantity",
            "closePosition",
            "createTime",
            "updateTime",
        ] {
            let mut value = open_algo_order("STOP_MARKET");
            value.as_object_mut().expect("object").remove(field);

            let error = binance_open_algo_orders_from_value(serde_json::json!([value]))
                .expect_err("missing provider field must fail closed");

            assert!(error.to_string().contains(field), "field={field}: {error}");
        }
    }

    fn open_algo_order(order_type: &str) -> Value {
        serde_json::json!({
            "algoId": 2000000953242572_u64,
            "clientAlgoId": "sl-rqethopen3",
            "algoStatus": "NEW",
            "orderType": order_type,
            "symbol": "ETHUSDT",
            "side": "SELL",
            "positionSide": "LONG",
            "triggerPrice": "2200",
            "quantity": "0",
            "closePosition": true,
            "createTime": 1779023785699_u64,
            "updateTime": 1779023785699_u64
        })
    }
}
