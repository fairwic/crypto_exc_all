use bitget_rs::BitgetClient;
use bitget_rs::api::market::{BitgetMarket, TickerRequest};
use bitget_rs::api::trade::{BitgetTrade, CancelOrderRequest, NewOrderRequest};
use serde_json::Value;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIRM_VALUE: &str = "I_UNDERSTAND_THIS_USES_REAL_FUNDS";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    require_live_confirmation()?;

    let symbol = env::var("BITGET_LIVE_SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    let product_type =
        env::var("BITGET_LIVE_PRODUCT_TYPE").unwrap_or_else(|_| "USDT-FUTURES".to_string());
    let margin_mode = env::var("BITGET_LIVE_MARGIN_MODE").unwrap_or_else(|_| "crossed".to_string());
    let margin_coin = env::var("BITGET_LIVE_MARGIN_COIN").unwrap_or_else(|_| "USDT".to_string());
    let side = env::var("BITGET_LIVE_SIDE").unwrap_or_else(|_| "buy".to_string());
    let trade_side = env::var("BITGET_LIVE_TRADE_SIDE").unwrap_or_else(|_| "open".to_string());
    let offset_bps = env::var("BITGET_LIVE_PRICE_OFFSET_BPS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(500.0);

    let market = BitgetMarket::new(BitgetClient::new_public()?);
    let trade = BitgetTrade::from_env()?;

    let contracts = market
        .get_contracts(&product_type, Some(&symbol))
        .await?
        .as_array()
        .cloned()
        .ok_or("contract response data must be an array")?;
    let contract = contracts
        .first()
        .ok_or_else(|| format!("contract not found for {symbol}"))?;
    let tickers = market
        .get_ticker(TickerRequest::new(&symbol, &product_type))
        .await?;
    let ticker = tickers
        .first()
        .ok_or_else(|| format!("ticker not found for {symbol}"))?;
    let ticker = serde_json::to_value(ticker)?;
    let plan = build_post_only_plan(contract, &ticker, &symbol, &side, offset_bps)?;

    let client_oid = format!(
        "sdk_bg_{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    );
    let order = NewOrderRequest::limit(
        &symbol,
        &product_type,
        &margin_mode,
        &margin_coin,
        plan.size.clone(),
        &side,
        plan.price.clone(),
    )
    .with_trade_side(trade_side.clone())
    .with_force("post_only")
    .with_client_oid(client_oid.clone());

    println!(
        "placing bitget live post-only order: symbol={} productType={} side={} tradeSide={} size={} price={} clientOid={}",
        symbol, product_type, side, trade_side, plan.size, plan.price, client_oid
    );

    let placed = trade.place_order(order).await?;
    println!("placed: {}", serde_json::to_string_pretty(&placed)?);

    if env::var("BITGET_LIVE_SKIP_CANCEL").as_deref() == Ok("true") {
        println!("skip cancel requested by BITGET_LIVE_SKIP_CANCEL=true");
        return Ok(());
    }

    let mut cancel_request =
        CancelOrderRequest::new(&symbol, &product_type).with_margin_coin(margin_coin);
    if let Some(order_id) = placed.get("orderId").and_then(Value::as_str) {
        cancel_request = cancel_request.with_order_id(order_id);
    } else {
        cancel_request = cancel_request.with_client_oid(client_oid);
    }
    let canceled = trade.cancel_order(cancel_request).await?;
    println!("canceled: {}", serde_json::to_string_pretty(&canceled)?);

    Ok(())
}

fn require_live_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    match env::var("BITGET_LIVE_ORDER_CONFIRM").as_deref() {
        Ok(CONFIRM_VALUE) => Ok(()),
        _ => Err(format!(
            "set BITGET_LIVE_ORDER_CONFIRM={CONFIRM_VALUE} to place a real futures order"
        )
        .into()),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PostOnlyPlan {
    price: String,
    size: String,
}

fn build_post_only_plan(
    contract: &Value,
    ticker: &Value,
    symbol: &str,
    side: &str,
    offset_bps: f64,
) -> Result<PostOnlyPlan, Box<dyn std::error::Error>> {
    if contract.get("symbol").and_then(Value::as_str) != Some(symbol) {
        return Err(format!("contract symbol mismatch for {symbol}").into());
    }
    if contract.get("symbolStatus").and_then(Value::as_str) != Some("normal") {
        return Err(format!("{symbol} is not normal").into());
    }

    let price_place = decimal_field(contract, "pricePlace")? as i32;
    let price_end_step = decimal_field(contract, "priceEndStep")?;
    let tick_size = price_end_step * 10_f64.powi(-price_place);
    let size_step = decimal_field(contract, "sizeMultiplier")?;
    let min_size = decimal_field(contract, "minTradeNum")?;
    let min_notional = decimal_field(contract, "minTradeUSDT").unwrap_or(5.0);
    let last_price = decimal_field(ticker, "lastPr")?;
    let bid_price = decimal_field(ticker, "bidPr").unwrap_or(last_price);
    let ask_price = decimal_field(ticker, "askPr").unwrap_or(last_price);
    let offset = (offset_bps / 10_000.0).clamp(0.0001, 0.5);

    let raw_price = match side {
        "buy" => round_down(bid_price * (1.0 - offset), tick_size),
        "sell" => round_up(ask_price * (1.0 + offset), tick_size),
        other => return Err(format!("unsupported side: {other}").into()),
    };
    if raw_price <= 0.0 {
        return Err("computed price must be positive".into());
    }

    let min_notional_size = round_up(min_notional / raw_price, size_step);
    let size = round_up(min_size.max(min_notional_size), size_step);

    Ok(PostOnlyPlan {
        price: format_decimal(raw_price, decimals_from_step(tick_size)),
        size: format_decimal(size, decimals_from_step(size_step)),
    })
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
    fn builds_buy_post_only_plan_from_contract_config() {
        let contract = json!({
            "symbol": "BTCUSDT",
            "symbolStatus": "normal",
            "minTradeNum": "0.0001",
            "sizeMultiplier": "0.0001",
            "minTradeUSDT": "5",
            "pricePlace": "1",
            "priceEndStep": "1"
        });
        let ticker = json!({
            "lastPr": "100000.0",
            "bidPr": "99999.9",
            "askPr": "100000.1"
        });

        let plan = build_post_only_plan(&contract, &ticker, "BTCUSDT", "buy", 1_000.0).unwrap();

        assert_eq!(plan.price, "89999.9");
        assert_eq!(plan.size, "0.0001");
    }
}
