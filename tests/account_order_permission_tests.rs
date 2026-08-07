use crypto_exc_all::{BinanceExchangeConfig, CryptoSdk, ExchangeId, OkxExchangeConfig, SdkConfig};
use mockito::{Matcher, Server};

/// Binance `canTrade` 是 provider 协议事实；SDK 不把它提升为业务准入结论。
#[tokio::test]
async fn binance_maps_signed_account_config_to_order_permission() {
    let mut server = Server::new_async().await;
    let account_config = server
        .mock("GET", "/fapi/v1/accountConfig")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"canTrade":true}"#)
        .create_async()
        .await;
    let sdk = binance_sdk(server.url());

    let permission = sdk
        .account(ExchangeId::Binance)
        .expect("binance account facade")
        .order_permission()
        .await
        .expect("signed account permission");

    assert_eq!(permission.exchange, ExchangeId::Binance);
    assert!(permission.can_create_orders);
    assert_eq!(permission.source_revision, "binance-usdm-account-config-v1");
    account_config.assert_async().await;
}

/// 缺失 `canTrade` 不能默认为允许，避免 provider 合同漂移时放开订单能力。
#[tokio::test]
async fn binance_rejects_account_config_without_can_trade() {
    let mut server = Server::new_async().await;
    let account_config = server
        .mock("GET", "/fapi/v1/accountConfig")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dualSidePosition":false}"#)
        .create_async()
        .await;
    let sdk = binance_sdk(server.url());

    let error = sdk
        .account(ExchangeId::Binance)
        .expect("binance account facade")
        .order_permission()
        .await
        .expect_err("missing canTrade must fail closed");

    assert!(error.to_string().contains("canTrade"));
    account_config.assert_async().await;
}

/// OKX 只接受逗号分隔权限中的精确 `trade` token，不做子串匹配。
#[tokio::test]
async fn okx_maps_exact_trade_token_to_order_permission() {
    let mut server = Server::new_async().await;
    let account_config = server
        .mock("GET", "/api/v5/account/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"perm":"read_only,trade"}]}"#)
        .create_async()
        .await;
    let sdk = okx_sdk(server.url());

    let permission = sdk
        .account(ExchangeId::Okx)
        .expect("okx account facade")
        .order_permission()
        .await
        .expect("signed account permission");

    assert_eq!(permission.exchange, ExchangeId::Okx);
    assert!(permission.can_create_orders);
    assert_eq!(permission.source_revision, "okx-account-config-v1");
    account_config.assert_async().await;
}

/// 只读 key 必须映射为禁止新建订单，而不是把 signed 请求成功误当成可交易。
#[tokio::test]
async fn okx_read_only_permission_denies_order_creation() {
    let mut server = Server::new_async().await;
    let account_config = server
        .mock("GET", "/api/v5/account/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"perm":"read_only"}]}"#)
        .create_async()
        .await;
    let sdk = okx_sdk(server.url());

    let permission = sdk
        .account(ExchangeId::Okx)
        .expect("okx account facade")
        .order_permission()
        .await
        .expect("read-only is a valid denied fact");

    assert!(!permission.can_create_orders);
    account_config.assert_async().await;
}

fn binance_sdk(api_url: String) -> CryptoSdk {
    CryptoSdk::from_config(SdkConfig {
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_owned(),
            api_secret: "binance-secret".to_owned(),
            api_url: Some(api_url),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: None,
            recv_window_ms: None,
            proxy_url: None,
        }),
        ..SdkConfig::default()
    })
    .expect("binance SDK")
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
    .expect("okx SDK")
}
