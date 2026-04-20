use binance_rs::api::market::BinanceMarket;
use binance_rs::api::trade::{BinanceTrade, NewOrderRequest, OrderIdRequest};
use binance_rs::client::BinanceClient;
use serde_json::Value;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIRM_VALUE: &str = "I_UNDERSTAND_THIS_USES_REAL_FUNDS";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    require_live_confirmation()?;

    let symbol = env::var("BINANCE_LIVE_SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    let side = env::var("BINANCE_LIVE_SIDE").unwrap_or_else(|_| "BUY".to_string());
    let offset_bps = env::var("BINANCE_LIVE_PRICE_OFFSET_BPS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(1_400.0);

    let market = BinanceMarket::new(BinanceClient::new_public()?);
    let trade = BinanceTrade::from_env()?;

    let exchange_info = market.get_exchange_info().await?;
    let ticker = market.get_ticker_24hr(Some(&symbol)).await?;
    let plan = build_post_only_plan(&exchange_info, &ticker, &symbol, &side, offset_bps)?;

    let client_order_id = format!(
        "sdk_live_{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    );
    let order = NewOrderRequest::limit(
        &symbol,
        &side,
        plan.quantity.clone(),
        plan.price.clone(),
        "GTX",
    )
    .with_new_client_order_id(client_order_id.clone())
    .with_new_order_resp_type("RESULT");
    let order = with_optional_position_side(order, live_position_side().as_deref());

    println!(
        "placing live post-only order: symbol={} side={} quantity={} price={} tif=GTX clientOrderId={}",
        symbol, side, plan.quantity, plan.price, client_order_id
    );

    let placed = trade.place_order(order).await?;
    println!("placed: {}", serde_json::to_string_pretty(&placed)?);

    if env::var("BINANCE_LIVE_SKIP_CANCEL").as_deref() == Ok("true") {
        println!("skip cancel requested by BINANCE_LIVE_SKIP_CANCEL=true");
        return Ok(());
    }

    let cancel_request = if let Some(order_id) = placed.get("orderId").and_then(Value::as_u64) {
        OrderIdRequest::new(&symbol).with_order_id(order_id)
    } else {
        OrderIdRequest::new(&symbol).with_orig_client_order_id(client_order_id)
    };
    let canceled = trade.cancel_order(cancel_request).await?;
    println!("canceled: {}", serde_json::to_string_pretty(&canceled)?);

    Ok(())
}

fn require_live_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    match env::var("BINANCE_LIVE_ORDER_CONFIRM").as_deref() {
        Ok(CONFIRM_VALUE) => Ok(()),
        _ => Err(format!(
            "set BINANCE_LIVE_ORDER_CONFIRM={CONFIRM_VALUE} to place a real futures order"
        )
        .into()),
    }
}

fn live_position_side() -> Option<String> {
    env::var("BINANCE_LIVE_POSITION_SIDE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn with_optional_position_side(
    order: NewOrderRequest,
    position_side: Option<&str>,
) -> NewOrderRequest {
    match position_side {
        Some(position_side) => order.with_position_side(position_side),
        None => order,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PostOnlyPlan {
    price: String,
    quantity: String,
}

fn build_post_only_plan(
    exchange_info: &Value,
    ticker: &Value,
    symbol: &str,
    side: &str,
    offset_bps: f64,
) -> Result<PostOnlyPlan, Box<dyn std::error::Error>> {
    let symbol_info = exchange_info
        .get("symbols")
        .and_then(Value::as_array)
        .and_then(|symbols| {
            symbols
                .iter()
                .find(|item| item.get("symbol").and_then(Value::as_str) == Some(symbol))
        })
        .ok_or_else(|| format!("symbol not found in exchangeInfo: {symbol}"))?;

    if symbol_info.get("status").and_then(Value::as_str) != Some("TRADING") {
        return Err(format!("{symbol} is not TRADING").into());
    }

    let price_filter = filter(symbol_info, "PRICE_FILTER")?;
    let lot_size = filter(symbol_info, "LOT_SIZE")?;
    let min_notional = filter(symbol_info, "MIN_NOTIONAL").ok();
    let percent_price = filter(symbol_info, "PERCENT_PRICE").ok();

    let tick_size = decimal_field(price_filter, "tickSize")?;
    let step_size = decimal_field(lot_size, "stepSize")?;
    let min_qty = decimal_field(lot_size, "minQty")?;
    let notional = min_notional
        .as_ref()
        .and_then(|filter| decimal_field(filter, "notional").ok())
        .unwrap_or(5.0);
    let last_price = decimal_field(ticker, "lastPrice")?;
    let offset = (offset_bps / 10_000.0).clamp(0.001, 0.5);

    let raw_price = match side {
        "BUY" => {
            let mut price = last_price * (1.0 - offset);
            if let Some(filter) = percent_price.as_ref() {
                let lower_bound = last_price * decimal_field(filter, "multiplierDown")? * 1.001;
                price = price.max(lower_bound);
            }
            round_down(price, tick_size)
        }
        "SELL" => {
            let mut price = last_price * (1.0 + offset);
            if let Some(filter) = percent_price.as_ref() {
                let upper_bound = last_price * decimal_field(filter, "multiplierUp")? * 0.999;
                price = price.min(upper_bound);
            }
            round_up(price, tick_size)
        }
        other => return Err(format!("unsupported side: {other}").into()),
    };

    if raw_price <= 0.0 {
        return Err("computed price must be positive".into());
    }

    let min_notional_qty = round_up(notional / raw_price, step_size);
    let quantity = round_up(min_qty.max(min_notional_qty), step_size);

    Ok(PostOnlyPlan {
        price: format_decimal(raw_price, decimals_from_step(tick_size)),
        quantity: format_decimal(quantity, decimals_from_step(step_size)),
    })
}

fn filter<'a>(symbol_info: &'a Value, filter_type: &str) -> Result<&'a Value, String> {
    symbol_info
        .get("filters")
        .and_then(Value::as_array)
        .and_then(|filters| {
            filters.iter().find(|filter| {
                filter.get("filterType").and_then(Value::as_str) == Some(filter_type)
            })
        })
        .ok_or_else(|| format!("missing {filter_type} filter"))
}

fn decimal_field(value: &Value, field: &str) -> Result<f64, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing decimal field: {field}"))?
        .parse::<f64>()
        .map_err(|err| format!("invalid decimal field {field}: {err}"))
}

fn round_down(value: f64, step: f64) -> f64 {
    ((value / step) + 1e-9).floor() * step
}

fn round_up(value: f64, step: f64) -> f64 {
    ((value / step) - 1e-9).ceil() * step
}

fn decimals_from_step(step: f64) -> usize {
    let formatted = format!("{step:.12}");
    formatted
        .trim_end_matches('0')
        .split_once('.')
        .map(|(_, decimals)| decimals.len())
        .unwrap_or(0)
}

fn format_decimal(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_buy_post_only_plan_above_percent_floor() {
        let exchange_info = json!({
            "symbols": [{
                "symbol": "BTCUSDT",
                "status": "TRADING",
                "filters": [
                    {"filterType": "PRICE_FILTER", "tickSize": "0.10"},
                    {"filterType": "LOT_SIZE", "minQty": "0.001", "stepSize": "0.001"},
                    {"filterType": "MIN_NOTIONAL", "notional": "100.0"},
                    {"filterType": "PERCENT_PRICE", "multiplierDown": "0.8500", "multiplierUp": "1.1500"}
                ]
            }]
        });
        let ticker = json!({"lastPrice": "100000.00"});

        let plan =
            build_post_only_plan(&exchange_info, &ticker, "BTCUSDT", "BUY", 1_600.0).expect("plan");

        assert_eq!(plan.price, "85085.0");
        assert_eq!(plan.quantity, "0.002");
    }

    #[test]
    fn applies_optional_position_side_to_order() {
        let order = NewOrderRequest::limit("BTCUSDT", "BUY", "0.001", "9000", "GTX");

        let order = with_optional_position_side(order, Some("LONG"));

        assert_eq!(order.position_side.as_deref(), Some("LONG"));
    }
}
