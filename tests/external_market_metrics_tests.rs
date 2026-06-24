use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CryptoSdk, ExchangeId,
    FundingRateQuery, GateExchangeConfig, Instrument, MarketStatsQuery, OkxExchangeConfig,
    SdkConfig,
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

    let mut gate_server = Server::new_async().await;
    let gate_ticker = gate_server
        .mock("GET", "/futures/usdt/tickers?contract=BTC_USDT")
        .expect(3)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "contract":"BTC_USDT",
                "last":"70013.00",
                "mark_price":"70013.10",
                "index_price":"70013.00",
                "funding_rate":"0.0004",
                "funding_next_apply":1730000900,
                "total_size":"456.78"
            }]"#,
        )
        .create_async()
        .await;
    let gate_funding_history = gate_server
        .mock(
            "GET",
            "/futures/usdt/funding_rate?contract=BTC_USDT&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"t":1729990000,"r":"0.00039"}]"#)
        .create_async()
        .await;
    let gate_all_tickers = gate_server
        .mock("GET", "/futures/usdt/tickers")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "contract":"ETH_USDT",
                "last":"3900",
                "mark_price":"3901",
                "funding_rate":"0.0002",
                "total_size":"789"
            }]"#,
        )
        .create_async()
        .await;

    let mut bybit_server = Server::new_async().await;
    let bybit_ticker = bybit_server
        .mock("GET", "/v5/market/tickers?category=linear&symbol=BTCUSDT")
        .expect(2)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{
                    "symbol":"BTCUSDT",
                    "lastPrice":"70014.00",
                    "markPrice":"70014.10",
                    "indexPrice":"70014.00",
                    "fundingRate":"0.0005",
                    "nextFundingTime":"1730001000000"
                }]}
            }"#,
        )
        .create_async()
        .await;
    let bybit_funding_history = bybit_server
        .mock(
            "GET",
            "/v5/market/funding/history?category=linear&limit=2&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{"symbol":"BTCUSDT","fundingRate":"0.00049","fundingRateTimestamp":"1729990000000"}]}
            }"#,
        )
        .create_async()
        .await;
    let bybit_open_interest = bybit_server
        .mock(
            "GET",
            "/v5/market/open-interest?category=linear&intervalTime=5min&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{"openInterest":"567.89","timestamp":"1730000000700"}]}
            }"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(
        binance_server.url(),
        okx_server.url(),
        bitget_server.url(),
        Some(bybit_server.url()),
        Some(gate_server.url()),
    );
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_market = sdk.market(ExchangeId::Binance).unwrap();
    let okx_market = sdk.market(ExchangeId::Okx).unwrap();
    let bitget_market = sdk.market(ExchangeId::Bitget).unwrap();
    let bybit_market = sdk.market(ExchangeId::Bybit).unwrap();
    let gate_market = sdk.market(ExchangeId::Gate).unwrap();

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

    let bybit_funding_rate = bybit_market.funding_rate(&btc_perp).await.unwrap();
    let bybit_history = bybit_market
        .funding_rate_history(FundingRateQuery::new(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let bybit_mark = bybit_market.mark_price(&btc_perp).await.unwrap();
    let bybit_oi = bybit_market.open_interest(&btc_perp).await.unwrap();

    let gate_funding_rate = gate_market.funding_rate(&btc_perp).await.unwrap();
    let gate_history = gate_market
        .funding_rate_history(FundingRateQuery::new(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let gate_mark = gate_market.mark_price(&btc_perp).await.unwrap();
    let gate_oi = gate_market.open_interest(&btc_perp).await.unwrap();
    let gate_tickers = gate_market.tickers("usdt").await.unwrap();

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

    assert_eq!(bybit_funding_rate.exchange_symbol, "BTCUSDT");
    assert_eq!(bybit_funding_rate.funding_rate, "0.0005");
    assert_eq!(bybit_funding_rate.next_funding_time, Some(1730001000000));
    assert_eq!(bybit_history[0].funding_rate, "0.00049");
    assert_eq!(bybit_mark.mark_price, "70014.10");
    assert_eq!(bybit_mark.index_price.as_deref(), Some("70014.00"));
    assert_eq!(bybit_oi.open_interest, "567.89");

    assert_eq!(gate_funding_rate.exchange_symbol, "BTC_USDT");
    assert_eq!(gate_funding_rate.funding_rate, "0.0004");
    assert_eq!(gate_funding_rate.next_funding_time, Some(1730000900000));
    assert_eq!(gate_history[0].funding_rate, "0.00039");
    assert_eq!(gate_history[0].funding_time, Some(1729990000000));
    assert_eq!(gate_mark.mark_price, "70013.10");
    assert_eq!(gate_mark.index_price.as_deref(), Some("70013.00"));
    assert_eq!(gate_oi.open_interest, "456.78");
    assert_eq!(gate_tickers[0].exchange_symbol, "ETH_USDT");
    assert_eq!(gate_tickers[0].last_price, "3900");

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
    bybit_ticker.assert_async().await;
    bybit_funding_history.assert_async().await;
    bybit_open_interest.assert_async().await;
    gate_ticker.assert_async().await;
    gate_funding_history.assert_async().await;
    gate_all_tickers.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_market_sentiment_stats() {
    let mut binance_server = Server::new_async().await;
    let binance_ratio = binance_server
        .mock(
            "GET",
            "/futures/data/topLongShortAccountRatio?symbol=BTCUSDT&period=5m&limit=2",
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

    let mut bybit_server = Server::new_async().await;
    let bybit_ratio = bybit_server
        .mock(
            "GET",
            "/v5/market/account-ratio?category=linear&limit=2&period=5min&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{"symbol":"BTCUSDT","buyRatio":"0.60","sellRatio":"0.40","timestamp":"1730000000600"}]}
            }"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(
        binance_server.url(),
        okx_server.url(),
        bitget_server.url(),
        Some(bybit_server.url()),
        None,
    );
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
    let bybit_ratio_items = sdk
        .market(ExchangeId::Bybit)
        .unwrap()
        .long_short_ratio(MarketStatsQuery::new(btc_perp, "5m").with_limit(2))
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
    assert_eq!(bybit_ratio_items[0].exchange_symbol, "BTCUSDT");
    assert_eq!(bybit_ratio_items[0].ratio, "1.5");
    assert_eq!(bybit_ratio_items[0].long_ratio.as_deref(), Some("0.60"));
    assert_eq!(bybit_ratio_items[0].short_ratio.as_deref(), Some("0.40"));

    binance_ratio.assert_async().await;
    binance_taker.assert_async().await;
    okx_ratio.assert_async().await;
    okx_taker.assert_async().await;
    bitget_ratio.assert_async().await;
    bitget_taker.assert_async().await;
    bybit_ratio.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_top_trader_position_ratio() {
    let mut binance_server = Server::new_async().await;
    let binance_position_ratio = binance_server
        .mock(
            "GET",
            "/futures/data/topLongShortPositionRatio?symbol=BTCUSDT&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "longShortRatio":"1.40",
                "longPosition":"0.58",
                "shortPosition":"0.42",
                "timestamp":1730000000800
            }]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_position_ratio = okx_server
        .mock(
            "GET",
            "/api/v5/rubik/stat/contracts/long-short-position-ratio-contract-top-trader?instId=BTC-USDT-SWAP&period=5m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[["1730000000900","1.60"]]
            }"#,
        )
        .create_async()
        .await;

    let bitget_server = Server::new_async().await;
    let gate_server = Server::new_async().await;

    let sdk = configured_sdk(
        binance_server.url(),
        okx_server.url(),
        bitget_server.url(),
        None,
        Some(gate_server.url()),
    );
    let btc_perp = Instrument::perp("BTC", "USDT");
    let query = MarketStatsQuery::new(btc_perp, "5m").with_limit(2);

    let binance_items = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .top_trader_position_ratio(query.clone())
        .await
        .unwrap();
    let okx_items = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .top_trader_position_ratio(query.clone())
        .await
        .unwrap();
    let bitget_error = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .top_trader_position_ratio(query.clone())
        .await
        .expect_err("Bitget top-trader position ratio should be explicit unsupported");
    let gate_error = sdk
        .market(ExchangeId::Gate)
        .unwrap()
        .top_trader_position_ratio(query)
        .await
        .expect_err("Gate top-trader position ratio should be explicit unsupported");

    assert_eq!(binance_items[0].exchange_symbol, "BTCUSDT");
    assert_eq!(binance_items[0].ratio, "1.40");
    assert_eq!(binance_items[0].long_ratio.as_deref(), Some("0.58"));
    assert_eq!(binance_items[0].short_ratio.as_deref(), Some("0.42"));
    assert_eq!(binance_items[0].timestamp, Some(1_730_000_000_800));

    assert_eq!(okx_items[0].exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx_items[0].ratio, "1.60");
    assert_eq!(okx_items[0].timestamp, Some(1_730_000_000_900));

    assert!(bitget_error.to_string().contains("bitget"));
    assert!(
        bitget_error
            .to_string()
            .contains("top trader position ratio")
    );
    assert!(gate_error.to_string().contains("gate"));
    assert!(gate_error.to_string().contains("top trader position ratio"));

    binance_position_ratio.assert_async().await;
    okx_position_ratio.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_open_interest_history() {
    let mut binance_server = Server::new_async().await;
    let binance_oi_history = binance_server
        .mock(
            "GET",
            "/futures/data/openInterestHist?symbol=BTCUSDT&period=5m&limit=2&startTime=1700000000000&endTime=1700007200000",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "sumOpenInterest":"123.45",
                "sumOpenInterestValue":"1234500",
                "timestamp":1700000100000
            }]"#,
        )
        .create_async()
        .await;

    let mut bybit_server = Server::new_async().await;
    let bybit_oi_history = bybit_server
        .mock(
            "GET",
            "/v5/market/open-interest?category=linear&endTime=1700007200000&intervalTime=5min&limit=2&startTime=1700000000000&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{"openInterest":"567.89","timestamp":"1700000200000"}]}
            }"#,
        )
        .create_async()
        .await;

    let gate_server = Server::new_async().await;
    let sdk = CryptoSdk::from_config(SdkConfig {
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
        bybit: Some(BybitExchangeConfig {
            api_key: "bybit-key".to_string(),
            api_secret: "bybit-secret".to_string(),
            api_url: Some(bybit_server.url()),
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
            category: Some("linear".to_string()),
        }),
        gate: Some(GateExchangeConfig {
            api_key: "gate-key".to_string(),
            api_secret: "gate-secret".to_string(),
            api_url: Some(gate_server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            settle: Some("usdt".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let btc_perp = Instrument::perp("BTC", "USDT");
    let query = MarketStatsQuery::new(btc_perp, "5m")
        .with_limit(2)
        .with_start_time(1_700_000_000_000)
        .with_end_time(1_700_007_200_000);

    let binance_history = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .open_interest_history(query.clone())
        .await
        .unwrap();
    let bybit_history = sdk
        .market(ExchangeId::Bybit)
        .unwrap()
        .open_interest_history(query.clone())
        .await
        .unwrap();
    let gate_error = sdk
        .market(ExchangeId::Gate)
        .unwrap()
        .open_interest_history(query)
        .await
        .expect_err("Gate open interest history should be explicit unsupported");

    assert_eq!(binance_history[0].exchange_symbol, "BTCUSDT");
    assert_eq!(binance_history[0].open_interest, "123.45");
    assert_eq!(
        binance_history[0].open_interest_value.as_deref(),
        Some("1234500")
    );
    assert_eq!(binance_history[0].timestamp, Some(1_700_000_100_000));

    assert_eq!(bybit_history[0].exchange_symbol, "BTCUSDT");
    assert_eq!(bybit_history[0].open_interest, "567.89");
    assert_eq!(bybit_history[0].timestamp, Some(1_700_000_200_000));

    assert!(gate_error.to_string().contains("gate"));
    assert!(gate_error.to_string().contains("open interest history"));

    binance_oi_history.assert_async().await;
    bybit_oi_history.assert_async().await;
}

fn configured_sdk(
    binance_url: String,
    okx_url: String,
    bitget_url: String,
    bybit_url: Option<String>,
    gate_url: Option<String>,
) -> CryptoSdk {
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
        bybit: bybit_url.map(|api_url| BybitExchangeConfig {
            api_key: "bybit-key".to_string(),
            api_secret: "bybit-secret".to_string(),
            api_url: Some(api_url),
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
            category: Some("linear".to_string()),
        }),
        gate: gate_url.map(|api_url| GateExchangeConfig {
            api_key: "gate-key".to_string(),
            api_secret: "gate-secret".to_string(),
            api_url: Some(api_url),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            settle: Some("usdt".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap()
}
