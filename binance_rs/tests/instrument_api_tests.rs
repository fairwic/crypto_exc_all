use binance_rs::Error;
use binance_rs::api::market::BinanceMarket;
use binance_rs::client::{BinanceClient, BinancePublicFailureKind, BinancePublicRequestFailure};
use binance_rs::dto::market::BinanceWireDecimal;
use mockito::{Matcher, Server};

fn public_market(server_url: String) -> BinanceMarket {
    let mut client = BinanceClient::new_public().expect("public client should build");
    client.set_base_url(server_url);
    BinanceMarket::new(client)
}

fn public_failure(error: Error) -> BinancePublicRequestFailure {
    match error {
        Error::BinancePublicRequestFailed { failure } => *failure,
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn typed_exchange_info_preserves_wire_fields_unknowns_and_http_evidence() {
    let mut server = Server::new_async().await;
    let exchange_info = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .match_query(Matcher::Missing)
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("x-mbx-used-weight-1m", "7")
        .with_header("x-mbx-order-count-10s", "3")
        .with_header("retry-after", "9")
        .with_body(
            r#"{
                "timezone": "UTC",
                "serverTime": 1565613908500,
                "rateLimits": [{
                    "rateLimitType": "REQUEST_WEIGHT",
                    "interval": "MINUTE",
                    "intervalNum": 1,
                    "limit": 2400,
                    "quotaScope": "IP"
                }],
                "exchangeFilters": [],
                "assets": [{
                    "asset": "USDT",
                    "marginAvailable": true,
                    "autoAssetExchange": "-10000",
                    "portfolioMarginOnly": false
                }],
                "symbols": [{
                    "symbol": "BTCUSDT",
                    "pair": "BTCUSDT",
                    "contractType": "PERPETUAL",
                    "deliveryDate": 4133404800000,
                    "onboardDate": 1569398400000,
                    "status": "FUTURE_PROVIDER_STATUS",
                    "maintMarginPercent": "2.5000",
                    "requiredMarginPercent": "5.0000",
                    "baseAsset": "BTC",
                    "quoteAsset": "USDT",
                    "marginAsset": "USDT",
                    "pricePrecision": 2,
                    "quantityPrecision": 3,
                    "baseAssetPrecision": 8,
                    "quotePrecision": 8,
                    "underlyingType": "COIN",
                    "underlyingSubType": ["PoW"],
                    "settlePlan": 0,
                    "triggerProtect": 1e-8,
                    "filters": [
                        {
                            "filterType": "PRICE_FILTER",
                            "minPrice": "0.01",
                            "maxPrice": "1000000",
                            "tickSize": 5e-8,
                            "futureStepMode": "strict"
                        },
                        {
                            "filterType": "LOT_SIZE",
                            "minQty": "0.001",
                            "maxQty": "1000",
                            "stepSize": "0.001"
                        },
                        {
                            "filterType": "MIN_NOTIONAL",
                            "notional": "5"
                        },
                        {
                            "filterType": "FUTURE_FILTER",
                            "mystery": 9e-12
                        }
                    ],
                    "orderTypes": ["LIMIT", "MARKET"],
                    "timeInForce": ["GTC", "IOC"],
                    "liquidationFee": "1e-8",
                    "marketTakeBound": "0.30",
                    "newLifecycleField": "alpha"
                }],
                "futureTopLevel": {"version": 1}
            }"#,
        )
        .create_async()
        .await;

    let response = public_market(server.url())
        .get_exchange_info_typed()
        .await
        .expect("typed exchangeInfo should decode");

    assert_eq!(response.evidence.http_status, 200);
    assert_eq!(
        response
            .evidence
            .used_weight_headers
            .get("x-mbx-used-weight-1m")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        response
            .evidence
            .order_count_headers
            .get("x-mbx-order-count-10s")
            .map(String::as_str),
        Some("3")
    );
    assert_eq!(response.evidence.retry_after.as_deref(), Some("9"));
    assert_eq!(response.data.timezone, "UTC");
    assert!(response.data.extra.contains_key("futureTopLevel"));
    assert!(
        response.data.rate_limits[0]
            .extra
            .contains_key("quotaScope")
    );
    assert!(
        response.data.assets[0]
            .extra
            .contains_key("portfolioMarginOnly")
    );

    let symbol = &response.data.symbols[0];
    assert_eq!(symbol.status, "FUTURE_PROVIDER_STATUS");
    assert!(symbol.extra.contains_key("newLifecycleField"));
    match &symbol.trigger_protect {
        BinanceWireDecimal::Number(value) => assert_eq!(value.to_string(), "1e-8"),
        other => panic!("expected numeric wire decimal, got {other:?}"),
    }
    match symbol.filters[0]
        .tick_size
        .as_ref()
        .expect("tick size should be present")
    {
        BinanceWireDecimal::Number(value) => assert_eq!(value.to_string(), "5e-8"),
        other => panic!("expected numeric tick size, got {other:?}"),
    }
    assert!(symbol.filters[0].extra.contains_key("futureStepMode"));
    assert_eq!(symbol.filters[3].filter_type, "FUTURE_FILTER");
    assert!(symbol.filters[3].extra.contains_key("mystery"));
    assert_eq!(
        symbol.liquidation_fee,
        BinanceWireDecimal::Text("1e-8".to_owned())
    );

    exchange_info.assert_async().await;
}

#[tokio::test]
async fn malformed_required_symbol_field_fails_the_whole_typed_response() {
    let mut server = Server::new_async().await;
    let exchange_info = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .match_query(Matcher::Missing)
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "timezone": "UTC",
                "serverTime": 1565613908500,
                "rateLimits": [],
                "exchangeFilters": [],
                "assets": [],
                "symbols": [{"pair": "BTCUSDT"}]
            }"#,
        )
        .create_async()
        .await;

    let failure = public_failure(
        public_market(server.url())
            .get_exchange_info_typed()
            .await
            .expect_err("missing required symbol must fail closed"),
    );

    assert_eq!(failure.kind, BinancePublicFailureKind::Decode);
    assert_eq!(
        failure
            .evidence
            .as_ref()
            .map(|evidence| evidence.http_status),
        Some(200)
    );
    assert!(failure.message.contains("missing field"));
    exchange_info.assert_async().await;
}

#[tokio::test]
async fn typed_exchange_info_preserves_binance_provider_error() {
    let mut server = Server::new_async().await;
    let exchange_info = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .match_query(Matcher::Missing)
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":-1121,"msg":"Invalid symbol.","requestId":"req-1"}"#)
        .create_async()
        .await;

    let failure = public_failure(
        public_market(server.url())
            .get_exchange_info_typed()
            .await
            .expect_err("provider error should remain structured"),
    );

    assert_eq!(failure.kind, BinancePublicFailureKind::Provider);
    assert_eq!(failure.provider_code, Some(-1121));
    assert_eq!(failure.message, "Invalid symbol.");
    assert_eq!(
        failure
            .evidence
            .as_ref()
            .map(|evidence| evidence.http_status),
        Some(400)
    );
    assert!(failure.provider_extra.contains_key("requestId"));
    exchange_info.assert_async().await;
}

#[tokio::test]
async fn typed_exchange_info_preserves_429_quota_headers_without_retrying() {
    let mut server = Server::new_async().await;
    let exchange_info = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .expect(1)
        .match_query(Matcher::Missing)
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("x-mbx-used-weight-1m", "2400")
        .with_header("x-mbx-order-count-10s", "100")
        .with_header("retry-after", "2")
        .with_body(r#"{"code":-1003,"msg":"Too many requests."}"#)
        .create_async()
        .await;

    let failure = public_failure(
        public_market(server.url())
            .get_exchange_info_typed()
            .await
            .expect_err("HTTP 429 should remain a provider failure"),
    );
    let evidence = failure
        .evidence
        .expect("HTTP response should carry quota evidence");

    assert_eq!(failure.kind, BinancePublicFailureKind::Provider);
    assert_eq!(failure.provider_code, Some(-1003));
    assert_eq!(evidence.http_status, 429);
    assert_eq!(
        evidence
            .used_weight_headers
            .get("x-mbx-used-weight-1m")
            .map(String::as_str),
        Some("2400")
    );
    assert_eq!(
        evidence
            .order_count_headers
            .get("x-mbx-order-count-10s")
            .map(String::as_str),
        Some("100")
    );
    assert_eq!(evidence.retry_after.as_deref(), Some("2"));
    exchange_info.assert_async().await;
}
