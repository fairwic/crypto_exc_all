use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CancelOrderRequest,
    CandleQuery, CryptoSdk, EnsureOrderMarginModeRequest, Error, ExchangeId, FillListQuery,
    Instrument, MarginMode, MarginModeApplyMethod, OkxExchangeConfig, OrderBookQuery,
    OrderListQuery, OrderQuery, OrderSide, OrderType, PlaceOrderRequest, PositionMode,
    PrepareOrderSettingsRequest, SdkConfig, SetLeverageRequest, SetPositionModeRequest,
    SetSymbolMarginModeRequest, TimeInForce,
};
use mockito::{Matcher, Server};

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
        ..SdkConfig::default()
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

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_orderbook_and_candles() {
    let mut binance_server = Server::new_async().await;
    let binance_orderbook = binance_server
        .mock("GET", "/fapi/v1/depth")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "5".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "lastUpdateId":42,
                "E":1730000000000,
                "bids":[["59999.90","0.5"]],
                "asks":[["60000.10","0.4"]]
            }"#,
        )
        .create_async()
        .await;
    let binance_candles = binance_server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[[1730000000000,"59000","61000","58000","60000","12.34",1730000059999,"740400","10","6","360000","0"]]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_orderbook = okx_server
        .mock("GET", "/api/v5/market/books?instId=BTC-USDT-SWAP&sz=5")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instId":"BTC-USDT-SWAP",
                    "bids":[["59999.80","0.6","0","2"]],
                    "asks":[["60000.20","0.7","0","3"]],
                    "ts":"1730000000001"
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_candles = okx_server
        .mock(
            "GET",
            "/api/v5/market/candles?instId=BTC-USDT-SWAP&bar=1m&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[["1730000000001","59001","61001","58001","60001","12.35","12.35","740401","1"]]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_orderbook = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/orderbook?limit=5&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"bids":[["59999.70","0.8"]],"asks":[["60000.30","0.9"]],"ts":"1730000000002"}}"#,
        )
        .create_async()
        .await;
    let bitget_candles = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/market/candles?granularity=1m&limit=2&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":[["1730000000002","59002","61002","58002","60002","12.36","740402"]]}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_book = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .orderbook(OrderBookQuery::new(btc_perp.clone()).with_limit(5))
        .await
        .unwrap();
    let binance_candle_items = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .candles(CandleQuery::new(btc_perp.clone(), "1m").with_limit(2))
        .await
        .unwrap();
    let okx_book = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .orderbook(OrderBookQuery::new(btc_perp.clone()).with_limit(5))
        .await
        .unwrap();
    let okx_candle_items = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .candles(CandleQuery::new(btc_perp.clone(), "1m").with_limit(2))
        .await
        .unwrap();
    let bitget_book = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .orderbook(OrderBookQuery::new(btc_perp.clone()).with_limit(5))
        .await
        .unwrap();
    let bitget_candle_items = sdk
        .market(ExchangeId::Bitget)
        .unwrap()
        .candles(CandleQuery::new(btc_perp, "1m").with_limit(2))
        .await
        .unwrap();

    assert_eq!(binance_book.bids[0].price, "59999.90");
    assert_eq!(binance_book.asks[0].size, "0.4");
    assert_eq!(binance_candle_items[0].close, "60000");
    assert_eq!(
        binance_candle_items[0].quote_volume.as_deref(),
        Some("740400")
    );
    assert_eq!(okx_book.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx_book.asks[0].price, "60000.20");
    assert_eq!(okx_candle_items[0].closed, Some(true));
    assert_eq!(bitget_book.bids[0].size, "0.8");
    assert_eq!(bitget_candle_items[0].open_time, Some(1730000000002));
    assert_eq!(
        bitget_candle_items[0].quote_volume.as_deref(),
        Some("740402")
    );

    binance_orderbook.assert_async().await;
    binance_candles.assert_async().await;
    okx_orderbook.assert_async().await;
    okx_candles.assert_async().await;
    bitget_orderbook.assert_async().await;
    bitget_candles.assert_async().await;
}

#[tokio::test]
async fn okx_adapter_normalizes_candle_interval_for_okx_bar_parameter() {
    let mut okx_server = Server::new_async().await;
    let okx_candles = okx_server
        .mock(
            "GET",
            "/api/v5/market/candles?instId=BTC-USDT-SWAP&bar=4H&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[["1730000000001","59001","61001","58001","60001","12.35","12.35","740401","1"]]
            }"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(
        "http://127.0.0.1:1".to_string(),
        okx_server.url(),
        "http://127.0.0.1:1".to_string(),
    );
    let candles = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .candles(CandleQuery::new(Instrument::perp("BTC", "USDT"), "4h").with_limit(2))
        .await
        .unwrap();

    assert_eq!(candles[0].exchange_symbol, "BTC-USDT-SWAP");
    okx_candles.assert_async().await;
}

#[tokio::test]
async fn binance_adapter_normalizes_candle_interval_for_binance_interval_parameter() {
    let mut binance_server = Server::new_async().await;
    let binance_4h_candles = binance_server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "4h".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[[1730000000000,"59000","61000","58000","60000","12.34",1730000059999,"740400","10","6","360000","0"]]"#,
        )
        .create_async()
        .await;
    let binance_1d_candles = binance_server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1d".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[[1730000000000,"59000","61000","58000","60000","12.34",1730000059999,"740400","10","6","360000","0"]]"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(
        binance_server.url(),
        "http://127.0.0.1:1".to_string(),
        "http://127.0.0.1:1".to_string(),
    );
    let btc_perp = Instrument::perp("BTC", "USDT");
    let candles_4h = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .candles(CandleQuery::new(btc_perp.clone(), "4H").with_limit(2))
        .await
        .unwrap();
    let candles_1d = sdk
        .market(ExchangeId::Binance)
        .unwrap()
        .candles(CandleQuery::new(btc_perp, "1DUTC").with_limit(2))
        .await
        .unwrap();

    assert_eq!(candles_4h[0].exchange_symbol, "BTCUSDT");
    assert_eq!(candles_1d[0].exchange_symbol, "BTCUSDT");
    binance_4h_candles.assert_async().await;
    binance_1d_candles.assert_async().await;
}

#[tokio::test]
async fn bybit_adapter_normalizes_candle_interval_for_bybit_kline_parameter() {
    let mut bybit_server = Server::new_async().await;
    let bybit_candles = bybit_server
        .mock("GET", "/v5/market/kline")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("category".into(), "linear".into()),
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{
                    "category":"linear",
                    "symbol":"BTCUSDT",
                    "list":[["1730000000002","59002","61002","58002","60002","12.36","740402"]]
                }
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        bybit: Some(BybitExchangeConfig {
            api_key: "bybit-key".to_string(),
            api_secret: "bybit-secret".to_string(),
            api_url: Some(bybit_server.url()),
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
            category: Some("linear".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let candles = sdk
        .market(ExchangeId::Bybit)
        .unwrap()
        .candles(CandleQuery::new(Instrument::perp("BTC", "USDT"), "1m").with_limit(2))
        .await
        .unwrap();

    assert_eq!(candles[0].exchange_symbol, "BTCUSDT");
    bybit_candles.assert_async().await;
}

#[tokio::test]
async fn okx_adapter_exposes_unified_tickers_with_24h_fields() {
    let mut okx_server = Server::new_async().await;
    let okx_tickers = okx_server
        .mock("GET", "/api/v5/market/tickers?instType=SWAP")
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
                    "volCcy24h":"456.7",
                    "vol24h":"100000",
                    "sodUtc0":"0",
                    "sodUtc8":"0",
                    "ts":"1730000000001"
                }]
            }"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(
        "http://127.0.0.1:1".to_string(),
        okx_server.url(),
        "http://127.0.0.1:1".to_string(),
    );
    let tickers = sdk
        .market(ExchangeId::Okx)
        .unwrap()
        .tickers("SWAP")
        .await
        .unwrap();

    assert_eq!(tickers.len(), 1);
    assert_eq!(tickers[0].exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(tickers[0].instrument.base, "BTC");
    assert_eq!(tickers[0].instrument.quote, "USDT");
    assert_eq!(tickers[0].instrument_type.as_deref(), Some("SWAP"));
    assert_eq!(tickers[0].volume_24h.as_deref(), Some("100000"));
    assert_eq!(tickers[0].quote_volume_24h.as_deref(), None);
    assert_eq!(tickers[0].base_volume_24h.as_deref(), Some("456.7"));
    assert_eq!(tickers[0].open_24h.as_deref(), Some("69000"));
    assert_eq!(tickers[0].high_24h.as_deref(), Some("71000"));
    assert_eq!(tickers[0].low_24h.as_deref(), Some("68000"));
    okx_tickers.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_positions() {
    let mut binance_server = Server::new_async().await;
    let binance_positions = binance_server
        .mock("GET", "/fapi/v3/positionRisk")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "positionSide":"LONG",
                "positionAmt":"0.010",
                "entryPrice":"69000",
                "markPrice":"70000",
                "unRealizedProfit":"10.5",
                "leverage":"20",
                "marginType":"cross",
                "liquidationPrice":"50000"
            }]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_positions = okx_server
        .mock(
            "GET",
            "/api/v5/account/positions?instType=SWAP&instId=BTC-USDT-SWAP",
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
                    "lever":"20",
                    "pos":"0.010",
                    "posSide":"long",
                    "avgPx":"69001",
                    "upl":"11.5",
                    "margin":"34.5",
                    "mgnMode":"cross",
                    "liqPx":"50001"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_positions = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/position/all-position?marginCoin=USDT&productType=USDT-FUTURES",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "data":[{
                    "symbol":"BTCUSDT",
                    "holdSide":"long",
                    "total":"0.010",
                    "openPriceAvg":"69002",
                    "markPrice":"70002",
                    "unrealizedPL":"12.5",
                    "leverage":"20",
                    "marginMode":"crossed",
                    "liquidationPrice":"50002"
                }]
            }"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance = sdk
        .positions(ExchangeId::Binance)
        .unwrap()
        .list(Some(&btc_perp))
        .await
        .unwrap();
    let okx = sdk
        .positions(ExchangeId::Okx)
        .unwrap()
        .list(Some(&btc_perp))
        .await
        .unwrap();
    let bitget = sdk
        .positions(ExchangeId::Bitget)
        .unwrap()
        .list(Some(&btc_perp))
        .await
        .unwrap();

    assert_eq!(binance[0].exchange_symbol, "BTCUSDT");
    assert_eq!(binance[0].size, "0.010");
    assert_eq!(binance[0].side.as_deref(), Some("LONG"));
    assert_eq!(okx[0].exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx[0].entry_price.as_deref(), Some("69001"));
    assert_eq!(okx[0].margin_mode.as_deref(), Some("cross"));
    assert_eq!(bitget[0].exchange_symbol, "BTCUSDT");
    assert_eq!(bitget[0].unrealized_pnl.as_deref(), Some("12.5"));
    assert_eq!(bitget[0].liquidation_price.as_deref(), Some("50002"));

    binance_positions.assert_async().await;
    okx_positions.assert_async().await;
    bitget_positions.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_set_leverage() {
    let mut binance_server = Server::new_async().await;
    let binance_leverage = binance_server
        .mock("POST", "/fapi/v1/leverage")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("leverage".into(), "20".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","leverage":20,"maxNotionalValue":"1000000"}"#)
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_leverage = okx_server
        .mock("POST", "/api/v5/account/set-leverage")
        .match_body(Matcher::JsonString(
            r#"{"instId":"BTC-USDT-SWAP","lever":"20","mgnMode":"cross","posSide":"long"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "instId":"BTC-USDT-SWAP",
                    "lever":"20",
                    "mgnMode":"cross",
                    "posSide":"long"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_leverage = bitget_server
        .mock("POST", "/api/v2/mix/account/set-leverage")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","leverage":"20"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","leverage":"20"}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");
    let request = SetLeverageRequest::new(btc_perp.clone(), "20")
        .with_margin_mode("cross")
        .with_margin_coin("USDT")
        .with_position_side("long");

    let binance = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .set_leverage(request.clone())
        .await
        .unwrap();
    let okx = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .set_leverage(request.clone())
        .await
        .unwrap();
    let bitget = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .set_leverage(request)
        .await
        .unwrap();

    assert_eq!(binance.exchange_symbol, "BTCUSDT");
    assert_eq!(binance.leverage, "20");
    assert_eq!(okx.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx.margin_mode.as_deref(), Some("cross"));
    assert_eq!(okx.position_side.as_deref(), Some("long"));
    assert_eq!(bitget.exchange_symbol, "BTCUSDT");
    assert_eq!(bitget.leverage, "20");

    binance_leverage.assert_async().await;
    okx_leverage.assert_async().await;
    bitget_leverage.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_set_position_mode() {
    let mut binance_server = Server::new_async().await;
    let binance_position_mode = binance_server
        .mock("POST", "/fapi/v1/positionSide/dual")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("dualSidePosition".into(), "true".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_config = okx_server
        .mock("GET", "/api/v5/account/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"posMode":"net_mode"}]}"#)
        .create_async()
        .await;
    let okx_position_mode = okx_server
        .mock("POST", "/api/v5/account/set-position-mode")
        .match_body(Matcher::JsonString(
            r#"{"posMode":"long_short_mode"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "posMode":"long_short_mode"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_position_mode = bitget_server
        .mock("POST", "/api/v2/mix/account/set-position-mode")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","posMode":"hedge_mode"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"posMode":"hedge_mode"}}"#)
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let request =
        SetPositionModeRequest::new(PositionMode::Hedge).with_product_type("USDT-FUTURES");

    let binance = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .set_position_mode(request.clone())
        .await
        .unwrap();
    let okx = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .set_position_mode(request.clone())
        .await
        .unwrap();
    let bitget = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .set_position_mode(request)
        .await
        .unwrap();

    assert_eq!(binance.mode, PositionMode::Hedge);
    assert_eq!(binance.raw_mode.as_deref(), Some("true"));
    assert_eq!(okx.mode, PositionMode::Hedge);
    assert_eq!(okx.raw_mode.as_deref(), Some("long_short_mode"));
    assert_eq!(bitget.mode, PositionMode::Hedge);
    assert_eq!(bitget.raw_mode.as_deref(), Some("hedge_mode"));

    binance_position_mode.assert_async().await;
    okx_config.assert_async().await;
    okx_position_mode.assert_async().await;
    bitget_position_mode.assert_async().await;
}

#[tokio::test]
async fn external_consumer_can_discover_margin_capabilities_and_set_symbol_margin_mode() {
    let mut binance_server = Server::new_async().await;
    let binance_margin_mode = binance_server
        .mock("POST", "/fapi/v1/marginType")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("marginType".into(), "CROSSED".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;

    let okx_server = Server::new_async().await;

    let mut bitget_server = Server::new_async().await;
    let bitget_margin_mode = bitget_server
        .mock("POST", "/api/v2/mix/account/set-margin-mode")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","marginMode":"crossed"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","marginCoin":"USDT","marginMode":"crossed"}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_capabilities = sdk.account(ExchangeId::Binance).unwrap().capabilities();
    let okx_capabilities = sdk.account(ExchangeId::Okx).unwrap().capabilities();
    let bitget_capabilities = sdk.account(ExchangeId::Bitget).unwrap().capabilities();

    assert!(binance_capabilities.set_symbol_margin_mode);
    assert!(!binance_capabilities.order_level_margin_mode);
    assert!(!okx_capabilities.set_symbol_margin_mode);
    assert!(okx_capabilities.order_level_margin_mode);
    assert!(bitget_capabilities.set_symbol_margin_mode);
    assert!(bitget_capabilities.order_level_margin_mode);

    let request = SetSymbolMarginModeRequest::new(btc_perp.clone(), MarginMode::Cross)
        .with_product_type("USDT-FUTURES")
        .with_margin_coin("USDT");

    let binance = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .set_symbol_margin_mode(request.clone())
        .await
        .unwrap();
    let bitget = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .set_symbol_margin_mode(request.clone())
        .await
        .unwrap();
    let okx_error = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .set_symbol_margin_mode(request)
        .await
        .unwrap_err();

    assert_eq!(binance.exchange_symbol, "BTCUSDT");
    assert_eq!(binance.mode, MarginMode::Cross);
    assert_eq!(binance.raw_mode.as_deref(), Some("CROSSED"));
    assert_eq!(bitget.exchange_symbol, "BTCUSDT");
    assert_eq!(bitget.mode, MarginMode::Cross);
    assert_eq!(bitget.raw_mode.as_deref(), Some("crossed"));
    assert!(matches!(
        okx_error,
        Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "set_symbol_margin_mode"
        }
    ));

    binance_margin_mode.assert_async().await;
    bitget_margin_mode.assert_async().await;
}

#[tokio::test]
async fn external_consumer_can_ensure_order_margin_mode_without_exchange_branching() {
    let mut binance_server = Server::new_async().await;
    let binance_margin_mode = binance_server
        .mock("POST", "/fapi/v1/marginType")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("marginType".into(), "ISOLATED".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;

    let okx_server = Server::new_async().await;

    let mut bitget_server = Server::new_async().await;
    let bitget_margin_mode = bitget_server
        .mock("POST", "/api/v2/mix/account/set-margin-mode")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","marginMode":"isolated"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","marginCoin":"USDT","marginMode":"isolated"}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");
    let request = EnsureOrderMarginModeRequest::new(btc_perp.clone(), MarginMode::Isolated)
        .with_product_type("USDT-FUTURES")
        .with_margin_coin("USDT");

    let binance = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .ensure_order_margin_mode(request.clone())
        .await
        .unwrap();
    let okx = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .ensure_order_margin_mode(request.clone())
        .await
        .unwrap();
    let bitget = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .ensure_order_margin_mode(request)
        .await
        .unwrap();

    assert_eq!(binance.exchange_symbol, "BTCUSDT");
    assert_eq!(binance.mode, MarginMode::Isolated);
    assert_eq!(
        binance.apply_method,
        MarginModeApplyMethod::SymbolConfiguration
    );
    assert_eq!(binance.raw_mode.as_deref(), Some("ISOLATED"));

    assert_eq!(okx.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(okx.mode, MarginMode::Isolated);
    assert_eq!(okx.apply_method, MarginModeApplyMethod::OrderLevel);
    assert_eq!(okx.raw_mode.as_deref(), Some("isolated"));

    assert_eq!(bitget.exchange_symbol, "BTCUSDT");
    assert_eq!(bitget.mode, MarginMode::Isolated);
    assert_eq!(
        bitget.apply_method,
        MarginModeApplyMethod::SymbolConfiguration
    );
    assert_eq!(bitget.raw_mode.as_deref(), Some("isolated"));

    binance_margin_mode.assert_async().await;
    bitget_margin_mode.assert_async().await;
}

#[tokio::test]
async fn external_consumer_can_prepare_order_settings_without_exchange_branching() {
    let mut binance_server = Server::new_async().await;
    let binance_position_mode = binance_server
        .mock("POST", "/fapi/v1/positionSide/dual")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("dualSidePosition".into(), "true".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;
    let binance_margin_mode = binance_server
        .mock("POST", "/fapi/v1/marginType")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("marginType".into(), "ISOLATED".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;
    let binance_leverage = binance_server
        .mock("POST", "/fapi/v1/leverage")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("leverage".into(), "15".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","leverage":15,"maxNotionalValue":"1000000"}"#)
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_config = okx_server
        .mock("GET", "/api/v5/account/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"posMode":"net_mode"}]}"#)
        .create_async()
        .await;
    let okx_position_mode = okx_server
        .mock("POST", "/api/v5/account/set-position-mode")
        .match_body(Matcher::JsonString(
            r#"{"posMode":"long_short_mode"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"posMode":"long_short_mode"}]}"#)
        .create_async()
        .await;
    let okx_leverage = okx_server
        .mock("POST", "/api/v5/account/set-leverage")
        .match_body(Matcher::JsonString(
            r#"{"instId":"BTC-USDT-SWAP","lever":"15","mgnMode":"isolated","posSide":"long"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"instId":"BTC-USDT-SWAP","lever":"15","mgnMode":"isolated","posSide":"long"}]}"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_position_mode = bitget_server
        .mock("POST", "/api/v2/mix/account/set-position-mode")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","posMode":"hedge_mode"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"posMode":"hedge_mode"}}"#)
        .create_async()
        .await;
    let bitget_margin_mode = bitget_server
        .mock("POST", "/api/v2/mix/account/set-margin-mode")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","marginMode":"isolated"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","marginCoin":"USDT","marginMode":"isolated"}}"#,
        )
        .create_async()
        .await;
    let bitget_leverage = bitget_server
        .mock("POST", "/api/v2/mix/account/set-leverage")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","leverage":"15"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","leverage":"15"}}"#)
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");
    let request = PrepareOrderSettingsRequest::new(btc_perp.clone())
        .with_position_mode(PositionMode::Hedge)
        .with_margin_mode(MarginMode::Isolated)
        .with_leverage("15")
        .with_product_type("USDT-FUTURES")
        .with_margin_coin("USDT")
        .with_position_side("long");

    let binance = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .prepare_order_settings(request.clone())
        .await
        .unwrap();
    let okx = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .prepare_order_settings(request.clone())
        .await
        .unwrap();
    let bitget = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .prepare_order_settings(request)
        .await
        .unwrap();

    assert_eq!(binance.exchange_symbol, "BTCUSDT");
    assert_eq!(
        binance.position_mode.as_ref().unwrap().mode,
        PositionMode::Hedge
    );
    assert_eq!(
        binance.margin_mode.as_ref().unwrap().mode,
        MarginMode::Isolated
    );
    assert_eq!(
        binance.margin_mode.as_ref().unwrap().apply_method,
        MarginModeApplyMethod::SymbolConfiguration
    );
    assert_eq!(binance.leverage.as_ref().unwrap().leverage, "15");

    assert_eq!(okx.exchange_symbol, "BTC-USDT-SWAP");
    assert_eq!(
        okx.position_mode.as_ref().unwrap().mode,
        PositionMode::Hedge
    );
    assert_eq!(
        okx.margin_mode.as_ref().unwrap().apply_method,
        MarginModeApplyMethod::OrderLevel
    );
    assert_eq!(
        okx.margin_mode.as_ref().unwrap().raw_mode.as_deref(),
        Some("isolated")
    );
    assert_eq!(
        okx.leverage.as_ref().unwrap().margin_mode.as_deref(),
        Some("isolated")
    );
    assert_eq!(
        okx.leverage.as_ref().unwrap().position_side.as_deref(),
        Some("long")
    );

    assert_eq!(bitget.exchange_symbol, "BTCUSDT");
    assert_eq!(
        bitget.position_mode.as_ref().unwrap().mode,
        PositionMode::Hedge
    );
    assert_eq!(
        bitget.margin_mode.as_ref().unwrap().apply_method,
        MarginModeApplyMethod::SymbolConfiguration
    );
    assert_eq!(bitget.leverage.as_ref().unwrap().leverage, "15");

    binance_position_mode.assert_async().await;
    binance_margin_mode.assert_async().await;
    binance_leverage.assert_async().await;
    okx_config.assert_async().await;
    okx_position_mode.assert_async().await;
    okx_leverage.assert_async().await;
    bitget_position_mode.assert_async().await;
    bitget_margin_mode.assert_async().await;
    bitget_leverage.assert_async().await;
}

#[tokio::test]
async fn external_consumer_prepare_order_settings_treats_binance_noop_mode_and_margin_as_success() {
    let mut binance_server = Server::new_async().await;
    let binance_position_mode = binance_server
        .mock("POST", "/fapi/v1/positionSide/dual")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("dualSidePosition".into(), "true".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":-4059,"msg":"No need to change position side."}"#)
        .create_async()
        .await;
    let binance_margin_mode = binance_server
        .mock("POST", "/fapi/v1/marginType")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("marginType".into(), "CROSSED".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":-4046,"msg":"No need to change margin type."}"#)
        .create_async()
        .await;
    let binance_leverage = binance_server
        .mock("POST", "/fapi/v1/leverage")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("leverage".into(), "3".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Regex("(^|&)timestamp=".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"ETHUSDT","leverage":3,"maxNotionalValue":"1000000"}"#)
        .create_async()
        .await;

    let okx_server = Server::new_async().await;
    let bitget_server = Server::new_async().await;
    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let request = PrepareOrderSettingsRequest::new(Instrument::perp("ETH", "USDT"))
        .with_position_mode(PositionMode::Hedge)
        .with_margin_mode(MarginMode::Cross)
        .with_leverage("3")
        .with_product_type("USDT-FUTURES")
        .with_margin_coin("USDT")
        .with_position_side("long");

    let result = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .prepare_order_settings(request)
        .await
        .unwrap();

    assert_eq!(
        result.position_mode.as_ref().unwrap().mode,
        PositionMode::Hedge
    );
    assert_eq!(
        result.position_mode.as_ref().unwrap().raw["idempotent"],
        true
    );
    assert_eq!(result.margin_mode.as_ref().unwrap().mode, MarginMode::Cross);
    assert_eq!(result.margin_mode.as_ref().unwrap().raw["idempotent"], true);
    assert_eq!(result.leverage.as_ref().unwrap().leverage, "3");

    binance_position_mode.assert_async().await;
    binance_margin_mode.assert_async().await;
    binance_leverage.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_place_and_cancel_order() {
    let mut binance_server = Server::new_async().await;
    let binance_place = binance_server
        .mock("POST", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("side".into(), "BUY".into()),
            Matcher::UrlEncoded("type".into(), "LIMIT".into()),
            Matcher::UrlEncoded("timeInForce".into(), "GTX".into()),
            Matcher::UrlEncoded("quantity".into(), "0.001".into()),
            Matcher::UrlEncoded("price".into(), "60000".into()),
            Matcher::UrlEncoded("newClientOrderId".into(), "root-binance-1".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"symbol":"BTCUSDT","orderId":12345,"clientOrderId":"root-binance-1","status":"NEW"}"#,
        )
        .create_async()
        .await;
    let binance_cancel = binance_server
        .mock("DELETE", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderId".into(), "12345".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"symbol":"BTCUSDT","orderId":12345,"clientOrderId":"root-binance-1","status":"CANCELED"}"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_place = okx_server
        .mock("POST", "/api/v5/trade/order")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#""instId":"BTC-USDT-SWAP""#.into()),
            Matcher::Regex(r#""tdMode":"cross""#.into()),
            Matcher::Regex(r#""ordType":"post_only""#.into()),
            Matcher::Regex(r#""clOrdId":"root-okx-1""#.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "ordId":"okx-1",
                    "clOrdId":"root-okx-1",
                    "tag":"",
                    "ts":"1730000000100",
                    "sCode":"0",
                    "sMsg":""
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_cancel = okx_server
        .mock("POST", "/api/v5/trade/cancel-order")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#""instId":"BTC-USDT-SWAP""#.into()),
            Matcher::Regex(r#""ordId":"okx-1""#.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "ordId":"okx-1",
                    "clOrdId":"root-okx-1",
                    "sCode":"0",
                    "sMsg":""
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_place = bitget_server
        .mock("POST", "/api/v2/mix/order/place-order")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginMode":"crossed","marginCoin":"USDT","size":"0.001","side":"buy","orderType":"limit","price":"60000","force":"post_only","clientOid":"root-bitget-1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"orderId":"bitget-1","clientOid":"root-bitget-1"}}"#,
        )
        .create_async()
        .await;
    let bitget_cancel = bitget_server
        .mock("POST", "/api/v2/mix/order/cancel-order")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","orderId":"bitget-1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"orderId":"bitget-1","clientOid":"root-bitget-1"}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_order = sdk
        .trade(ExchangeId::Binance)
        .unwrap()
        .place_order(post_only_limit(
            btc_perp.clone(),
            "0.001",
            "60000",
            "root-binance-1",
        ))
        .await
        .unwrap();
    let binance_cancelled = sdk
        .trade(ExchangeId::Binance)
        .unwrap()
        .cancel_order(CancelOrderRequest::by_order_id(btc_perp.clone(), "12345"))
        .await
        .unwrap();

    let okx_order = sdk
        .trade(ExchangeId::Okx)
        .unwrap()
        .place_order(post_only_limit(
            btc_perp.clone(),
            "0.001",
            "60000",
            "root-okx-1",
        ))
        .await
        .unwrap();
    let okx_cancelled = sdk
        .trade(ExchangeId::Okx)
        .unwrap()
        .cancel_order(CancelOrderRequest::by_order_id(btc_perp.clone(), "okx-1"))
        .await
        .unwrap();

    let bitget_order = sdk
        .trade(ExchangeId::Bitget)
        .unwrap()
        .place_order(post_only_limit(
            btc_perp.clone(),
            "0.001",
            "60000",
            "root-bitget-1",
        ))
        .await
        .unwrap();
    let bitget_cancelled = sdk
        .trade(ExchangeId::Bitget)
        .unwrap()
        .cancel_order(CancelOrderRequest::by_order_id(btc_perp, "bitget-1"))
        .await
        .unwrap();

    assert_eq!(binance_order.order_id.as_deref(), Some("12345"));
    assert_eq!(binance_cancelled.status.as_deref(), Some("CANCELED"));
    assert_eq!(okx_order.order_id.as_deref(), Some("okx-1"));
    assert_eq!(okx_cancelled.client_order_id.as_deref(), Some("root-okx-1"));
    assert_eq!(bitget_order.order_id.as_deref(), Some("bitget-1"));
    assert_eq!(
        bitget_cancelled.client_order_id.as_deref(),
        Some("root-bitget-1")
    );

    binance_place.assert_async().await;
    binance_cancel.assert_async().await;
    okx_place.assert_async().await;
    okx_cancel.assert_async().await;
    bitget_place.assert_async().await;
    bitget_cancel.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_order_queries() {
    let mut binance_server = Server::new_async().await;
    let binance_detail = binance_server
        .mock("GET", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderId".into(), "12345".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "symbol":"BTCUSDT",
                "orderId":12345,
                "clientOrderId":"root-binance-1",
                "side":"BUY",
                "type":"LIMIT",
                "price":"60000",
                "origQty":"0.001",
                "executedQty":"0",
                "avgPrice":"0",
                "status":"NEW",
                "time":1730000000000,
                "updateTime":1730000000100
            }"#,
        )
        .create_async()
        .await;
    let binance_open = binance_server
        .mock("GET", "/fapi/v1/openOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "orderId":12346,
                "clientOrderId":"root-binance-open",
                "side":"SELL",
                "type":"LIMIT",
                "price":"61000",
                "origQty":"0.002",
                "executedQty":"0.001",
                "avgPrice":"60500",
                "status":"PARTIALLY_FILLED",
                "time":1730000000200,
                "updateTime":1730000000300
            }]"#,
        )
        .create_async()
        .await;
    let binance_history = binance_server
        .mock("GET", "/fapi/v1/allOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "orderId":12347,
                "clientOrderId":"root-binance-history",
                "side":"BUY",
                "type":"MARKET",
                "price":"0",
                "origQty":"0.003",
                "executedQty":"0.003",
                "avgPrice":"59900",
                "status":"FILLED",
                "time":1730000000400,
                "updateTime":1730000000500
            }]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_detail = okx_server
        .mock(
            "GET",
            "/api/v5/trade/order?instId=BTC-USDT-SWAP&ordId=okx-1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(okx_order_detail_body("okx-1", "root-okx-1", "live"))
        .create_async()
        .await;
    let okx_open = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-pending?instType=SWAP&instId=BTC-USDT-SWAP&limit=2",
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
                    "lever":"20",
                    "px":"61000",
                    "sz":"0.002",
                    "ordId":"okx-open",
                    "clOrdId":"root-okx-open",
                    "fillSz":"0.001",
                    "fillPx":"60500",
                    "fillTime":"1730000000300",
                    "ordType":"limit",
                    "side":"sell",
                    "posSide":"short",
                    "state":"partially_filled",
                    "cTime":"1730000000200",
                    "uTime":"1730000000300"
                }]
            }"#,
        )
        .create_async()
        .await;
    let okx_history = okx_server
        .mock(
            "GET",
            "/api/v5/trade/orders-history-archive?instType=SWAP&instId=BTC-USDT-SWAP&begin=1720000000000&end=1730000000000&limit=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(okx_order_detail_body(
            "okx-history",
            "root-okx-history",
            "filled",
        ))
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_detail = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/order/detail?orderId=bitget-1&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"symbol":"BTCUSDT","orderId":"bitget-1","clientOid":"root-bitget-1","side":"buy","orderType":"limit","price":"60000","size":"0.001","baseVolume":"0","priceAvg":"0","status":"live","cTime":"1730000000000","uTime":"1730000000100"}}"#,
        )
        .create_async()
        .await;
    let bitget_open = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/order/orders-pending?limit=2&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"entrustedList":[{"symbol":"BTCUSDT","orderId":"bitget-open","clientOid":"root-bitget-open","side":"sell","orderType":"limit","price":"61000","size":"0.002","baseVolume":"0.001","priceAvg":"60500","status":"partially_filled","cTime":"1730000000200","uTime":"1730000000300"}]}}"#,
        )
        .create_async()
        .await;
    let bitget_history = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/order/orders-history?limit=2&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"entrustedList":[{"symbol":"BTCUSDT","orderId":"bitget-history","clientOid":"root-bitget-history","side":"buy","orderType":"market","price":"0","size":"0.003","baseVolume":"0.003","priceAvg":"59900","status":"filled","cTime":"1730000000400","uTime":"1730000000500"}]}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_detail_order = sdk
        .orders(ExchangeId::Binance)
        .unwrap()
        .get(OrderQuery::by_order_id(btc_perp.clone(), "12345"))
        .await
        .unwrap();
    let binance_open_orders = sdk
        .orders(ExchangeId::Binance)
        .unwrap()
        .open(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let binance_history_orders = sdk
        .orders(ExchangeId::Binance)
        .unwrap()
        .history(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();

    let okx_detail_order = sdk
        .orders(ExchangeId::Okx)
        .unwrap()
        .get(OrderQuery::by_order_id(btc_perp.clone(), "okx-1"))
        .await
        .unwrap();
    let okx_open_orders = sdk
        .orders(ExchangeId::Okx)
        .unwrap()
        .open(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let okx_history_orders = sdk
        .orders(ExchangeId::Okx)
        .unwrap()
        .history(
            OrderListQuery::for_instrument(btc_perp.clone())
                .with_start_time(1720000000000)
                .with_end_time(1730000000000)
                .with_limit(2),
        )
        .await
        .unwrap();

    let bitget_detail_order = sdk
        .orders(ExchangeId::Bitget)
        .unwrap()
        .get(OrderQuery::by_order_id(btc_perp.clone(), "bitget-1"))
        .await
        .unwrap();
    let bitget_open_orders = sdk
        .orders(ExchangeId::Bitget)
        .unwrap()
        .open(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let bitget_history_orders = sdk
        .orders(ExchangeId::Bitget)
        .unwrap()
        .history(OrderListQuery::for_instrument(btc_perp).with_limit(2))
        .await
        .unwrap();

    assert_eq!(binance_detail_order.order_id.as_deref(), Some("12345"));
    assert_eq!(binance_open_orders[0].filled_size.as_deref(), Some("0.001"));
    assert_eq!(binance_history_orders[0].status.as_deref(), Some("FILLED"));
    assert_eq!(okx_detail_order.order_id.as_deref(), Some("okx-1"));
    assert_eq!(
        okx_open_orders[0].client_order_id.as_deref(),
        Some("root-okx-open")
    );
    assert_eq!(okx_history_orders[0].status.as_deref(), Some("filled"));
    assert_eq!(bitget_detail_order.order_id.as_deref(), Some("bitget-1"));
    assert_eq!(
        bitget_open_orders[0].average_price.as_deref(),
        Some("60500")
    );
    assert_eq!(
        bitget_history_orders[0].order_type.as_deref(),
        Some("market")
    );

    binance_detail.assert_async().await;
    binance_open.assert_async().await;
    binance_history.assert_async().await;
    okx_detail.assert_async().await;
    okx_open.assert_async().await;
    okx_history.assert_async().await;
    bitget_detail.assert_async().await;
    bitget_open.assert_async().await;
    bitget_history.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_unified_fills() {
    let mut binance_server = Server::new_async().await;
    let binance_fills = binance_server
        .mock("GET", "/fapi/v1/userTrades")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
            Matcher::Regex("(^|&)signature=".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "symbol":"BTCUSDT",
                "id":111,
                "orderId":12345,
                "side":"BUY",
                "price":"60000",
                "qty":"0.001",
                "quoteQty":"60",
                "commission":"-0.01",
                "commissionAsset":"USDT",
                "maker":false,
                "time":1730000000000
            }]"#,
        )
        .create_async()
        .await;

    let mut okx_server = Server::new_async().await;
    let okx_fills = okx_server
        .mock(
            "GET",
            "/api/v5/trade/fills-history?instType=SWAP&instId=BTC-USDT-SWAP&begin=1720000000000&end=1730000000000&limit=2",
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
                    "tradeId":"okx-trade-1",
                    "ordId":"okx-1",
                    "side":"sell",
                    "fillPx":"61000",
                    "fillSz":"0.002",
                    "fee":"-0.02",
                    "feeCcy":"USDT",
                    "execType":"M",
                    "ts":"1730000000100"
                }]
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget_fills = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/order/fills?limit=2&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"fillList":[{"symbol":"BTCUSDT","tradeId":"bitget-trade-1","orderId":"bitget-1","side":"buy","price":"60000","baseVolume":"0.001","fee":"-0.01","feeCcy":"USDT","role":"taker","cTime":"1730000000200"}]}}"#,
        )
        .create_async()
        .await;

    let sdk = configured_sdk(binance_server.url(), okx_server.url(), bitget_server.url());
    let btc_perp = Instrument::perp("BTC", "USDT");

    let binance_items = sdk
        .fills(ExchangeId::Binance)
        .unwrap()
        .list(FillListQuery::for_instrument(btc_perp.clone()).with_limit(2))
        .await
        .unwrap();
    let okx_items = sdk
        .fills(ExchangeId::Okx)
        .unwrap()
        .list(
            FillListQuery::for_instrument(btc_perp.clone())
                .with_start_time(1720000000000)
                .with_end_time(1730000000000)
                .with_limit(2),
        )
        .await
        .unwrap();
    let bitget_items = sdk
        .fills(ExchangeId::Bitget)
        .unwrap()
        .list(FillListQuery::for_instrument(btc_perp).with_limit(2))
        .await
        .unwrap();

    assert_eq!(binance_items[0].trade_id.as_deref(), Some("111"));
    assert_eq!(binance_items[0].role.as_deref(), Some("taker"));
    assert_eq!(okx_items[0].order_id.as_deref(), Some("okx-1"));
    assert_eq!(okx_items[0].role.as_deref(), Some("maker"));
    assert_eq!(bitget_items[0].trade_id.as_deref(), Some("bitget-trade-1"));
    assert_eq!(bitget_items[0].fee_asset.as_deref(), Some("USDT"));

    binance_fills.assert_async().await;
    okx_fills.assert_async().await;
    bitget_fills.assert_async().await;
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

fn post_only_limit(
    instrument: Instrument,
    size: &str,
    price: &str,
    client_order_id: &str,
) -> PlaceOrderRequest {
    PlaceOrderRequest::new(
        instrument,
        OrderSide::Buy,
        OrderType::Limit,
        size.to_string(),
    )
    .with_price(price)
    .with_time_in_force(TimeInForce::PostOnly)
    .with_client_order_id(client_order_id)
}

fn okx_order_detail_body(order_id: &str, client_order_id: &str, status: &str) -> String {
    r#"{
            "code":"0",
            "msg":"",
            "data":[{
                "instType":"SWAP",
                "instId":"BTC-USDT-SWAP",
                "tgtCcy":"",
                "ccy":"",
                "ordId":"$ORDER_ID",
                "clOrdId":"$CLIENT_ORDER_ID",
                "tag":"",
                "px":"60000",
                "pxUsd":"",
                "pxVol":"",
                "pxType":"",
                "sz":"0.001",
                "pnl":"",
                "ordType":"limit",
                "side":"buy",
                "posSide":"long",
                "tdMode":"cross",
                "accFillSz":"0",
                "fillPx":"",
                "tradeId":"",
                "fillSz":"",
                "fillTime":"",
                "avgPx":"0",
                "state":"$STATUS",
                "lever":"20",
                "attachAlgoClOrdId":"",
                "tpTriggerPx":"",
                "tpTriggerPxType":"",
                "tpOrdPx":"",
                "slTriggerPx":"",
                "slTriggerPxType":"",
                "slOrdPx":"",
                "attachAlgoOrds":[],
                "linkedAlgoOrd":{"algoId":""},
                "stpId":"",
                "stpMode":"",
                "feeCcy":"",
                "fee":"",
                "rebateCcy":"",
                "source":"",
                "rebate":"",
                "category":"normal",
                "reduceOnly":"false",
                "cancelSource":"",
                "cancelSourceReason":"",
                "quickMgnType":"",
                "algoClOrdId":"",
                "algoId":"",
                "isTpLimit":"false",
                "uTime":"1730000000100",
                "cTime":"1730000000000",
                "tradeQuoteCcy":"USDT"
            }]
        }"#
    .replace("$ORDER_ID", order_id)
    .replace("$CLIENT_ORDER_ID", client_order_id)
    .replace("$STATUS", status)
}
