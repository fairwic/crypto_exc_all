use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, CryptoSdk, ExchangeId, FundingRateQuery,
    Instrument, MarketStatsQuery, OkxExchangeConfig, SdkConfig,
};
use mockito::Server;

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_derivatives_market_metrics() {
    let mut binance_server = Server::new_async().await;
    let binance_premium = binance_server
        .mock("GET", "/fapi/v1/premiumIndex?symbol=BTCUSDT")
        .expect(2)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "symbol":"BTCUSDT",
                "markPrice":"70010.10",
                "indexPrice":"70009.90",
                "lastFundingRate":"0.0001",
                "nextFundingTime":1730000300000,
                "time":1730000000000
            }"#,
        )
        .create_async()
        .await;
    let binance_funding_history = binance_server
        .mock("GET", "/fapi/v1/fundingRate?symbol=BTCUSDT&limit=2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "fundingRate":"0.00009",
                "fundingTime":1729990000000,
                "markPrice":"69990"
            }]"#,
        )
        .create_async()
        .await;
    let binance_open_interest = binance_server
        .mock("GET", "/fapi/v1/openInterest?symbol=BTCUSDT")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "symbol":"BTCUSDT",
                "openInterest":"123.45",
                "time":1730000000100
            }"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_funding = okx_server
        .mock("GET", "/api/v5/public/funding-rate?instId=BTC-USDT-SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instType":"SWAP",
                    "instId":"BTC-USDT-SWAP",
                    "method":"current_period",
                    "fundingRate":"0.0002",
                    "nextFundingRate":"0.00021",
                    "fundingTime":"1730000400000",
                    "nextFundingTime":"1730000700000",
                    "minFundingRate":"-0.00375",
                    "maxFundingRate":"0.00375",
                    "ts":"1730000000200"
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_funding_history = okx_server
        .mock(
            "GET",
            "/api/v5/public/funding-rate-history?instId=BTC-USDT-SWAP&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instType":"SWAP",
                    "instId":"BTC-USDT-SWAP",
                    "formulaType":"withRate",
                    "fundingRate":"0.00019",
                    "realizedRate":"0.00018",
                    "fundingTime":"1729990000000",
                    "method":"current_period"
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_mark_price = okx_server
        .mock(
            "GET",
            "/api/v5/public/mark-price?instType=SWAP&instId=BTC-USDT-SWAP",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instType":"SWAP",
                    "instId":"BTC-USDT-SWAP",
                    "markPx":"70011.10",
                    "ts":"1730000000300"
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_open_interest = okx_server
        .mock(
            "GET",
            "/api/v5/public/open-interest?instType=SWAP&instId=BTC-USDT-SWAP",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instType":"SWAP",
                    "instId":"BTC-USDT-SWAP",
                    "oi":"234.56",
                    "oiCcy":"2345600",
                    "ts":"1730000000400"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_funding = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/current-fund-rate?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT","fundingRate":"0.0003","fundingTime":"1730000500000","nextUpdate":"1730000800000"}]}"#,
        )
        .create_async()
        .await;
    let bitget_funding_history = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/history-fund-rate?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT","fundingRate":"0.00029","fundingTime":"1729990000000"}]}"#,
        )
        .create_async()
        .await;
    let bitget_mark_price = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/symbol-price?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT","markPrice":"70012.10","indexPrice":"70012.00","ts":"1730000000500"}]}"#,
        )
        .create_async()
        .await;
    let bitget_open_interest = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/open-interest?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","openInterest":"345.67","openInterestValue":"3456700","ts":"1730000000600"}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_market = sdk.market(ExchangeId::Binance).unwrap();
    let okx_market = sdk.market(ExchangeId::Okx).unwrap();
    let bitget_market = sdk.market(ExchangeId::Bitget).unwrap();

    let binance_funding = binance_market.funding_rate(&btc_perp).await.unwrap();
    let binance_history = binance_market
        .funding_rate_history(FundingRateQuery::new(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let binance_mark = binance_market.mark_price(&btc_perp).await.unwrap();
    let binance_oi = binance_market.open_interest(&btc_perp).await.unwrap();

    let okx_funding_rate = okx_market.funding_rate(&btc_perp).await.unwrap();
    let okx_history = okx_market
        .funding_rate_history(FundingRateQuery::new(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let okx_mark = okx_market.mark_price(&btc_perp).await.unwrap();
    let okx_oi = okx_market.open_interest(&btc_perp).await.unwrap();

    let bitget_funding_rate = bitget_market.funding_rate(&btc_perp).await.unwrap();
    let bitget_history = bitget_market
        .funding_rate_history(FundingRateQuery::new(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let bitget_mark = bitget_market.mark_price(&btc_perp).await.unwrap();
    let bitget_oi = bitget_market.open_interest(&btc_perp).await.unwrap();

    assert_eq!(binance_funding.funding_rate, "0.0001");
    assert_eq!(binance_funding.next_funding_time, Some(1730000300000));
    assert_eq!(binance_history[0].funding_rate, "0.00009");
    assert_eq!(binance_mark.mark_price, "70010.10");
    assert_eq!(binance_mark.index_price.as_deref(), Some("70009.90"));
    assert_eq!(binance_oi.open_interest, "123.45");

    assert_eq!(okx_funding_rate.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx_funding_rate.funding_rate, "0.0002");
    assert_eq!(okx_history[0].funding_rate, "0.00019");
    assert_eq!(okx_mark.mark_price, "70011.10");
    assert_eq!(okx_oi.open_interest, "234.56");
    assert_eq!(okx_oi.open_interest_value.as_deref(), Some("2345600"));

    assert_eq!(bitget_funding_rate.exchange_symbol, "BTCUSDT");
    assert_eq!(bitget_funding_rate.funding_rate, "0.0003");
    assert_eq!(bitget_history[0].funding_rate, "0.00029");
    assert_eq!(bitget_mark.mark_price, "70012.10");
    assert_eq!(bitget_mark.index_price.as_deref(), Some("70012.00"));
    assert_eq!(bitget_oi.open_interest, "345.67");

    binance_premium.assert_async().await;
    binance_funding_history.assert_async().await;
    binance_open_interest.assert_async().await;
    okx_funding.assert_async().await;
    okx_funding_history.assert_async().await;
    okx_mark_price.assert_async().await;
    okx_open_interest.assert_async().await;
    bitget_funding.assert_async().await;
    bitget_funding_history.assert_async().await;
    bitget_mark_price.assert_async().await;
    bitget_open_interest.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_market_sentiment_stats() {
    let mut binance_server = Server::new_async().await;
    let binance_ratio = binance_server
        .mock(
            "GET",
            "/futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "longShortRatio":"1.10",
                "longAccount":"0.52",
                "shortAccount":"0.48",
                "timestamp":1730000000000
            }]"#,
        )
        .create_async()
        .await;
    let binance_taker = binance_server
        .mock(
            "GET",
            "/futures/data/takerlongshortRatio?symbol=BTCUSDT&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "buySellRatio":"1.50",
                "buyVol":"12",
                "sellVol":"8",
                "timestamp":1730000000100
            }]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_ratio = okx_server
        .mock(
            "GET",
            "/api/v5/rubik/stat/contracts/long-short-account-ratio-contract-top-trader?instId=BTC-USDT-SWAP&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[["1730000000200","1.20"]]
            }"#,
        )
        .create_async()
        .await;
    let okx_taker = okx_server
        .mock(
            "GET",
            "/api/v5/rubik/stat/taker-volume-contract?instId=BTC-USDT-SWAP&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[["1730000000300","9","13"]]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_ratio = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/account-long-short?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT","longShortRatio":"1.30","longAccountRatio":"0.565","shortAccountRatio":"0.435","ts":"1730000000400"}]}"#,
        )
        .create_async()
        .await;
    let bitget_taker = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/taker-buy-sell?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT","buyVolume":"14","sellVolume":"10","buySellRatio":"1.40","ts":"1730000000500"}]}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");
    let query = MarketStatsQuery::new(btc_perp.clone(), "5m").with_limit(2);

    let binance_ratio_items = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .long_short_ratio(query.clone())
        .await
        .unwrap();
    let binance_taker_items = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .taker_buy_sell_volume(query.clone())
        .await
        .unwrap();
    let okx_ratio_items = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .long_short_ratio(query.clone())
        .await
        .unwrap();
    let okx_taker_items = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .taker_buy_sell_volume(query.clone())
        .await
        .unwrap();
    let bitget_ratio_items = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .long_short_ratio(query.clone())
        .await
        .unwrap();
    let bitget_taker_items = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .taker_buy_sell_volume(query)
        .await
        .unwrap();

    assert_eq!(binance_ratio_items[0].ratio, "1.10");
    assert_eq!(binance_ratio_items[0].long_ratio.as_deref(), Some("0.52"));
    assert_eq!(
        binance_taker_items[0].buy_sell_ratio.as_deref(),
        Some("1.50")
    );
    assert_eq!(binance_taker_items[0].buy_volume, "12");
    assert_eq!(okx_ratio_items[0].ratio, "1.20");
    assert_eq!(okx_taker_items[0].sell_volume, "9");
    assert_eq!(okx_taker_items[0].buy_volume, "13");
    assert_eq!(bitget_ratio_items[0].exchange_symbol, "BTCUSDT");
    assert_eq!(bitget_ratio_items[0].ratio, "1.30");
    assert_eq!(bitget_taker_items[0].buy_volume, "14");
    assert_eq!(bitget_taker_items[0].sell_volume, "10");

    binance_ratio.assert_async().await;
    binance_taker.assert_async().await;
    okx_ratio.assert_async().await;
    okx_taker.assert_async().await;
    bitget_ratio.assert_async().await;
    bitget_taker.assert_async().await;
}

fn configured_sdk(binance_url: String, okx_url: String, bitget_url: String) -> CryptoSdk {
    CryptoSdk::from_config(SdkConfig {
        okx: Some(OkxExchangeConfig {
            api_key: "okx-key".to_string(),
            api_secret: "okx-secret".to_string(),
            passphrase: "okx-pass".to_string(),
            simulated: true,
            api_url: Some(okx_url),
            request_expiration_ms: Some(1_000),
        }),
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
        bitget: Some(BitgetExchangeConfig {
            api_key: "bitget-key".to_string(),
            api_secret: "bitget-secret".to_string(),
            passphrase: "bitget-pass".to_string(),
            api_url: Some(bitget_url),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            product_type: Some("USDT-FUTURES".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap()
}
