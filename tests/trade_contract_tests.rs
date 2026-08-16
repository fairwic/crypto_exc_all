use crypto_exc_all::{
    AmendProtectiveStopRequest, BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig,
    CancelOrderRequest, CryptoSdk, Error, ExchangeId, GateExchangeConfig,
    HyperliquidExchangeConfig, Instrument, OkxExchangeConfig, OrderSide, PlaceOrderRequest,
    ProtectiveOrderWorkingType, SdkConfig,
};
use mockito::{Matcher, Server};

#[test]
fn external_consumer_can_discover_trade_capabilities_without_exchange_branching() {
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_string(),
        "https://okx.invalid".to_string(),
        "https://bitget.invalid".to_string(),
        "https://bybit.invalid".to_string(),
        "https://gate.invalid".to_string(),
        "https://hyperliquid.invalid".to_string(),
    );

    let binance_capabilities = sdk.trade(ExchangeId::Binance).unwrap().capabilities();
    let okx_capabilities = sdk.trade(ExchangeId::Okx).unwrap().capabilities();
    let bitget_capabilities = sdk.trade(ExchangeId::Bitget).unwrap().capabilities();
    let bybit_capabilities = sdk.trade(ExchangeId::Bybit).unwrap().capabilities();
    let gate_capabilities = sdk.trade(ExchangeId::Gate).unwrap().capabilities();
    let hyperliquid_capabilities = sdk.trade(ExchangeId::Hyperliquid).unwrap().capabilities();

    assert!(!binance_capabilities.attached_stop_loss_on_place_order);
    assert!(!binance_capabilities.attached_take_profit_on_place_order);
    assert!(binance_capabilities.protective_order);
    assert!(!binance_capabilities.protective_stop_amendment);
    assert!(okx_capabilities.attached_stop_loss_on_place_order);
    assert!(okx_capabilities.attached_take_profit_on_place_order);
    assert!(!okx_capabilities.protective_order);
    assert!(okx_capabilities.protective_stop_amendment);
    assert!(bitget_capabilities.attached_stop_loss_on_place_order);
    assert!(!bitget_capabilities.attached_take_profit_on_place_order);
    assert!(!bitget_capabilities.protective_order);
    assert!(!bitget_capabilities.protective_stop_amendment);
    assert!(!bybit_capabilities.attached_stop_loss_on_place_order);
    assert!(!bybit_capabilities.attached_take_profit_on_place_order);
    assert!(!bybit_capabilities.protective_order);
    assert!(!bybit_capabilities.protective_stop_amendment);
    assert!(!gate_capabilities.attached_stop_loss_on_place_order);
    assert!(!gate_capabilities.attached_take_profit_on_place_order);
    assert!(!gate_capabilities.protective_order);
    assert!(!gate_capabilities.protective_stop_amendment);
    assert!(!hyperliquid_capabilities.attached_stop_loss_on_place_order);
    assert!(!hyperliquid_capabilities.attached_take_profit_on_place_order);
    assert!(!hyperliquid_capabilities.protective_order);
    assert!(!hyperliquid_capabilities.protective_stop_amendment);
}

#[tokio::test]
async fn binance_market_order_requests_final_filled_result() {
    let mut binance_server = Server::new_async().await;
    let place = binance_server
        .mock("POST", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("side".into(), "BUY".into()),
            Matcher::UrlEncoded("type".into(), "MARKET".into()),
            Matcher::UrlEncoded("quantity".into(), "0.1".into()),
            Matcher::UrlEncoded("positionSide".into(), "LONG".into()),
            Matcher::UrlEncoded("newOrderRespType".into(), "RESULT".into()),
            Matcher::UrlEncoded(
                "newClientOrderId".into(),
                "rq0123456789abcdef0123456789abcd".into(),
            ),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"symbol":"ETHUSDT","orderId":12345,"clientOrderId":"rq0123456789abcdef0123456789abcd","status":"FILLED","executedQty":"0.1","avgPrice":"2200"}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        binance_server.url(),
        "https://okx.invalid".to_owned(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let ack = sdk
        .trade(ExchangeId::Binance)
        .expect("Binance trade facade")
        .place_order(
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                .with_position_side("long")
                .with_client_order_id("rq0123456789abcdef0123456789abcd"),
        )
        .await
        .expect("Binance final market result");

    assert_eq!(ack.status.as_deref(), Some("FILLED"));
    place.assert_async().await;
}

#[tokio::test]
async fn unsupported_attached_take_profit_is_rejected_before_place_order_submission() {
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_string(),
        "https://okx.invalid".to_string(),
        "https://bitget.invalid".to_string(),
        "https://bybit.invalid".to_string(),
        "https://gate.invalid".to_string(),
        "https://hyperliquid.invalid".to_string(),
    );

    let result = sdk
        .trade(ExchangeId::Bitget)
        .unwrap()
        .place_order(
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                .with_attached_take_profit_price("2800"),
        )
        .await;

    assert!(matches!(
        result,
        Err(Error::Unsupported {
            exchange: ExchangeId::Bitget,
            capability: "attached take profit on place_order",
        })
    ));
}

#[tokio::test]
async fn okx_place_order_serializes_one_attached_take_profit_and_stop_algo() {
    let mut okx_server = Server::new_async().await;
    let place = okx_server
        .mock("POST", "/api/v5/trade/order")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#""instId":"ETH-USDT-SWAP""#.into()),
            Matcher::Regex(r#""tdMode":"isolated""#.into()),
            Matcher::Regex(r#""posSide":"long""#.into()),
            Matcher::Regex(
                r#""attachAlgoClOrdId":"rq0123456789abcdef0123456789abcd""#.into(),
            ),
            Matcher::Regex(r#""tpTriggerPx":"2800""#.into()),
            Matcher::Regex(r#""tpOrdPx":"-1""#.into()),
            Matcher::Regex(r#""tpTriggerPxType":"mark""#.into()),
            Matcher::Regex(r#""slTriggerPx":"2200.5""#.into()),
            Matcher::Regex(r#""slOrdPx":"-1""#.into()),
            Matcher::Regex(r#""slTriggerPxType":"mark""#.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"ordId":"okx-open-1","clOrdId":"rqopen00000000000000000000000000","tag":"","ts":"1730000000100","sCode":"0","sMsg":""}]}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let ack = sdk
        .trade(ExchangeId::Okx)
        .expect("OKX trade facade")
        .place_order(
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "1")
                .with_margin_mode(crypto_exc_all::MarginMode::Isolated)
                .with_position_side("long")
                .with_client_order_id("rqopen00000000000000000000000000")
                .with_attached_stop_loss_price("2200.5")
                .with_attached_take_profit_price("2800")
                .with_attached_stop_loss_client_order_id("rq0123456789abcdef0123456789abcd"),
        )
        .await
        .expect("OKX attached exit order");

    assert_eq!(ack.order_id.as_deref(), Some("okx-open-1"));
    place.assert_async().await;
}

#[tokio::test]
async fn unsupported_attached_stop_loss_is_rejected_before_place_order_submission() {
    let binance_server = Server::new_async().await;
    let bybit_server = Server::new_async().await;
    let gate_server = Server::new_async().await;
    let sdk = configured_all_order_sdk(
        binance_server.url(),
        "https://okx.invalid".to_string(),
        "https://bitget.invalid".to_string(),
        bybit_server.url(),
        gate_server.url(),
        "https://hyperliquid.invalid".to_string(),
    );

    for exchange in [ExchangeId::Binance, ExchangeId::Bybit, ExchangeId::Gate] {
        let result = sdk
            .trade(exchange)
            .unwrap()
            .place_order(
                PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                    .with_attached_stop_loss_price("2200.5"),
            )
            .await;

        match result {
            Err(Error::Unsupported {
                exchange: actual,
                capability: "attached stop loss on place_order",
            }) => assert_eq!(actual, exchange),
            other => panic!("expected attached stop loss unsupported error, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn okx_protective_cancellation_uses_algo_order_endpoint_and_identity() {
    let mut okx_server = Server::new_async().await;
    let cancel = okx_server
        .mock("POST", "/api/v5/trade/cancel-algos")
        .match_body(Matcher::Json(serde_json::json!([{
            "instId": "ETH-USDT-SWAP",
            "algoId": "2510789768709120"
        }])))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"algoId":"2510789768709120","sCode":"0","sMsg":""}]}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let ack = sdk
        .trade(ExchangeId::Okx)
        .expect("OKX trade facade")
        .cancel_protective_order(
            CancelOrderRequest::by_order_id(Instrument::perp("ETH", "USDT"), "2510789768709120")
                .with_client_order_id("rq111111111111111111111111111111"),
        )
        .await
        .expect("cancel protective order");

    assert_eq!(ack.order_id.as_deref(), Some("2510789768709120"));
    assert_eq!(
        ack.client_order_id.as_deref(),
        Some("rq111111111111111111111111111111")
    );
    assert_eq!(ack.status.as_deref(), Some("0"));
    cancel.assert_async().await;
}

#[tokio::test]
async fn okx_protective_stop_amendment_uses_algo_endpoint_and_keeps_stop_active_on_failure() {
    let mut okx_server = Server::new_async().await;
    let amend = okx_server
        .mock("POST", "/api/v5/trade/amend-algos")
        .match_body(Matcher::Json(serde_json::json!({
            "instId": "ETH-USDT-SWAP",
            "algoId": "2510789768709120",
            "cxlOnFail": false,
            "reqId": "rq0123456789abcdef0123456789abcd",
            "newSz": "0.02",
            "newTpTriggerPx": "2800",
            "newTpOrdPx": "-1",
            "newTpTriggerPxType": "mark",
            "newSlTriggerPx": "2400.5",
            "newSlOrdPx": "-1",
            "newSlTriggerPxType": "mark"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"algoId":"2510789768709120","algoClOrdId":"rq111111111111111111111111111111","reqId":"rq0123456789abcdef0123456789abcd","sCode":"0","sMsg":""}]}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let ack = sdk
        .trade(ExchangeId::Okx)
        .expect("OKX trade facade")
        .amend_protective_stop(
            AmendProtectiveStopRequest::new(
                Instrument::perp("ETH", "USDT"),
                "2510789768709120",
                "rq0123456789abcdef0123456789abcd",
                "0.02",
                "2400.5",
                ProtectiveOrderWorkingType::MarkPrice,
            )
            .with_take_profit_price("2800"),
        )
        .await
        .expect("amend protective stop");

    assert_eq!(ack.order_id.as_deref(), Some("2510789768709120"));
    assert_eq!(
        ack.client_order_id.as_deref(),
        Some("rq0123456789abcdef0123456789abcd")
    );
    assert_eq!(ack.status.as_deref(), Some("0"));
    amend.assert_async().await;
}

#[tokio::test]
async fn okx_protective_stop_amendment_maps_item_rejection() {
    let mut okx_server = Server::new_async().await;
    let amend = okx_server
        .mock("POST", "/api/v5/trade/amend-algos")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"algoId":"2510789768709120","reqId":"rq0123456789abcdef0123456789abcd","sCode":"51538","sMsg":"Stop-loss amendment rejected"}]}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let error = sdk
        .trade(ExchangeId::Okx)
        .expect("OKX trade facade")
        .amend_protective_stop(AmendProtectiveStopRequest::new(
            Instrument::perp("ETH", "USDT"),
            "2510789768709120",
            "rq0123456789abcdef0123456789abcd",
            "0.02",
            "2400.5",
            ProtectiveOrderWorkingType::MarkPrice,
        ))
        .await
        .expect_err("item rejection must not become an accepted amendment");

    assert!(matches!(
        error,
        Error::Api {
            exchange: ExchangeId::Okx,
            status: Some(200),
            ref code,
            ..
        } if code == "51538"
    ));
    amend.assert_async().await;
}

#[tokio::test]
async fn okx_pending_protective_orders_use_signed_algo_list_and_preserve_identity() {
    let mut okx_server = Server::new_async().await;
    let pending = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-algo-pending?ordType=conditional&instType=SWAP&limit=100",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"instId":"ETH-USDT-SWAP","algoId":"2510789768709120","algoClOrdId":"rq111111111111111111111111111111","side":"sell","ordType":"conditional","slTriggerPx":"1880","sz":"1","state":"live","cTime":"1786522000000","uTime":"1786522000100"}]}"#,
        )
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let orders = sdk
        .orders(ExchangeId::Okx)
        .expect("OKX order facade")
        .open_protective_orders()
        .await
        .expect("pending protective orders");

    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].exchange_symbol, "ETH-USDT-SWAP");
    assert_eq!(orders[0].order_id.as_deref(), Some("2510789768709120"));
    assert_eq!(
        orders[0].client_order_id.as_deref(),
        Some("rq111111111111111111111111111111")
    );
    pending.assert_async().await;
}

#[tokio::test]
async fn okx_protective_history_preserves_triggered_identity_and_fill() {
    let mut okx_server = Server::new_async().await;
    let effective = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-algo-history?ordType=conditional&state=effective&instType=SWAP&limit=100",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"instId":"ETH-USDT-SWAP","algoId":"2510789768709120","algoClOrdId":"rq111111111111111111111111111111","side":"sell","posSide":"long","ordType":"conditional","slTriggerPx":"1880","slOrdPx":"-1","slTriggerPxType":"mark","sz":"1","actualSz":"1","actualPx":"1879.9","state":"effective","ordId":"2510789768709121","cTime":"1786522000000","uTime":"1786522000100"}]}"#,
        )
        .create_async()
        .await;
    let canceled = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-algo-history?ordType=conditional&state=canceled&instType=SWAP&limit=100",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[]}"#)
        .create_async()
        .await;
    let failed = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-algo-history?ordType=conditional&state=order_failed&instType=SWAP&limit=100",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[]}"#)
        .create_async()
        .await;
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_owned(),
        okx_server.url(),
        "https://bitget.invalid".to_owned(),
        "https://bybit.invalid".to_owned(),
        "https://gate.invalid".to_owned(),
        "https://hyperliquid.invalid".to_owned(),
    );

    let orders = sdk
        .orders(ExchangeId::Okx)
        .expect("OKX order facade")
        .protective_order_history()
        .await
        .expect("protective order history");

    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].status.as_deref(), Some("effective"));
    assert_eq!(orders[0].filled_size.as_deref(), Some("1"));
    assert_eq!(orders[0].average_price.as_deref(), Some("1879.9"));
    assert_eq!(
        orders[0].client_order_id.as_deref(),
        Some("rq111111111111111111111111111111")
    );
    effective.assert_async().await;
    canceled.assert_async().await;
    failed.assert_async().await;
}

#[tokio::test]
async fn hyperliquid_live_mutation_is_explicitly_unsupported() {
    let sdk = configured_all_order_sdk(
        "https://binance.invalid".to_string(),
        "https://okx.invalid".to_string(),
        "https://bitget.invalid".to_string(),
        "https://bybit.invalid".to_string(),
        "https://gate.invalid".to_string(),
        "https://hyperliquid.invalid".to_string(),
    );

    let result = sdk
        .trade(ExchangeId::Hyperliquid)
        .unwrap()
        .place_order(PlaceOrderRequest::market(
            Instrument::perp("ETH", "USDC"),
            OrderSide::Buy,
            "0.1",
        ))
        .await;

    match result {
        Err(Error::Unsupported {
            exchange: ExchangeId::Hyperliquid,
            capability: "place order",
        }) => {}
        other => panic!("expected Hyperliquid place order unsupported error, got {other:?}"),
    }
}

fn configured_all_order_sdk(
    binance_url: String,
    okx_url: String,
    bitget_url: String,
    bybit_url: String,
    gate_url: String,
    hyperliquid_url: String,
) -> CryptoSdk {
    CryptoSdk::from_config(SdkConfig {
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(binance_url),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        okx: Some(OkxExchangeConfig {
            api_key: "okx-key".to_string(),
            api_secret: "okx-secret".to_string(),
            passphrase: "okx-pass".to_string(),
            simulated: true,
            api_url: Some(okx_url),
            request_expiration_ms: Some(1_000),
        }),
        bitget: Some(BitgetExchangeConfig {
            api_key: "bitget-key".to_string(),
            api_secret: "bitget-secret".to_string(),
            passphrase: "bitget-pass".to_string(),
            api_url: Some(bitget_url),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            product_type: Some("USDT-FUTURES".to_string()),
        }),
        bybit: Some(BybitExchangeConfig {
            api_key: "bybit-key".to_string(),
            api_secret: "bybit-secret".to_string(),
            api_url: Some(bybit_url),
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
            category: Some("linear".to_string()),
        }),
        gate: Some(GateExchangeConfig {
            api_key: "gate-key".to_string(),
            api_secret: "gate-secret".to_string(),
            api_url: Some(gate_url),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            settle: Some("usdt".to_string()),
        }),
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(hyperliquid_url),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some("0x00000000000000000000000000000000000000ab".to_string()),
        }),
    })
    .unwrap()
}
