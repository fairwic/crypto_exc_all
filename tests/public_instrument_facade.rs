#![cfg(all(feature = "binance-public-instrument", feature = "okx-public-market"))]

use crypto_exc_all::{
    BinanceUsdmPublicInstrumentClient, BinanceUsdmPublicInstrumentConfig,
    OkxSwapPublicInstrumentClient, OkxSwapPublicInstrumentConfig,
};
use mockito::{Matcher, Server};

/// 根门面必须保持 Binance endpoint 无 query、无账户认证，并返回 typed provider DTO。
#[tokio::test]
async fn binance_root_facade_is_anonymous_and_typed() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .match_query(Matcher::Missing)
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "timezone":"UTC",
                "serverTime":1,
                "rateLimits":[],
                "exchangeFilters":[],
                "assets":[],
                "symbols":[]
            }"#,
        )
        .create_async()
        .await;
    let client = BinanceUsdmPublicInstrumentClient::new(BinanceUsdmPublicInstrumentConfig {
        api_url: Some(server.url()),
    })
    .expect("Binance public facade");

    let response = client.exchange_info().await.expect("typed exchangeInfo");

    request.assert_async().await;
    assert_eq!(response.data.timezone, "UTC");
    assert!(response.data.symbols.is_empty());
}

/// 根门面必须固定 OKX SWAP query，并阻止账户认证头进入公共请求。
#[tokio::test]
async fn okx_root_facade_is_anonymous_and_swap_scoped() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .match_header("OK-ACCESS-KEY", Matcher::Missing)
        .match_header("OK-ACCESS-SIGN", Matcher::Missing)
        .match_header("OK-ACCESS-PASSPHRASE", Matcher::Missing)
        .match_header("x-simulated-trading", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[]}"#)
        .create_async()
        .await;
    let client = OkxSwapPublicInstrumentClient::new(OkxSwapPublicInstrumentConfig {
        api_url: Some(server.url()),
    })
    .expect("OKX public facade");

    let response = client.instruments().await.expect("typed SWAP instruments");

    request.assert_async().await;
    assert!(response.data.is_empty());
    assert_eq!(response.evidence.okx_code, "0");
}
