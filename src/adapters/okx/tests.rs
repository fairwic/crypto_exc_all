use super::order_mapping::{okx_algo_order_from_value, okx_order_ack_from_response};
use super::*;

#[test]
fn isolated_long_short_leverage_info_keeps_each_position_side() {
    let settings = okx_leverage_info_from_value(
        ExchangeId::Okx,
        Instrument::perp("ETH", "USDT"),
        "ETH-USDT-SWAP".to_owned(),
        "isolated",
        serde_json::json!([
            {
                "instId": "ETH-USDT-SWAP",
                "mgnMode": "isolated",
                "posSide": "long",
                "lever": "1"
            },
            {
                "instId": "ETH-USDT-SWAP",
                "mgnMode": "isolated",
                "posSide": "short",
                "lever": "2"
            }
        ]),
    )
    .expect("valid isolated leverage settings");

    assert_eq!(settings.len(), 2);
    assert_eq!(settings[0].position_side.as_deref(), Some("long"));
    assert_eq!(settings[0].leverage, "1");
    assert_eq!(settings[1].position_side.as_deref(), Some("short"));
    assert_eq!(settings[1].leverage, "2");
}

#[test]
fn maps_okx_algo_order_detail_to_protective_order() {
    let protective_client_order_id = format!("rq{}", "1".repeat(30));
    let order = okx_algo_order_from_value(
        ExchangeId::Okx,
        Some(Instrument::perp("ETH", "USDT")),
        Some("ETH-USDT-SWAP".to_owned()),
        serde_json::json!({
            "algoId": "2510789768709120",
            "algoClOrdId": protective_client_order_id,
            "instId": "ETH-USDT-SWAP",
            "ordType": "conditional",
            "side": "sell",
            "slTriggerPx": "2200.5",
            "sz": "1",
            "actualSz": "0",
            "state": "live",
            "cTime": "1779023785699",
            "uTime": "1779023785700",
            "reduceOnly": "true",
            "failCode": ""
        }),
    )
    .expect("valid protective order");

    assert_eq!(order.order_id.as_deref(), Some("2510789768709120"));
    assert_eq!(
        order.client_order_id.as_deref(),
        Some(protective_client_order_id.as_str())
    );
    assert_eq!(order.status.as_deref(), Some("live"));
    assert_eq!(order.price.as_deref(), Some("2200.5"));
    assert_eq!(order.size.as_deref(), Some("1"));
    assert_eq!(order.side.as_deref(), Some("sell"));
}

#[test]
fn okx_account_bills_map_only_matching_instrument_items() {
    let bills = okx_account_bills_from_value(
        ExchangeId::Okx,
        Some(Instrument::perp("BTC", "USDT")),
        Some("BTC-USDT-SWAP".to_string()),
        serde_json::json!([
            {
                "billId": "bill-1",
                "instId": "BTC-USDT-SWAP",
                "ccy": "USDT",
                "balChg": "9.7",
                "bal": "8211.49",
                "fee": "-0.3",
                "pnl": "10",
                "type": "2",
                "subType": "1",
                "ordId": "order-1",
                "tradeId": "fill-1",
                "ts": "1773810600000"
            },
            {
                "billId": "bill-transfer",
                "ccy": "USDT",
                "balChg": "100",
                "ts": "1773810600000"
            },
            {
                "billId": "bill-eth",
                "instId": "ETH-USDT-SWAP",
                "ccy": "USDT",
                "balChg": "1",
                "ts": "1773810600000"
            }
        ]),
    )
    .expect("map account bills");

    assert_eq!(bills.len(), 1);
    assert_eq!(bills[0].bill_id.as_deref(), Some("bill-1"));
    assert_eq!(bills[0].exchange_symbol.as_deref(), Some("BTC-USDT-SWAP"));
    assert_eq!(bills[0].balance_change.as_deref(), Some("9.7"));
    assert_eq!(bills[0].fee.as_deref(), Some("-0.3"));
    assert_eq!(bills[0].pnl.as_deref(), Some("10"));
}

#[test]
fn okx_position_history_maps_closed_position_fields() {
    let history = okx_position_history_from_value(
        ExchangeId::Okx,
        Some(Instrument::perp("ASTER", "USDT")),
        Some("ASTER-USDT-SWAP".to_string()),
        serde_json::json!({
            "posId": "okx-position-1",
            "instId": "ASTER-USDT-SWAP",
            "instType": "SWAP",
            "posSide": "long",
            "direction": "long",
            "mgnMode": "cross",
            "lever": "3",
            "openAvgPx": "0.6208",
            "closeAvgPx": "0.6047",
            "openMaxPos": "1",
            "closeTotalPos": "1",
            "realizedPnl": "-0.01",
            "pnl": "-0.01",
            "pnlRatio": "-0.0817",
            "fee": "-0.0002",
            "fundingFee": "0",
            "liqPenalty": "0",
            "type": "2",
            "cTime": "1780980141000",
            "uTime": "1781122152000"
        }),
    )
    .expect("map position history");

    assert_eq!(history.exchange_symbol, "ASTER-USDT-SWAP");
    assert_eq!(history.position_id.as_deref(), Some("okx-position-1"));
    assert_eq!(history.side.as_deref(), Some("long"));
    assert_eq!(history.direction.as_deref(), Some("long"));
    assert_eq!(history.leverage.as_deref(), Some("3"));
    assert_eq!(history.margin_mode.as_deref(), Some("cross"));
    assert_eq!(history.open_avg_price.as_deref(), Some("0.6208"));
    assert_eq!(history.close_avg_price.as_deref(), Some("0.6047"));
    assert_eq!(history.open_max_position.as_deref(), Some("1"));
    assert_eq!(history.close_total_position.as_deref(), Some("1"));
    assert_eq!(history.realized_pnl.as_deref(), Some("-0.01"));
    assert_eq!(history.pnl_ratio.as_deref(), Some("-0.0817"));
    assert_eq!(history.close_type.as_deref(), Some("2"));
    assert_eq!(history.open_time, Some(1_780_980_141_000));
    assert_eq!(history.close_time, Some(1_781_122_152_000));
}

#[test]
fn okx_order_ack_rejects_nonzero_order_s_code() {
    let order = OrderResDto {
        ord_id: String::new(),
        cl_ord_id: Some("rq-okx-1".to_string()),
        tag: None,
        ts: "1710000000000".to_string(),
        s_code: "51076".to_string(),
        s_msg: Some("Attached TP/SL parameter error".to_string()),
    };

    let error = okx_order_ack_from_response(
        Instrument::perp("ETH", "USDT"),
        "ETH-USDT-SWAP".to_string(),
        order,
    )
    .unwrap_err();

    assert!(matches!(
        &error,
        Error::Api {
            status: Some(200),
            code,
            ..
        } if code == "51076"
    ));
    assert!(error.to_string().contains("51076"));
    assert!(error.to_string().contains("Attached TP/SL parameter error"));
}

#[test]
fn okx_account_level_uses_current_official_mode_names() {
    assert_eq!(okx_account_mode("1"), Some("spot"));
    assert_eq!(okx_account_mode("2"), Some("futures"));
    assert_eq!(okx_account_mode("3"), Some("multi_currency_margin"));
    assert_eq!(okx_account_mode("4"), Some("portfolio_margin"));
    assert_eq!(okx_account_mode("5"), None);
}

#[test]
fn okx_main_account_does_not_report_itself_as_parent() {
    assert_eq!(okx_parent_account_id("100", Some("100".to_owned())), None);
    assert_eq!(
        okx_parent_account_id("101", Some("100".to_owned())),
        Some("100".to_owned())
    );
}

#[test]
fn simulated_credentials_cannot_silently_open_the_production_private_stream() {
    let adapter = OkxAdapter::new(OkxExchangeConfig {
        api_key: "demo-key".to_owned(),
        api_secret: "demo-secret".to_owned(),
        passphrase: "demo-passphrase".to_owned(),
        simulated: true,
        api_url: None,
        request_expiration_ms: None,
    })
    .unwrap();

    assert!(adapter.private_stream.is_none());
}

#[test]
fn okx_candle_interval_uses_okx_bar_case() {
    assert_eq!(okx_candle_interval("4h"), "4H");
    assert_eq!(okx_candle_interval("1h"), "1H");
    assert_eq!(okx_candle_interval("1d"), "1D");
    assert_eq!(okx_candle_interval("1Dutc"), "1Dutc");
    assert_eq!(okx_candle_interval("1m"), "1m");
}
