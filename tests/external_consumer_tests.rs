use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, CryptoSdk, ExchangeId, Instrument,
    OkxExchangeConfig, SdkConfig,
};
use mockito::Server;

#[tokio::test]
async fn external_consumer_uses_root_crate_for_binance_okx_and_bitget_tickers() {
    let mut binance_server = Server::new_async().await;
    let binance_ticker = binance_server
        .mock("GET", "/fapi/v1/ticker/24hr?symbol=BTCUSDT")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "symbol":"BTCUSDT",
                "lastPrice":"70000.10",
                "bidPrice":"69999.90",
                "askPrice":"70000.20",
                "volume":"1234.5",
                "closeTime":1730000000000
            }"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_ticker = okx_server
        .mock("GET", "/api/v5/market/ticker?instId=BTC-USDT-SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instType":"SWAP",
                    "instId":"BTC-USDT-SWAP",
                    "last":"70001.20",
                    "lastSz":"0.1",
                    "askPx":"70001.30",
                    "askSz":"0.2",
                    "bidPx":"70001.10",
                    "bidSz":"0.3",
                    "open24h":"69000",
                    "high24h":"71000",
                    "low24h":"68000",
                    "volCcy24h":"100000",
                    "vol24h":"456.7",
                    "sodUtc0":"0",
                    "sodUtc8":"0",
                    "ts":"1730000000001"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_ticker = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/ticker?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "requestTime":1695794095685,
                "data":[{
                    "symbol":"BTCUSDT",
                    "lastPr":"70002.10",
                    "askPr":"70002.30",
                    "bidPr":"70002.00",
                    "baseVolume":"789.1",
                    "quoteVolume":"55240000",
                    "ts":"1730000000002"
                }]
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: Some(OkxExchangeConfig {
            api_key: "okx-key".to_string(),
            api_secret: "okx-secret".to_string(),
            passphrase: "okx-pass".to_string(),
            simulated: true,
            api_url: Some(okx_server.url()),
            request_expiration_ms: Some(1_000),
        }),
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(binance_server.url()),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: Some(BitgetExchangeConfig {
            api_key: "bitget-key".to_string(),
            api_secret: "bitget-secret".to_string(),
            passphrase: "bitget-pass".to_string(),
            api_url: Some(bitget_server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            product_type: Some("USDT-FUTURES".to_string()),
        }),
    })
    .unwrap();

    let btc_perp = Instrument::perp("BTC", "USDT");
    let binance = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .ticker(&btc_perp)
        .await
        .unwrap();
    let okx = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .ticker(&btc_perp)
        .await
        .unwrap();
    let bitget = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .ticker(&btc_perp)
        .await
        .unwrap();

    assert_eq!(binance.exchange_symbol, "BTCUSDT");
    assert_eq!(binance.last_price, "70000.10");
    assert_eq!(binance.bid_price.as_deref(), Some("69999.90"));
    assert_eq!(okx.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx.last_price, "70001.20");
    assert_eq!(okx.ask_price.as_deref(), Some("70001.30"));
    assert_eq!(bitget.exchange_symbol, "BTCUSDT");
    assert_eq!(bitget.last_price, "70002.10");
    assert_eq!(bitget.bid_price.as_deref(), Some("70002.00"));
    assert_eq!(bitget.ask_price.as_deref(), Some("70002.30"));

    binance_ticker.assert_async().await;
    okx_ticker.assert_async().await;
    bitget_ticker.assert_async().await;
}
