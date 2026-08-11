use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CancelOrderRequest,
    CryptoSdk, Error, ExchangeId, GateExchangeConfig, HyperliquidExchangeConfig, Instrument,
    OkxExchangeConfig, OrderSide, PlaceOrderRequest, SdkConfig,
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
    assert!(okx_capabilities.attached_stop_loss_on_place_order);
    assert!(okx_capabilities.attached_take_profit_on_place_order);
    assert!(!okx_capabilities.protective_order);
    assert!(bitget_capabilities.attached_stop_loss_on_place_order);
    assert!(!bitget_capabilities.attached_take_profit_on_place_order);
    assert!(!bitget_capabilities.protective_order);
    assert!(!bybit_capabilities.attached_stop_loss_on_place_order);
    assert!(!bybit_capabilities.attached_take_profit_on_place_order);
    assert!(!bybit_capabilities.protective_order);
    assert!(!gate_capabilities.attached_stop_loss_on_place_order);
    assert!(!gate_capabilities.attached_take_profit_on_place_order);
    assert!(!gate_capabilities.protective_order);
    assert!(!hyperliquid_capabilities.attached_stop_loss_on_place_order);
    assert!(!hyperliquid_capabilities.attached_take_profit_on_place_order);
    assert!(!hyperliquid_capabilities.protective_order);
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
    assert_eq!(ack.status.as_deref(), Some("0"));
    cancel.assert_async().await;
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
