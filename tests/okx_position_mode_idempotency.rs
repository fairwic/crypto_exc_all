use crypto_exc_all::{
    CryptoSdk, ExchangeId, OkxExchangeConfig, PositionMode, SdkConfig, SetPositionModeRequest,
};
use mockito::Server;

#[tokio::test]
async fn okx_set_position_mode_skips_mutation_when_mode_already_matches() {
    let mut okx_server = Server::new_async().await;
    let okx_config = okx_server
        .mock("GET", "/api/v5/account/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"posMode":"long_short_mode"}]}"#)
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
        ..SdkConfig::default()
    })
    .unwrap();

    let result = sdk
        .account(ExchangeId::Okx)
        .unwrap()
        .set_position_mode(SetPositionModeRequest::new(PositionMode::Hedge))
        .await
        .unwrap();

    assert_eq!(result.mode, PositionMode::Hedge);
    assert_eq!(result.raw_mode.as_deref(), Some("long_short_mode"));
    assert_eq!(result.raw["idempotent"], true);
    assert_eq!(result.raw["source"], "account/config");
    okx_config.assert_async().await;
}
