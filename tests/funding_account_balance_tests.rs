use crypto_exc_all::{
    BinanceExchangeConfig, CryptoSdk, Error, ExchangeId, OkxExchangeConfig, SdkConfig,
};
use mockito::{Matcher, Server};

#[tokio::test]
async fn okx_reads_the_requested_funding_account_asset() {
    let mut server = Server::new_async().await;
    let balance = server
        .mock("GET", "/api/v5/asset/balances")
        .match_header("ok-access-key", "okx-key")
        .match_query(Matcher::UrlEncoded("ccy".into(), "USDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"0","msg":"","data":[{"ccy":"USDT","bal":"15","frozenBal":"0","availBal":"15"}]}"#,
        )
        .create_async()
        .await;
    let sdk = okx_sdk(server.url());

    let balances = sdk
        .account(ExchangeId::Okx)
        .expect("OKX account facade")
        .funding_account_balances(Some("USDT"))
        .await
        .expect("funding account balance");

    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].asset, "USDT");
    assert_eq!(balances[0].total, "15");
    assert_eq!(balances[0].available, "15");
    balance.assert_async().await;
}

#[tokio::test]
async fn binance_does_not_inherit_okx_funding_account_semantics() {
    let sdk = CryptoSdk::from_config(SdkConfig {
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_owned(),
            api_secret: "binance-secret".to_owned(),
            api_url: Some("http://127.0.0.1:1".to_owned()),
            sapi_api_url: Some("http://127.0.0.1:1".to_owned()),
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(100),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        ..SdkConfig::default()
    })
    .expect("Binance SDK");

    let error = sdk
        .account(ExchangeId::Binance)
        .expect("Binance account facade")
        .funding_account_balances(Some("USDT"))
        .await
        .expect_err("Binance funding account semantics are not equivalent");

    assert!(matches!(
        error,
        Error::Unsupported {
            exchange: ExchangeId::Binance,
            capability: "funding account balances",
        }
    ));
}

fn okx_sdk(api_url: String) -> CryptoSdk {
    CryptoSdk::from_config(SdkConfig {
        okx: Some(OkxExchangeConfig {
            api_key: "okx-key".to_owned(),
            api_secret: "okx-secret".to_owned(),
            passphrase: "okx-passphrase".to_owned(),
            simulated: false,
            api_url: Some(api_url),
            request_expiration_ms: Some(1_000),
        }),
        ..SdkConfig::default()
    })
    .expect("OKX SDK")
}
