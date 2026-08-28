#![cfg(feature = "binance-public-kline")]

use crypto_exc_all::{
    BinancePublicFailureKind, BinancePublicMarketSdkError, BinanceUsdmPublicKlineClient,
    BinanceUsdmPublicKlineConfig, BinanceUsdmPublicKlineQuery, BinanceUsdmPublicMarkPriceClient,
    BinanceUsdmPublicMarkPriceConfig, BinanceWireDecimal,
};
use mockito::{Matcher, Server};

/// 根层最小 feature 只暴露匿名 typed K 线门面，并保留调用方选择的时间窗口。
#[tokio::test]
async fn binance_root_kline_facade_is_anonymous_typed_and_window_preserving() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "5m".into()),
            Matcher::UrlEncoded("startTime".into(), "1000".into()),
            Matcher::UrlEncoded("endTime".into(), "2000".into()),
            Matcher::UrlEncoded("limit".into(), "3".into()),
        ]))
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[1000,"1","2","0.5","1.5","10",1999,"15",4,"6","9","0"]]"#)
        .create_async()
        .await;
    let client = BinanceUsdmPublicKlineClient::new(BinanceUsdmPublicKlineConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public Kline facade");

    let response = client
        .klines(
            BinanceUsdmPublicKlineQuery::new("ETHUSDT", "5m")
                .with_start_time(1000)
                .with_end_time(2000)
                .with_limit(3),
        )
        .await
        .expect("typed Kline response");

    request.assert_async().await;
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].open_time, 1000);
    assert_eq!(response.data[0].close_time, 1999);
}

/// Market 后继必须能结构化读取 provider 限频错误，而不是解析 Display 字符串。
#[tokio::test]
async fn binance_root_kline_facade_exposes_typed_failure_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
        ]))
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "4")
        .with_body(r#"{"code":-1003,"msg":"rate limited"}"#)
        .create_async()
        .await;
    let client = BinanceUsdmPublicKlineClient::new(BinanceUsdmPublicKlineConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public Kline facade");

    let error = client
        .klines(BinanceUsdmPublicKlineQuery::new("BTCUSDT", "1m"))
        .await
        .expect_err("typed failure");

    request.assert_async().await;
    let BinancePublicMarketSdkError::BinancePublicRequestFailed { failure } = error else {
        panic!("expected Binance typed public failure");
    };
    assert_eq!(failure.kind, BinancePublicFailureKind::Provider);
    assert_eq!(failure.provider_code, Some(-1003));
    assert_eq!(
        failure
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.retry_after.as_deref()),
        Some("4")
    );
}

#[tokio::test]
async fn binance_mark_price_is_anonymous_typed_and_identity_exact() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/premiumIndex")
        .match_query(Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()))
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"symbol":"ETHUSDT","markPrice":"1892.56","indexPrice":"1892.10","time":1786522657867}"#,
        )
        .create_async()
        .await;
    let client = BinanceUsdmPublicMarkPriceClient::new(BinanceUsdmPublicMarkPriceConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public mark-price facade");

    let response = client
        .mark_price("ETHUSDT")
        .await
        .expect("typed mark-price response");

    request.assert_async().await;
    assert_eq!(response.data.symbol, "ETHUSDT");
    assert_eq!(response.data.time, 1786522657867);
    assert_eq!(
        response.data.mark_price,
        BinanceWireDecimal::Text("1892.56".to_owned())
    );
}

#[tokio::test]
async fn binance_mark_price_rejects_provider_symbol_drift() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/premiumIndex")
        .match_query(Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","markPrice":"60000","time":1786522657867}"#)
        .create_async()
        .await;
    let client = BinanceUsdmPublicMarkPriceClient::new(BinanceUsdmPublicMarkPriceConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public mark-price facade");

    let error = client
        .mark_price("ETHUSDT")
        .await
        .expect_err("provider identity drift must fail closed");

    request.assert_async().await;
    assert!(error.to_string().contains("identity"));
}

#[tokio::test]
async fn binance_mark_price_rejects_missing_provider_timestamp() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/premiumIndex")
        .match_query(Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"ETHUSDT","markPrice":"1892.56"}"#)
        .create_async()
        .await;
    let client = BinanceUsdmPublicMarkPriceClient::new(BinanceUsdmPublicMarkPriceConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public mark-price facade");

    let error = client
        .mark_price("ETHUSDT")
        .await
        .expect_err("missing provider time must fail closed");

    request.assert_async().await;
    let BinancePublicMarketSdkError::BinancePublicRequestFailed { failure } = error else {
        panic!("expected typed decode failure");
    };
    assert_eq!(failure.kind, BinancePublicFailureKind::Decode);
}
