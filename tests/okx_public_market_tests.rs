#![cfg(feature = "okx-public-market")]

use crypto_exc_all::{
    OkxCandleDataset, OkxPublicCandleQuery, OkxPublicFailureKind, OkxPublicMarketClient,
    OkxPublicMarketConfig, OkxPublicMarketSdkError,
};
use mockito::{Matcher, Server};

/// 公共历史 K 线必须走 SDK endpoint、保留双边界和九字段，同时不发送账户认证头。
#[tokio::test]
async fn public_history_candles_are_anonymous_and_lossless() {
    let mut server = Server::new_async().await;
    let request = server
        .mock(
            "GET",
            "/api/v5/market/history-candles?instId=BTC-USDT-SWAP&bar=1Dutc&after=1700172800000&before=1700000000000&limit=300",
        )
        .match_header("OK-ACCESS-KEY", Matcher::Missing)
        .match_header("OK-ACCESS-SIGN", Matcher::Missing)
        .match_header("OK-ACCESS-PASSPHRASE", Matcher::Missing)
        .match_header("x-simulated-trading", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "2")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[[
                    "1700086400000",
                    "37000.1",
                    "38000.2",
                    "36000.3",
                    "37500.4",
                    "123",
                    "1.23",
                    "45678.90",
                    "1"
                ]]
            }"#,
        )
        .create_async()
        .await;
    let client = OkxPublicMarketClient::new(OkxPublicMarketConfig {
        api_url: Some(server.url()),
    })
    .expect("public client");
    let query = OkxPublicCandleQuery::new("BTC-USDT-SWAP", "1Dutc")
        .with_after("1700172800000")
        .with_before("1700000000000")
        .with_limit(300);

    let response = client
        .candles(OkxCandleDataset::History, query)
        .await
        .expect("history candles");

    request.assert_async().await;
    let candles = response.data;
    assert_eq!(candles.len(), 1);
    assert_eq!(candles[0].timestamp, "1700086400000");
    assert_eq!(candles[0].volume, "123");
    assert_eq!(candles[0].volume_currency, "1.23");
    assert_eq!(candles[0].quote_volume, "45678.90");
    assert_eq!(candles[0].confirm, "1");
    assert_eq!(response.evidence.http_status, 200);
    assert_eq!(response.evidence.okx_code, "0");
    assert_eq!(response.evidence.retry_after.as_deref(), Some("2"));
}

/// provider 短行必须作为 SDK 解析错误返回，不能 panic 或丢弃。
#[tokio::test]
async fn malformed_public_candle_row_fails_closed() {
    let mut server = Server::new_async().await;
    let response = server
        .mock(
            "GET",
            "/api/v5/market/candles?instId=BTC-USDT&bar=1m&limit=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[["1700000000000"]]}"#)
        .create_async()
        .await;
    let client = OkxPublicMarketClient::new(OkxPublicMarketConfig {
        api_url: Some(server.url()),
    })
    .expect("public client");
    let query = OkxPublicCandleQuery::new("BTC-USDT", "1m").with_limit(1);

    let error = client
        .candles(OkxCandleDataset::Recent, query)
        .await
        .expect_err("short row must fail");

    response.assert_async().await;
    assert!(error.to_string().contains("期望至少 9 个字段"));
}

/// K 线失败必须保留 HTTP、OKX code 与 Retry-After，不能退化成字符串错误。
#[tokio::test]
async fn public_candle_failure_keeps_typed_retry_after_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock(
            "GET",
            "/api/v5/market/candles?instId=BTC-USDT-SWAP&bar=1m&limit=1",
        )
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "3")
        .with_body(r#"{"code":"50011","msg":"rate limited","data":[]}"#)
        .create_async()
        .await;
    let client = OkxPublicMarketClient::new(OkxPublicMarketConfig {
        api_url: Some(server.url()),
    })
    .expect("public client");

    let error = client
        .candles(
            OkxCandleDataset::Recent,
            OkxPublicCandleQuery::new("BTC-USDT-SWAP", "1m").with_limit(1),
        )
        .await
        .expect_err("rate limit must remain typed");

    request.assert_async().await;
    let OkxPublicMarketSdkError::PublicApiError(evidence) = error else {
        panic!("expected typed OKX public failure");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::HttpStatus);
    assert_eq!(evidence.http_status, 429);
    assert_eq!(evidence.okx_code.as_deref(), Some("50011"));
    assert_eq!(evidence.retry_after.as_deref(), Some("3"));
}
