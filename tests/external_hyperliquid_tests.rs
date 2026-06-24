use crypto_exc_all::{
    AccountBillQuery, CandleQuery, CryptoSdk, ExchangeId, FillListQuery, HyperliquidExchangeConfig,
    Instrument, OrderBookQuery, OrderListQuery, OrderQuery, SdkConfig,
};
use mockito::{Matcher, Server};

const USER_ADDRESS: &str = "0x00000000000000000000000000000000000000ab";

#[tokio::test]
async fn external_consumer_uses_root_crate_for_hyperliquid_read_only_api() {
    let mut server = Server::new_async().await;
    let meta_and_asset_ctxs = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"metaAndAssetCtxs"}"#.to_string()))
        .expect(3)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[
                {"universe":[{"name":"BTC","szDecimals":5,"maxLeverage":50},{"name":"ETH","szDecimals":4,"maxLeverage":50}]},
                [
                    {"dayNtlVlm":"1234567","funding":"0.0000125","impactPxs":["70001","70002"],"markPx":"70010.5","midPx":"70010.0","openInterest":"1234.56","oraclePx":"70009.9","premium":"0.0001","prevDayPx":"69000"},
                    {"dayNtlVlm":"7654321","funding":"0.0000225","markPx":"3900.5","midPx":"3900.0","openInterest":"2222.22","oraclePx":"3899.9","prevDayPx":"3800"}
                ]
            ]"#,
        )
        .create_async()
        .await;
    let l2_book = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"l2Book","coin":"BTC"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"coin":"BTC","time":1730000000000,"levels":[[{"px":"70000","sz":"1.2","n":3},{"px":"69999","sz":"2.4","n":1}],[{"px":"70001","sz":"0.8","n":2},{"px":"70002","sz":"1.6","n":1}]]}"#,
        )
        .create_async()
        .await;
    let candles = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"candleSnapshot","req":{"coin":"BTC","interval":"1m","startTime":1700000000000,"endTime":1700000060000}}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"t":1700000000000,"T":1700000060000,"s":"BTC","i":"1m","o":"70000","c":"70010","h":"70020","l":"69990","v":"12.5","n":42}]"#,
        )
        .create_async()
        .await;
    let funding_history = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"fundingHistory","coin":"BTC","startTime":1700000000000,"endTime":1700007200000}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"BTC","fundingRate":"0.00001","premium":"0.00002","time":1700000000000}]"#,
        )
        .create_async()
        .await;
    let predicted_fundings = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"predictedFundings"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[["ETH",[["HlPerp",{"fundingRate":"0.00002","nextFundingTime":1700003600000}]]],["BTC",[["BinPerp",{"fundingRate":"0.00003","nextFundingTime":1700003600000}],["HlPerp",{"fundingRate":"0.000015","nextFundingTime":1700007200000}]]]]"#,
        )
        .create_async()
        .await;
    let clearinghouse_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"clearinghouseState","user":"{USER_ADDRESS}"}}"#
        )))
        .expect(2)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "marginSummary":{"accountValue":"1000","totalMarginUsed":"250"},
                "withdrawable":"740",
                "assetPositions":[{
                    "type":"oneWay",
                    "position":{
                        "coin":"BTC",
                        "szi":"0.01",
                        "entryPx":"69000",
                        "positionValue":"700",
                        "unrealizedPnl":"10",
                        "liquidationPx":"50000",
                        "leverage":{"type":"cross","value":5}
                    }
                }]
            }"#,
        )
        .create_async()
        .await;
    let spot_clearinghouse_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"spotClearinghouseState","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"balances":[]}"#)
        .create_async()
        .await;
    let ledger = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            format!(
                r#"{{"type":"userNonFundingLedgerUpdates","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
            ),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"time":1700000000000,"hash":"0x1","delta":{"type":"deposit","usdc":"100.5"}}]"#)
        .create_async()
        .await;
    let user_funding = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"userFunding","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"time":1700000100000,"hash":"0x2","delta":{"type":"funding","coin":"BTC","usdc":"-0.12","fundingRate":"0.00001","szi":"0.01"}}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let btc_perp = Instrument::perp("BTC", "USDC");

    let ticker = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .ticker(&btc_perp)
        .await
        .unwrap();
    let book = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .orderbook(OrderBookQuery::new(btc_perp.clone()).with_limit(1))
        .await
        .unwrap();
    let candle_items = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .candles(
            CandleQuery::new(btc_perp.clone(), "1m")
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_000_060_000),
        )
        .await
        .unwrap();
    let funding = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .funding_rate(&btc_perp)
        .await
        .unwrap();
    let funding_items = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .funding_rate_history(
            crypto_exc_all::FundingRateQuery::new(btc_perp.clone())
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();
    let open_interest = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .open_interest(&btc_perp)
        .await
        .unwrap();
    let balances = sdk
        .account(ExchangeId::Hyperliquid)
        .unwrap()
        .balances()
        .await
        .unwrap();
    let positions = sdk
        .positions(ExchangeId::Hyperliquid)
        .unwrap()
        .list(Some(&btc_perp))
        .await
        .unwrap();
    let bills = sdk
        .account(ExchangeId::Hyperliquid)
        .unwrap()
        .bills(
            AccountBillQuery::new()
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(ticker.exchange_symbol, "BTC");
    assert_eq!(ticker.last_price, "70010.0");
    assert_eq!(ticker.quote_volume_24h.as_deref(), Some("1234567"));
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);
    assert_eq!(book.bids[0].price, "70000");
    assert_eq!(book.asks[0].size, "0.8");
    assert_eq!(candle_items[0].open, "70000");
    assert_eq!(candle_items[0].close, "70010");
    assert_eq!(funding.funding_rate, "0.0000125");
    assert_eq!(funding.mark_price.as_deref(), Some("70010.5"));
    assert_eq!(funding.next_funding_rate.as_deref(), Some("0.000015"));
    assert_eq!(funding.next_funding_time, Some(1_700_007_200_000));
    assert_eq!(funding_items[0].funding_rate, "0.00001");
    assert_eq!(open_interest.open_interest, "1234.56");
    assert_eq!(balances[0].asset, "USDC");
    assert_eq!(balances[0].total, "1000");
    assert_eq!(balances[0].available, "740");
    assert_eq!(positions[0].exchange_symbol, "BTC");
    assert_eq!(positions[0].size, "0.01");
    assert_eq!(positions[0].margin_mode.as_deref(), Some("cross"));
    assert_eq!(bills.len(), 2);
    assert_eq!(bills[0].bill_id.as_deref(), Some("0x1"));
    assert_eq!(bills[0].bill_type.as_deref(), Some("deposit"));
    assert_eq!(bills[0].balance_change.as_deref(), Some("100.5"));
    assert_eq!(bills[1].bill_id.as_deref(), Some("0x2"));
    assert_eq!(bills[1].exchange_symbol.as_deref(), Some("BTC"));
    assert_eq!(bills[1].bill_type.as_deref(), Some("funding"));
    assert_eq!(bills[1].balance_change.as_deref(), Some("-0.12"));

    meta_and_asset_ctxs.assert_async().await;
    l2_book.assert_async().await;
    candles.assert_async().await;
    funding_history.assert_async().await;
    predicted_fundings.assert_async().await;
    clearinghouse_state.assert_async().await;
    spot_clearinghouse_state.assert_async().await;
    ledger.assert_async().await;
    user_funding.assert_async().await;
}

#[tokio::test]
async fn external_consumer_can_request_only_hyperliquid_funding_bills() {
    let mut server = Server::new_async().await;
    let user_funding = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"userFunding","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"time":1700000100000,"hash":"0x2","delta":{"type":"funding","coin":"BTC","usdc":"-0.12"}}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let bills = sdk
        .account(ExchangeId::Hyperliquid)
        .unwrap()
        .bills(
            AccountBillQuery::new()
                .with_bill_type("funding")
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(bills.len(), 1);
    assert_eq!(bills[0].bill_id.as_deref(), Some("0x2"));
    assert_eq!(bills[0].exchange_symbol.as_deref(), Some("BTC"));
    assert_eq!(bills[0].bill_type.as_deref(), Some("funding"));
    assert_eq!(bills[0].balance_change.as_deref(), Some("-0.12"));

    user_funding.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_hyperliquid_spot_info_api() {
    let mut server = Server::new_async().await;
    let spot_meta_and_asset_ctxs = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotMetaAndAssetCtxs"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[
                {
                    "tokens":[
                        {"name":"USDC","szDecimals":8,"weiDecimals":8,"index":0,"tokenId":"0x0","isCanonical":true},
                        {"name":"PURR","szDecimals":0,"weiDecimals":5,"index":1,"tokenId":"0x1","isCanonical":true}
                    ],
                    "universe":[{"name":"PURR/USDC","tokens":[1,0],"index":0,"isCanonical":true}]
                },
                [{"dayNtlVlm":"1000","markPx":"0.12","midPx":"0.121","prevDayPx":"0.11"}]
            ]"#,
        )
        .expect(2)
        .create_async()
        .await;
    let clearinghouse_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"clearinghouseState","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "marginSummary":{"accountValue":"1000","totalMarginUsed":"250"},
                "withdrawable":"740",
                "assetPositions":[]
            }"#,
        )
        .create_async()
        .await;
    let spot_clearinghouse_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"spotClearinghouseState","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"balances":[
                {"coin":"USDC","token":0,"hold":"0","total":"100.5","entryNtl":"0"},
                {"coin":"PURR","token":1,"hold":"2","total":"10","entryNtl":"1.2"}
            ]}"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let purr_spot = Instrument::spot("PURR", "USDC");

    let ticker = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .ticker(&purr_spot)
        .await
        .unwrap();
    let tickers = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .tickers("spot")
        .await
        .unwrap();
    let balances = sdk
        .account(ExchangeId::Hyperliquid)
        .unwrap()
        .balances()
        .await
        .unwrap();

    assert_eq!(purr_spot.symbol_for(ExchangeId::Hyperliquid), "PURR/USDC");
    assert_eq!(ticker.instrument_type.as_deref(), Some("spot"));
    assert_eq!(ticker.exchange_symbol, "PURR/USDC");
    assert_eq!(ticker.last_price, "0.121");
    assert_eq!(tickers[0].instrument, purr_spot);
    assert_eq!(balances[0].asset, "USDC");
    assert_eq!(balances[0].total, "1000");
    assert_eq!(balances[1].asset, "USDC");
    assert_eq!(balances[1].total, "100.5");
    assert_eq!(balances[1].available, "100.5");
    assert_eq!(balances[2].asset, "PURR");
    assert_eq!(balances[2].frozen.as_deref(), Some("2"));

    spot_meta_and_asset_ctxs.assert_async().await;
    clearinghouse_state.assert_async().await;
    spot_clearinghouse_state.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_hyperliquid_spot_index_coin_for_non_purr_books_and_candles() {
    let mut server = Server::new_async().await;
    let spot_meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"spotMeta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tokens":[
                    {"name":"USDC","index":0},
                    {"name":"HYPE","index":150}
                ],
                "universe":[{"name":"HYPE/USDC","tokens":[150,0],"index":107}]
            }"#,
        )
        .expect(2)
        .create_async()
        .await;
    let l2_book = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"l2Book","coin":"@107"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"coin":"@107","time":1730000000000,"levels":[[{"px":"40","sz":"1","n":1}],[{"px":"41","sz":"2","n":1}]]}"#,
        )
        .create_async()
        .await;
    let candles = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"candleSnapshot","req":{"coin":"@107","interval":"1m","startTime":1700000000000,"endTime":1700000060000}}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"t":1700000000000,"T":1700000060000,"s":"@107","i":"1m","o":"40","c":"41","h":"42","l":"39","v":"12","n":1}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let hype_spot = Instrument::spot("HYPE", "USDC");

    let book = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .orderbook(OrderBookQuery::new(hype_spot.clone()))
        .await
        .unwrap();
    let candle_items = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .candles(
            CandleQuery::new(hype_spot, "1m")
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_000_060_000),
        )
        .await
        .unwrap();

    assert_eq!(book.exchange_symbol, "@107");
    assert_eq!(book.bids[0].price, "40");
    assert_eq!(candle_items[0].exchange_symbol, "@107");
    assert_eq!(candle_items[0].close, "41");

    spot_meta.assert_async().await;
    l2_book.assert_async().await;
    candles.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_hyperliquid_spot_index_coin_for_non_purr_tickers() {
    let mut server = Server::new_async().await;
    let spot_meta_and_asset_ctxs = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotMetaAndAssetCtxs"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[
                {
                    "tokens":[
                        {"name":"USDC","szDecimals":8,"weiDecimals":8,"index":0,"tokenId":"0x0","isCanonical":true},
                        {"name":"HYPE","szDecimals":2,"weiDecimals":8,"index":150,"tokenId":"0x2","isCanonical":true}
                    ],
                    "universe":[{"name":"@107","tokens":[150,0],"index":107,"isCanonical":false}]
                },
                [{"dayNtlVlm":"2000","markPx":"40","midPx":"40.1","prevDayPx":"39"}]
            ]"#,
        )
        .expect(2)
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let hype_spot = Instrument::spot("HYPE", "USDC");

    let ticker = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .ticker(&hype_spot)
        .await
        .unwrap();
    let tickers = sdk
        .market(ExchangeId::Hyperliquid)
        .unwrap()
        .tickers("spot")
        .await
        .unwrap();

    assert_eq!(ticker.instrument, hype_spot);
    assert_eq!(ticker.instrument_type.as_deref(), Some("spot"));
    assert_eq!(ticker.exchange_symbol, "@107");
    assert_eq!(ticker.last_price, "40.1");
    assert_eq!(tickers[0].instrument, hype_spot);
    assert_eq!(tickers[0].exchange_symbol, "@107");

    spot_meta_and_asset_ctxs.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_hyperliquid_order_and_fill_reads() {
    let mut server = Server::new_async().await;
    let frontend_open_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"frontendOpenOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"BTC","side":"B","limitPx":"70000","sz":"0.01","oid":12345,"timestamp":1700000000100,"origSz":"0.02","orderType":"Limit","reduceOnly":true,"isPositionTpsl":true,"isTrigger":true,"triggerCondition":"Price above","triggerPx":"70500"}]"#,
        )
        .create_async()
        .await;
    let order_status = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"orderStatus","user":"{USER_ADDRESS}","oid":12345}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"status":"order","order":{"coin":"BTC","side":"B","limitPx":"70000","sz":"0.01","oid":12345,"timestamp":1700000000100,"origSz":"0.02"}}"#,
        )
        .create_async()
        .await;
    let historical_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"historicalOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"status":"filled","statusTimestamp":1700000000200,"order":{"coin":"BTC","side":"A","limitPx":"71000","sz":"0","oid":12346,"timestamp":1700000000000,"origSz":"0.03"}}]"#,
        )
        .create_async()
        .await;
    let fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"userFillsByTime","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"BTC","px":"70000","sz":"0.01","side":"B","time":1700000000300,"startPosition":"0","dir":"Open Long","closedPnl":"0","hash":"0xabc","oid":12345,"tid":98765,"fee":"0.01","feeToken":"USDC"}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let btc_perp = Instrument::perp("BTC", "USDC");

    let open = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .open(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(5))
        .await
        .unwrap();
    let detail = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .get(OrderQuery::by_order_id(btc_perp.clone(), "12345"))
        .await
        .unwrap();
    let history = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .history(OrderListQuery::for_instrument(btc_perp.clone()).with_limit(5))
        .await
        .unwrap();
    let fill_items = sdk
        .fills(ExchangeId::Hyperliquid)
        .unwrap()
        .list(
            FillListQuery::for_instrument(btc_perp)
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(open[0].exchange_symbol, "BTC");
    assert_eq!(open[0].order_id.as_deref(), Some("12345"));
    assert_eq!(open[0].side.as_deref(), Some("buy"));
    assert_eq!(open[0].order_type.as_deref(), Some("Limit"));
    assert_eq!(open[0].size.as_deref(), Some("0.02"));
    assert_eq!(open[0].filled_size.as_deref(), Some("0.01"));
    assert_eq!(open[0].raw["isPositionTpsl"], true);
    assert_eq!(open[0].raw["triggerPx"], "70500");
    assert_eq!(detail.order_id.as_deref(), Some("12345"));
    assert_eq!(detail.status.as_deref(), Some("order"));
    assert_eq!(history[0].order_id.as_deref(), Some("12346"));
    assert_eq!(history[0].status.as_deref(), Some("filled"));
    assert_eq!(fill_items[0].trade_id.as_deref(), Some("98765"));
    assert_eq!(fill_items[0].order_id.as_deref(), Some("12345"));
    assert_eq!(fill_items[0].fee_asset.as_deref(), Some("USDC"));

    frontend_open_orders.assert_async().await;
    order_status.assert_async().await;
    historical_orders.assert_async().await;
    fills.assert_async().await;
}

#[tokio::test]
async fn external_consumer_filters_hyperliquid_spot_orders_and_fills_by_spot_index_coin() {
    let mut server = Server::new_async().await;
    let spot_meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"spotMeta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tokens":[
                    {"name":"USDC","index":0},
                    {"name":"HYPE","index":150}
                ],
                "universe":[{"name":"HYPE/USDC","tokens":[150,0],"index":107}]
            }"#,
        )
        .expect(3)
        .create_async()
        .await;
    let frontend_open_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"frontendOpenOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"@107","side":"B","limitPx":"40","sz":"4","oid":111,"timestamp":1700000000100,"origSz":"4","orderType":"Limit"}]"#,
        )
        .create_async()
        .await;
    let historical_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"historicalOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"status":"filled","statusTimestamp":1700000000200,"order":{"coin":"@107","side":"A","limitPx":"41","sz":"0","oid":112,"timestamp":1700000000000,"origSz":"4"}}]"#,
        )
        .create_async()
        .await;
    let fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"userFillsByTime","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"@107","px":"40","sz":"4","side":"B","time":1700000000300,"hash":"0xspot","oid":112,"tid":98766,"fee":"0.01","feeToken":"USDC"}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let hype_spot = Instrument::spot("HYPE", "USDC");

    let open = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .open(OrderListQuery::for_instrument(hype_spot.clone()).with_limit(5))
        .await
        .unwrap();
    let history = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .history(OrderListQuery::for_instrument(hype_spot.clone()).with_limit(5))
        .await
        .unwrap();
    let fill_items = sdk
        .fills(ExchangeId::Hyperliquid)
        .unwrap()
        .list(
            FillListQuery::for_instrument(hype_spot.clone())
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(open[0].instrument, hype_spot);
    assert_eq!(open[0].exchange_symbol, "@107");
    assert_eq!(open[0].order_id.as_deref(), Some("111"));
    assert_eq!(history[0].instrument, hype_spot);
    assert_eq!(history[0].exchange_symbol, "@107");
    assert_eq!(history[0].order_id.as_deref(), Some("112"));
    assert_eq!(fill_items[0].instrument, hype_spot);
    assert_eq!(fill_items[0].exchange_symbol, "@107");
    assert_eq!(fill_items[0].trade_id.as_deref(), Some("98766"));

    spot_meta.assert_async().await;
    frontend_open_orders.assert_async().await;
    historical_orders.assert_async().await;
    fills.assert_async().await;
}

#[tokio::test]
async fn external_consumer_maps_hyperliquid_spot_orders_and_fills_without_instrument_filter() {
    let mut server = Server::new_async().await;
    let spot_meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"spotMeta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tokens":[
                    {"name":"USDC","index":0},
                    {"name":"HYPE","index":150}
                ],
                "universe":[{"name":"@107","tokens":[150,0],"index":107}]
            }"#,
        )
        .expect(3)
        .create_async()
        .await;
    let frontend_open_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"frontendOpenOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"@107","side":"B","limitPx":"40","sz":"4","oid":111,"timestamp":1700000000100,"origSz":"4","orderType":"Limit"}]"#,
        )
        .create_async()
        .await;
    let historical_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"historicalOrders","user":"{USER_ADDRESS}"}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"status":"filled","statusTimestamp":1700000000200,"order":{"coin":"@107","side":"A","limitPx":"41","sz":"0","oid":112,"timestamp":1700000000000,"origSz":"4"}}]"#,
        )
        .create_async()
        .await;
    let fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(format!(
            r#"{{"type":"userFillsByTime","user":"{USER_ADDRESS}","startTime":1700000000000,"endTime":1700007200000}}"#
        )))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"coin":"@107","px":"40","sz":"4","side":"B","time":1700000000300,"hash":"0xspot","oid":112,"tid":98766,"fee":"0.01","feeToken":"USDC"}]"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        hyperliquid: Some(HyperliquidExchangeConfig {
            api_url: Some(server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            user_address: Some(USER_ADDRESS.to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();
    let hype_spot = Instrument::spot("HYPE", "USDC");

    let open = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .open(OrderListQuery::new().with_limit(5))
        .await
        .unwrap();
    let history = sdk
        .orders(ExchangeId::Hyperliquid)
        .unwrap()
        .history(OrderListQuery::new().with_limit(5))
        .await
        .unwrap();
    let fill_items = sdk
        .fills(ExchangeId::Hyperliquid)
        .unwrap()
        .list(
            FillListQuery::new()
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(open[0].instrument, hype_spot);
    assert_eq!(open[0].exchange_symbol, "@107");
    assert_eq!(history[0].instrument, hype_spot);
    assert_eq!(history[0].exchange_symbol, "@107");
    assert_eq!(fill_items[0].instrument, hype_spot);
    assert_eq!(fill_items[0].exchange_symbol, "@107");

    spot_meta.assert_async().await;
    frontend_open_orders.assert_async().await;
    historical_orders.assert_async().await;
    fills.assert_async().await;
}
