use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CryptoSdk, Error, ExchangeId,
    GateExchangeConfig, HyperliquidExchangeConfig, Instrument, OkxExchangeConfig, OrderSide,
    PlaceOrderRequest, SdkConfig,
};
use mockito::Server;

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
    assert!(binance_capabilities.protective_order);
    assert!(okx_capabilities.attached_stop_loss_on_place_order);
    assert!(!okx_capabilities.protective_order);
    assert!(bitget_capabilities.attached_stop_loss_on_place_order);
    assert!(!bitget_capabilities.protective_order);
    assert!(!bybit_capabilities.attached_stop_loss_on_place_order);
    assert!(!bybit_capabilities.protective_order);
    assert!(!gate_capabilities.attached_stop_loss_on_place_order);
    assert!(!gate_capabilities.protective_order);
    assert!(!hyperliquid_capabilities.attached_stop_loss_on_place_order);
    assert!(!hyperliquid_capabilities.protective_order);
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
        ..SdkConfig::default()
    })
    .unwrap()
}
